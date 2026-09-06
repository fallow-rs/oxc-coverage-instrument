// Unit tests for scripts/patch-wasi-browser-shim.mjs. Exercises the patch
// against synthetic shim contents so the gate fires even on jobs that do
// not build the wasm binding.

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(__dirname, 'patch-wasi-browser-shim.mjs');
const REAL_SHIM = join(__dirname, '..', 'coverage-instrument.wasi-browser.js');
const SENTINEL = '// oxc-coverage-instrument: SharedArrayBuffer guard (issue #89)';

const SYNTHETIC_SHIM = `import {
  createOnMessage as __wasmCreateOnMessageForFsProxy,
  getDefaultContext as __emnapiGetDefaultContext,
  instantiateNapiModuleSync as __emnapiInstantiateNapiModuleSync,
  WASI as __WASI,
} from '@napi-rs/wasm-runtime'



const __wasi = new __WASI({ version: 'preview1' })
const __wasmUrl = new URL('./coverage-instrument.wasm32-wasi.wasm', import.meta.url).href
const __sharedMemory = new WebAssembly.Memory({ initial: 4000, maximum: 65536, shared: true })
`;

// The script resolves the shim path itself, so swap the real shim aside for
// the duration of each test if present.
function withShimAside(fn) {
  let backup = null;
  if (existsSync(REAL_SHIM)) {
    backup = `${REAL_SHIM}.test-backup`;
    renameSync(REAL_SHIM, backup);
  }
  try {
    return fn();
  } finally {
    rmSync(REAL_SHIM, { force: true });
    if (backup) {
      renameSync(backup, REAL_SHIM);
    }
  }
}

function runScript() {
  return spawnSync(process.execPath, [SCRIPT], { encoding: 'utf8' });
}

function runWith(syntheticContents) {
  return withShimAside(() => {
    writeFileSync(REAL_SHIM, syntheticContents);
    const result = runScript();
    return { result, after: readFileSync(REAL_SHIM, 'utf8') };
  });
}

console.log('Testing patch-wasi-browser-shim.mjs...\n');

// Test 1: guard is injected on a fresh shim
{
  const { result, after } = runWith(SYNTHETIC_SHIM);
  assert.equal(result.status, 0, `expected exit 0, got ${result.status}: ${result.stderr}`);
  assert.ok(after.includes(SENTINEL), 'guard sentinel missing after patch');
  assert.ok(
    after.includes("typeof SharedArrayBuffer === 'undefined'"),
    'SharedArrayBuffer check missing',
  );
  assert.ok(
    after.includes('Cross-Origin-Opener-Policy'),
    'COOP guidance missing from error message',
  );
  assert.ok(
    after.includes('Cross-Origin-Embedder-Policy'),
    'COEP guidance missing from error message',
  );
  // Guard must precede the shared-memory allocation.
  assert.ok(
    after.indexOf(SENTINEL) < after.indexOf('new WebAssembly.Memory'),
    'guard injected after WebAssembly.Memory allocation',
  );
  console.log('  [PASS] Guard injected with diagnostic prose');
}

// Test 2: idempotent (second run is a no-op)
{
  const patchedOnce = runWith(SYNTHETIC_SHIM).after;
  const { result, after } = runWith(patchedOnce);
  assert.equal(result.status, 0, 'second run should succeed');
  assert.equal(after, patchedOnce, 'second run mutated already-patched shim');
  // Sentinel should appear exactly once.
  const occurrences = after.split(SENTINEL).length - 1;
  assert.equal(occurrences, 1, `expected exactly 1 sentinel, found ${occurrences}`);
  console.log('  [PASS] Idempotent on re-run');
}

// Test 3: missing shim is a no-op (non-wasm builds)
withShimAside(() => {
  const result = runScript();
  assert.equal(result.status, 0, 'missing shim should exit 0');
  assert.ok(!existsSync(REAL_SHIM), 'patch script created a shim where none existed');
  console.log('  [PASS] Missing shim is a no-op');
});

// Test 4: malformed shim (no import block) errors loudly
{
  const { result } = runWith('const x = 1;\n');
  assert.notEqual(result.status, 0, 'malformed shim should exit non-zero');
  assert.ok(
    result.stderr.includes('could not locate'),
    `expected diagnostic stderr, got: ${result.stderr}`,
  );
  console.log('  [PASS] Malformed shim errors loudly');
}

console.log('\nAll patch-shim tests passed.');
