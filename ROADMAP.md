# Roadmap

## v0.1.0 (current)

Working coverage map generation and source-level counter injection.

- [x] AST visitor collecting statement, function, and branch spans
- [x] Istanbul-compatible `coverage-final.json` output
- [x] Named functions, arrow functions, class methods
- [x] Branches: if/else, ternary, switch, logical &&/||
- [x] Function name resolution (variable-assigned arrows, method definitions)
- [x] Runtime preamble generation (global `__coverage__` initialization)
- [x] Source-level counter injection
- [x] `InstrumentOptions` with configurable coverage variable name
- [x] `unhandled_pragmas` field in result (placeholder for pragma handling)
- [x] `source_map` field in API (placeholder, always `None`)

### Known issues in v0.1.0

Source-level injection produces incorrect output for some arrow expressions. This is a fundamental limitation of text-level insertion. AST-level mutation fixes this.

## v0.2.0

Correct instrumented output via AST mutation.

- [ ] **AST-level counter injection via `Traverse`**: replace source-level insertion with proper AST mutation using `oxc_transformer::Traverse`, then emit via `oxc_codegen`. Fixes all edge cases with arrow functions, template literals, and expression-bodied functions.
- [ ] **`/* istanbul ignore */` pragma handling**: support `/* istanbul ignore next */`, `/* istanbul ignore else */`, and `/* istanbul ignore if */`. Suppressed statements/branches/functions get counters but are marked in the coverage map.
- [ ] **`/* v8 ignore */` pragma handling**: same semantics as Istanbul pragmas.
- [ ] **Source map output**: emit a source map alongside the instrumented code so coverage reports map back to original line numbers. Required for TypeScript files.
- [ ] **Branch coverage for `??` and `?.`**: track nullish coalescing and optional chaining as branches.
- [ ] **`for`/`for-in`/`for-of` branch coverage**: track loop entry vs skip.
- [ ] **Conformance test suite**: run both `@babel/istanbul-instrument` and this crate on a shared fixture set, assert counter structures match.

## v0.3.0

Ecosystem integration.

- [ ] **npm package**: publish Node.js bindings via napi-rs so the crate can be used from JavaScript build tools directly
- [ ] **CLI binary**: `oxc-coverage-instrument <file>` for standalone use (instrument a file, print to stdout)
- [ ] **Rolldown plugin example**: demonstrate integration as a Rolldown transform plugin
- [ ] **Vite plugin example**: demonstrate integration as a Vite transform plugin
- [ ] **`coverage-final.json` ingestion**: parse existing coverage data (for tools like fallow that consume coverage rather than produce it)
- [ ] **Configurable counter style**: support comma-operator wrapping (`(cov.s[0]++, expr)`) as an alternative to statement-prepend for expression contexts

## Future

- **fallow integration**: `fallow health --coverage coverage-final.json` ingests real per-function coverage for CRAP metric scoring (CC^2 * (1-cov/100)^3 + CC instead of the current binary model)
- **Merge with `istanbul-oxide`**: if both projects mature, consider consolidating the Istanbul types into a shared crate
- **Oxc org transfer**: if the Oxc project wants to host this (see [oxc#21108](https://github.com/oxc-project/oxc/issues/21108)), transfer the repo
