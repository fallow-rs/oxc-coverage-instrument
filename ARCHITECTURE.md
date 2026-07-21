# Architecture

This document describes the high-level structure of the oxc-coverage suite: how
source flows through the crates, what each crate owns, and which concerns cut
across all of them. For usage, see the README of the crate you need.

## Bird's Eye View

Coverage counters and runtime setup are inserted as AST nodes. No regular
expression rewrites source, no generated setup is reparsed, and no semantic
rebuild follows instrumentation.

```
source code (JS/TS)
    |
    v
oxc_parser          parse to AST
    |
    v
SemanticBuilder     build scope tree
    |
    v
oxc_coverage_transform
                    traverse AST, inject counters, return neutral metadata
    |
    v
runtime setup       build setup AST and register scopes, symbols, references
    |
    v
oxc_codegen         emit instrumented code and source map
    |
    v
instrumented code + coverage map
```

`oxc_coverage_transform` is an unpublished, provisional Oxc extraction target.
It owns the `oxc_traverse` visitor, ignore semantics, collision-safe generated
bindings, counter mutation, and neutral ordered metadata. Its normal dependency
graph contains only Oxc AST, semantic, span, syntax, allocator, and traverse
layers. The published instrument crate adapts that output to Istanbul types,
applies source-map policy, and owns the runtime setup and source-to-source API.

The standalone adapter converts neutral Oxc spans to Istanbul positions, builds
the coverage-data literal as ordered AST, inserts the runtime setup after the
directive prologue, and updates the existing `Scoping` directly. Generated
setup nodes use synthetic spans, so codegen shifts original source mappings
without a post-codegen line-offset repair.

