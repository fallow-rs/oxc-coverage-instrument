#![expect(clippy::print_stderr, reason = "example binary: eprintln is the output")]
//! Profile each phase of the instrumentation pipeline.
//!
//! Run with: `cargo run --release --example profile`

use std::time::{Duration, Instant};

use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

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

    eprintln!("File: large-module.js ({} bytes)", source.len());
    eprintln!("Iterations: {iterations}\n");

    // Parse only
    let start = Instant::now();
    for _ in 0..iterations {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("large-module.js").unwrap_or_default();
        let _ = Parser::new(&allocator, &source, source_type).parse();
    }
    let parse_time = start.elapsed();
    let parse_avg = micros_per_iteration(parse_time, iterations);

    // Parse and semantic
    let start = Instant::now();
    for _ in 0..iterations {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("large-module.js").unwrap_or_default();
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        let _ = SemanticBuilder::new().build(&parsed.program);
    }
    let semantic_time = start.elapsed();
    let semantic_avg = micros_per_iteration(semantic_time, iterations);

    // Parse, semantic and codegen, without the coverage transform
    let start = Instant::now();
    for _ in 0..iterations {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("large-module.js").unwrap_or_default();
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        let semantic_ret = SemanticBuilder::new().build(&parsed.program);
        let scoping = semantic_ret.semantic.into_scoping();
        let _ = Codegen::new()
            .with_source_text(&source)
            .with_scoping(Some(scoping))
            .build(&parsed.program);
    }
    let codegen_time = start.elapsed();
    let codegen_avg = micros_per_iteration(codegen_time, iterations);

    // The full pipeline
    let start = Instant::now();
    for _ in 0..iterations {
        let opts = oxc_coverage_instrument::InstrumentOptions::default();
        let _ = oxc_coverage_instrument::instrument(&source, "large-module.js", &opts);
    }
    let full_time = start.elapsed();
    let full_avg = micros_per_iteration(full_time, iterations);

    eprintln!("Phase breakdown:");
    eprintln!("  Parse:         {parse_avg:>8.1}µs ({:.0}%)", parse_avg / full_avg * 100.0);
    eprintln!(
        "  + Semantic:    {:>8.1}µs ({:.0}% incremental)",
        semantic_avg,
        (semantic_avg - parse_avg) / full_avg * 100.0
    );
    eprintln!(
        "  + Codegen:     {:>8.1}µs ({:.0}% incremental)",
        codegen_avg,
        (codegen_avg - semantic_avg) / full_avg * 100.0
    );
    eprintln!("  Full pipeline: {full_avg:>8.1}µs");
    eprintln!(
        "  Transform:     {:>8.1}µs ({:.0}%, coverage transform plus preamble)",
        full_avg - codegen_avg,
        (full_avg - codegen_avg) / full_avg * 100.0
    );
    let source_mib = f64::from(u32::try_from(source.len()).unwrap_or(u32::MAX)) / (1024.0 * 1024.0);
    eprintln!("\nThroughput: {:.1} MiB/s", source_mib / (full_avg / 1_000_000.0));
}
