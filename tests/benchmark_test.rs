//! Benchmark-style tests that instrument fixture files and validate output quality.
//!
//! Each fixture file is instrumented, and we verify:
//! 1. Instrumentation succeeds (no panics, no parse errors)
//! 2. Instrumented code can be re-parsed by Oxc (valid JS/TS output)
//! 3. Coverage map is well-formed (hit counts match maps, branches consistent)
//! 4. Specific feature expectations per fixture

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn read_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

fn instrument_fixture(name: &str) -> oxc_coverage_instrument::InstrumentResult {
    let source = read_fixture(name);
    instrument(&source, name, &InstrumentOptions::default())
        .unwrap_or_else(|e| panic!("Instrumentation failed for {name}: {e}"))
}

fn validate_coverage_map(result: &oxc_coverage_instrument::InstrumentResult, name: &str) {
    let cm = &result.coverage_map;

    // Hit counts match map sizes
    assert_eq!(cm.s.len(), cm.statement_map.len(), "{name}: s count mismatch");
    assert_eq!(cm.f.len(), cm.fn_map.len(), "{name}: f count mismatch");
    assert_eq!(cm.b.len(), cm.branch_map.len(), "{name}: b count mismatch");

    // Branch hit count arrays match location counts
    for (id, entry) in &cm.branch_map {
        let counts = &cm.b[id];
        assert_eq!(
            counts.len(),
            entry.locations.len(),
            "{name}: branch {id} hit count array mismatch"
        );
    }

    // All hit counts are zero (no execution happened)
    for (id, count) in &cm.s {
        assert_eq!(*count, 0, "{name}: s[{id}] should be 0");
    }
    for (id, count) in &cm.f {
        assert_eq!(*count, 0, "{name}: f[{id}] should be 0");
    }

    // Coverage map serializes to valid JSON
    let json = serde_json::to_string(cm).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object(), "{name}: coverage map not an object");
    assert_eq!(parsed["path"], name, "{name}: path mismatch in JSON");

    // Istanbul required fields
    assert!(parsed["statementMap"].is_object(), "{name}: missing statementMap");
    assert!(parsed["fnMap"].is_object(), "{name}: missing fnMap");
}

fn validate_output_reparseable(result: &oxc_coverage_instrument::InstrumentResult, name: &str) {
    // Strip the preamble (first line) and verify the rest can be parsed
    let code = result.code.find('\n').map_or(&result.code as &str, |i| &result.code[i + 1..]);

    let allocator = oxc_allocator::Allocator::default();
    let source_type = oxc_span::SourceType::from_path(name).unwrap_or_default();
    let parsed = oxc_parser::Parser::new(&allocator, code, source_type).parse();

    assert!(
        parsed.errors.is_empty(),
        "{name}: instrumented code has parse errors: {:?}",
        parsed.errors.iter().map(|e| format!("{e}")).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Fixture: react-hooks.jsx
// ---------------------------------------------------------------------------

#[test]
fn benchmark_react_hooks() {
    let result = instrument_fixture("react-hooks.jsx");
    validate_coverage_map(&result, "react-hooks.jsx");
    validate_output_reparseable(&result, "react-hooks.jsx");

    // Should have multiple functions (component + callbacks)
    assert!(
        result.coverage_map.fn_map.len() >= 4,
        "Expected at least 4 functions in react-hooks.jsx, got {}",
        result.coverage_map.fn_map.len()
    );

    // Should have if/else branches
    let if_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "if").count();
    assert!(if_branches >= 2, "Expected at least 2 if-branches");

    // Should have nullish coalescing branches (from ?? patterns)
    let binary_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "binary-expr").count();
    assert!(binary_branches >= 1, "Expected nullish coalescing branches");
}

// ---------------------------------------------------------------------------
// Fixture: while-loops.js
// ---------------------------------------------------------------------------

