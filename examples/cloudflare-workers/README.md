# Cloudflare Workers example

Secret-free smoke scaffold for the single-threaded `wasm32-wasip1` binding in Cloudflare Workers. The Worker imports `oxc-coverage-instrument`, instruments a deterministic 100-line fixture, and returns the produced Istanbul `FileCoverage`.

CI builds the native binding and the local single-threaded WASM artifact first, prepares a local `@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded` package from that artifact, bundles the Worker with Wrangler, then compares the Worker `FileCoverage` byte-for-byte with the native Node binding.

## Run

```bash
cd examples/cloudflare-workers
npm install
npm run build
npm run smoke
```

`npm run smoke` starts `wrangler dev --local` on `127.0.0.1:8787`, calls the Worker, compares the returned `FileCoverage` with the native binding, and then shuts Wrangler down. It does not deploy and does not need Cloudflare credentials.
