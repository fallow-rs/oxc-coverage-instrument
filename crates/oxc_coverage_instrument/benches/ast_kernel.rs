//! Transform-only benchmark for a host-owned Oxc AST.

use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_allocator::Allocator;
use oxc_coverage_instrument::{InstrumentOptions, instrument_program};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

fn source_with_functions(count: usize) -> String {
    (0..count)
        .map(|index| format!("function f{index}(value) {{ return value ? {index} : -{index}; }}\n"))
        .collect()
}

fn bench_ast_kernel(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_kernel");

    for &count in &[10, 100, 500] {
        let source = source_with_functions(count);
        group.bench_with_input(BenchmarkId::new("functions", count), &source, |b, source| {
            b.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let allocator = Allocator::default();
                    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
                    assert!(!parsed.diagnostics.has_errors());
                    let scoping =
                        SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();

                    let started = Instant::now();
                    let result = instrument_program(
                        &allocator,
                        &mut parsed.program,
                        scoping,
                        source,
                        "kernel.js",
                        &InstrumentOptions::default(),
                    )
                    .expect("benchmark input must instrument");
                    elapsed += started.elapsed();
                    std::hint::black_box(result);
                }
                elapsed
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_ast_kernel);
criterion_main!(benches);
