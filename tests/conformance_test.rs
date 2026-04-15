//! Conformance tests comparing our output against istanbul-lib-instrument.
//!
//! These tests validate that our coverage map structure matches Istanbul's
//! canonical output for the same source code. Run the reference generator
//! first: `node scripts/compare-istanbul.mjs`

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn instrument_js(source: &str, filename: &str) -> oxc_coverage_instrument::InstrumentResult {
    instrument(source, filename, &InstrumentOptions::default()).unwrap()
}

// Istanbul reference data (from running scripts/compare-istanbul.mjs)
// Format: (name, source, istanbul_stmts, istanbul_fns, istanbul_branches, istanbul_branch_types)
type IstanbulRef = (&'static str, &'static str, usize, usize, usize, &'static [&'static str]);
const ISTANBUL_REFERENCE: &[IstanbulRef] = &[
    ("simple_function", "function add(a, b) { return a + b; }", 1, 1, 0, &[]),
    ("arrow_expression", "const double = (x) => x * 2;", 2, 1, 0, &[]),
    ("arrow_block", "const add = (a, b) => { return a + b; };", 2, 1, 0, &[]),
    ("if_else", "function f(x) { if (x > 0) { return 1; } else { return -1; } }", 3, 1, 1, &["if"]),
    ("ternary", "function f(x) { return x > 0 ? 1 : -1; }", 1, 1, 1, &["cond-expr"]),
    (
        "switch",
        "function f(x) { switch(x) { case 1: return \"one\"; case 2: return \"two\"; default: return \"other\"; } }",
        4,
        1,
        1,
        &["switch"],
    ),
    ("logical_and_or", "function f(a, b) { return a && b || false; }", 1, 1, 1, &["binary-expr"]),
    ("nullish_coalescing", "function f(a, b) { return a ?? b; }", 1, 1, 1, &["binary-expr"]),
    (
        "for_loop",
        "function f(arr) { for (let i = 0; i < arr.length; i++) { console.log(arr[i]); } }",
        3,
        1,
        0,
        &[],
    ),
    ("for_of", "function f(arr) { for (const item of arr) { console.log(item); } }", 2, 1, 0, &[]),
    ("while_loop", "function f() { let i = 0; while (i < 10) { i++; } return i; }", 4, 1, 0, &[]),
    ("do_while", "function f() { let i = 0; do { i++; } while (i < 10); return i; }", 4, 1, 0, &[]),
    (
        "class_methods",
        "class Calc { add(a, b) { return a + b; } sub(a, b) { return a - b; } }",
        2,
        2,
        0,
        &[],
    ),
    (
        "nested_if",
        "function f(a, b) { if (a) { if (b) { return 1; } else { return 2; } } else { return 3; } }",
        5,
        1,
        2,
        &["if", "if"],
    ),
    (
        "multiple_functions",
        "function a() { return 1; }\nfunction b() { return 2; }\nconst c = () => 3;\nconst d = function() { return 4; };",
        6,
        4,
        0,
        &[],
    ),
];

/// Test that function counts match Istanbul exactly.
#[test]
fn conformance_function_counts() {
    for &(name, source, _stmts, expected_fns, _branches, _) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));
        assert_eq!(
            result.coverage_map.fn_map.len(),
            expected_fns,
            "Function count mismatch for '{name}': got {}, Istanbul expects {expected_fns}",
            result.coverage_map.fn_map.len()
        );
    }
}

/// Test that Istanbul-standard branch types match.
/// We may have EXTRA branch types (for/while/do-while) that Istanbul doesn't have,
/// but Istanbul's branch types must all be present in our output.
#[test]
fn conformance_istanbul_branch_types_present() {
    for &(name, source, _stmts, _fns, _branches, expected_types) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));

        let mut our_types: Vec<&str> = result
            .coverage_map
            .branch_map
            .values()
            .filter(|b| {
                // Only compare Istanbul-standard branch types, skip our extras
                matches!(
                    b.branch_type.as_str(),
                    "if" | "switch" | "cond-expr" | "binary-expr" | "default-arg"
                )
            })
            .map(|b| b.branch_type.as_str())
            .collect();
        our_types.sort_unstable();

        let mut expected: Vec<&str> = expected_types.to_vec();
        expected.sort_unstable();

        assert_eq!(
            our_types, expected,
            "Istanbul branch types mismatch for '{name}': got {our_types:?}, expected {expected:?}"
        );
    }
}