When an `inputSourceMap` is supplied, the instrumenter composes the codegen
output map with the input map, so downstream remappers (Vitest, nyc, monocart)
resolve coverage positions back to the original source. Composition is delegated
to [`srcmap-remapping`](https://crates.io/crates/srcmap-remapping), which mirrors
`@ampproject/remapping` semantics, the primitive `istanbul-lib-source-maps` and
the major bundlers rely on.

## Code Map

The workspace provides a broad Istanbul-compatible coverage pipeline, with one
crate per responsibility. It does not claim to port every Istanbul package or
reporter; the reporting crate implements the formats named below.

| Crate | Replaces |
|:------|:---------|
| `oxc_coverage_transform` (unpublished proposal) | reusable Oxc AST mutation primitive |
| [`oxc_coverage_instrument`](crates/oxc_coverage_instrument/README.md) | `istanbul-lib-instrument` |
| [`oxc_coverage_types`](crates/oxc_coverage_types/README.md) | `istanbul-lib-coverage` (data model) |
| [`oxc_coverage_source_maps`](crates/oxc_coverage_source_maps/README.md) | `istanbul-lib-source-maps` |
| [`oxc_coverage_v8`](crates/oxc_coverage_v8/README.md) | `v8-to-istanbul` |
| [`oxc_coverage_report`](crates/oxc_coverage_report/README.md) | `istanbul-lib-report` |
| [`oxc_coverage_reports`](crates/oxc_coverage_reports/README.md) | `istanbul-reports` (partial) |

Three crates are workspace-local rather than published to crates.io. The
provisional `oxc_coverage_transform` exists only for upstream boundary review.
[`oxc_coverage_instrument_cli`](crates/oxc_coverage_instrument_cli/README.md)
ships the `oxc-coverage-instrument` binary, and
[`oxc_coverage_instrument_napi`](crates/oxc_coverage_instrument_napi/README.md)
ships the `oxc-coverage-instrument` npm package.

At runtime the crates compose into one pipeline, from source on disk to a
rendered report.

```
   source code (JS/TS)
        |
        v
   +-----------------------------+
   | oxc_coverage_instrument     |  parse, transform, codegen
   +-----------------------------+
        |
        |  instrumented code + composed source map
        v
   +-----------------------------+
   | runtime collection          |  browser / Node / V8
   | (writes __coverage__)       |
   +-----------------------------+
        |
        |  raw FileCoverage objects
        v
   +-----------------------------+
   | oxc_coverage_source_maps    |  remap to original source paths
   +-----------------------------+
        |
        |  remapped FileCoverage
        v
   +-----------------------------+
   | oxc_coverage_report         |  tree, summary, visitor
   | oxc_coverage_reports        |  text, text-summary, json-summary,
   |                             |  lcov, cobertura, html
   +-----------------------------+
```

`oxc_coverage_types` sits underneath every box in that diagram. It defines
`FileCoverage`, `FnEntry`, `BranchEntry`, `Location`, and `Position`, and it owns
the serde representation that has to round-trip `coverage-final.json` unchanged.
Every other crate consumes those types rather than defining its own.

`oxc_coverage_v8` is a second entry point into the same model: instead of
instrumenting source, it fills the hit-count vectors of a `FileCoverage` from V8
inspector ranges, so V8-collected data joins the pipeline at the same place
runtime-collected `__coverage__` does.

With the experimental `ast-api` feature, an Oxc host can enter the first box at
the transform boundary. `instrument_program` accepts a host-owned `Program` and
`Scoping`, mutates the AST with counters and runtime setup, and returns coverage
metadata plus the complete updated `Scoping`. Parsing, TypeScript or JSX
lowering, and code generation remain owned by the host.

`oxc_coverage_instrument` re-exports the remap, V8, and data-model surfaces, so a
consumer that only needs the default path can depend on that one crate.

## Cross-Cutting Concerns

### Istanbul conformance

Output is checked against `istanbul-lib-instrument` on a shared fixture corpus
covering every branch type, function form, Unicode columns, pragma boundaries,
hashbangs, directive prologues, binding collisions, class fields, stripped
TypeScript, and edge cases. The corpus lives in
`crates/oxc_coverage_instrument/tests/conformance/`. The suite asserts that
statement, function, and branch counts match exactly, that branch types and
per-branch location counts match, that the JSON field set matches, and that the
instrumented output re-parses as valid JavaScript.

CI also runs a blocking byte-for-byte diff over the same corpus under the strict
Istanbul compatibility profile, without divergence filters. A separate gate
compares the default profile with that strict shape and permits only documented
name, method-span, synthetic-else, logical-assignment, and optional-chain
extensions. The default Oxc extensions are enumerated in
[the instrumenter's README](crates/oxc_coverage_instrument/README.md#differences-from-istanbul-lib-instrument),
and `scripts/istanbul-diff.mjs` is the tool that enforces profile parity.

### Position semantics

Istanbul's `Position` is a 1-based line plus a 0-based UTF-16 column. Every
public `Location` in this workspace uses that convention, so `statementMap`,
`fnMap`, `branchMap`, and `unhandledPragmas` columns match Babel and
`istanbul-lib-instrument` on non-ASCII sources.

The internal representations disagree, and each boundary converts explicitly:

- Oxc spans are UTF-8 byte offsets.
- V8 inspector ranges are absolute UTF-16 code-unit offsets.
- `srcmap-sourcemap`'s `original_position_for` is 0-based on both axes.

Conversion happens at the lookup boundary in each crate rather than being pushed
into the shared types, which keeps `oxc_coverage_types` free of any encoding
assumption.

### Benchmarks

`./scripts/benchmark-comparison.sh` times this instrumenter against
`istanbul-lib-instrument`, `babel-plugin-istanbul`, and
`swc-plugin-coverage-instrument` on five pinned library builds (React, lodash,
Vue, D3, three.js), reporting the median of several runs per file. The Node.js
table runs every tool in one process, so the numbers are comparable; a second
table times the native CLI, which pays process startup on top.

The script downloads the pinned library builds but installs the current release
of each competing instrumenter, so results are only meaningful alongside the
resolved versions and the machine they were produced on. Run it locally rather
than quoting a number from elsewhere.

`swc-plugin-coverage-instrument` is written in Rust but runs as a WASM module
inside SWC's plugin sandbox, so its numbers include WASM and serialization
overhead at every AST boundary rather than measuring native Rust.

CodSpeed runs the Rust benchmarks under `crates/*/benches/` on every push to
`main` and every pull request that touches `crates/` or a workspace manifest.
The provisional transform crate has its own shard. Its setup phase clones the
input program and builds semantic state outside measurement; the measured
routine contains only coverage traversal and AST mutation.

That shard is a regression gate for the kernel, not proof of a Rolldown or
Vitest speedup. The integration claim must compare complete pipelines on the
same real-world modules and account for the parse, lowering, semantic, and
codegen phases the host can reuse. The AST-native prototype setup is a host
contract proof, not a claimed standalone throughput improvement.
