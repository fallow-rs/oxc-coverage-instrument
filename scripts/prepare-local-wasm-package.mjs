#!/usr/bin/env node

import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const napiDir = join(repoRoot, 'crates', 'oxc_coverage_instrument_napi');

let packageName = '@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded';
const installRoots = [];

for (let i = 2; i < process.argv.length; i += 1) {
  const arg = process.argv[i];
  if (arg === '--package-name') {
    packageName = process.argv[++i];
  } else if (arg === '--install-root') {
    installRoots.push(resolve(repoRoot, process.argv[++i]));
  } else {
    throw new Error(`unknown argument: ${arg}`);
  }
}

if (!packageName || !packageName.startsWith('@oxc-coverage-instrument/')) {
  throw new Error(`invalid package name: ${packageName}`);
}

if (installRoots.length === 0) {
  throw new Error('usage: node scripts/prepare-local-wasm-package.mjs --install-root <dir> [--install-root <dir> ...]');
}

const rootPackage = JSON.parse(readFileSync(join(napiDir, 'package.json'), 'utf8'));
const files = [
  'coverage-instrument.wasm32-wasi.wasm',
  'coverage-instrument.wasi.cjs',
  'coverage-instrument.wasi-browser.js',
  'wasi-worker.mjs',
  'wasi-worker-browser.mjs',
];

for (const file of files) {
  const path = join(napiDir, file);
  if (!existsSync(path)) {
    throw new Error(`expected local wasm package file is missing: ${path}`);
  }
}

for (const installRoot of installRoots) {
  const packageDir = join(installRoot, 'node_modules', ...packageName.split('/'));
  rmSync(packageDir, { recursive: true, force: true });
  mkdirSync(packageDir, { recursive: true });

  for (const file of files) {
    copyFileSync(join(napiDir, file), join(packageDir, file));
  }

  writeFileSync(
    join(packageDir, 'package.json'),
    `${JSON.stringify(
      {
        name: packageName,
        version: rootPackage.version,
        main: 'coverage-instrument.wasi.cjs',
        browser: 'coverage-instrument.wasi-browser.js',
        files,
        dependencies: {
          '@napi-rs/wasm-runtime': rootPackage.dependencies['@napi-rs/wasm-runtime'],
        },
      },
      null,
      2,
    )}\n`,
  );

  console.log(`prepared ${packageName} from local wasm artifacts at ${packageDir}`);
}
