//! Performance benchmarks for coverage instrumentation.
//!
//! Measures instrumentation throughput across different file sizes
//! and complexity levels. Run with: `cargo bench`

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn read_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"))
}

fn bench_instrument_fixtures(c: &mut Criterion) {
    let fixtures = [
        ("small_pragma", "pragmas.js"),
        ("small_while", "while-loops.js"),
        ("medium_react", "react-hooks.jsx"),
        ("medium_app", "medium-app.js"),
        ("medium_typescript", "typescript-advanced.ts"),
        ("large_module", "large-module.js"),
    ];

    let mut group = c.benchmark_group("instrument");
    let opts = InstrumentOptions::default();

    for (label, filename) in &fixtures {
        let source = read_fixture(filename);
        let bytes = source.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("file", label), &source, |b, source| {
            b.iter(|| {
                instrument(source, filename, &opts).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_instrument_with_source_map(c: &mut Criterion) {
    let source = read_fixture("medium-app.js");
    let opts_no_sm = InstrumentOptions::default();
    let opts_sm = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };

    let mut group = c.benchmark_group("source_map");

    group.bench_function("without_source_map", |b| {
        b.iter(|| instrument(&source, "medium-app.js", &opts_no_sm).unwrap());
    });
    group.bench_function("with_source_map", |b| {
        b.iter(|| instrument(&source, "medium-app.js", &opts_sm).unwrap());
    });

    group.finish();
}

fn bench_instrument_scaling(c: &mut Criterion) {
    // Generate synthetic source of increasing sizes to measure scaling
    let base = "function f_N() { if (Math.random() > 0.5) { return N; } else { return -N; } }\n";

    let mut group = c.benchmark_group("scaling");

    for &count in &[10, 50, 100, 500] {
        let source: String = (0..count).map(|i| base.replace('N', &i.to_string())).collect();
        let bytes = source.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("functions", count), &source, |b, source| {
            b.iter(|| {
                instrument(source, "synthetic.js", &InstrumentOptions::default()).unwrap();
            });
        });
    }
    group.finish();
}

/// Mirrors what the napi binding does end-to-end: instrument, then surface a
/// JSON string of the coverage map. Pre-round-5 the binding ran a second
/// `serde_json::to_string(&coverage_map)` after `instrument()`; post-round-5
/// it consumes `result.coverage_map_json` directly. Two benches let us see
/// how much the napi-side serialization cost.
fn bench_napi_path(c: &mut Criterion) {
    let fixtures = [
        ("small_pragma", "pragmas.js"),
        ("medium_app", "medium-app.js"),
        ("medium_typescript", "typescript-advanced.ts"),
        ("large_module", "large-module.js"),
    ];
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };

    let mut group = c.benchmark_group("napi_path");
    for (label, filename) in &fixtures {
        let source = read_fixture(filename);

        // Old napi path: re-serializes the coverage map.
        group.bench_with_input(BenchmarkId::new("legacy", label), &source, |b, source| {
            b.iter(|| {
                let r = instrument(source, filename, &opts).unwrap();
                std::hint::black_box(serde_json::to_string(&r.coverage_map).unwrap());
            });
        });

        // New napi path: consumes the cached JSON produced internally.
        group.bench_with_input(BenchmarkId::new("cached", label), &source, |b, source| {
            b.iter(|| {
                let r = instrument(source, filename, &opts).unwrap();
                std::hint::black_box(r.coverage_map_json);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_instrument_fixtures,
    bench_instrument_with_source_map,
    bench_instrument_scaling,
    bench_napi_path
);
criterion_main!(benches);
