//! Performance benchmarks for coverage instrumentation.
//!
//! Measures instrumentation throughput across different file sizes
//! and complexity levels. Run with: `cargo bench`

use divan::Bencher;
use oxc_coverage_instrument::{InstrumentOptions, instrument};

const FIXTURES: &[(&str, &str)] = &[
    ("small_pragma", "pragmas.js"),
    ("small_while", "while-loops.js"),
    ("medium_react", "react-hooks.jsx"),
    ("medium_app", "medium-app.js"),
    ("medium_typescript", "typescript-advanced.ts"),
    ("large_module", "large-module.js"),
];

const NAPI_FIXTURES: &[(&str, &str)] = &[
    ("small_pragma", "pragmas.js"),
    ("medium_app", "medium-app.js"),
    ("medium_typescript", "typescript-advanced.ts"),
    ("large_module", "large-module.js"),
];

const SCALE_COUNTS: &[usize] = &[10, 50, 100, 500];

fn main() {
    divan::main();
}

fn read_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"))
}

#[divan::bench(args = FIXTURES)]
fn instrument_file(bencher: Bencher, case: &(&str, &str)) {
    let (_, filename) = *case;
    let source = read_fixture(filename);
    let opts = InstrumentOptions::default();

    bencher.bench(|| instrument(&source, filename, &opts).unwrap());
}

#[divan::bench]
fn source_map_without_source_map(bencher: Bencher) {
    let source = read_fixture("medium-app.js");
    let opts = InstrumentOptions::default();

    bencher.bench(|| instrument(&source, "medium-app.js", &opts).unwrap());
}

#[divan::bench]
fn source_map_with_source_map(bencher: Bencher) {
    let source = read_fixture("medium-app.js");
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };

    bencher.bench(|| instrument(&source, "medium-app.js", &opts).unwrap());
}

#[divan::bench(args = SCALE_COUNTS)]
fn scaling_functions(bencher: Bencher, count: usize) {
    let base = "function f_N() { if (Math.random() > 0.5) { return N; } else { return -N; } }\n";
    let source: String = (0..count).map(|i| base.replace('N', &i.to_string())).collect();

    bencher.bench(|| instrument(&source, "synthetic.js", &InstrumentOptions::default()).unwrap());
}

/// Mirrors what the napi binding does end-to-end: instrument, then surface a
/// JSON string of the coverage map. Pre-round-5 the binding ran a second
/// `serde_json::to_string(&coverage_map)` after `instrument()`; post-round-5
/// it consumes `result.coverage_map_json` directly. Two benches let us see
/// how much the napi-side serialization cost.
#[divan::bench(args = NAPI_FIXTURES)]
fn napi_path_legacy(bencher: Bencher, case: &(&str, &str)) {
    let (_, filename) = *case;
    let source = read_fixture(filename);
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };

    bencher.bench(|| {
        let result = instrument(&source, filename, &opts).unwrap();
        std::hint::black_box(serde_json::to_string(&result.coverage_map).unwrap())
    });
}

#[divan::bench(args = NAPI_FIXTURES)]
fn napi_path_cached(bencher: Bencher, case: &(&str, &str)) {
    let (_, filename) = *case;
    let source = read_fixture(filename);
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };

    bencher.bench(|| {
        let result = instrument(&source, filename, &opts).unwrap();
        std::hint::black_box(result.coverage_map_json)
    });
}
