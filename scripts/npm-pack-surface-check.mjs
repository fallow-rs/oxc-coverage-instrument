#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

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
  {
    dir: 'crates/oxc-coverage-instrument-napi/npm/wasm32-wasi',
    files: [
      'package/coverage-instrument.wasm32-wasi.wasm',
      'package/coverage-instrument.wasi.cjs',
      'package/coverage-instrument.wasi-browser.js',
      'package/wasi-worker.mjs',
      'package/wasi-worker-browser.mjs',
    ],
  },
  {
    dir: 'crates/oxc-coverage-instrument-napi/npm/wasm32-wasi-singlethreaded',
    files: [
      'package/coverage-instrument.wasm32-wasi.wasm',
      'package/coverage-instrument.wasi.cjs',
      'package/coverage-instrument.wasi-browser.js',
      'package/wasi-worker.mjs',
      'package/wasi-worker-browser.mjs',
    ],
  },
];

for (const spec of specs) {
  const result = spawnSync('npm', ['pack', '--dry-run', '--json'], {
    cwd: resolve(spec.dir),
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
