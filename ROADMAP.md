# Roadmap

## v0.1.0

Working coverage map generation and source-level counter injection.

- [x] AST visitor collecting statement, function, and branch spans
- [x] Istanbul-compatible `coverage-final.json` output
- [x] Named functions, arrow functions, class methods
- [x] Branches: if/else, ternary, switch, logical &&/||
- [x] Function name resolution (variable-assigned arrows, method definitions)
- [x] Runtime preamble generation (global `__coverage__` initialization)
- [x] Source-level counter injection
- [x] `InstrumentOptions` with configurable coverage variable name

## v0.2.0 (current)

Correct instrumented output via AST mutation. Istanbul-conformant.

- [x] **AST-level counter injection via `Traverse`**: replaced source-level insertion with proper AST mutation using `oxc_traverse::Traverse`, then emit via `oxc_codegen`
- [x] **Pragma handling**: `istanbul ignore next/if/else/file`, `v8 ignore`, `c8 ignore`
- [x] **Source map output**: via `oxc_codegen` with preamble line offset correction
- [x] **Branch coverage**: `??`, `??=`/`||=`/`&&=`, `default-arg`, chained logical flattening
- [x] **Istanbul conformance**: prefix `++`, `branchMap.loc`, verified against `istanbul-lib-instrument` on 25 fixtures
- [x] **npm package**: Node.js bindings via napi-rs (`oxc-coverage-instrument`)
- [x] **CLI binary**: `oxc-coverage-instrument <file>` for standalone use
- [x] **Coverage ingestion**: `parse_coverage_map()` and `FileCoverage::from_json()` for reading coverage data
- [x] **Conformance test suite**: 175 automated checks against Istanbul reference output
- [x] **277 tests**, 98.9% line coverage, strict clippy (all+pedantic+nursery)

## v0.3.0

Polish and ecosystem.

- [ ] **Publish to crates.io**: `cargo publish`
- [ ] **Publish to npm**: cross-platform CI build + publish workflow
- [ ] **Configurable counter style**: comma-operator wrapping for expression contexts
- [ ] **Input source map composition**: chain output source map through input source map

## Future

- **fallow integration**: `fallow health --coverage coverage-final.json` ingests real per-function coverage
- **Oxc org transfer**: if the Oxc project wants to host this (see [oxc#21108](https://github.com/oxc-project/oxc/issues/21108))
