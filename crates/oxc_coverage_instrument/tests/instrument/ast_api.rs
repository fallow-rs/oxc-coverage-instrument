use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_coverage_instrument::{InstrumentError, InstrumentOptions, instrument, instrument_program};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

#[test]
fn ast_api_matches_source_api_without_inserting_the_preamble() {
    let source = "function answer() { return 42; } answer();";
    let filename = "ast-api.js";
    let options = InstrumentOptions::default();
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();

    let result =
        instrument_program(&allocator, &mut parsed.program, scoping, source, filename, &options)
            .expect("instrument parsed program");
    let source_result = instrument(source, filename, &options).expect("instrument source");

    assert_eq!(result.coverage_map_json, source_result.coverage_map_json);
    assert!(result.preamble.as_deref().is_some_and(|preamble| preamble.contains("coverageData")));

    let code = Codegen::new().with_scoping(Some(result.scoping)).build(&parsed.program).code;
    assert!(code.contains(".s["));
    assert!(!code.contains("coverageData"));
}

#[test]
fn ast_api_reports_an_ignored_file_without_a_preamble() {
    let source = "/* istanbul ignore file */\nfunction ignored() { return 1; }";
    let filename = "ignored.js";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();

    let result = instrument_program(
        &allocator,
        &mut parsed.program,
        scoping,
        source,
        filename,
        &InstrumentOptions::default(),
    )
    .expect("instrument ignored program");

    assert!(result.preamble.is_none());
    assert!(result.coverage_map.statement_map.is_empty());
    assert!(result.coverage_map.fn_map.is_empty());
    assert!(result.coverage_map.branch_map.is_empty());
}

#[test]
fn ast_api_rejects_an_invalid_coverage_variable() {
    let source = "answer();";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();
    let options = InstrumentOptions {
        coverage_variable: "not.valid".to_string(),
        ..InstrumentOptions::default()
    };

    let result = instrument_program(
        &allocator,
        &mut parsed.program,
        scoping,
        source,
        "invalid-option.js",
        &options,
    );

    assert!(matches!(result, Err(InstrumentError::InvalidCoverageVariable(_))));
}
