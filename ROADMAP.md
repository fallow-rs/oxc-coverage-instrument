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

## v0.5.x (in progress)

Coverage suite (umbrella [#45](https://github.com/fallow-rs/oxc-coverage-instrument/issues/45)): port the rest of the istanbuljs stack into Rust-native crates so the entire pipeline lives in one workspace.

- [x] **PR A**: workspace restructure into `crates/*` (#48)
- [x] **PR B**: `oxc_coverage_types` data-model crate, port of `istanbul-lib-coverage` (#49)
- [x] **PR C**: `oxc_coverage_source_maps` crate, port of `istanbul-lib-source-maps` (#50)
- [x] **PR D**: `oxc_coverage_v8` crate, replaces the `v8-to-istanbul` npm package (#51)
- [x] **PR E**: `oxc_coverage_report` + `oxc_coverage_reports` base + initial renderers
  - `oxc_coverage_report`: port of `istanbul-lib-report` (`CoverageSummary`, `ReportNode` tree, `summarize()`, `Visitor` trait with default no-op methods)
  - `oxc_coverage_reports`: `text`, `text-summary`, `json-summary` (matches `istanbul-reports` shape; `pct` rounded to 2 decimals; `total` first in JSON output)
  - CLI: new `report --format <fmt> COVERAGE.json` subcommand on the existing binary; existing `instrument FILE` invocation unchanged
- [x] **PR F**: `lcov` and `cobertura` reporters
  - Hand-rolled `lcov` emitter with correct `BRDA` block numbering (use `BranchEntry` id, not line) and configurable `SF:` root-relative path normalization
  - Hand-rolled `cobertura` XML; relative `<class filename=...>` for GitLab + Jenkins, `complexity="0"` on `<method>` for Azure DevOps, no `<missing-branches>`, `line-rate` / `branch-rate` / `timestamp` at root
  - CLI: `--root <dir>` flag on the `report` subcommand to relativize source-file paths (defaults to cwd)
- [ ] **PR G**: `html` reporter
  - Per-line gutter highlights driven through `oxc_coverage_source_maps` so TypeScript projects see original source, not compiled output
  - Embedded CSS / icons / sortable JS so reports render offline behind corporate proxies
  - Dark mode via `prefers-color-scheme`

## Future

- **fallow integration**: `fallow health --coverage coverage-final.json` ingests real per-function coverage
- **Oxc org transfer**: if the Oxc project wants to host this (see [oxc#21108](https://github.com/oxc-project/oxc/issues/21108))

## Deferred / conditional

- **`bO` (branch-operator) channel — DEFERRED pending reporter.** Optional `reportOperators: true` flag emitting an extra `bO` map that preserves operator-to-leaf mapping for chained logical expressions. The flat `binary-expr` model in `branchMap` erases operator boundaries (you cannot tell from a report whether the inner `||` in `a && (b || c)` was ever exercised, only that b/c were evaluated). Implementation in Rust is cheap: one extra field on `CoverageTransform`, collected during the existing `collect_logical_leaf_spans` walk, no new AST mutations, no runtime cost in instrumented output. Proposed shape: `bO: BTreeMap<String, Vec<{ operator, leafIndices }>>` — pure static index overlay, counts stay in `b`, merge semantics trivially inherited.
  - **Blocker:** no mainstream coverage reporter (codecov, Sonar, istanbul-reports HTML, Vitest UI) reads extra channels today. The existing `bT` channel (enabled via `reportLogic`) is barely consumed in the wild. Shipping `bO` alone would add a second orphan channel.
  - **Unblocks when:** we (or someone) ship a companion reporter that visualizes operator-level coverage — either as a custom HTML reporter in this repo or as a patch upstream. Feature + reporter must land together.
  - **Precedent:** see user-panel review 2026-04-15.
