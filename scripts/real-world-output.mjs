#!/usr/bin/env node

import assert from 'node:assert/strict';
import vm from 'node:vm';
import { instrument } from '../crates/oxc-coverage-instrument-napi/index.js';

const normalize = (value) => JSON.parse(JSON.stringify(value));

const matchingIds = (entries, predicate) =>
  Object.entries(entries)
    .filter(([, entry]) => predicate(entry))
    .map(([id]) => id)
    .sort((left, right) => Number(left) - Number(right));

const statementCount = (coverageMap, coverage, line, context) => {
  const ids = matchingIds(coverageMap.statementMap, (entry) => entry.start.line === line);
  assert.equal(ids.length, 1, `${context}: statement at line ${line}`);
  return coverage.s[ids[0]];
};

const functionCount = (coverageMap, coverage, predicate, context) => {
  const ids = matchingIds(coverageMap.fnMap, predicate);
  assert.equal(ids.length, 1, `${context}: function counter`);
  return coverage.f[ids[0]];
};

const branchCounts = (coverageMap, coverage, type, line) => {
  const ids = matchingIds(
    coverageMap.branchMap,
    (entry) => entry.type === type && entry.loc.start.line === line,
  );
  return ids.map((id) => normalize(coverage.b[id]));
};

const cases = [
  {
    name: 'plain JavaScript control flow',
    filename: 'runtime/control-flow.js',
    source: `function classify(value) {
  if (value > 0) {
    return 'positive';
  }
  return 'non-positive';
}
globalThis.results = [classify(3), classify(-2)];`,
    verify: ({ coverageMap, coverage, sandbox }) => {
      assert.deepEqual(normalize(sandbox.results), ['positive', 'non-positive']);
      assert.equal(statementCount(coverageMap, coverage, 3, 'control flow'), 1);
      assert.equal(statementCount(coverageMap, coverage, 5, 'control flow'), 1);
      assert.equal(
        functionCount(coverageMap, coverage, (entry) => entry.name === 'classify', 'control flow'),
        2,
      );
      assert.deepEqual(branchCounts(coverageMap, coverage, 'if', 2), [[1, 1]]);
    },
  },
  {
    name: 'optional chaining tracked',
    filename: 'runtime/optional-chain-on.js',
    source: `function readName(user) {
  return user?.profile?.name ?? 'missing';
}
globalThis.values = [readName({ profile: { name: 'Ada' } }), readName(null)];`,
    verify: ({ coverageMap, coverage, sandbox }) => {
      assert.deepEqual(normalize(sandbox.values), ['Ada', 'missing']);
      assert.equal(statementCount(coverageMap, coverage, 2, 'optional chain on'), 2);
      assert.equal(
        functionCount(coverageMap, coverage, (entry) => entry.name === 'readName', 'optional chain on'),
        2,
      );
      assert.deepEqual(branchCounts(coverageMap, coverage, 'binary-expr', 2), [[2, 1]]);
      assert.deepEqual(branchCounts(coverageMap, coverage, 'optional-chain', 2), [
        [1, 1],
        [1, 1],
      ]);
    },
  },
  {
    name: 'optional chaining untracked',
    filename: 'runtime/optional-chain-off.js',
    options: { trackOptionalChainBranches: false },
    source: `function readName(user) {
  return user?.profile?.name ?? 'missing';
}
globalThis.values = [readName({ profile: { name: 'Ada' } }), readName(null)];`,
    verify: ({ coverageMap, coverage, result, sandbox }) => {
      assert.deepEqual(normalize(sandbox.values), ['Ada', 'missing']);
      assert.equal(statementCount(coverageMap, coverage, 2, 'optional chain off'), 2);
      assert.equal(
        functionCount(coverageMap, coverage, (entry) => entry.name === 'readName', 'optional chain off'),
        2,
      );
      assert.deepEqual(branchCounts(coverageMap, coverage, 'binary-expr', 2), [[2, 1]]);
      assert.deepEqual(branchCounts(coverageMap, coverage, 'optional-chain', 2), []);
      assert.equal(result.code.includes('_oc('), false, 'optional chain off: helper emission');
    },
  },
  {
    name: 'stripped TypeScript service',
    filename: 'runtime/user-service.ts',
    options: { stripTypescript: true },
    source: `interface User { id: string; name: string; }
class UserService {
  private users: Map<string, User> = new Map();
  add(user: User): void {
    this.users.set(user.id, user);
  }
  find(id: string): User | null {
    return this.users.get(id) ?? null;
  }
}
const service = new UserService();
service.add({ id: '1', name: 'Ada' });
globalThis.serviceResults = [service.find('1')?.name, service.find('2')];`,
    verify: ({ coverageMap, coverage, result, sandbox }) => {
      assert.deepEqual(normalize(sandbox.serviceResults), ['Ada', null]);
      assert.equal(result.code.includes('interface User'), false, 'TypeScript interface was stripped');
      assert.equal(statementCount(coverageMap, coverage, 5, 'TypeScript service'), 1);
      assert.equal(statementCount(coverageMap, coverage, 8, 'TypeScript service'), 2);
      assert.equal(
        functionCount(coverageMap, coverage, (entry) => entry.name === 'add', 'TypeScript service'),
        1,
      );
      assert.equal(
        functionCount(coverageMap, coverage, (entry) => entry.name === 'find', 'TypeScript service'),
        2,
      );
      assert.deepEqual(branchCounts(coverageMap, coverage, 'binary-expr', 8), [[2, 1]]);
      assert.deepEqual(branchCounts(coverageMap, coverage, 'optional-chain', 13), [[0, 1]]);
    },
  },
  {
    name: 'async function paths',
    filename: 'runtime/async.js',
    source: `async function load(flag) {
  if (flag) {
    return await Promise.resolve('ready');
  }
  return 'skipped';
}
globalThis.completion = Promise.all([load(true), load(false)]).then(values => {
  globalThis.asyncResults = values;
});`,
    verify: ({ coverageMap, coverage, sandbox }) => {
      assert.deepEqual(normalize(sandbox.asyncResults), ['ready', 'skipped']);
      assert.equal(statementCount(coverageMap, coverage, 3, 'async paths'), 1);
      assert.equal(statementCount(coverageMap, coverage, 5, 'async paths'), 1);
      assert.equal(
        functionCount(coverageMap, coverage, (entry) => entry.name === 'load', 'async paths'),
        2,
      );
      assert.equal(
        functionCount(coverageMap, coverage, (entry) => entry.decl.start.line === 7, 'async paths'),
        1,
      );
      assert.deepEqual(branchCounts(coverageMap, coverage, 'if', 2), [[1, 1]]);
    },
  },
];

const runCase = async (spec) => {
  const result = instrument(spec.source, spec.filename, spec.options);
  const coverageMap = JSON.parse(result.coverageMap);
  const sandbox = { console };
  sandbox.globalThis = sandbox;

  vm.runInNewContext(result.code, sandbox, { filename: spec.filename });
  if (sandbox.completion) {
    await sandbox.completion;
  }

  const coverage = sandbox.__coverage__?.[spec.filename];
  assert(coverage, `${spec.name}: runtime coverage object`);
  spec.verify({ coverageMap, coverage, result, sandbox });
  console.log(`[OK] ${spec.name}`);
};

for (const spec of cases) {
  await runCase(spec);
}

console.log('PASS: production-like emitted output preserved behavior and counter placement.');
