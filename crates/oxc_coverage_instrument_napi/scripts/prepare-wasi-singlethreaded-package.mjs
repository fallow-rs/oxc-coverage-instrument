#!/usr/bin/env node
// Populate the manually named single-threaded WASI optional package.
//
// Keep the generated threaded package as-is, then copy the single-threaded
// build artifacts into this repo-owned package name and patch the browser shim
// so it does not require SharedArrayBuffer or worker-backed shared memory. The
// single-threaded loaders do not start workers, so the package has no worker
// shims.

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  normalizeWasip1Outputs,
  patchSingleThreadedBrowserShim,
  patchSingleThreadedShim,
  patchSingleThreadedWasm,
} from './patch-wasi-singlethreaded-artifacts.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(__dirname, '..');
let sourceDir = packageRoot;
const installRoots = [];
for (let i = 2; i < process.argv.length; i += 1) {
  const arg = process.argv[i];
  if (arg === '--install-root') {
    installRoots.push(resolve(packageRoot, process.argv[++i]));
  } else if (arg === '--source-dir') {
    sourceDir = resolve(packageRoot, process.argv[++i]);
  } else if (!arg.startsWith('--')) {
    sourceDir = resolve(packageRoot, arg);
  } else {
    console.error(`[prepare-wasi-singlethreaded-package] unknown argument: ${arg}`);
    process.exit(1);
  }
}
const targetDir = join(packageRoot, 'npm', 'wasm32-wasi-singlethreaded');
const singlePackage = '@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded';

const requiredFiles = [
  'coverage-instrument.wasm32-wasi.wasm',
  'coverage-instrument.wasi.cjs',
  'coverage-instrument.wasi-browser.js',
];

function requirePath(path) {
  if (!existsSync(path)) {
    console.error(`[prepare-wasi-singlethreaded-package] missing required file: ${path}`);
    process.exit(1);
  }
  return path;
}

const readRequired = (path) => readFileSync(requirePath(path), 'utf8');

normalizeWasip1Outputs(sourceDir);
mkdirSync(targetDir, { recursive: true });
for (const staleWorker of ['wasi-worker.mjs', 'wasi-worker-browser.mjs']) {
  rmSync(join(targetDir, staleWorker), { force: true });
}
for (const file of requiredFiles) {
  const sourcePath = join(sourceDir, file);
  const targetPath = join(targetDir, file);
  if (file === 'coverage-instrument.wasi-browser.js') {
    writeFileSync(targetPath, patchSingleThreadedBrowserShim(readRequired(sourcePath)));
  } else if (file.endsWith('.js') || file.endsWith('.cjs')) {
    writeFileSync(targetPath, patchSingleThreadedShim(readRequired(sourcePath)));
  } else if (file.endsWith('.wasm')) {
    const result = patchSingleThreadedWasm(readFileSync(requirePath(sourcePath)));
    writeFileSync(targetPath, result.buffer);
    if (result.patched) {
      console.log(`[prepare-wasi-singlethreaded-package] made wasm global #${result.globalIndex} mutable.`);
    }
  } else {
    copyFileSync(requirePath(sourcePath), targetPath);
  }
}

console.log(
  `[prepare-wasi-singlethreaded-package] populated ${targetDir} from ${sourceDir}.`,
);

for (const installRoot of installRoots) {
  const packageDir = join(installRoot, 'node_modules', ...singlePackage.split('/'));
  rmSync(packageDir, { recursive: true, force: true });
  mkdirSync(packageDir, { recursive: true });
  for (const file of [...requiredFiles, 'package.json']) {
    copyFileSync(join(targetDir, file), join(packageDir, file));
  }
  console.log(`[prepare-wasi-singlethreaded-package] installed ${singlePackage} at ${packageDir}.`);
}
