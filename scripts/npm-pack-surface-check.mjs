#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = fileURLToPath(new URL('..', import.meta.url));
const napiDir = 'crates/oxc-coverage-instrument-napi';
const platformDir = `${napiDir}/npm`;

const targetPackages = new Map([
  ['aarch64-apple-darwin', 'darwin-arm64'],
  ['x86_64-apple-darwin', 'darwin-x64'],
  ['aarch64-unknown-linux-gnu', 'linux-arm64-gnu'],
  ['x86_64-unknown-linux-gnu', 'linux-x64-gnu'],
  ['x86_64-unknown-linux-musl', 'linux-x64-musl'],
  ['wasm32-wasip1-threads', 'wasm32-wasi'],
  ['wasm32-wasip1', 'wasm32-wasi-singlethreaded'],
  ['aarch64-pc-windows-msvc', 'win32-arm64-msvc'],
  ['x86_64-pc-windows-msvc', 'win32-x64-msvc'],
]);
const packageName = (packageDir) => `@oxc-coverage-instrument/binding-${packageDir}`;

const platformPackSpecs = [
  { packageDir: 'darwin-arm64', files: [] },
  { packageDir: 'darwin-x64', files: [] },
  { packageDir: 'linux-arm64-gnu', files: [] },
  { packageDir: 'linux-x64-gnu', files: [] },
  { packageDir: 'linux-x64-musl', files: [] },
  {
    packageDir: 'wasm32-wasi',
    files: [
      'package/coverage-instrument.wasm32-wasi.wasm',
      'package/coverage-instrument.wasi.cjs',
      'package/coverage-instrument.wasi-browser.js',
      'package/wasi-worker.mjs',
      'package/wasi-worker-browser.mjs',
    ],
  },
  {
    packageDir: 'wasm32-wasi-singlethreaded',
    files: [
      'package/coverage-instrument.wasm32-wasi.wasm',
      'package/coverage-instrument.wasi.cjs',
      'package/coverage-instrument.wasi-browser.js',
      'package/wasi-worker.mjs',
      'package/wasi-worker-browser.mjs',
    ],
  },
  { packageDir: 'win32-arm64-msvc', files: [] },
  { packageDir: 'win32-x64-msvc', files: [] },
];

const args = process.argv.slice(2);
const metadataOnly = args.length === 1 && args[0] === '--metadata-only';
if (args.length > 0 && !metadataOnly) {
  throw new Error(`Unknown argument: ${args.join(' ')}`);
}

const rootManifestPath = `${napiDir}/package.json`;
const rootManifest = JSON.parse(readFileSync(resolve(rootDir, rootManifestPath), 'utf8'));
const expectedTargets = new Set(targetPackages.keys());
const expectedPackages = new Set([...targetPackages.values()].map(packageName));

const contractErrors = [];
const checkSet = (label, expected, actual) => {
  const missing = [...expected].filter((value) => !actual.has(value)).sort();
  const unexpected = [...actual].filter((value) => !expected.has(value)).sort();
  if (missing.length === 0 && unexpected.length === 0) {
    return;
  }
  const details = [];
  if (missing.length > 0) details.push(`missing ${missing.join(', ')}`);
  if (unexpected.length > 0) details.push(`unexpected ${unexpected.join(', ')}`);
  contractErrors.push(`${label}: ${details.join('; ')}`);
};

checkSet('napi.targets', expectedTargets, new Set(rootManifest.napi?.targets ?? []));
checkSet(
  'optionalDependencies',
  expectedPackages,
  new Set(
    Object.keys(rootManifest.optionalDependencies ?? {}).filter((name) =>
      name.startsWith('@oxc-coverage-instrument/binding-'),
    ),
  ),
);

const platformManifests = readdirSync(resolve(rootDir, platformDir), {
  withFileTypes: true,
})
  .filter((entry) => entry.isDirectory())
  .map((entry) => `${platformDir}/${entry.name}/package.json`)
  .filter((manifest) => existsSync(resolve(rootDir, manifest)))
  .sort();

const platformPackages = new Set(
  platformManifests.map((manifest) => {
    const packageJson = JSON.parse(readFileSync(resolve(rootDir, manifest), 'utf8'));
    return packageJson.name;
  }),
);
checkSet('platform manifests', expectedPackages, platformPackages);

const releaseWorkflow = readFileSync(resolve(rootDir, '.github/workflows/release-npm.yml'), 'utf8');
const releaseTargets = new Set(
  [...releaseWorkflow.matchAll(/^\s+target:\s+([^\s#]+)\s*$/gm)].map((match) => match[1]),
);
checkSet('release targets', expectedTargets, releaseTargets);

const packPackages = new Set(platformPackSpecs.map((spec) => packageName(spec.packageDir)));
checkSet('pack specs', expectedPackages, packPackages);

if (contractErrors.length > 0) {
  throw new Error(`Invalid npm target contracts:\n${contractErrors.join('\n')}`);
}

const manifests = [
  { path: rootManifestPath, expectedDirectory: napiDir },
  ...platformManifests.map((path) => ({
    path,
    expectedDirectory: path.slice(0, -'/package.json'.length),
  })),
];

const metadataErrors = [];
for (const manifest of manifests) {
  const packageJson = JSON.parse(readFileSync(resolve(rootDir, manifest.path), 'utf8'));
  const actualDirectory = packageJson.repository?.directory;
  if (actualDirectory !== manifest.expectedDirectory) {
    const expected = JSON.stringify(manifest.expectedDirectory);
    const actual = JSON.stringify(actualDirectory);
    metadataErrors.push(
      `${manifest.path}: expected repository.directory ${expected}, got ${actual}`,
    );
  }
}

if (metadataErrors.length > 0) {
  throw new Error(`Invalid npm repository metadata:\n${metadataErrors.join('\n')}`);
}
console.log(`npm repository metadata OK: ${manifests.length} manifests`);
console.log(`npm target contracts OK: ${expectedTargets.size} targets`);

if (metadataOnly) {
  process.exit(0);
}

const specs = [
  {
    dir: 'crates/oxc-coverage-instrument-napi',
    files: [
      'package/browser.js',
      'package/index.js',
      'package/index.d.ts',
      'package/coverage-instrument.wasi.cjs',
      'package/coverage-instrument.wasi-browser.js',
      'package/wasi-worker.mjs',
      'package/wasi-worker-browser.mjs',
      'package/vitest.js',
      'package/vitest.d.ts',
    ],
  },
  ...platformPackSpecs.map((spec) => ({
    dir: `${platformDir}/${spec.packageDir}`,
    files: spec.files,
  })),
];

for (const spec of specs) {
  const result = spawnSync('npm', ['pack', '--dry-run', '--json'], {
    cwd: resolve(rootDir, spec.dir),
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    process.stderr.write(result.stderr);
    throw new Error(`npm pack failed in ${spec.dir}`);
  }

  const pack = JSON.parse(result.stdout)[0];
  const actual = new Set(pack.files.map((file) => file.path.replace(/^package\//, '')));
  const missing = spec.files.filter((file) => !actual.has(file.replace(/^package\//, '')));
  if (missing.length > 0) {
    throw new Error(`${spec.dir} package is missing files: ${missing.join(', ')}`);
  }
  console.log(`npm pack surface OK: ${spec.dir}`);
}
