//! Transform-only benchmark for a host-owned Oxc AST.

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_allocator::{Allocator, CloneIn};
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
            let allocator = Allocator::default();
            let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
            assert!(!parsed.diagnostics.has_errors());
            b.iter_batched(
                || {
                    let program = parsed.program.clone_in(&allocator);
                    let scoping = SemanticBuilder::new().build(&program).semantic.into_scoping();
                    let (pragmas, _) = PragmaMap::from_program(&program, source);
                    (program, scoping, pragmas)
                },
                |(mut program, scoping, pragmas)| {
                    let result = transform_program(TransformProgramInput {
                        program: &mut program,
                        scoping,
                        pragmas,
                        transform: TransformInit {
                            allocator: &allocator,
                            cov_fn_name: "cov_benchmark".to_string(),
                            report_logic: false,
                            track_optional_chain: true,
                            ignore_class_methods: Vec::new(),
                            name_callback_arguments: false,
                            istanbul_compat: false,
                        },
                    })
                    .expect("valid benchmark transform");
                    std::hint::black_box(result);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_ast_kernel);
criterion_main!(benches);
