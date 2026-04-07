# oxc_coverage_instrument

Istanbul-compatible JavaScript/TypeScript coverage instrumentation using the [Oxc](https://oxc.rs) parser.

Parses JS/TS source, identifies statements, functions, and branches, injects `__coverage__` counter expressions, and emits instrumented code with an Istanbul-compatible coverage map (`coverage-final.json` format).

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

## What it tracks

- **Statements**: every executable statement
- **Functions**: declarations, expressions, arrows, class methods
- **Branches**: if/else, ternary, switch cases, logical `&&`/`||`

## Output format

The coverage map serializes to Istanbul's `coverage-final.json` format. This is the same format consumed by Jest, Vitest, c8, nyc, Codecov, and most JS coverage tools.

```json
{
  "example.js": {
    "path": "example.js",
    "fnMap": { "0": { "name": "add", "line": 1, "decl": {...}, "loc": {...} } },
    "statementMap": { "0": { "start": { "line": 1, "column": 21 }, "end": {...} } },
    "branchMap": {},
    "f": { "0": 0 },
    "s": { "0": 0 },
    "b": {}
  }
}
```

## Why this exists

[`swc-coverage-instrument`](https://github.com/nicolo-ribaudo/swc-plugin-coverage-instrument) does the same thing for SWC and gets ~407K monthly npm downloads. This crate fills the same gap for the Oxc ecosystem: any tool built on `oxc_parser` can add coverage instrumentation without pulling in SWC or Babel.

Function names come from the same Oxc parser, so they are consistent with other Oxc-based tools analyzing the same source.

## Status

**v0.1.0**: working coverage map generation and source-level counter injection. Produces valid Istanbul output for the core JS/TS syntax set.

Not yet implemented:
- Source map support
- `/* istanbul ignore */` and `/* v8 ignore */` pragma handling
- AST-level mutation via `oxc_transformer::Traverse` (current approach uses source-level injection)

## Running the example

```bash
cargo run --example instrument
```

## License

MIT
