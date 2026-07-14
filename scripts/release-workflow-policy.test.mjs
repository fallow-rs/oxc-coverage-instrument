#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const workflow = readFileSync(`${root}/.github/workflows/release-npm.yml`, 'utf8');
const lines = workflow.split(/\r?\n/);

const findBlock = (parent, key, indent) => {
  const header = `${' '.repeat(indent)}${key}:`;
  const start = lines.findIndex(
    (line, index) => index >= parent.start && index < parent.end && line === header,
  );
  assert.notEqual(start, -1, `release workflow must define ${key}`);

  let end = parent.end;
  for (let index = start + 1; index < parent.end; index += 1) {
    const line = lines[index];
    const trimmed = line.trimStart();
    if (trimmed === '' || trimmed.startsWith('#')) continue;
    if (line.length - trimmed.length <= indent) {
      end = index;
      break;
    }
  }
  return { start: start + 1, end };
};

const textOf = (block) => lines.slice(block.start, block.end).join('\n');
const rootBlock = { start: 0, end: lines.length };
const jobs = findBlock(rootBlock, 'jobs', 0);
const prepublishBlock = findBlock(jobs, 'prepublish', 2);
const prepublish = textOf(prepublishBlock);
const prepublishPermissions = textOf(findBlock(prepublishBlock, 'permissions', 4));
const build = textOf(findBlock(jobs, 'build', 2));
const publishCrate = textOf(findBlock(jobs, 'publish-crate', 2));
const publish = textOf(findBlock(jobs, 'publish', 2));
const versionSyncCommand = '        run: ./scripts/check.sh version-sync';
const packageNpmCiCommand = [
  '        working-directory: crates/oxc-coverage-instrument-napi',
  '        run: npm ci',
].join('\n');

const assertReadOnlyPermissions = (text) => {
  const normalized = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .join('\n');
  assert.equal(
    normalized,
    'contents: read',
    'release workflow prepublish permissions must contain only contents: read',
  );
};

const assertPrepublishOrder = (text) => {
  const versionSyncIndex = text.indexOf(versionSyncCommand);
  const packageNpmCiIndex = text.indexOf(packageNpmCiCommand);
  assert.notEqual(versionSyncIndex, -1, 'release workflow must run version-sync');
  assert.notEqual(packageNpmCiIndex, -1, 'release workflow must run package-local npm ci');
  assert.ok(
    versionSyncIndex < packageNpmCiIndex,
    'release workflow must run version-sync before package-local npm ci',
  );
};

assert.match(prepublish, /^\s{4}runs-on: ubuntu-latest$/m);
assertReadOnlyPermissions(prepublishPermissions);
assert.throws(
  () => assertReadOnlyPermissions('      contents: read\n      id-token: write'),
  /release workflow prepublish permissions must contain only contents: read/,
);
assert.match(prepublish, /uses: actions\/checkout@[a-f0-9]+/);
assert.match(prepublish, /uses: actions\/setup-node@[a-f0-9]+/);
assert.match(prepublish, /^\s{10}node-version: 22$/m);
assert.match(prepublish, /^\s{8}run: npm install -g npm@11\.12\.1$/m);
assert.match(prepublish, /^\s{8}run: \.\/scripts\/check\.sh version-sync$/m);
assert.match(
  prepublish,
  /working-directory: crates\/oxc-coverage-instrument-napi\n\s+run: npm ci/,
);
assertPrepublishOrder(prepublish);
assert.throws(
  () => assertPrepublishOrder(`${packageNpmCiCommand}\n${versionSyncCommand}`),
  /release workflow must run version-sync before package-local npm ci/,
);
assert.match(build, /^\s{4}needs: prepublish$/m);
assert.match(publishCrate, /^\s{4}needs: build$/m);
assert.match(publish, /^\s{4}needs: publish-crate$/m);

console.log('release workflow policy tests: PASS');