#[test]
fn benchmark_while_loops() {
    let result = instrument_fixture("while-loops.js");
    validate_coverage_map(&result, "while-loops.js");
    validate_output_reparseable(&result, "while-loops.js");

    // Loops do NOT produce branch entries (matching Istanbul behavior).
    // Coverage is tracked via statement counters only.
    let loop_branches: usize = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| matches!(b.branch_type.as_str(), "for" | "while" | "do-while"))
        .count();
    assert_eq!(loop_branches, 0, "Loops should not produce branch entries");
}

// ---------------------------------------------------------------------------
// Fixture: typescript-advanced.ts
// ---------------------------------------------------------------------------

#[test]
fn benchmark_typescript_advanced() {
    let result = instrument_fixture("typescript-advanced.ts");
    validate_coverage_map(&result, "typescript-advanced.ts");
    validate_output_reparseable(&result, "typescript-advanced.ts");

    // Should have class methods
    let fn_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();
    assert!(fn_names.contains(&"on"), "Missing 'on' method");
    assert!(fn_names.contains(&"emit"), "Missing 'emit' method");

    // Should have switch branches (for discriminated union)
    let switch_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "switch").count();
    assert!(switch_branches >= 1, "Expected switch branch for Shape");

    // Should have ternary branches
    let cond_branches: usize =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "cond-expr").count();
    assert!(cond_branches >= 1, "Expected ternary branches");
}

// ---------------------------------------------------------------------------
// Fixture: edge-cases.js (istanbul ignore file)
// ---------------------------------------------------------------------------

#[test]
fn benchmark_edge_cases_ignore_file() {
    let source = read_fixture("edge-cases.js");
    let result = instrument(&source, "edge-cases.js", &InstrumentOptions::default()).unwrap();

    // Entire file should be ignored
    assert!(
        result.coverage_map.fn_map.is_empty(),
        "istanbul ignore file should produce empty fn_map"
    );
    assert!(
        result.coverage_map.statement_map.is_empty(),
        "istanbul ignore file should produce empty statement_map"
    );
    // Code should be returned unmodified
    assert_eq!(result.code, source);
}

// ---------------------------------------------------------------------------
// Fixture: pragmas.js (selective ignoring)
// ---------------------------------------------------------------------------

#[test]
fn benchmark_pragmas() {
    let result = instrument_fixture("pragmas.js");
    validate_coverage_map(&result, "pragmas.js");
    validate_output_reparseable(&result, "pragmas.js");

    let fn_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();

    // alwaysCounted should be instrumented
    assert!(fn_names.contains(&"alwaysCounted"), "alwaysCounted should be in fn_map");

    // ignoredFunction should NOT be instrumented (istanbul ignore next)
    assert!(!fn_names.contains(&"ignoredFunction"), "ignoredFunction should NOT be in fn_map");

    // v8Ignored should NOT be instrumented
    assert!(!fn_names.contains(&"v8Ignored"), "v8Ignored should NOT be in fn_map");

    // c8Ignored should NOT be instrumented
    assert!(!fn_names.contains(&"c8Ignored"), "c8Ignored should NOT be in fn_map");

    // withIgnoredBranch and withIgnoredElse should still be instrumented as functions
    assert!(fn_names.contains(&"withIgnoredBranch"), "withIgnoredBranch should be in fn_map");
    assert!(fn_names.contains(&"withIgnoredElse"), "withIgnoredElse should be in fn_map");
}

// ---------------------------------------------------------------------------
// Source map validation
// ---------------------------------------------------------------------------

#[test]
fn benchmark_source_map_output() {
    let source = read_fixture("typescript-advanced.ts");
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };
    let result = instrument(&source, "typescript-advanced.ts", &opts).unwrap();

    let sm = result.source_map.as_ref().expect("Source map should be present");
    let parsed: serde_json::Value = serde_json::from_str(sm).unwrap();
    assert_eq!(parsed["version"], 3);
    assert!(parsed["mappings"].is_string());
    assert!(parsed["sources"].is_array(), "Source map should have sources array");
}
