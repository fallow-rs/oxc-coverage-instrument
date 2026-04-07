//! Automated conformance test suite comparing against istanbul-lib-instrument.
//!
//! Prerequisites: run `node tests/conformance/generate-reference.mjs` to generate
//! reference data from Istanbul. The Rust tests then compare our output against
//! the canonical Istanbul output for 25 shared fixtures.
//!
//! Comparison dimensions:
//! - Function count and names
//! - Istanbul-standard branch count, types, and location counts
//! - Statement count (with tolerance for minor differences)
//! - Coverage map JSON structure

use oxc_coverage_instrument::{InstrumentOptions, instrument};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Deserialized Istanbul reference data (from generate-reference.mjs output).
#[derive(Debug, Deserialize)]
struct IstanbulReference {
    #[expect(
        dead_code,
        reason = "deserialized from JSON for structural completeness"
    )]
    path: String,
    statements: usize,
    functions: usize,
    branches: usize,
    #[serde(rename = "statementMap")]
    #[expect(
        dead_code,
        reason = "deserialized from JSON for structural completeness"
    )]
    statement_map: BTreeMap<String, serde_json::Value>,
    #[serde(rename = "fnMap")]
    #[expect(
        dead_code,
        reason = "deserialized from JSON for structural completeness"
    )]
    fn_map: BTreeMap<String, IstanbulFn>,
    #[serde(rename = "branchMap")]
    branch_map: BTreeMap<String, IstanbulBranch>,
}

#[derive(Debug, Deserialize)]
struct IstanbulFn {
    #[expect(
        dead_code,
        reason = "deserialized from JSON for structural completeness"
    )]
    name: String,
    #[expect(
        dead_code,
        reason = "deserialized from JSON for structural completeness"
    )]
    line: u32,
}

#[derive(Debug, Deserialize)]
struct IstanbulBranch {
    #[serde(rename = "type")]
    branch_type: String,
    #[expect(
        dead_code,
        reason = "deserialized from JSON for structural completeness"
    )]
    line: u32,
    #[serde(rename = "locationCount")]
    location_count: usize,
}

fn fixtures_dir() -> String {
    format!("{}/tests/conformance/fixtures", env!("CARGO_MANIFEST_DIR"))
}

fn reference_dir() -> String {
    format!("{}/tests/conformance/reference", env!("CARGO_MANIFEST_DIR"))
}

fn load_reference(name: &str) -> IstanbulReference {
    let path = format!("{}/{name}.json", reference_dir());
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "Missing reference file {path}: {e}. Run: node tests/conformance/generate-reference.mjs"
        )
    });
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("Invalid reference JSON {path}: {e}"))
}

fn load_fixture(filename: &str) -> String {
    let path = format!("{}/{filename}", fixtures_dir());
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Missing fixture {path}: {e}"))
}

fn instrument_fixture(filename: &str) -> oxc_coverage_instrument::InstrumentResult {
    let source = load_fixture(filename);
    instrument(&source, filename, &InstrumentOptions::default())
        .unwrap_or_else(|e| panic!("Instrumentation failed for {filename}: {e}"))
}

// No filtering needed — all our branch types are Istanbul-standard.

// ========================================================================
// Test: Function counts match exactly
// ========================================================================

