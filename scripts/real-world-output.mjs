#!/usr/bin/env node

import assert from 'node:assert/strict';
import vm from 'node:vm';
import { instrument } from '../crates/oxc_coverage_instrument_napi/index.js';

const normalize = (value) => JSON.parse(JSON.stringify(value));

const location = (startLine, startColumn, endLine, endColumn) => ({
  start: { line: startLine, column: startColumn },
  end: { line: endLine, column: endColumn },
});

const matchingIds = (entries, predicate) =>
  Object.entries(entries)
    .filter(([, entry]) => predicate(entry))
    .map(([id]) => id)
    .sort((left, right) => Number(left) - Number(right));

const locationsEqual = (actual, expected) =>
  actual.start.line === expected.start.line &&
  actual.start.column === expected.start.column &&
  actual.end.line === expected.end.line &&
  actual.end.column === expected.end.column;

const matchingId = (entries, predicate, context) => {
  const ids = matchingIds(entries, predicate);
  assert.equal(ids.length, 1, `${context}: expected exactly one coverage entry`);
  return ids[0];
};

const statementCount = (coverageMap, coverage, line, context) => {
  const ids = matchingIds(coverageMap.statementMap, (entry) => entry.start.line === line);
  assert.equal(ids.length, 1, `${context}: statement at line ${line}`);
  return coverage.s[ids[0]];
};

const functionCountAt = (coverageMap, coverage, expected, context) => {
  const id = matchingId(
    coverageMap.fnMap,
    (entry) =>
      locationsEqual(entry.decl, expected.decl) && locationsEqual(entry.loc, expected.loc),
    `${context}: function location`,
  );
  return coverage.f[id];
};

const branchCountAt = (coverageMap, coverage, expected, context) => {
  const id = matchingId(
    coverageMap.branchMap,
    (entry) => entry.type === expected.type && locationsEqual(entry.loc, expected.loc),
    `${context}: branch location`,
  );
  return normalize(coverage.b[id]);
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
        functionCountAt(
          coverageMap,
          coverage,
          {
            decl: location(1, 9, 1, 17),
            loc: location(1, 25, 6, 1),
          },
          'control flow',
        ),
        2,
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'if', loc: location(2, 2, 4, 3) },
          'control flow',
        ),
        [1, 1],
      );
    },
  },
  {
    name: 'optional chaining tracked',
    filename: 'runtime/optional-chain-on.js',
    source: `function readName(user) {
  return user?.profile?.name ?? 'missing';
}
globalThis.values = [
  readName({ profile: { name: 'Ada' } }),
  readName(null),
  readName({ profile: null }),
];`,
    verify: ({ coverageMap, coverage, sandbox }) => {
      assert.deepEqual(normalize(sandbox.values), ['Ada', 'missing', 'missing']);
      assert.equal(statementCount(coverageMap, coverage, 2, 'optional chain on'), 3);
      assert.equal(
        functionCountAt(
          coverageMap,
          coverage,
          {
            decl: location(1, 9, 1, 17),
            loc: location(1, 24, 3, 1),
          },
          'optional chain on',
        ),
        3,
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'binary-expr', loc: location(2, 9, 2, 41) },
          'optional chain on',
        ),
        [3, 2],
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'optional-chain', loc: location(2, 9, 2, 28) },
          'optional chain on outer link',
        ),
        [2, 1],
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'optional-chain', loc: location(2, 9, 2, 22) },
          'optional chain on inner link',
        ),
        [1, 2],
      );
    },
  },
  {
    name: 'optional chaining untracked',
    filename: 'runtime/optional-chain-off.js',
    options: { trackOptionalChainBranches: false },
    source: `function readName(user) {
  return user?.profile?.name ?? 'missing';
}
globalThis.values = [
  readName({ profile: { name: 'Ada' } }),
  readName(null),
  readName({ profile: null }),
];`,
    verify: ({ coverageMap, coverage, result, sandbox }) => {
      assert.deepEqual(normalize(sandbox.values), ['Ada', 'missing', 'missing']);
      assert.equal(statementCount(coverageMap, coverage, 2, 'optional chain off'), 3);
      assert.equal(
        functionCountAt(
          coverageMap,
          coverage,
          {
            decl: location(1, 9, 1, 17),
            loc: location(1, 24, 3, 1),
          },
          'optional chain off',
        ),
        3,
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'binary-expr', loc: location(2, 9, 2, 41) },
          'optional chain off',
        ),
        [3, 2],
      );
      assert.equal(
        matchingIds(coverageMap.branchMap, (entry) => entry.type === 'optional-chain').length,
        0,
        'optional chain off: branch entries',
      );
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
        functionCountAt(
          coverageMap,
          coverage,
          {
            decl: location(4, 2, 4, 5),
            loc: location(4, 24, 6, 3),
          },
          'TypeScript service add',
        ),
        1,
      );
      assert.equal(
        functionCountAt(
          coverageMap,
          coverage,
          {
            decl: location(7, 2, 7, 6),
            loc: location(7, 32, 9, 3),
          },
          'TypeScript service find',
        ),
        2,
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'binary-expr', loc: location(8, 11, 8, 37) },
          'TypeScript service fallback',
        ),
        [2, 1],
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'optional-chain', loc: location(13, 29, 13, 52) },
          'TypeScript service optional chain',
        ),
        [0, 1],
      );
    },
  },
  {
    name: 'private method in parameter default',
    filename: 'runtime/private-method-default.js',
    source: `function f(x = class { #m() {} }) {}
f();`,
    verify: ({ coverageMap, coverage }) => {
      assert.equal(coverageMap.fnMap['0'].name, 'f');
      assert.equal(coverage.f['0'], 1, 'the enclosing function counter must increment');
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
        functionCountAt(
          coverageMap,
          coverage,
          {
            decl: location(1, 15, 1, 19),
            loc: location(1, 26, 6, 1),
          },
          'async load',
        ),
        2,
      );
      assert.equal(
        functionCountAt(
          coverageMap,
          coverage,
          {
            decl: location(7, 68, 7, 69),
            loc: location(7, 78, 9, 1),
          },
          'async completion callback',
        ),
        1,
      );
      assert.deepEqual(
        branchCountAt(
          coverageMap,
          coverage,
          { type: 'if', loc: location(2, 2, 4, 3) },
          'async paths',
        ),
        [1, 1],
      );
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
