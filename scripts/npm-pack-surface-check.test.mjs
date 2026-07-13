#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  cpSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = fileURLToPath(new URL('..', import.meta.url));
const sourceScript = resolve(rootDir, 'scripts/npm-pack-surface-check.mjs');
const sourceNapiDir = resolve(rootDir, 'crates/oxc-coverage-instrument-napi');
const platformNames = [
  'darwin-arm64',
  'darwin-x64',
  'linux-arm64-gnu',
  'linux-x64-gnu',
  'linux-x64-musl',
  'wasm32-wasi',
  'wasm32-wasi-singlethreaded',
  'win32-arm64-msvc',
  'win32-x64-msvc',
];

const makeFixture = () => {
  const fixture = mkdtempSync(join(tmpdir(), 'npm-pack-surface-check-'));
  const napiDir = resolve(fixture, 'crates/oxc-coverage-instrument-napi');
  const script = resolve(fixture, 'scripts/npm-pack-surface-check.mjs');
  mkdirSync(dirname(script), { recursive: true });
  mkdirSync(resolve(fixture, '.github/workflows'), { recursive: true });
  cpSync(sourceScript, script);
  cpSync(resolve(sourceNapiDir, 'package.json'), resolve(napiDir, 'package.json'));
  cpSync(
    resolve(rootDir, '.github/workflows/release-npm.yml'),
    resolve(fixture, '.github/workflows/release-npm.yml'),
  );
  for (const platformName of platformNames) {
    const targetDir = resolve(napiDir, 'npm', platformName);
    mkdirSync(targetDir, { recursive: true });
    cpSync(
      resolve(sourceNapiDir, 'npm', platformName, 'package.json'),
      resolve(targetDir, 'package.json'),
    );
  }

  const binDir = resolve(fixture, 'bin');
  mkdirSync(binDir);
  const npm = resolve(binDir, 'npm');
  writeFileSync(
    npm,
    `#!/usr/bin/env node
import { readFileSync } from 'node:fs';
if (process.env.FAIL_IF_NPM === '1') {
  console.error('metadata-only validation invoked npm');
  process.exit(99);
}
const manifest = JSON.parse(readFileSync('package.json', 'utf8'));
let files = manifest.files ?? [];
if (process.env.OMIT_PACK_PACKAGE === manifest.name) {
  files = files.filter((file) => file !== process.env.OMIT_PACK_FILE);
}
console.log(JSON.stringify([{ files: files.map((file) => ({ path: \`package/\${file}\` })) }]));
`,
  );
  chmodSync(npm, 0o755);
  return { fixture, napiDir, script, binDir };
};

const runFixture = (
  fixture,
  script,
  binDir,
  args = ['--metadata-only'],
  env = { FAIL_IF_NPM: '1' },
) =>
  spawnSync(process.execPath, [script, ...args], {
    cwd: fixture,
    encoding: 'utf8',
    env: { ...process.env, ...env, PATH: `${binDir}:${process.env.PATH}` },
  });

