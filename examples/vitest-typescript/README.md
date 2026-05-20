# Vitest + TypeScript example

End-to-end example showing how to use `oxc-coverage-instrument` as a drop-in Istanbul instrumenter for Vitest with **zero TypeScript pre-transform**. Coverage reports point directly at `.ts` source lines.

## What this demonstrates

- `vitest.config.ts` wires `coverage.instrumenter` to `createOxcInstrumenter` from `oxc-coverage-instrument/vitest`.
- The adapter auto-detects raw TypeScript on `.ts` / `.tsx` filenames when no `inputSourceMap` is provided, so no separate Babel / tsc pre-transform step is needed.
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
