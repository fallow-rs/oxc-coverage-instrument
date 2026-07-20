// Unit tests for browser.js package selection. The test imports a temporary
// copy of browser.js with stub optional dependencies so it does not need real
// wasm artifacts or a Workers account.

import { strict as assert } from 'node:assert';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BROWSER_JS = join(__dirname, '..', 'browser.js');
const THREADED = '@oxc-coverage-instrument/binding-wasm32-wasi';
const SINGLE_THREADED = '@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded';

function writeStubPackage(root, name, selectedName, shouldThrow = false) {
  const packageDir = join(root, 'node_modules', ...name.split('/'));
  mkdirSync(packageDir, { recursive: true });
  writeFileSync(
    join(packageDir, 'package.json'),
    JSON.stringify({ name, version: '0.0.0-test', type: 'module', exports: './index.js' }),
  );
  const body = shouldThrow
    ? `throw new Error(${JSON.stringify(`unexpected import: ${name}`)});\n`
    : `export function instrument() { return ${JSON.stringify(selectedName)}; }
export function remapCoverageMap() { return ${JSON.stringify(selectedName)}; }
export function remapCoverageMapWithLoader() { return ${JSON.stringify(selectedName)}; }
export function v8ToIstanbul() { return ${JSON.stringify(selectedName)}; }
export function v8ToIstanbulWithLoader() { return ${JSON.stringify(selectedName)}; }
export default { selected: ${JSON.stringify(selectedName)} };
`;
  writeFileSync(join(packageDir, 'index.js'), body);
}

async function importBrowserLoader({ hasSharedArrayBuffer, forceSingleThreaded = false }) {
  const root = mkdtempSync(join(tmpdir(), 'oxc-browser-loader-'));
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'SharedArrayBuffer');
  const forceDescriptor = Object.getOwnPropertyDescriptor(
    globalThis,
    'OXC_COVERAGE_FORCE_WASI_SINGLE_THREADED',
  );
  try {
    writeFileSync(join(root, 'package.json'), '{"type":"module"}\n');
    writeFileSync(join(root, 'browser.mjs'), readFileSync(BROWSER_JS, 'utf8'));

    if (hasSharedArrayBuffer) {
      writeStubPackage(root, THREADED, 'threaded', forceSingleThreaded);
      writeStubPackage(root, SINGLE_THREADED, 'single-threaded', !forceSingleThreaded);
      Object.defineProperty(globalThis, 'SharedArrayBuffer', {
        configurable: true,
        writable: true,
        value: function SharedArrayBufferStub() {},
      });
    } else {
      writeStubPackage(root, THREADED, 'threaded', true);
      writeStubPackage(root, SINGLE_THREADED, 'single-threaded');
      Object.defineProperty(globalThis, 'SharedArrayBuffer', {
        configurable: true,
        writable: true,
        value: undefined,
      });
    }
    Object.defineProperty(globalThis, 'OXC_COVERAGE_FORCE_WASI_SINGLE_THREADED', {
      configurable: true,
      writable: true,
      value: forceSingleThreaded,
    });

    return await import(pathToFileURL(join(root, 'browser.mjs')).href);
  } finally {
    if (descriptor) {
      Object.defineProperty(globalThis, 'SharedArrayBuffer', descriptor);
    } else {
      delete globalThis.SharedArrayBuffer;
    }
    if (forceDescriptor) {
      Object.defineProperty(globalThis, 'OXC_COVERAGE_FORCE_WASI_SINGLE_THREADED', forceDescriptor);
    } else {
      delete globalThis.OXC_COVERAGE_FORCE_WASI_SINGLE_THREADED;
    }
    rmSync(root, { recursive: true, force: true });
  }
}

console.log('Testing browser.js wasm package selection...\n');

{
  const binding = await importBrowserLoader({ hasSharedArrayBuffer: true });
  assert.equal(binding.instrument(), 'threaded');
  assert.equal(binding.default.selected, 'threaded');
  console.log('  [PASS] SharedArrayBuffer selects threaded wasm binding');
}

{
  const binding = await importBrowserLoader({ hasSharedArrayBuffer: false });
  assert.equal(binding.instrument(), 'single-threaded');
  assert.equal(binding.default.selected, 'single-threaded');
  console.log('  [PASS] Missing SharedArrayBuffer selects single-threaded wasm binding');
}

{
  const binding = await importBrowserLoader({
    hasSharedArrayBuffer: true,
    forceSingleThreaded: true,
  });
  assert.equal(binding.instrument(), 'single-threaded');
  assert.equal(binding.default.selected, 'single-threaded');
  console.log('  [PASS] Force flag selects single-threaded wasm binding');
}

console.log('\nAll browser-loader tests passed.');