const expectParityFailure = (name, mutate, expected) => {
  const { fixture, napiDir, script, binDir } = makeFixture();
  try {
    mutate({ fixture, napiDir, script });
    const result = runFixture(fixture, script, binDir);
    assert.notEqual(result.status, 0, `${name} unexpectedly passed`);
    assert.match(result.stderr, expected, `${name} reported the wrong contract failure`);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
};

{
  const { fixture, script, binDir } = makeFixture();
  try {
    const result = runFixture(fixture, script, binDir);
    assert.equal(result.status, 0, result.stderr);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
}

{
  const { fixture, script, binDir } = makeFixture();
  try {
    const result = runFixture(fixture, script, binDir, ['--require-release-artifacts'], {});
    assert.equal(result.status, 0, result.stderr);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
}

{
  const { fixture, script, binDir } = makeFixture();
  try {
    const result = runFixture(fixture, script, binDir, ['--require-release-artifacts'], {
      OMIT_PACK_PACKAGE: '@oxc-coverage-instrument/binding-darwin-x64',
      OMIT_PACK_FILE: 'coverage-instrument.darwin-x64.node',
    });
    assert.notEqual(result.status, 0, 'strict release validation accepted a missing native binary');
    assert.match(result.stderr, /coverage-instrument\.darwin-x64\.node/);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
}

expectParityFailure(
  'deleted native platform manifest',
  ({ napiDir }) => {
    rmSync(resolve(napiDir, 'npm/darwin-arm64/package.json'));
  },
  /platform manifests/,
);

expectParityFailure(
  'unregistered root napi target',
  ({ napiDir }) => {
    const manifestPath = resolve(napiDir, 'package.json');
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    manifest.napi.targets = manifest.napi.targets.filter(
      (target) => target !== 'aarch64-apple-darwin',
    );
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  },
  /napi\.targets/,
);

expectParityFailure(
  'duplicate root napi target',
  ({ napiDir }) => {
    const manifestPath = resolve(napiDir, 'package.json');
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    manifest.napi.targets.push('aarch64-apple-darwin');
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  },
  /duplicate napi\.targets/,
);

expectParityFailure(
  'missing optional platform dependency',
  ({ napiDir }) => {
    const manifestPath = resolve(napiDir, 'package.json');
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    delete manifest.optionalDependencies['@oxc-coverage-instrument/binding-darwin-arm64'];
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  },
  /optionalDependencies/,
);

expectParityFailure(
  'missing release target',
  ({ fixture }) => {
    const workflowPath = resolve(fixture, '.github/workflows/release-npm.yml');
    const workflow = readFileSync(workflowPath, 'utf8').replace(
      '            target: aarch64-apple-darwin\n',
      '',
    );
    writeFileSync(workflowPath, workflow);
  },
  /release targets/,
);

expectParityFailure(
  'missing pack specification',
  ({ script }) => {
    const source = readFileSync(script, 'utf8').replace(
      "  { packageDir: 'darwin-arm64', files: ['package/coverage-instrument.darwin-arm64.node'] },\n",
      '',
    );
    writeFileSync(script, source);
  },
  /pack specs/,
);

expectParityFailure(
  'duplicate pack specification',
  ({ script }) => {
    const entry =
      "  { packageDir: 'darwin-arm64', files: ['package/coverage-instrument.darwin-arm64.node'] },\n";
    const source = readFileSync(script, 'utf8').replace(entry, `${entry}${entry}`);
    writeFileSync(script, source);
  },
  /duplicate pack specs/,
);

expectParityFailure(
  'duplicate platform manifest name',
  ({ napiDir }) => {
    const manifestPath = resolve(napiDir, 'npm/linux-arm64-gnu/package.json');
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    manifest.name = '@oxc-coverage-instrument/binding-darwin-arm64';
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  },
  /duplicate platform manifest names/,
);

expectParityFailure(
  'extra platform manifest name',
  ({ napiDir }) => {
    const targetDir = resolve(napiDir, 'npm/linux-riscv64-gnu');
    mkdirSync(targetDir, { recursive: true });
    const manifest = JSON.parse(
      readFileSync(resolve(napiDir, 'npm/linux-arm64-gnu/package.json'), 'utf8'),
    );
    manifest.name = '@oxc-coverage-instrument/binding-linux-riscv64-gnu';
    manifest.repository.directory =
      'crates/oxc-coverage-instrument-napi/npm/linux-riscv64-gnu';
    writeFileSync(resolve(targetDir, 'package.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  },
  /platform manifests:.*unexpected.*linux-riscv64-gnu/,
);

expectParityFailure(
  'swapped darwin directory mappings',
  ({ napiDir }) => {
    const armPath = resolve(napiDir, 'npm/darwin-arm64/package.json');
    const x64Path = resolve(napiDir, 'npm/darwin-x64/package.json');
    const arm = JSON.parse(readFileSync(armPath, 'utf8'));
    const x64 = JSON.parse(readFileSync(x64Path, 'utf8'));
    [arm.name, x64.name] = [x64.name, arm.name];
    writeFileSync(armPath, `${JSON.stringify(arm, null, 2)}\n`);
    writeFileSync(x64Path, `${JSON.stringify(x64, null, 2)}\n`);
  },
  /platform directory mappings/,
);

expectParityFailure(
  'duplicate release target',
  ({ fixture }) => {
    const workflowPath = resolve(fixture, '.github/workflows/release-npm.yml');
    const workflow = readFileSync(workflowPath, 'utf8').replace(
      '            target: aarch64-apple-darwin\n',
      '            target: aarch64-apple-darwin\n            target: aarch64-apple-darwin\n',
    );
    writeFileSync(workflowPath, workflow);
  },
  /duplicate release targets/,
);

console.log('npm pack surface contract tests: PASS');
