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

- **fallow integration**: `fallow health --coverage coverage-final.json` ingests real per-function coverage
- **Oxc org transfer**: if the Oxc project wants to host this (see [oxc#21108](https://github.com/oxc-project/oxc/issues/21108))

## Deferred / conditional

- **`bO` (branch-operator) channel — DEFERRED pending reporter.** Optional `reportOperators: true` flag emitting an extra `bO` map that preserves operator-to-leaf mapping for chained logical expressions. The flat `binary-expr` model in `branchMap` erases operator boundaries (you cannot tell from a report whether the inner `||` in `a && (b || c)` was ever exercised, only that b/c were evaluated). Implementation in Rust is cheap: one extra field on `CoverageTransform`, collected during the existing `collect_logical_leaf_spans` walk, no new AST mutations, no runtime cost in instrumented output. Proposed shape: `bO: BTreeMap<String, Vec<{ operator, leafIndices }>>` — pure static index overlay, counts stay in `b`, merge semantics trivially inherited.
  - **Blocker:** no mainstream coverage reporter (codecov, Sonar, istanbul-reports HTML, Vitest UI) reads extra channels today. The existing `bT` channel (enabled via `reportLogic`) is barely consumed in the wild. Shipping `bO` alone would add a second orphan channel.
  - **Unblocks when:** we (or someone) ship a companion reporter that visualizes operator-level coverage — either as a custom HTML reporter in this repo or as a patch upstream. Feature + reporter must land together.
  - **Precedent:** see user-panel review 2026-04-15.
