#!/usr/bin/env node

import { existsSync, readdirSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const napiDir = join(repoRoot, 'crates', 'oxc-coverage-instrument-napi');

const packageNames = [
  '@oxc-coverage-instrument/binding-wasm32-wasi',
  '@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded',
];

const installRoots = [
  join(repoRoot, 'crates', 'oxc-coverage-instrument-napi'),
  join(repoRoot, 'examples', 'wasm-node'),
  join(repoRoot, 'examples', 'cloudflare-workers'),
];

const localWasm = existsSync(napiDir)
  ? readdirSync(napiDir).filter((name) => name.startsWith('coverage-instrument.wasm32-wasi') && name.endsWith('.wasm'))
  : [];

if (localWasm.length === 0) {
  throw new Error(`no local coverage-instrument.wasm32-wasi*.wasm artifact found in ${napiDir}`);
}

let removed = 0;

for (const installRoot of installRoots) {
  for (const packageName of packageNames) {
    const packageDir = join(installRoot, 'node_modules', ...packageName.split('/'));
    if (!existsSync(packageDir)) {
      continue;
    }
    for (const entry of readdirSync(packageDir)) {
      if (entry.startsWith('coverage-instrument.wasm32-wasi') && entry.endsWith('.wasm')) {
        rmSync(join(packageDir, entry), { force: true });
        removed += 1;
      }
    }
  }
}

console.log(
  `local wasm artifacts available: ${localWasm.join(', ')}; removed ${removed} published optional-package wasm file(s)`,
);
