#![expect(
    clippy::print_stderr,
    reason = "example binary — eprintln is the intended output mechanism"
)]
//! Detailed profiling of transform vs preamble.
//!
//! Run with: `cargo run --release --example profile_detail`

use std::time::Instant;

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
    let full_avg = start.elapsed().as_micros() as f64 / f64::from(iterations);

    // Measure JSON serialization cost (approximation of preamble)
    let opts = oxc_coverage_instrument::InstrumentOptions::default();
    let result = oxc_coverage_instrument::instrument(&source, "large-module.js", &opts).unwrap();
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = serde_json::to_string(&result.coverage_map).unwrap();
    }
    let json_avg = start.elapsed().as_micros() as f64 / f64::from(iterations);

    eprintln!("Full pipeline:     {full_avg:.1}µs");
    eprintln!(
        "JSON serialize:    {json_avg:.1}µs ({:.0}%)",
        json_avg / full_avg * 100.0
    );
    eprintln!(
        "Rest (parse+sem+traverse+codegen): {:.1}µs ({:.0}%)",
        full_avg - json_avg,
        (full_avg - json_avg) / full_avg * 100.0
    );

    // Coverage map size
    let json = serde_json::to_string(&result.coverage_map).unwrap();
    eprintln!("\nCoverage map JSON: {} bytes", json.len());
    eprintln!(
        "Preamble overhead: {:.1}x of source",
        json.len() as f64 / source.len() as f64
    );
}
