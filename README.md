# oxc_coverage_instrument

[![Crates.io](https://img.shields.io/crates/v/oxc_coverage_instrument.svg)](https://crates.io/crates/oxc_coverage_instrument)
[![docs.rs](https://docs.rs/oxc_coverage_instrument/badge.svg)](https://docs.rs/oxc_coverage_instrument)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Istanbul-compatible JavaScript/TypeScript coverage instrumentation, built on the [Oxc](https://oxc.rs) parser.

Takes JS/TS source, parses it with `oxc_parser`, identifies statements, functions, and branches, injects `__coverage__` counter expressions, and produces an Istanbul-compatible coverage map (`coverage-final.json` format).

## Why

[`swc-coverage-instrument`](https://github.com/nicolo-ribaudo/swc-plugin-coverage-instrument) fills this role for SWC (~407K monthly npm downloads, mostly via Next.js/Jest). There is no equivalent for the Oxc ecosystem. Any tool built on `oxc_parser` that needs coverage instrumentation currently has to pull in SWC or Babel.

This crate fills that gap. Function names come from the same Oxc parser, so they are consistent with other Oxc-based tools analyzing the same source.

## Install

```toml
[dependencies]
oxc_coverage_instrument = "0.1"
```

## Usage

```rust
use oxc_coverage_instrument::{instrument, InstrumentOptions};

let source = "function add(a, b) { return a + b; }";
let result = instrument(source, "add.js", &InstrumentOptions::default()).unwrap();

// Istanbul-compatible coverage map
assert_eq!(result.coverage_map.fn_map["0"].name, "add");
assert_eq!(result.coverage_map.statement_map.len(), 1);

// Instrumented source with counters injected
println!("{}", result.code);
```

Run the included example to see full output:

```bash
cargo run --example instrument
```

## What it tracks

| Dimension | What gets a counter |
|:----------|:-------------------|
| Statements | Every executable statement (variable declarations, return, throw, expression statements) |
| Functions | Declarations, expressions, arrow functions, class methods |
| Branches | `if`/`else`, ternary `? :`, `switch` cases, logical `&&`/`\|\|` |

## Output format

The coverage map serializes to Istanbul's `coverage-final.json` format. This is the same format consumed by Jest, Vitest, c8, nyc, Codecov, and most JS coverage tools.

```json
{
  "example.js": {
    "path": "example.js",
    "statementMap": {
      "0": { "start": { "line": 1, "column": 0 }, "end": { "line": 1, "column": 36 } }
    },
    "fnMap": {
      "0": { "name": "add", "line": 1, "decl": { ... }, "loc": { ... } }
    },
    "branchMap": {},
    "s": { "0": 0 },
    "f": { "0": 0 },
    "b": {}
  }
}
```

## API

### `instrument(source, filename, options) -> Result<InstrumentResult, InstrumentError>`

Main entry point. Parses and instruments a single source file.

**`InstrumentOptions`**

| Field | Type | Default | Description |
|:------|:-----|:--------|:------------|
| `coverage_variable` | `String` | `"__coverage__"` | Name of the global coverage object |
| `input_source_map` | `Option<String>` | `None` | Reserved for future source map support |

**`InstrumentResult`**

| Field | Type | Description |
|:------|:-----|:------------|
| `code` | `String` | Instrumented source with coverage counters |
| `coverage_map` | `FileCoverage` | Istanbul-compatible coverage data |
| `source_map` | `Option<String>` | Always `None` (source map support planned) |
| `unhandled_pragmas` | `Vec<UnhandledPragma>` | Coverage pragma comments that were not processed |

### Coverage types

All types derive `Serialize` and produce Istanbul-compatible JSON:

- `FileCoverage` -- per-file coverage data (statement/function/branch maps + hit counts)
- `FnEntry` -- function name, declaration span, body span
- `BranchEntry` -- branch type, location spans per arm
- `Location` / `Position` -- 1-based line, 0-based column source positions

## Architecture

```
source code
    |
    v
oxc_parser::Parser    -- parse to AST
    |
    v
CoverageVisitor       -- walk AST, collect statement/function/branch spans
    |
    v
FileCoverage          -- Istanbul-compatible coverage map
    |
    v
inject_counters()     -- insert __coverage__.s[N]++ / .f[N]++ / .b[N][M]++
    |
    v
instrumented code + coverage map
```

The coverage map is produced from a read-only AST walk (`Visit` trait). Counter injection currently operates at the source text level. Future versions will use `Traverse` for AST-level mutation followed by `oxc_codegen` for emission, which handles edge cases (arrow expressions, template literals) more reliably.

## Known limitations

- **Source-level injection**: counter insertion operates on source text, not AST nodes. This causes incorrect output for some arrow function expressions (e.g., `const f = (x) => x * 2` may produce concatenated counters). AST-level mutation via `Traverse` + `oxc_codegen` is the fix, planned for v0.2.0.
- **No source map support**: instrumented output does not include source maps. Line numbers in coverage reports will be shifted for TypeScript files. Planned for v0.2.0.
- **No pragma handling**: `/* istanbul ignore next */`, `/* istanbul ignore else */`, and `/* v8 ignore next */` comments are not yet processed. The `unhandled_pragmas` field in `InstrumentResult` is reserved for surfacing these. Planned for v0.2.0.
- **No branch coverage for `??` or `?.`**: nullish coalescing and optional chaining are not yet tracked as branches.

## Related projects

| Project | Language | AST | Notes |
|:--------|:---------|:----|:------|
| [`istanbul-lib-instrument`](https://github.com/istanbuljs/istanbuljs) | JavaScript | Babel | The original Istanbul instrumenter |
| [`swc-coverage-instrument`](https://github.com/nicolo-ribaudo/swc-plugin-coverage-instrument) | Rust | SWC | SWC equivalent (~407K monthly npm downloads) |
| [`istanbul-oxide`](https://crates.io/crates/istanbul-oxide) | Rust | None | Istanbul data types crate (from the SWC project) |
| **this crate** | Rust | Oxc | First Oxc-native coverage instrumenter |

## License

MIT