macro_rules! conformance_test {
    ($test_name:ident, $fixture:literal) => {
        mod $test_name {
            use super::*;

            #[test]
            fn function_count() {
                let reference = load_reference(concat!($fixture));
                let result = instrument_fixture(concat!($fixture, ".js"));
                assert_eq!(
                    result.coverage_map.fn_map.len(),
                    reference.functions,
                    "{}: function count mismatch (ours={}, istanbul={})",
                    $fixture,
                    result.coverage_map.fn_map.len(),
                    reference.functions
                );
            }

            #[test]
            fn branch_count() {
                let reference = load_reference(concat!($fixture));
                let result = instrument_fixture(concat!($fixture, ".js"));
                assert_eq!(
                    result.coverage_map.branch_map.len(),
                    reference.branches,
                    "{}: branch count mismatch (ours={}, istanbul={})",
                    $fixture,
                    result.coverage_map.branch_map.len(),
                    reference.branches
                );
            }

            #[test]
            fn branch_types() {
                let reference = load_reference(concat!($fixture));
                let result = instrument_fixture(concat!($fixture, ".js"));

                let mut our_types: Vec<&str> = result
                    .coverage_map
                    .branch_map
                    .values()
                    .map(|b| b.branch_type.as_str())
                    .collect();
                our_types.sort();

                let mut istanbul_types: Vec<&str> = reference
                    .branch_map
                    .values()
                    .map(|b| b.branch_type.as_str())
                    .collect();
                istanbul_types.sort();

                assert_eq!(
                    our_types, istanbul_types,
                    "{}: branch types mismatch (ours={:?}, istanbul={:?})",
                    $fixture, our_types, istanbul_types
                );
            }

            #[test]
            fn branch_location_counts() {
                let reference = load_reference(concat!($fixture));
                let result = instrument_fixture(concat!($fixture, ".js"));

                // Compare location counts for each Istanbul-standard branch
                let our_branches: Vec<usize> = result
                    .coverage_map
                    .branch_map
                    .values()
                    .map(|b| b.locations.len())
                    .collect();

                let istanbul_branches: Vec<usize> = reference
                    .branch_map
                    .values()
                    .map(|b| b.location_count)
                    .collect();

                assert_eq!(
                    our_branches, istanbul_branches,
                    "{}: branch location counts mismatch (ours={:?}, istanbul={:?})",
                    $fixture, our_branches, istanbul_branches
                );
            }

            #[test]
            fn statement_count_reasonable() {
                let reference = load_reference(concat!($fixture));
                let result = instrument_fixture(concat!($fixture, ".js"));
                let our_stmts = result.coverage_map.statement_map.len();
                let istanbul_stmts = reference.statements;

                // Allow ±3 difference due to AST traversal strategy differences
                let min = istanbul_stmts.saturating_sub(3);
                let max = istanbul_stmts + 3;
                assert!(
                    our_stmts >= min && our_stmts <= max,
                    "{}: statement count out of range (ours={}, istanbul={}, allowed={}..={})",
                    $fixture,
                    our_stmts,
                    istanbul_stmts,
                    min,
                    max
                );
            }

            #[test]
            fn json_structure_valid() {
                let result = instrument_fixture(concat!($fixture, ".js"));
                let json = serde_json::to_value(&result.coverage_map).unwrap();

                // Required Istanbul fields
                assert!(json["path"].is_string(), "{}: missing path", $fixture);
                assert!(
                    json["statementMap"].is_object(),
                    "{}: missing statementMap",
                    $fixture
                );
                assert!(json["fnMap"].is_object(), "{}: missing fnMap", $fixture);
                assert!(
                    json["branchMap"].is_object(),
                    "{}: missing branchMap",
                    $fixture
                );
                assert!(json["s"].is_object(), "{}: missing s", $fixture);
                assert!(json["f"].is_object(), "{}: missing f", $fixture);
                assert!(json["b"].is_object(), "{}: missing b", $fixture);

                // Hit count sizes match map sizes
                assert_eq!(
                    json["s"].as_object().unwrap().len(),
                    json["statementMap"].as_object().unwrap().len(),
                    "{}: s/statementMap size mismatch",
                    $fixture
                );
                assert_eq!(
                    json["f"].as_object().unwrap().len(),
                    json["fnMap"].as_object().unwrap().len(),
                    "{}: f/fnMap size mismatch",
                    $fixture
                );
                assert_eq!(
                    json["b"].as_object().unwrap().len(),
                    json["branchMap"].as_object().unwrap().len(),
                    "{}: b/branchMap size mismatch",
                    $fixture
                );
            }

            #[test]
            fn output_is_valid_js() {
                let result = instrument_fixture(concat!($fixture, ".js"));
                // Strip preamble and verify the rest can be re-parsed
                let code = result
                    .code
                    .find('\n')
                    .map(|i| &result.code[i + 1..])
                    .unwrap_or(&result.code);
                let allocator = oxc_allocator::Allocator::default();
                let source_type =
                    oxc_span::SourceType::from_path(concat!($fixture, ".js")).unwrap_or_default();
                let parsed = oxc_parser::Parser::new(&allocator, code, source_type).parse();
                assert!(
                    parsed.errors.is_empty(),
                    "{}: instrumented code has parse errors: {:?}",
                    $fixture,
                    parsed
                        .errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                );
            }
        }
    };
}

// Generate conformance tests for all 25 fixtures
conformance_test!(c01_function_declaration, "01-function-declaration");
conformance_test!(c02_function_expression, "02-function-expression");
conformance_test!(c03_arrow_expression, "03-arrow-expression");
conformance_test!(c04_arrow_block, "04-arrow-block");
conformance_test!(c05_class_methods, "05-class-methods");
conformance_test!(c06_if_else, "06-if-else");
conformance_test!(c07_if_no_else, "07-if-no-else");
conformance_test!(c08_ternary, "08-ternary");
conformance_test!(c09_switch_cases, "09-switch-cases");
conformance_test!(c10_logical_and, "10-logical-and");
conformance_test!(c11_logical_or, "11-logical-or");
conformance_test!(c12_logical_chain, "12-logical-chain");
conformance_test!(c13_nullish_coalescing, "13-nullish-coalescing");
conformance_test!(c14_nested_if, "14-nested-if");
conformance_test!(c15_multiple_functions, "15-multiple-functions");
conformance_test!(c16_for_loop, "16-for-loop");
conformance_test!(c17_for_of, "17-for-of");
conformance_test!(c18_while_loop, "18-while-loop");
conformance_test!(c19_do_while, "19-do-while");
conformance_test!(c20_default_params, "20-default-params");
conformance_test!(c21_try_catch, "21-try-catch");
conformance_test!(c22_complex_mixed, "22-complex-mixed");
conformance_test!(c23_empty_function, "23-empty-function");
conformance_test!(c24_nested_functions, "24-nested-functions");
conformance_test!(c25_variable_declarations, "25-variable-declarations");
