# oxc_coverage_instrument

[![CI](https://github.com/fallow-rs/oxc-coverage-instrument/actions/workflows/ci.yml/badge.svg)](https://github.com/fallow-rs/oxc-coverage-instrument/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/fallow-rs/oxc-coverage-instrument/badges/coverage.json)](https://github.com/fallow-rs/oxc-coverage-instrument/actions/workflows/coverage.yml)
[![Crates.io](https://img.shields.io/crates/v/oxc_coverage_instrument.svg)](https://crates.io/crates/oxc_coverage_instrument)
[![npm](https://img.shields.io/npm/v/oxc-coverage-instrument.svg)](https://www.npmjs.com/package/oxc-coverage-instrument)
[![docs.rs](https://docs.rs/oxc_coverage_instrument/badge.svg)](https://docs.rs/oxc_coverage_instrument)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Istanbul-compatible JavaScript/TypeScript coverage instrumentation, built on the [Oxc](https://oxc.rs) parser. **58x faster** than `istanbul-lib-instrument`.

## Why

[`swc-coverage-instrument`](https://github.com/kwonoj/swc-plugin-coverage-instrument) fills this role for SWC. There is no equivalent for the Oxc ecosystem. Any tool built on `oxc_parser` that needs coverage instrumentation currently has to pull in SWC or Babel.

This crate fills that gap. AST-level instrumentation via `oxc_traverse` + `oxc_codegen` produces correct Istanbul-compatible output, verified against the canonical `istanbul-lib-instrument` on 25 shared fixtures.

## Install

### Rust

```toml
[dependencies]
oxc_coverage_instrument = "0.2"
```

### Node.js

```bash
npm install oxc-coverage-instrument
```

### CLI

```bash
cargo install oxc-coverage-instrument-cli
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

### Vite plugin example

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

## What it tracks

| Dimension | What gets a counter |
|:----------|:-------------------|
| **Statements** | Every executable statement |
| **Functions** | Declarations, expressions, arrows, class methods |
| **Branches** | `if`/`else`, ternary, `switch`, `&&`/`\|\|`/`??`, `??=`/`\|\|=`/`&&=`, `default-arg` |
| **Pragmas** | `istanbul ignore next/if/else/file`, `v8 ignore`, `c8 ignore` |

## Istanbul conformance

Verified against `istanbul-lib-instrument` on 25 shared fixtures covering all branch types, function forms, and edge cases. 175 automated conformance checks validate:

- Function counts and names match exactly
- Branch counts, types, and location counts match exactly
- Statement counts within tolerance
- JSON structure matches Istanbul's field set
- Instrumented output re-parses as valid JS

## Performance

| Tool | Throughput | Relative |
|:-----|:-----------|:---------|
| **oxc-coverage-instrument** | **50-67 MiB/s** | **58x faster** |
| istanbul-lib-instrument | 1.1 MiB/s | baseline |

From Node.js (via napi): **~19 MiB/s** (18x faster than Istanbul).

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

## Related projects

| Project | AST | Notes |
|:--------|:----|:------|
| [`istanbul-lib-instrument`](https://github.com/istanbuljs/istanbuljs) | Babel | The canonical Istanbul instrumenter |
| [`swc-coverage-instrument`](https://github.com/kwonoj/swc-plugin-coverage-instrument) | SWC | SWC equivalent (~407K monthly npm downloads) |
| **this crate** | Oxc | First Oxc-native coverage instrumenter, 58x faster |

## Compatibility

- **Rust**: 1.92+ (2024 edition)
- **Oxc**: 0.124.x
- **Istanbul**: `coverage-final.json` v3+ format
- **Node.js**: 18+ (via napi-rs)

## License

MIT