/// Test that our branch counts are >= Istanbul's.
/// We may have MORE branches (for/while/do-while extras) but never fewer
/// Istanbul-standard branches.
#[test]
fn conformance_branch_counts_superset() {
    for &(name, source, _stmts, _fns, expected_branches, _) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));

        // Count only Istanbul-standard branch types
        let our_istanbul_branches: usize = result
            .coverage_map
            .branch_map
            .values()
            .filter(|b| {
                matches!(
                    b.branch_type.as_str(),
                    "if" | "switch" | "cond-expr" | "binary-expr" | "default-arg"
                )
            })
            .count();

        assert_eq!(
            our_istanbul_branches, expected_branches,
            "Istanbul-standard branch count mismatch for '{name}': got {our_istanbul_branches}, Istanbul expects {expected_branches}"
        );
    }
}

/// Test that statement counts match Istanbul exactly.
#[test]
fn conformance_statement_counts_match() {
    for &(name, source, expected_stmts, _fns, _branches, _) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));
        let our_stmts = result.coverage_map.statement_map.len();
        assert_eq!(
            our_stmts, expected_stmts,
            "Statement count for '{name}' differs from Istanbul: got {our_stmts}, expected {expected_stmts}"
        );
    }
}

/// Test that our output matches Istanbul's field set exactly.
/// Istanbul v7 output: path, statementMap, fnMap, branchMap, s, f, b
#[test]
fn conformance_exact_field_set() {
    let result = instrument_js("function f() { return 1; }", "test.js");
    let json = serde_json::to_value(&result.coverage_map).unwrap();
    let keys: Vec<&str> = json.as_object().unwrap().keys().map(|k| k.as_str()).collect();

    // Must have all Istanbul fields
    for field in &["path", "statementMap", "fnMap", "branchMap", "s", "f", "b"] {
        assert!(keys.contains(field), "Missing Istanbul field: {field}");
    }

    // Should NOT have fields Istanbul doesn't produce
    assert!(
        json.get("_coverageSchema").is_none(),
        "Should not include _coverageSchema (Istanbul v7 doesn't)"
    );
    assert!(json.get("hash").is_none(), "Should not include hash (Istanbul v7 doesn't)");
    assert!(json.get("inputSourceMap").is_none(), "Should not include inputSourceMap");
}

/// Test that the coverage map serializes to valid Istanbul-consumable JSON.
#[test]
fn conformance_json_format() {
    for &(name, source, _, _, _, _) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));
        let json = serde_json::to_value(&result.coverage_map).unwrap();

        // Must have the Istanbul-required fields
        assert!(json["path"].is_string(), "{name}: missing path");
        assert!(json["statementMap"].is_object(), "{name}: missing statementMap");
        assert!(json["fnMap"].is_object(), "{name}: missing fnMap");
        assert!(json["branchMap"].is_object(), "{name}: missing branchMap");
        assert!(json["s"].is_object(), "{name}: missing s");
        assert!(json["f"].is_object(), "{name}: missing f");
        assert!(json["b"].is_object(), "{name}: missing b");

        // Branch map entries must have 'type' (not 'branch_type')
        for (id, entry) in json["branchMap"].as_object().unwrap() {
            assert!(entry["type"].is_string(), "{name}: branchMap[{id}] missing 'type'");
            assert!(entry["locations"].is_array(), "{name}: branchMap[{id}] missing 'locations'");
        }

        // Function map entries must have name, line, decl, loc
        for (id, entry) in json["fnMap"].as_object().unwrap() {
            assert!(entry["name"].is_string(), "{name}: fnMap[{id}] missing 'name'");
            assert!(entry["line"].is_number(), "{name}: fnMap[{id}] missing 'line'");
            assert!(entry["decl"].is_object(), "{name}: fnMap[{id}] missing 'decl'");
            assert!(entry["loc"].is_object(), "{name}: fnMap[{id}] missing 'loc'");
        }
    }
}
