# Vitest + TypeScript example

End-to-end example showing how to use `oxc-coverage-instrument` as a drop-in Istanbul instrumenter for Vitest. Vitest transforms TypeScript the way it always does (Vite's transform pipeline strips the types and emits a source map); the adapter instruments that transformed output and the coverage map is keyed back to the `.ts` source through that source map. No separate Babel pass is needed just to collect coverage, and reports point at `.ts` source lines.

## What this demonstrates

- `vitest.config.ts` wires `coverage.instrumenter` to `createOxcInstrumenter` from `oxc-coverage-instrument/vitest`.
- Vite hands the adapter the already-transpiled JavaScript plus an `inputSourceMap`. With `stripTypescript: false` the adapter skips its own strip pass and instruments that output; the coverage entries are remapped onto the `.ts` source via the input source map, so no separate Babel / tsc pre-transform step is needed just for coverage.
- `coverage-final.json` is keyed by the `.ts` source path (`src/math.ts`), and `statementMap` / `fnMap` / `branchMap` entries reference `.ts` line numbers.

## Run

```bash
cd examples/vitest-typescript
npm install
npm run coverage
npm run verify
```

`npm run coverage` runs Vitest with the Istanbul provider; `npm run verify` checks the resulting `coverage/coverage-final.json` for the assertions above and exits non-zero on any mismatch. CI runs both steps in the `Vitest TypeScript example` job.

## Layout

```
examples/vitest-typescript/
  package.json
  tsconfig.json
  vitest.config.ts
  src/
    math.ts        # typed function under test
    math.test.ts   # vitest test invoking compute()
  scripts/
    verify-coverage.mjs
```
