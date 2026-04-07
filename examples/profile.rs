#![expect(
    clippy::print_stderr,
    reason = "example binary — eprintln is the intended output mechanism"
)]
//! Profile each phase of the instrumentation pipeline.
//!
//! Run with: `cargo run --release --example profile`

use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use std::time::Instant;

fn main() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/large-module.js"
    ))
    .unwrap();
    let iterations = 3000;

    eprintln!("File: large-module.js ({} bytes)", source.len());
    eprintln!("Iterations: {iterations}\n");

    // Phase 1: Parse only
    let start = Instant::now();
    for _ in 0..iterations {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("large-module.js").unwrap_or_default();
        let _ = Parser::new(&allocator, &source, source_type).parse();
    }
    let parse_time = start.elapsed();
    let parse_avg = parse_time.as_micros() as f64 / f64::from(iterations);

    // Phase 2: Parse + Semantic
    let start = Instant::now();
    for _ in 0..iterations {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("large-module.js").unwrap_or_default();
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        let _ = SemanticBuilder::new().build(&parsed.program);
    }
    let semantic_time = start.elapsed();
    let semantic_avg = semantic_time.as_micros() as f64 / f64::from(iterations);

    // Phase 3: Parse + Semantic + Codegen (no transform)
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
    let codegen_avg = codegen_time.as_micros() as f64 / f64::from(iterations);

    // Phase 4: Full instrument
    let start = Instant::now();
    for _ in 0..iterations {
        let opts = oxc_coverage_instrument::InstrumentOptions::default();
        let _ = oxc_coverage_instrument::instrument(&source, "large-module.js", &opts);
    }
    let full_time = start.elapsed();
    let full_avg = full_time.as_micros() as f64 / f64::from(iterations);

    eprintln!("Phase breakdown:");
    eprintln!(
        "  Parse:         {parse_avg:>8.1}µs ({:.0}%)",
        parse_avg / full_avg * 100.0
    );
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
        "  Transform:     {:>8.1}µs ({:.0}% — our code + preamble)",
        full_avg - codegen_avg,
        (full_avg - codegen_avg) / full_avg * 100.0
    );
    eprintln!(
        "\nThroughput: {:.1} MiB/s",
        (source.len() as f64 / 1024.0 / 1024.0) / (full_avg / 1_000_000.0)
    );
}
