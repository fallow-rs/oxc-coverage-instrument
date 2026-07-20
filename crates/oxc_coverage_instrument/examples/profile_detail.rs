#![expect(clippy::print_stderr, reason = "example binary: eprintln is the output")]
//! Detailed profiling of transform vs preamble.
//!
//! Run with: `cargo run --release --example profile_detail`

use std::time::{Duration, Instant};

fn micros_per_iteration(elapsed: Duration, iterations: u32) -> f64 {
    elapsed.as_secs_f64() * 1_000_000.0 / f64::from(iterations)
}

fn main() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/large-module.js"
    ))
    .unwrap();
    let iterations = 3000;

    // Full pipeline
    let start = Instant::now();
    for _ in 0..iterations {
        let opts = oxc_coverage_instrument::InstrumentOptions::default();
        let _ = oxc_coverage_instrument::instrument(&source, "large-module.js", &opts);
    }
    let full_avg = micros_per_iteration(start.elapsed(), iterations);

    // JSON serialization, which approximates the preamble cost
    let opts = oxc_coverage_instrument::InstrumentOptions::default();
    let result = oxc_coverage_instrument::instrument(&source, "large-module.js", &opts).unwrap();
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = serde_json::to_string(&result.coverage_map).unwrap();
    }
    let json_avg = micros_per_iteration(start.elapsed(), iterations);

    eprintln!("Full pipeline:     {full_avg:.1}µs");
    eprintln!("JSON serialize:    {json_avg:.1}µs ({:.0}%)", json_avg / full_avg * 100.0);
    eprintln!(
        "Rest (parse+sem+traverse+codegen): {:.1}µs ({:.0}%)",
        full_avg - json_avg,
        (full_avg - json_avg) / full_avg * 100.0
    );

    let json = serde_json::to_string(&result.coverage_map).unwrap();
    let json_bytes = f64::from(u32::try_from(json.len()).unwrap_or(u32::MAX));
    let source_bytes = f64::from(u32::try_from(source.len()).unwrap_or(u32::MAX));
    eprintln!("\nCoverage map JSON: {} bytes", json.len());
    eprintln!("Preamble overhead: {:.1}x of source", json_bytes / source_bytes);
}
