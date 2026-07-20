# `@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded`

This is the **wasm32-wasip1** single-threaded binding for `@oxc-coverage-instrument/binding`.

It is selected automatically by the host package `oxc-coverage-instrument` from the `browser` export when `SharedArrayBuffer` is unavailable. That covers Cloudflare Workers, browsers without COOP/COEP isolation, and other runtimes that disallow shared WebAssembly memory.

When `SharedArrayBuffer` is available, the host package prefers `@oxc-coverage-instrument/binding-wasm32-wasi`, the threaded **wasm32-wasip1-threads** binding.

See the main package's README for the full runtime support matrix.
