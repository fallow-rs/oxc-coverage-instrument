# oxc_coverage_instrument

[![CI](https://github.com/fallow-rs/oxc-coverage-instrument/actions/workflows/ci.yml/badge.svg)](https://github.com/fallow-rs/oxc-coverage-instrument/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/fallow-rs/oxc-coverage-instrument/badges/coverage.json)](https://github.com/fallow-rs/oxc-coverage-instrument/actions/workflows/coverage.yml)
[![Crates.io](https://img.shields.io/crates/v/oxc_coverage_instrument.svg)](https://crates.io/crates/oxc_coverage_instrument)
[![npm](https://img.shields.io/npm/v/oxc-coverage-instrument.svg)](https://www.npmjs.com/package/oxc-coverage-instrument)
[![docs.rs](https://docs.rs/oxc_coverage_instrument/badge.svg)](https://docs.rs/oxc_coverage_instrument)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Istanbul-compatible JavaScript/TypeScript coverage instrumentation, built on the [Oxc](https://oxc.rs) parser. **8-44x faster** than existing tools.

## Why

[`swc-coverage-instrument`](https://github.com/kwonoj/swc-plugin-coverage-instrument) fills this role for SWC. There is no equivalent for the Oxc ecosystem. Any tool built on `oxc_parser` that needs coverage instrumentation currently has to pull in SWC or Babel.

This crate fills that gap. AST-level instrumentation via `oxc_traverse` + `oxc_codegen` produces correct Istanbul-compatible output, verified against the canonical `istanbul-lib-instrument` on 27 shared fixtures.

## Install

### Rust

```toml
[dependencies]
oxc_coverage_instrument = "0.10"
```

### Node.js

```bash
npm install oxc-coverage-instrument
```

### CLI

```bash
cargo install --git https://github.com/fallow-rs/oxc-coverage-instrument oxc-coverage-instrument-cli
```

## Usage

### Rust

```rust
use oxc_coverage_instrument::{instrument, InstrumentOptions};

let source = "function add(a, b) { return a + b; }";
let result = instrument(source, "add.js", &InstrumentOptions::default()).unwrap();

// Istanbul-compatible coverage map
assert_eq!(result.coverage_map.fn_map["0"].name, "add");

// Instrumented source with counters injected
println!("{}", result.code);
```

### Node.js

```javascript
import { instrument } from 'oxc-coverage-instrument';

const result = instrument(source, 'file.js', {
  coverageVariable: '__coverage__',  // optional
  sourceMap: true,                    // optional
});

result.code;                          // instrumented source
const coverageMap = JSON.parse(result.coverageMap);  // Istanbul format
result.sourceMap;                     // source map JSON (if enabled)
```

### CLI

```bash
# Print instrumented code to stdout
oxc-coverage-instrument src/app.js

# Write to file
oxc-coverage-instrument src/app.js -o dist/app.js

# Print coverage map only
oxc-coverage-instrument src/app.js --coverage-map

# With source map
oxc-coverage-instrument src/app.js -o dist/app.js --source-map
```

When `-o/--output` is used, the CLI writes the instrumented code to that path and writes the Istanbul coverage map next to it as `<output>.map.json`.

### Vitest integration

Requires `vitest` and `@vitest/coverage-istanbul` **>= 4.1.5** (`coverage.instrumenter` option).

```typescript
import { defineConfig } from 'vitest/config'
import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest'

export default defineConfig({
  test: {
    coverage: {
      provider: 'istanbul',
      instrumenter: (options) => createOxcInstrumenter(options),
    }
  }
})
```

The factory forwards `coverageVariable` and `ignoreClassMethods` to the native instrumenter. Everything else in the Istanbul provider (collection, merging, reporting) keeps working unchanged.

By default the adapter auto-detects raw TypeScript: when the filename ends in `.ts` / `.tsx` (also `.mts` / `.cts`) AND no `inputSourceMap` was supplied (the source has not already been transformed by Vite / Babel / tsc), it runs an in-process TypeScript-strip pass so the output is executable JavaScript and the coverage map points at the original `.ts` source. No `@babel/preset-typescript` or upstream tsc step is required. Set `stripTypescript: false` on `createOxcInstrumenter({ ... })` to disable the auto-detect when running under a toolchain that pre-transforms TypeScript without emitting an `inputSourceMap` (`@vitejs/plugin-react-swc` in some configurations, Bun's native TS runner, Node 23+ with `--experimental-strip-types`). Set `stripTypescript: true` to force the strip pass regardless of filename or `inputSourceMap` presence.

#### Legacy decorators (NestJS, Angular, TypeORM, class-validator)

By default decorator syntax (Stage 3 and legacy `experimentalDecorators` alike) flows through the strip pass verbatim: counters land on the surrounding class bodies and methods, and a downstream tool (Babel / tsc / SWC / native Node decorators) is responsible for lowering them at runtime. Set `experimentalDecorators: true` on the instrument call (or on `createOxcInstrumenter({ ... })`) to lower legacy decorators in-process into `_decorate(...)` calls; set `emitDecoratorMetadata: true` additionally to emit `_decorateMetadata("design:type", ...)`, `_decorateMetadata("design:paramtypes", ...)`, and `_decorateMetadata("design:returntype", ...)` calls. The latter is what NestJS dependency injection, TypeORM column type inference, and class-validator's metadata-driven validation rely on. `createOxcInstrumenter` auto-promotes `emitDecoratorMetadata: true` to also enable `experimentalDecorators` (matching `tsconfig.json` semantics); the bare `instrument()` napi entry point rejects the same combination with an `Error` so the underlying Rust `DecoratorMode` enum keeps invalid states unrepresentable.

The lowered output imports helpers from [`@oxc-project/runtime`](https://www.npmjs.com/package/@oxc-project/runtime):

```bash
npm install @oxc-project/runtime
# or: pnpm add @oxc-project/runtime, yarn add @oxc-project/runtime
```

If you see `Cannot find module '@oxc-project/runtime/helpers/decorate'` at test time, the package is missing from the consumer; install it as a regular dependency (NOT a dev dependency) so it's available wherever the instrumented code runs. Mirrors the relationship between `babel-plugin-transform-typescript-metadata` and `@babel/runtime` in a Babel-based pipeline.

### vite-plugin-istanbul integration

Requires `vite-plugin-istanbul` **>= 9.0.0** (`instrumenter` option).

```typescript
import { defineConfig } from 'vite'
import istanbul from 'vite-plugin-istanbul'
import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest'

export default defineConfig({
  plugins: [
    istanbul({
      instrumenter: createOxcInstrumenter(),
      include: ['src/**/*.{js,ts,jsx,tsx}'],
      exclude: ['node_modules', 'test/'],
    }),
  ],
})
```

### Custom Vite/Rollup plugin

If you're not using `vite-plugin-istanbul`, you can call `instrument` directly from a `transform` hook:

```javascript
import { instrument } from 'oxc-coverage-instrument';

export function coveragePlugin() {
  return {
    name: 'coverage-instrument',
    transform(code, id) {
      if (process.env.COVERAGE && /\.[jt]sx?$/.test(id) && !id.includes('node_modules')) {
        const result = instrument(code, id, { sourceMap: true });
        return { code: result.code, map: result.sourceMap ? JSON.parse(result.sourceMap) : undefined };
      }
    },
  };
}
```

### Reading existing coverage data

```rust
use oxc_coverage_instrument::parse_coverage_map;

// Parse a coverage-final.json file
let json = std::fs::read_to_string("coverage-final.json").unwrap();
let map = parse_coverage_map(&json).unwrap();

for (path, coverage) in &map {
    println!("{}: {} statements, {} functions, {} branches",
        path, coverage.s.len(), coverage.f.len(), coverage.b.len());
}
```

### Remapping coverage through `inputSourceMap`

`remap_coverage` walks a `FileCoverage` through its embedded `inputSourceMap`, rewriting every position back to the original source. This is the Mode A flow Vitest's istanbul reporter uses. When the map sits next to the instrumented file on disk rather than embedded, `remap_coverage_with_loader` accepts a loader callback (matching `istanbul-lib-source-maps`'s `sourceStore` semantics used by nyc):

```rust
use oxc_coverage_instrument::remap_coverage_with_loader;

let remapped = remap_coverage_with_loader(&fc, |path| {
    std::fs::read_to_string(format!("{path}.map")).ok()
});
```

The single-file helpers return `Some` only when the mapped locations belong to
one original file. Use `oxc_coverage_source_maps::remap_coverage_to_map` when
one generated bundle contains mappings for several sources. Coverage-map
helpers and `SourceMapStore::transform_coverage_map` use that one-to-many path
automatically, so each original source receives its own `FileCoverage`.

When generated chunks map to the same original path, their coverage is merged
instead of replaced. Statements merge by location, functions by declaration
location, and branches by ordered arm locations, matching
`istanbul-lib-source-maps`. The first function or branch supplies metadata such
as name, body, type, and umbrella location. Matching `s`, `f`, `b`, and `bT`
counts use saturating `u32` addition. The optional
`x_fallow_functionMap` follows renumbered function ids; if equivalent functions
carry conflicting identities, the optional overlay is dropped rather than
selecting one record. Statement-only chunks do not affect overlay completeness.

For runners (Jest with `transform`, plugins that hand maps in incrementally) that want continuous remapping during collection, `SourceMapStore` accumulates per-file maps via `add_map` and applies them via `transform_coverage`.

Surviving positions are resolved through an istanbul `getMapping`-equivalent range remap, not a direct per-position lookup: starts resolve with greatest-lower-bound and ends resolve to the next original segment (or the end of the original line), byte-for-byte with `createSourceMapStore().transformCoverage`. So an exclusive end that lands between source-map segments widens to the full token instead of truncating backward, and a sub-segment span (e.g. a 1-char arrow declaration) balloons to its enclosing span. Line numbers and coverage percentages are unaffected, but because `istanbul-lib-coverage`'s `keyFromLoc` includes columns, flush pre-`0.4.0` coverage caches before merging them with newer runs.

By default, positions whose source-map lookup returns `None` are silently kept at their generated-output coordinates for an unambiguous single-source map (matching the legacy Mode A flow). An unmapped position in a multi-source map has no safe original owner and is omitted. Pass `RemapOptions { drop_unmapped: true }` (or `{ dropUnmapped: true }` from JS) to drop statement / function / branch entries that fail to remap, along with their matching `s` / `f` / `b` / `bT` slots. Drop semantics align with `istanbul-lib-source-maps`'s `transformer.js`: statements drop when start or end fails, functions drop when any of `decl` / `loc` start or end fails, and branch arms drop per arm. The whole branch drops when no arms survive or retained mapped arms disagree on their source. Branch ownership comes from those arms, so an unmapped umbrella falls back to the first retained arm and a mapped umbrella remains branch metadata even when it resolves elsewhere. Use this when instrumenting compiler-emitted boilerplate that has no original-source mapping (e.g. the `?vue&type=script` chunk produced by `@vitejs/plugin-vue`) to keep downstream Istanbul reporters from rendering chunk-line positions against `.vue` paths.

```rust
use oxc_coverage_instrument::{remap_coverage_map_with_options, RemapOptions};

let remapped =
    remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
```

```js
import { remapCoverageMap } from 'oxc-coverage-instrument';

const remapped = JSON.parse(
    remapCoverageMap(JSON.stringify(coverageMap), { dropUnmapped: true }),
);
```

#### Composing eagerly during `instrument()`

The flow above is lazy: `instrument()` embeds the `inputSourceMap` and a downstream `remapCoverageMap` walks every entry back to the original source at report time. For collectors that dump `window.__coverage__` directly (Playwright E2E, per-test snapshots), that round-trip happens once per collected file. Set `composeInputSourceMap: true` (Rust: `compose_input_source_map: true`) alongside `inputSourceMap` to fold the map in once, during instrumentation:

```js
const { code, coverageMap } = instrument(intermediateJs, 'intermediate.js', {
    inputSourceMap: JSON.stringify(inputSourceMap),
    composeInputSourceMap: true,
});
// coverageMap is keyed by the original source path with original-source
// positions and carries no inputSourceMap. The runtime __coverage__ baked into
// `code` is keyed the same way, so collection is just:
//   const raw = await page.evaluate(() => window.__coverage__);
//   writeFileSync(out, JSON.stringify(raw));
// remapCoverageMap(raw) is then a no-op.
```

An entry is dropped exactly when the lazy `remapCoverageMap` path with `dropUnmapped: true` would drop it: both resolve each span through the same istanbul `getMapping` lookup (greatest-lower-bound start with a least-upper-bound fallback, plus the matching end resolution), so a statement whose generated column sits just before its line's first mapping is kept by both rather than dropped by one (issue #122). The eager path bakes positions into the runtime `__coverage__` literal with no later remap opportunity, so an unmapped entry would otherwise be stranded at its generated coordinate and re-keyed past the end of the original file (e.g. the compiler boilerplate in a `?vue&type=script` chunk that has no mapping back to the `.vue`). Dropping is unconditional here; there is no keep-generated-position option at instrument time. The composed result therefore keeps the same surviving entries at the same original-source positions as instrument-without-compose followed by `remap_coverage_with_options(.., RemapOptions { drop_unmapped: true })`; the two are byte-identical when nothing is dropped, and when entries drop the eager path renumbers the surviving statement / function / branch ids contiguously while the lazy path preserves the original ids with gaps (istanbul treats these as equivalent, since it merges entries by location). When the input map is unusable (declares no source, fails to parse), composition backs off and the `inputSourceMap` is left embedded so the lazy path still works (and can apply its own drop policy). The flag has no effect when `inputSourceMap` is unset. The `x_fallow_functionMap` overlay, if requested, keeps its pre-remap-derived ids in both the eager and lazy paths (the remap pipeline does not rewrite it).

### Converting V8 byte-range coverage to Istanbul

`v8_to_istanbul` accepts the same shape Node's inspector and `@vitest/coverage-v8` emit. With block coverage enabled, statement/function/branch counts are populated by intersecting V8 ranges with locations recovered from a visit-only AST pass. Inline `//# sourceMappingURL=data:...` trailers are decoded automatically; external map references resolve through the optional loader on `v8_to_istanbul_with_loader`.

```rust
use oxc_coverage_instrument::v8_to_istanbul_with_loader;

let fc = v8_to_istanbul_with_loader(source, "app.js", &functions, 0, |url| {
    std::fs::read_to_string(url).ok()
})?;
```

## What it tracks

| Dimension | What gets a counter |
|:----------|:-------------------|
| **Statements** | Every executable statement |
| **Functions** | Declarations, expressions, arrows, class methods |
| **Branches** | `if`/`else`, ternary, `switch`, `&&`/`\|\|`/`??`, `??=`/`\|\|=`/`&&=`, `default-arg` |
| **Pragmas** | `istanbul`/`v8`/`c8 ignore next/if/else/file/start/stop` |

### Function identity overlay (Fallow extension)

Set `functionIdentityOverlay: true` on the `instrument` call or `createOxcInstrumenter` Vitest adapter (or `function_identity_overlay: true` in Rust) to attach an optional `x_fallow_functionMap` to the resulting coverage map. The overlay carries a `fallow:fn:<8 hex>` identity per function, keyed by the same ids as `fnMap`, computed as `SHA-256(path + name + decl.start.line + "function")` truncated to the first 4 bytes. This is bit-equal to `fallow_cov_protocol::function_identity_id`, so consumers in the fallow ecosystem can join the overlay directly against V8 dumps, Istanbul ingesters, and source-mapped findings without recomputing.

Renames or moving the function to a different line change the id; column-level edits on the same line do not. Columns survive on the overlay's `decl` / `loc` fields for display and same-line disambiguation, but are deliberately excluded from the hash so producers observing the same function with different positional fidelity all agree on the id.

This is a non-Istanbul extension. Standard Istanbul consumers ignore the `x_`-prefixed field, so default output (option off) stays byte-identical to what nyc / Vitest / Jest / Codecov expect. When `inputSourceMap` is consumed, the overlay still references the pre-remap positions; consumers that remap downstream must recompute identity against the post-remap positions.

The path enters the hash verbatim from the `filename` argument, so callers that need stable ids across different tools must normalise paths before instrumentation (`./app.js`, `app.js`, and `/abs/repo/app.js` all hash differently). Pick one canonical form per project, typically a workspace-root-relative POSIX path.

## Istanbul conformance

Verified against `istanbul-lib-instrument` on 27 shared fixtures covering all branch types, function forms, Unicode columns, pragma boundaries, and edge cases. 189 automated conformance checks validate:

- Function counts match exactly
- Branch counts, types, and location counts match exactly
- Statement counts match exactly
- JSON structure matches Istanbul's field set
- Instrumented output re-parses as valid JS

CI also runs a blocking byte-for-byte Istanbul diff over the shared fixture corpus after filtering documented intentional divergences. This catches span-level and counter-shape drift that count-only tests can miss.

Real-world verification: **1,061 TS/TSX/JS files** from a production React monorepo produce byte-for-byte identical statement, function, and branch counts to `istanbul-lib-instrument` (when both instrumenters receive the same Babel-transpiled input).

Independently validated against the Vitest test suite: from v0.3.5 onward, `coverage-final.json` for the Vitest `math.ts` fixture is byte-for-byte identical to `@vitest/coverage-istanbul`'s output — including `statementMap`, `fnMap` spans, `branchMap`, and all counter arrays.

**Column conventions:** all `start.column` / `end.column` values in `statementMap`, `fnMap`, `branchMap`, and `unhandledPragmas` are reported as **UTF-16 code units** (JavaScript string indices), matching Babel and `istanbul-lib-instrument`. Sources containing non-ASCII characters — `π`, accented identifiers, emoji — produce the same column numbers as the reference tool. Verified by the `26-non-ascii-identifiers.js` conformance fixture (`crates/oxc-coverage-instrument/tests/conformance/fixtures/`).

## Differences from istanbul-lib-instrument

Intentional divergences from `istanbul-lib-instrument`:

### 1. ES2021 logical-assignment operators are instrumented as branches

`x ??= y`, `x ||= y`, and `x &&= y` each contain a genuine short-circuit conditional: the right-hand side is evaluated (and the assignment happens) only when the left operand matches the operator's polarity. `oxc-coverage-instrument` emits one `binary-expr` branch entry per logical-assignment with two locations (left = always reached, right = conditional). `istanbul-lib-instrument` has no `AssignmentExpression` visitor entry and emits zero branches for these operators.

Pinned by `crates/oxc-coverage-instrument/tests/conformance_test.rs::logical_assignment_is_intentional_branch_superset`.

### 2. Inferred function names over `(anonymous_N)`

For anonymous function expressions assigned to a variable or declared as a class method, `oxc-coverage-instrument` uses the name the JavaScript runtime actually assigns to `Function.prototype.name`:

| Source | oxc `fnMap[].name` | istanbul `fnMap[].name` |
|---|---|---|
| `const f = function() {}` | `f` | `(anonymous_0)` |
| `const g = () => 1` | `g` | `(anonymous_0)` |
| `class C { bar() {} }` | `bar` | `(anonymous_0)` |
| `(function() {})()` (IIFE) | `(anonymous_0)` | `(anonymous_0)` |

Coverage reports and stack traces benefit from real names. Pinned by `crates/oxc-coverage-instrument/tests/conformance_test.rs::fn_name_inference_is_intentional_superset`.

### 3. Full method-key spans in `fnMap[*].decl`

For class and object methods, `oxc-coverage-instrument` records the whole method key as the declaration span. `istanbul-lib-instrument` truncates method declarations to the key's first character.

| Source | oxc `decl` | istanbul `decl` |
|---|---|---|
| `class C { bar() {} }` | `bar` | `b` |

The byte-diff check still pins the method declaration start, line, body `loc`, and all non-method function declaration spans.

### 4. Real Location coordinates for synthetic `else` arms

For an `if` with no `else` clause, `istanbul-lib-instrument` records the synthetic alternate slot as `{ start: {}, end: {} }` (an empty placeholder). `oxc-coverage-instrument` anchors the slot as a real zero-width `Location` at the consequent's end. Reporters that pull `loc.start.line` on every arm crash on the empty form; the real coordinates make the slot safe to walk without special-casing.

The same applies to the surviving arm when `/* istanbul ignore if */` drops the consequent of a no-else `if`.

### 5. Optional-chain `?.` short-circuits tracked as branches

Each `?.` link surfaces in `branchMap` as an `optional-chain` entry with two arms: arm 0 when the observed value is `null`/`undefined` (the link short-circuits), arm 1 when the link continues. `istanbul-lib-instrument` does not track these. Reporters that walk `branchMap` by type-agnostic shape pick up the new entries automatically; reporters that hard-code the istanbul type names need to learn the new label.

Set `trackOptionalChainBranches: false` (Rust: `track_optional_chain: false`; vitest adapter: `createOxcInstrumenter({ trackOptionalChainBranches: false })`) to opt out: optional chains are left native, with no `_oc` helper and no `optional-chain` branches. This matches `istanbul-lib-instrument` byte-for-byte on `?.` and removes the per-operand helper-call overhead in optional-chain-dense hot paths. Statement, function, and other branch coverage are unaffected. Defaults to `true`.

### 6. Callback-argument names from the callee (opt-in, off by default)

Section 2 recovers names from a binding (variable declarator, property key, class method, assignment target). A function that is passed *directly as a call or `new` argument* has no such binding, so both `istanbul-lib-instrument` and this instrumenter fall back to `(anonymous_N)`. In callback-heavy code (route handlers, `.map`/`.filter`, promise `.then`, `describe`/`it`, `new Promise`) that fallback dominates the `fnMap`.

Set `nameCallbackArguments: true` (Rust: `name_callback_arguments: true`; vitest adapter: `createOxcInstrumenter({ nameCallbackArguments: true })`) to name these from the callee:

| Source | `fnMap[].name` with the option | default |
|---|---|---|
| `arr.map((x) => x)` | `map` | `(anonymous_0)` |
| `el.addEventListener("click", () => {})` | `addEventListener` | `(anonymous_0)` |
| `new Promise((resolve) => {})` | `Promise` | `(anonymous_0)` |
| `(function () {})()` (IIFE, callee not argument) | `(anonymous_0)` | `(anonymous_0)` |

Only the callee is used, never a sibling string argument (a route path or a test description): the traversal ancestor for an argument position exposes the callee but not the other arguments. A binding name (section 2) and an explicit named function expression both still take precedence; this only replaces the `(anonymous_N)` fallback. Because the name comes from the callee rather than a running counter, it is also **stable across rebuilds** (the `(anonymous_N)` index renumbers whenever an unrelated function is added), which matters for tools that key function identity on the name. Defaults to `false`, so the default output stays byte-identical to what Istanbul consumers expect.

**Migration from `@vitest/coverage-istanbul`:** a codebase that uses `??=`/`||=`/`&&=` heavily will see a higher branch-coverage denominator (and so a slightly lower branch %) after switching providers. To rebaseline CI thresholds after the swap:

```bash
vitest run --coverage --coverage.reporter=json-summary
jq '.total.branches.pct' coverage/coverage-summary.json
```

This is additional coverage signal, not a regression. Every extra branch represents a real runtime decision path.

## Performance

Benchmarked on real-world JavaScript libraries, all running in the same Node.js process for a fair comparison. Reproduce with `./scripts/benchmark-comparison.sh`.

| File | Size | oxc (napi) | babel-plugin-istanbul | swc-plugin (wasm) | istanbul-lib |
|:-----|:-----|:-----------|:----------------------|:------------------|:-------------|
| react.development.js | 107 KB | **1.8 ms** | 19.2 ms | 26.5 ms | 72.7 ms |
| lodash.js | 531 KB | **7.4 ms** | 57.0 ms | 100.1 ms | 226.3 ms |
| vue.global.js | 462 KB | **12.4 ms** | 125.1 ms | 225.5 ms | 548.3 ms |
| d3.js | 573 KB | **22.7 ms** | 192.9 ms | 311.1 ms | 773.8 ms |
| three.js | 1.2 MB | **30.7 ms** | 293.6 ms | 449.0 ms | 1094.0 ms |

**8-11x** faster than babel-plugin-istanbul, **13-18x** faster than swc-plugin-coverage-instrument (Rust/WASM), **30-44x** faster than istanbul-lib-instrument.

> **Note:** swc-plugin-coverage-instrument is written in Rust but runs as a WASM module inside SWC's sandbox, adding serialisation overhead at every AST boundary. The comparison measures end-to-end instrumentation time as users experience it.

## Architecture

```
source code (JS/TS)
    |
    v
oxc_parser          -- parse to AST
    |
    v
SemanticBuilder     -- build scope tree
    |
    v
CoverageTransform   -- traverse AST, inject ++cov().s[N] counters
    |
    v
oxc_codegen         -- emit instrumented code + source map
    |
    v
instrumented code + coverage map
```

## Coverage stack

This repository now contains a Rust-native coverage suite: instrumentation, Istanbul data types, source-map remapping, V8-to-Istanbul conversion, report-tree summarization, and report emitters.

```
   source code (JS/TS)
        |
        v
   +-----------------------------+
   | oxc-coverage-instrument     |   this crate
   | (parse, transform, codegen) |   AST-level Istanbul counters
   +-----------------------------+
        |
        |  instrumented code + composed source map
        v
   +-----------------------------+
   | runtime collection          |   browser / Node / V8
   | (writes __coverage__)       |
   +-----------------------------+
        |
        |  raw FileCoverage objects
        v
   +-----------------------------+
   | source map remap            |   oxc_coverage_source_maps
   | (-> original source paths)  |
   +-----------------------------+
        |
        |  remapped FileCoverage
        v
   +-----------------------------+
   | report                      |   oxc_coverage_report (tree, summary, visitor)
   | (text, text-summary,        |   oxc_coverage_reports (renderers)
   |  json-summary, lcov,        |
   |  cobertura, html)           |
   +-----------------------------+
```

When an `inputSourceMap` is supplied, the instrumenter composes the codegen's output map with the input map so that downstream remappers (Vitest, nyc, monocart) resolve coverage positions all the way back to the original source. Composition is delegated to [`srcmap-remapping`](https://crates.io/crates/srcmap-remapping), which mirrors `@ampproject/remapping` semantics (the same primitive `istanbul-lib-source-maps` and every major bundler rely on).

### Suite status

| Crate | Status | Replaces |
|:------|:-------|:---------|
| [`oxc_coverage_instrument`](https://crates.io/crates/oxc_coverage_instrument) | shipped | `istanbul-lib-instrument` |
| [`oxc_coverage_types`](https://crates.io/crates/oxc_coverage_types) | shipped | `istanbul-lib-coverage` (data model) |
| [`oxc_coverage_source_maps`](https://crates.io/crates/oxc_coverage_source_maps) | shipped | `istanbul-lib-source-maps` |
| [`oxc_coverage_v8`](https://crates.io/crates/oxc_coverage_v8) | shipped | `v8-to-istanbul` (npm) |
| [`oxc_coverage_report`](https://crates.io/crates/oxc_coverage_report) | shipped | `istanbul-lib-report` |
| [`oxc_coverage_reports`](https://crates.io/crates/oxc_coverage_reports) (`text`, `text-summary`, `json-summary`, `lcov`, `cobertura`, `html`) | shipped | `istanbul-reports` (partial) |

Use the new `report` subcommand to render any of the available formats:

```bash
oxc-coverage-instrument report --format text         coverage-final.json
oxc-coverage-instrument report --format text-summary coverage-final.json
oxc-coverage-instrument report --format json-summary coverage-final.json -o coverage-summary.json
oxc-coverage-instrument report --format lcov         --root . coverage-final.json -o lcov.info
oxc-coverage-instrument report --format cobertura    --root . coverage-final.json -o cobertura.xml
oxc-coverage-instrument report --format html         --root . coverage-final.json --output-dir coverage/

# Gate CI on aggregate line coverage after rendering the chosen format
oxc-coverage-instrument report --format text-summary coverage-final.json --fail-under 80
```

The `lcov` and `cobertura` formats use `--root` to relativize source-file paths in the output (defaults to the cwd); repo-relative paths are required by Codecov self-hosted, GitLab MR widget, Jenkins, and Azure DevOps.

The `html` format writes a self-contained directory tree (`--output-dir`, defaults to `coverage/`). Per-file detail pages show the original source with per-line hit / miss / partial-branch coloring; pages read source from disk via `--root`. Source-map remapping is wired in: files carrying an `inputSourceMap` are remapped through `oxc_coverage_source_maps` so TypeScript / JSX projects show original source rather than the instrumented JS. The reporter includes sortable tables, a filter box, explicit auto/light/dark theme toggle, server-side syntax highlighting, strict offline CSP, copyable line anchors, and a "Next uncovered" jump button.

## Related projects

| Project | AST | Notes |
|:--------|:----|:------|
| [`istanbul-lib-instrument`](https://github.com/istanbuljs/istanbuljs) | Babel | The canonical Istanbul instrumenter |
| [`babel-plugin-istanbul`](https://github.com/istanbuljs/babel-plugin-istanbul) | Babel | Babel plugin wrapper around istanbul-lib-instrument |
| [`swc-plugin-coverage-instrument`](https://github.com/kwonoj/swc-plugin-coverage-instrument) | SWC | SWC WASM plugin |
| **this crate** | Oxc | Native Rust, 8-44x faster |

## Compatibility

- **Rust**: 1.92+ (2024 edition)
- **Oxc**: 0.126.x
- **Istanbul**: `coverage-final.json` v3+ format
- **Node.js**: 18+ (via napi-rs)

### Runtime matrix

`oxc-coverage-instrument` ships prebuilt native bindings for seven platforms plus WebAssembly fallbacks (`@oxc-coverage-instrument/binding-wasm32-wasi` for runtimes with `SharedArrayBuffer`, and a single-threaded `wasm32-wasip1` variant for runtimes that disallow shared memory). The wasm binding is selected automatically when no matching native binary is available, or explicitly via `NAPI_RS_FORCE_WASI` (see [Forcing the WASM binding](#forcing-the-wasm-binding) below).

| Runtime | Status | Notes |
|:--------|:-------|:------|
| Node 18+ (darwin/linux/win32, x64/arm64) | native | Preferred when a matching `.node` is installed. |
| Node 22 LTS (any arch) | wasm fallback | Loads via `node:wasi`; emits an experimental-feature warning. |
| Deno 2.x | wasm | Uses Deno's `node:wasi` polyfill. Live deploy smoke tracked in [#88](https://github.com/fallow-rs/oxc-coverage-instrument/issues/88). |
| Browser (with COOP/COEP) | threaded wasm | Imports the `browser` export. Uses the threaded `wasm32-wasip1-threads` binding when `SharedArrayBuffer` is available (set `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp` on the host page) and an ESM bundler with top-level await support is present. webpack 4 and Parcel 1 are not supported. |
| Browser (without COOP/COEP) | single-threaded wasm | Falls back to the `wasm32-wasip1` binding when `SharedArrayBuffer` is unavailable. |
| Bun (supported targets) | native | Bun's `node:wasi` is incomplete ([oven-sh/bun#16156](https://github.com/oven-sh/bun/issues/16156)); falls back to the native binding. |
| Cloudflare Workers | single-threaded wasm | Workers lacks `SharedArrayBuffer`; the `browser` export selects the `wasm32-wasip1` binding. `examples/cloudflare-workers/` verifies Worker `FileCoverage` against native output byte-for-byte. |
| StackBlitz / WebContainer | partial | Newer WebContainer images load the wasm binding; older ones may fall through. Live smoke tracked in [#88](https://github.com/fallow-rs/oxc-coverage-instrument/issues/88). |

See `examples/wasm-node/` for a complete end-to-end Node smoke that exercises native plus WASM. `examples/cloudflare-workers/` contains the secret-free Workers acceptance smoke that compares Worker `FileCoverage` with native output byte-for-byte.

#### Forcing the WASM binding

Set `NAPI_RS_FORCE_WASI` to opt out of the native binding even when one is available. Useful for verifying parity locally or for CI gates that must exercise the WASM code path.

| Value | Behavior | When to use |
|:------|:---------|:------------|
| `1` (or any truthy non-`error` value) | Force the WASM binding. If the WASM binding cannot be loaded, fall back to the native binding without raising. | Local debugging; pre-release verification on a developer machine. |
| `error` | Force the WASM binding. If the WASM binding cannot be loaded, throw with a diagnostic error and do not fall back. | CI jobs that gate on the WASM path; release verification that must fail loudly if the WASM build is broken. |

The variable is read directly by the napi-rs 3 loader, so changing or replacing it would require forking the wrapper. Documented for completeness; most users will never need to set it.

## License

MIT
