#!/usr/bin/env node
// Post-build patch for the generated root browser export.
//
// napi-rs regenerates `browser.js` as a static re-export of the wasm32-wasi
// package. This package ships two WASI browser bindings: the threaded build for
// cross-origin-isolated hosts with SharedArrayBuffer, and a single-threaded
// build for Workers / non-isolated browsers. Rewrite the root browser export to
// pick the right optional dependency at module initialization time.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SENTINEL = '// oxc-coverage-instrument: browser WASI selector (issue #87)';
const BROWSER_LOADER = `${SENTINEL}
const binding =
  globalThis.OXC_COVERAGE_FORCE_WASI_SINGLE_THREADED === true ||
  typeof SharedArrayBuffer === 'undefined'
    ? await import('@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded')
    : await import('@oxc-coverage-instrument/binding-wasm32-wasi');

const api = binding.default ?? binding;

export default api;
export const instrument = binding.instrument ?? api.instrument;
export const remapCoverageMap = binding.remapCoverageMap ?? api.remapCoverageMap;
export const remapCoverageMapWithLoader =
  binding.remapCoverageMapWithLoader ?? api.remapCoverageMapWithLoader;
export const v8ToIstanbul = binding.v8ToIstanbul ?? api.v8ToIstanbul;
export const v8ToIstanbulWithLoader =
  binding.v8ToIstanbulWithLoader ?? api.v8ToIstanbulWithLoader;
`;

const __dirname = dirname(fileURLToPath(import.meta.url));
const browserPath = join(__dirname, '..', 'browser.js');

if (!existsSync(browserPath)) {
  console.log(`[patch-browser-loader] ${browserPath} not present; skipping.`);
  process.exit(0);
}

const original = readFileSync(browserPath, 'utf8');
if (original === BROWSER_LOADER) {
  console.log('[patch-browser-loader] browser loader already patched; nothing to do.');
  process.exit(0);
}

if (!original.includes(SENTINEL) && !original.includes('@oxc-coverage-instrument/binding-wasm32-wasi')) {
  console.error('[patch-browser-loader] generated browser.js shape is unexpected; aborting.');
  process.exit(1);
}

writeFileSync(browserPath, BROWSER_LOADER);
console.log(`[patch-browser-loader] rewrote ${browserPath} with SharedArrayBuffer selector.`);
