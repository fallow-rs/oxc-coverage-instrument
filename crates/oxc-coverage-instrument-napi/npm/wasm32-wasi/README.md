# `@oxc-coverage-instrument/binding-wasm32-wasi`

This is the **wasm32-wasip1-threads** binding for `@oxc-coverage-instrument/binding`. It is selected automatically by the host package `oxc-coverage-instrument` when:

- the host runtime has no matching native binding (e.g., browsers with `SharedArrayBuffer` enabled, Deno using the `node:wasi` shim), OR
- the consumer sets `NAPI_RS_FORCE_WASI=1` explicitly to bypass the native binding.

**Currently unsupported runtimes**: Cloudflare Workers lacks `SharedArrayBuffer`, which `wasm32-wasip1-threads` requires. Bun's `node:wasi` is incomplete (no `wasi.initialize()`), so the wasm binding does not load there; Bun on its officially-supported platforms uses the native binding instead.

See the main package's README for the full runtime support matrix.
