# WASM (Node) example

End-to-end smoke for the WASI bindings under Node. CI runs this example against both `wasm32-wasip1-threads` and `wasm32-wasip1`; the same `instrument()` API call works with either the native `.node` binding (preferred when available) or the currently-built WASI binding (automatic fallback on platforms without a prebuilt native binary, or forced via `NAPI_RS_FORCE_WASI=1`).

## What this demonstrates

- `oxc-coverage-instrument` works identically against the native binding and each WASM variant. No code change is required to opt into WASM; the loader picks the right binary at runtime.
- Each WASM build stays under the 2 MB brotli CI ceiling.

## Run

```bash
cd examples/wasm-node
npm install
npm run smoke       # uses the native binding (or wasm if no native is available)
npm run smoke:wasi  # forces the currently-built wasm32-wasi binding via NAPI_RS_FORCE_WASI=error
```

`smoke:wasi` will fail with a clear error if the WASI binding cannot be loaded, so it doubles as a regression gate that the WASM path stays healthy.

## Platform support

| Runtime | Native binding | WASM binding | Notes |
|:--------|:---------------|:-------------|:------|
| Node 22 LTS on darwin/linux/win32 | yes | yes (via `node:wasi`) | Native preferred automatically. |
| Deno 2.x | no | yes (Deno's `node:wasi` shim) | Untested in CI; community feedback welcome. |
| Bun (any version) | yes | no | Bun's `node:wasi` lacks `wasi.initialize()` ([bun#16156](https://github.com/oven-sh/bun/issues/16156)). Native binding works on all supported Bun platforms. |
| Browser (with COOP/COEP) | no | yes (via `browser` export + `fetch`) | Uses the threaded binding when `SharedArrayBuffer` is available. |
| Browser (without COOP/COEP) | no | yes (via `browser` export + `fetch`) | Uses the single-threaded binding when `SharedArrayBuffer` is unavailable. |
| Cloudflare Workers | no | yes (via `browser` export) | Uses the single-threaded binding; see `examples/cloudflare-workers/` for the local acceptance smoke. |
| StackBlitz / WebContainer | no | partial | Falls under the browser case above; WebContainer's `node:wasi` polyfill is incomplete in older releases. |
