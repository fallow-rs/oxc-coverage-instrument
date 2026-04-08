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

## v0.2.x (current)

Correct instrumented output via AST mutation. Istanbul-conformant. Published to npm.

- [x] **AST-level counter injection via `Traverse`**: proper AST mutation using `oxc_traverse::Traverse` + `oxc_codegen`
- [x] **Pragma handling**: `istanbul ignore next/if/else/file`, `v8 ignore`, `c8 ignore`
- [x] **Source map output**: via `oxc_codegen` with preamble line offset correction
- [x] **Source map composition**: chains output map through input source map (TS → JS → instrumented)
- [x] **Branch coverage**: `??`, `??=`/`||=`/`&&=`, `default-arg`, chained logical flattening
- [x] **Istanbul conformance**: prefix `++`, `branchMap.loc`, verified against `istanbul-lib-instrument` on 25 fixtures
- [x] **npm package**: Node.js bindings via napi-rs, 7 platform binaries, trusted publishing
- [x] **CLI binary**: `oxc-coverage-instrument <file>` for standalone use
- [x] **Coverage ingestion**: `parse_coverage_map()` and `FileCoverage::from_json()` for reading coverage data
- [x] **Conformance test suite**: 175 automated checks against Istanbul reference output
- [x] **282 tests**, 97% line coverage, strict clippy (all+pedantic+nursery, Oxc-level restrictions)
- [x] **CI**: cross-platform tests, MSRV, cargo-deny, napi test, typos, doc checks, coverage badge
- [x] **Published to crates.io**: automated publishing via CI on each release

## Future

- **Configurable counter style**: comma-operator wrapping for expression contexts
- **fallow integration**: `fallow health --coverage coverage-final.json` ingests real per-function coverage
- **Oxc org transfer**: if the Oxc project wants to host this (see [oxc#21108](https://github.com/oxc-project/oxc/issues/21108))
