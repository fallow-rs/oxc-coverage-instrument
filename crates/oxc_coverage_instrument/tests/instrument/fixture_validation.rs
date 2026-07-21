//! Instrument each fixture file and validate the resulting coverage map and
//! output.
//!
//! Every fixture is checked for a well-formed coverage map and re-parseable
//! output, then for the coverage entries its own syntax should produce.

use oxc_coverage_instrument::{InstrumentOptions, instrument};

use crate::common::{assert_coverage_map_well_formed, assert_reparses};

fn read_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

fn instrument_fixture(name: &str) -> oxc_coverage_instrument::InstrumentResult {
    let source = read_fixture(name);
    instrument(&source, name, &InstrumentOptions::default())
        .unwrap_or_else(|e| panic!("Instrumentation failed for {name}: {e}"))
}

#[test]
fn react_hooks_fixture() {
    let result = instrument_fixture("react-hooks.jsx");
    assert_coverage_map_well_formed(&result, "react-hooks.jsx");
    assert_reparses(&result.code, "react-hooks.jsx");

    assert!(
        result.coverage_map.fn_map.len() >= 4,
        "Expected at least 4 functions in react-hooks.jsx, got {}",
        result.coverage_map.fn_map.len()
    );

    let if_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "if").count();
    assert!(if_branches >= 2, "Expected at least 2 if-branches");

    let binary_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "binary-expr").count();
    assert!(binary_branches >= 1, "Expected nullish coalescing branches");
}

#[test]
fn while_loops_fixture_produces_no_loop_branches() {
    let result = instrument_fixture("while-loops.js");
    assert_coverage_map_well_formed(&result, "while-loops.js");
    assert_reparses(&result.code, "while-loops.js");

    // istanbul tracks loop coverage through statement counters alone.
    let loop_branches: usize = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| matches!(b.branch_type.as_str(), "for" | "while" | "do-while"))
        .count();
    assert_eq!(loop_branches, 0, "Loops should not produce branch entries");
}

#[test]
fn typescript_advanced_fixture() {
    let result = instrument_fixture("typescript-advanced.ts");
    assert_coverage_map_well_formed(&result, "typescript-advanced.ts");
    assert_reparses(&result.code, "typescript-advanced.ts");

    let fn_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();
    assert!(fn_names.contains(&"on"), "Missing 'on' method");
    assert!(fn_names.contains(&"emit"), "Missing 'emit' method");

    let switch_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "switch").count();
    assert!(switch_branches >= 1, "Expected switch branch for Shape");

    let cond_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "cond-expr").count();
    assert!(cond_branches >= 1, "Expected ternary branches");
}

#[test]
fn edge_cases_fixture_is_ignored_whole_file() {
    let source = read_fixture("edge-cases.js");
    let result = instrument(&source, "edge-cases.js", &InstrumentOptions::default()).unwrap();

    assert!(
        result.coverage_map.fn_map.is_empty(),
        "istanbul ignore file should produce empty fn_map"
    );
    assert!(
        result.coverage_map.statement_map.is_empty(),
        "istanbul ignore file should produce empty statement_map"
    );
    assert_eq!(result.code, source, "an ignored file is returned unmodified");
}

#[test]
fn pragmas_fixture_skips_only_the_pragma_marked_functions() {
    let result = instrument_fixture("pragmas.js");
    assert_coverage_map_well_formed(&result, "pragmas.js");
    assert_reparses(&result.code, "pragmas.js");

    let fn_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();

    assert!(fn_names.contains(&"alwaysCounted"), "alwaysCounted should be in fn_map");
    assert!(!fn_names.contains(&"ignoredFunction"), "ignoredFunction must not be in fn_map");
    assert!(!fn_names.contains(&"v8Ignored"), "v8Ignored must not be in fn_map");
    assert!(!fn_names.contains(&"c8Ignored"), "c8Ignored must not be in fn_map");

    // An ignored branch or else arm still leaves the enclosing function counted.
    assert!(fn_names.contains(&"withIgnoredBranch"), "withIgnoredBranch should be in fn_map");
    assert!(fn_names.contains(&"withIgnoredElse"), "withIgnoredElse should be in fn_map");
}

#[test]
fn source_map_output_is_v3_shaped() {
    let source = read_fixture("typescript-advanced.ts");
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };
    let result = instrument(&source, "typescript-advanced.ts", &opts).unwrap();

    let sm = result.source_map.as_ref().expect("Source map should be present");
    let parsed: serde_json::Value = serde_json::from_str(sm).unwrap();
    assert_eq!(parsed["version"], 3);
    assert!(parsed["mappings"].is_string());
    assert!(parsed["sources"].is_array(), "Source map should have sources array");
}
