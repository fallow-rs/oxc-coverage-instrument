//! Performance benchmarks for coverage instrumentation.
//!
//! Measures instrumentation throughput across different file sizes
//! and complexity levels. Run with: `cargo bench`

use std::fmt::Write as _;

use cow_utils::CowUtils;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

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
const FLAT_STATEMENT_COUNTS: &[usize] = &[100, 1_000, 5_000];

fn read_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"))
}

fn bench_instrument_fixtures(c: &mut Criterion) {
    let mut group = c.benchmark_group("instrument");
    let opts = InstrumentOptions::default();

    for (label, filename) in FIXTURES {
        let source = read_fixture(filename);
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

fn identity_source_map(source: &str, filename: &str) -> String {
    let line_count = source.lines().count();
    let mappings = std::iter::once("AAAA")
        .chain(std::iter::repeat_n("AACA", line_count.saturating_sub(1)))
        .collect::<Vec<_>>()
        .join(";");
    serde_json::json!({
        "version": 3,
        "sources": [filename],
        "sourcesContent": [source],
        "names": [],
        "mappings": mappings,
    })
    .to_string()
}

fn bench_eager_input_source_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("eager_input_source_map");

    for &(label, count) in &[("small", 100), ("large", 5_000)] {
        let source = flat_statements(count);
        let input_source_map = identity_source_map(&source, "original.ts");

        for &source_map in &[false, true] {
            let options = InstrumentOptions {
                source_map,
                input_source_map: Some(input_source_map.clone()),
                compose_input_source_map: true,
                ..InstrumentOptions::default()
            };
            let output = if source_map { "with_output_map" } else { "coverage_only" };
            group.bench_with_input(
                BenchmarkId::new(format!("{label}_{output}"), count),
                &source,
                |b, source| {
                    b.iter(|| {
                        std::hint::black_box(instrument(source, "generated.js", &options).unwrap());
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_instrument_scaling(c: &mut Criterion) {
    let base = "function f_N() { if (Math.random() > 0.5) { return N; } else { return -N; } }\n";
    let mut group = c.benchmark_group("scaling");

    for &count in SCALE_COUNTS {
        let source: String =
            (0..count).map(|i| base.cow_replace('N', &i.to_string()).into_owned()).collect();
        group.bench_with_input(BenchmarkId::new("functions", count), &source, |b, source| {
            b.iter(|| {
                instrument(source, "synthetic.js", &InstrumentOptions::default()).unwrap();
            });
        });
    }

    group.finish();
}

fn flat_statements(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        writeln!(source, "sink({index});").expect("writing to a String cannot fail");
    }
    source
}

fn bench_flat_statement_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_statements");

    for &count in FLAT_STATEMENT_COUNTS {
        let source = flat_statements(count);
        group.bench_with_input(BenchmarkId::new("siblings", count), &source, |b, source| {
            b.iter(|| instrument(source, "flat.js", &InstrumentOptions::default()).unwrap());
        });
    }

    group.finish();
}

/// Mirrors the napi binding end-to-end: instrument, then hand a JSON string of
/// the coverage map to the caller. The two benches separate serializing the
/// map a second time from reusing the string `instrument` already produced.
fn bench_napi_path(c: &mut Criterion) {
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };
    let mut group = c.benchmark_group("napi_path");

    for (label, filename) in NAPI_FIXTURES {
        let source = read_fixture(filename);

        group.bench_with_input(BenchmarkId::new("reserialize", label), &source, |b, source| {
            b.iter(|| {
                let result = instrument(source, filename, &opts).unwrap();
                std::hint::black_box(serde_json::to_string(&result.coverage_map).unwrap());
            });
        });

        group.bench_with_input(
            BenchmarkId::new("reuse_cached_json", label),
            &source,
            |b, source| {
                b.iter(|| {
                    let result = instrument(source, filename, &opts).unwrap();
                    std::hint::black_box(result.coverage_map_json);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_instrument_fixtures,
    bench_instrument_with_source_map,
    bench_eager_input_source_map,
    bench_instrument_scaling,
    bench_flat_statement_scaling,
    bench_napi_path
);
criterion_main!(benches);
