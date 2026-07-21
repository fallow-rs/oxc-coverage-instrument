//! Transform-only benchmark for a host-owned Oxc AST.

use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_allocator::Allocator;
use oxc_coverage_transform::{PragmaMap, TransformInit, TransformProgramInput, transform_program};
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
                    let (pragmas, _) = PragmaMap::from_program(&parsed.program, source);

                    let started = Instant::now();
                    let result = transform_program(TransformProgramInput {
                        program: &mut parsed.program,
                        scoping,
                        pragmas,
                        transform: TransformInit {
                            allocator: &allocator,
                            source,
                            cov_fn_name: "cov_benchmark",
                            report_logic: false,
                            track_optional_chain: true,
                            ignore_class_methods: Vec::new(),
                            name_callback_arguments: false,
                            istanbul_compat: false,
                            registration_policy: None,
                        },
                    });
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
