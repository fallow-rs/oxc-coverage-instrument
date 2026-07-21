use oxc_allocator::Allocator;
use oxc_coverage_transform::{
    PragmaMap, TransformError, TransformInit, TransformOutcome, TransformProgramInput,
    transform_program,
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};

fn transform<'arena>(
    allocator: &'arena Allocator,
    source: &str,
    program: &mut oxc_ast::ast::Program<'arena>,
    coverage_name: &str,
) -> Result<oxc_coverage_transform::TransformOutput, TransformError> {
    let scoping = SemanticBuilder::new().build(program).semantic.into_scoping();
    let (pragmas, _) = PragmaMap::from_program(program, source);
    transform_program(TransformProgramInput {
        program,
        scoping,
        pragmas,
        transform: TransformInit {
            allocator,
            cov_fn_name: coverage_name.to_string(),
            report_logic: false,
            track_optional_chain: true,
            ignore_class_methods: Vec::new(),
            name_callback_arguments: false,
            istanbul_compat: false,
        },
    })
}

#[test]
fn ignore_file_is_a_kernel_level_no_op() {
    let source = "/* istanbul ignore file */\nanswer();";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let original_span = parsed.program.body[0].span();

    let output = transform(&allocator, source, &mut parsed.program, "cov_host").unwrap();

    assert!(matches!(output.outcome, TransformOutcome::Ignored));
    assert_eq!(parsed.program.body.len(), 1);
    assert_eq!(parsed.program.body[0].span(), original_span);
    assert!(output.scoping.get_root_binding("cov_host".into()).is_none());
}

#[test]
fn direct_host_binding_is_collision_safe() {
    let source = "let cov_host = 1; answer();";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());

    let output = transform(&allocator, source, &mut parsed.program, "cov_host").unwrap();
    let TransformOutcome::Instrumented { names, metadata } = output.outcome else {
        panic!("program should be instrumented");
    };

    assert_eq!(names.coverage, "cov_host_1");
    assert!(output.scoping.get_root_binding("cov_host".into()).is_some());
    assert!(output.scoping.get_root_binding("cov_host_1".into()).is_some());
    assert_eq!(metadata.statements.len(), 2);
}

#[test]
fn invalid_binding_is_rejected_before_mutation() {
    let source = "answer();";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let original_span = parsed.program.body[0].span();

    let error = transform(&allocator, source, &mut parsed.program, "not.valid").unwrap_err();

    assert_eq!(error, TransformError::InvalidCoverageBindingName("not.valid".to_string()));
    assert_eq!(parsed.program.body.len(), 1);
    assert_eq!(parsed.program.body[0].span(), original_span);
}

#[test]
fn metadata_stays_in_oxc_byte_spans() {
    let source = "const emoji = '🙂';\nanswer(emoji);";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let expected = parsed.program.body[1].span();

    let output = transform(&allocator, source, &mut parsed.program, "cov_host").unwrap();
    let TransformOutcome::Instrumented { metadata, .. } = output.outcome else {
        panic!("program should be instrumented");
    };

    assert!(metadata.statements.contains(&expected));
    assert_eq!(&source[expected.start as usize..expected.end as usize], "answer(emoji);");
}
