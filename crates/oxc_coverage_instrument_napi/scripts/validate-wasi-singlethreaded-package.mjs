#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultPackageDir = resolve(__dirname, '..', 'npm', 'wasm32-wasi-singlethreaded');
const packageDir = process.argv[2] ? resolve(process.cwd(), process.argv[2]) : defaultPackageDir;
const packageName = '@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded';

const requiredFiles = [
  'coverage-instrument.wasm32-wasi.wasm',
  'coverage-instrument.wasi.cjs',
  'coverage-instrument.wasi-browser.js',
  'wasi-worker.mjs',
  'wasi-worker-browser.mjs',
  'package.json',
];

function readRequired(file, encoding) {
  const path = join(packageDir, file);
  if (!existsSync(path)) {
    throw new Error(`missing required single-threaded package file: ${path}`);
  }
  return readFileSync(path, encoding);
}

for (const file of requiredFiles) {
  readRequired(file);
}

const packageJson = JSON.parse(readRequired('package.json', 'utf8'));
if (packageJson.name !== packageName) {
  throw new Error(`expected package name ${packageName}, received ${packageJson.name}`);
}

const wasm = readRequired('coverage-instrument.wasm32-wasi.wasm');
if (!WebAssembly.validate(wasm)) {
  throw new Error('single-threaded WASI wasm does not pass WebAssembly.validate');
}

for (const file of ['coverage-instrument.wasi.cjs', 'coverage-instrument.wasi-browser.js']) {
  const source = readRequired(file, 'utf8');
  for (const forbidden of ['shared: true', 'reuseWorker: true', 'onCreateWorker()']) {
    if (source.includes(forbidden)) {
      throw new Error(`${file} still contains threaded shim marker: ${forbidden}`);
    }
  }
  if (!source.includes('asyncWorkPoolSize: 0')) {
    throw new Error(`${file} does not disable the async worker pool`);
  }
}

console.log(`[validate-wasi-singlethreaded-package] validated ${packageDir}.`);
