# `@oxc-coverage-instrument/binding-wasm32-wasi`

This is the **wasm32-wasip1-threads** binding for `@oxc-coverage-instrument/binding`. It is selected automatically by the host package `oxc-coverage-instrument` when:

- the browser runtime exposes `SharedArrayBuffer`, OR
- the Node/Deno host runtime has no matching native binding and loads the WASI fallback, OR
- the consumer sets `NAPI_RS_FORCE_WASI=1` explicitly to bypass the native binding.

Browsers and edge runtimes without `SharedArrayBuffer` (including Cloudflare Workers and pages without COOP/COEP isolation) are served by `@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded` through the host package's `browser` export instead.

**Currently unsupported runtimes**: Bun's `node:wasi` is incomplete (no `wasi.initialize()`), so the wasm binding does not load there; Bun on its officially-supported platforms uses the native binding instead.

See the main package's README for the full runtime support matrix.
