// Unit tests for the loader renames in scripts/patch-wasi-singlethreaded-artifacts.mjs.

import { strict as assert } from 'node:assert';

import { renameWasip1References } from './patch-wasi-singlethreaded-artifacts.mjs';

// The root loader stamps `wasm32-wasi` on the package it loads. A renamed
// loader that still stamps `wasm32-wasip1` makes it throw
// ERR_NAPI_BINDING_TARGET_CONFLICT.
for (const declaration of [
  "const __napiBindingTarget = 'wasm32-wasip1'",
  "export const __napiBindingTarget = 'wasm32-wasip1'",
]) {
  assert.match(
    renameWasip1References(declaration),
    /const __napiBindingTarget = 'wasm32-wasi'$/,
    `binding target must be renamed in: ${declaration}`,
  );
}

assert.equal(
  renameWasip1References("require('./coverage-instrument.wasm32-wasip1.wasm')"),
  "require('./coverage-instrument.wasm32-wasi.wasm')",
);

console.log('All wasip1 rename tests passed.');
