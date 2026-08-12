use std::{fmt::Write as _, path::Path, process::Command};

use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingIdentifier, IdentifierReference};
use oxc_ast_visit::{Visit, walk::walk_program};
use oxc_codegen::Codegen;
use oxc_coverage_instrument::{InstrumentError, InstrumentOptions, instrument, instrument_program};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_syntax::reference::ReferenceId;
use oxc_transformer::{TransformOptions, Transformer};

#[derive(Default)]
struct GeneratedSemanticAudit {
    bindings_without_symbol: Vec<String>,
    references_without_id: Vec<String>,
    references: Vec<(String, ReferenceId)>,
}

impl<'a> Visit<'a> for GeneratedSemanticAudit {
    fn visit_binding_identifier(&mut self, identifier: &BindingIdentifier<'a>) {
        if identifier.span.is_empty() && identifier.symbol_id.get().is_none() {
            self.bindings_without_symbol.push(identifier.name.to_string());
        }
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if identifier.span.is_empty() {
            if let Some(reference_id) = identifier.reference_id.get() {
                self.references.push((identifier.name.to_string(), reference_id));
            } else {
                self.references_without_id.push(identifier.name.to_string());
            }
        }
    }
}

#[test]
fn ast_api_matches_source_api_with_ast_native_runtime_setup() {
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
    assert!(result.runtime_setup_inserted);

    let code = Codegen::new().with_scoping(Some(result.scoping)).build(&parsed.program).code;
    assert!(code.contains(".s["));
    assert!(code.contains("coverageData"));
}

#[test]
fn ast_native_runtime_setup_registers_generated_semantic_ids() {
    let source = "const value = left && right; object?.property;";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();
    let options = InstrumentOptions { report_logic: true, ..InstrumentOptions::default() };

    let result = instrument_program(
        &allocator,
        &mut parsed.program,
        scoping,
        source,
        "semantic-setup.js",
        &options,
    )
    .expect("instrument program");

    let mut audit = GeneratedSemanticAudit::default();
    walk_program(&mut audit, &parsed.program);
    assert!(audit.bindings_without_symbol.is_empty(), "{:?}", audit.bindings_without_symbol);
    assert!(audit.references_without_id.is_empty(), "{:?}", audit.references_without_id);
    for (name, reference_id) in audit.references {
        if !matches!(name.as_str(), "Array" | "Object" | "global" | "globalThis" | "self") {
            assert!(
                result.scoping.get_reference(reference_id).symbol_id().is_some(),
                "generated reference {name} is unresolved",
            );
        }
    }
    assert!(result.runtime_setup_inserted);
}

#[test]
fn ast_api_survives_a_downstream_oxc_transform_and_executes() {
    let source = r#""use strict";
const path = "user";
const emoji = "🙂";
function answer(value) { return value?.result ?? 0; }
console.log(path, emoji, answer({ result: 42 }));"#;
    let filename = "host-pipeline.cjs";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::cjs()).parse();
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
    .expect("instrument host program");
    let transform = Transformer::new(&allocator, Path::new(filename), &TransformOptions::default())
        .build_with_scoping(result.scoping, &mut parsed.program);
    assert!(!transform.diagnostics.has_errors(), "{:?}", transform.diagnostics);

    let mut code = Codegen::new().with_scoping(Some(transform.scoping)).build(&parsed.program).code;
    write!(code, "\nconsole.log(JSON.stringify(globalThis.__coverage__[{filename:?}].s));")
        .expect("writing to a String cannot fail");
    let output =
        Command::new("node").arg("--eval").arg(code).output().expect("node must be available");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.lines().any(|line| line == "user 🙂 42"));
    let counts: serde_json::Value =
        serde_json::from_str(stdout.lines().last().expect("coverage counts")).unwrap();
    assert!(counts.as_object().unwrap().values().any(|count| count.as_u64().unwrap_or(0) > 0));
}

#[test]
fn ast_api_reports_an_ignored_file_without_runtime_setup() {
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

    assert!(!result.runtime_setup_inserted);
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
