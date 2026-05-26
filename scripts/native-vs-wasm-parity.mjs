#!/usr/bin/env node
// Byte-for-byte parity check between the native and WASM (wasm32-wasip1-threads)
// napi bindings.
//
// v0.7.1 shipped a one-off local check that the two bindings produce identical
// coverage maps. This script reproduces that check on every PR so any future
// drift between the two backends (e.g. an `cfg(target_arch = "wasm32")` branch
// that silently changes output shape) fails CI immediately.
//
// Modes:
//   --dump        : instrument every conformance fixture, print a single JSON
//                   object `{ fixturePath: coverageMap }` to stdout. The mode
//                   of the loaded binding is determined by whether the
//                   NAPI_RS_FORCE_WASI env var is set in the parent.
//   (no argument) : spawn this script twice (once forcing the wasi binding,
//                   once with the native binding) and diff the two JSON
//                   payloads. Exits non-zero on any divergence.
//
// Usage:
//   node scripts/native-vs-wasm-parity.mjs
//   node scripts/native-vs-wasm-parity.mjs --dump > maps.json

import { spawnSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..');
const fixturesDir = join(repoRoot, 'crates', 'oxc-coverage-instrument', 'tests', 'conformance', 'fixtures');

const isDumpMode = process.argv.includes('--dump');

const listFixtures = () =>
  readdirSync(fixturesDir)
    .filter((name) => name.endsWith('.js'))
    .sort();

const dump = async () => {
  const { createOxcInstrumenter } = await import(
    '../crates/oxc-coverage-instrument-napi/vitest.js'
  );
  const instrumenter = createOxcInstrumenter({ coverageVariable: '__coverage__' });
  const out = {};
  for (const fixture of listFixtures()) {
    const path = join(fixturesDir, fixture);
    const source = readFileSync(path, 'utf8');
    instrumenter.instrumentSync(source, path);
    out[fixture] = instrumenter.lastFileCoverage();
  }
  process.stdout.write(JSON.stringify(out));
};

const runChild = (envOverride) => {
  const result = spawnSync(
    process.execPath,
    [fileURLToPath(import.meta.url), '--dump'],
    {
      cwd: repoRoot,
      env: { ...process.env, ...envOverride },
      encoding: 'utf8',
      maxBuffer: 256 * 1024 * 1024,
    },
  );
  if (result.status !== 0) {
    process.stderr.write(result.stderr || '');
    throw new Error(`dump child exited with status ${result.status}`);
  }
  return JSON.parse(result.stdout);
};

const diff = () => {
  // Run native first so a missing native binding fails fast with a clear
  // error before paying for the (slower) wasi instantiation.
  const native = runChild({ NAPI_RS_FORCE_WASI: '' });
  const wasi = runChild({ NAPI_RS_FORCE_WASI: 'error' });

  const fixtures = new Set([...Object.keys(native), ...Object.keys(wasi)]);
  const mismatches = [];
  for (const fixture of [...fixtures].sort()) {
    const a = JSON.stringify(native[fixture]);
    const b = JSON.stringify(wasi[fixture]);
    if (a !== b) {
      mismatches.push(fixture);
    }
  }

  if (mismatches.length > 0) {
    console.error(`native vs wasm parity FAILED (${mismatches.length}/${fixtures.size}):`);
    for (const fixture of mismatches) {
      console.error(`  - ${fixture}`);
    }
    console.error('');
    console.error('First mismatch detail:');
    const first = mismatches[0];
    console.error(`  fixture: ${first}`);
    console.error(`  native : ${JSON.stringify(native[first]).slice(0, 400)}...`);
    console.error(`  wasi   : ${JSON.stringify(wasi[first]).slice(0, 400)}...`);
    process.exit(1);
  }

  console.log(`native vs wasm parity OK: ${fixtures.size} fixtures byte-identical across both bindings`);
};

if (isDumpMode) {
  await dump();
} else {
  diff();
}
