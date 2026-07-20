//! Conformance tests against istanbul-lib-instrument.
//!
//! Each case asserts that the coverage map for a source matches istanbul's
//! canonical output for the same source. Regenerate the reference data with
//! `node scripts/compare-istanbul.mjs`.

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn instrument_js(source: &str, filename: &str) -> oxc_coverage_instrument::InstrumentResult {
    instrument(source, filename, &InstrumentOptions::default()).unwrap()
}

/// `(name, source, statements, functions, branches, branch types)` as reported
/// by istanbul-lib-instrument.
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

#[test]
fn conformance_istanbul_branch_types_present() {
    for &(name, source, _stmts, _fns, _branches, expected_types) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));

        let mut our_types: Vec<&str> = result
            .coverage_map
            .branch_map
            .values()
            // `optional-chain` is the one branch type istanbul does not emit,
            // so it is excluded from the comparison.
            .filter(|b| {
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

#[test]
fn conformance_branch_counts_match() {
    for &(name, source, _stmts, _fns, expected_branches, _) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));

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

/// Pinned divergence: for an anonymous function expression or a class method,
/// this crate reports the name the JS runtime infers, where
/// istanbul-lib-instrument reports `(anonymous_N)`. The inferred name is what
/// `Function.prototype.name` holds at runtime, which is what stack traces and
/// HTML coverage reports show.
///
/// See the "Differences from istanbul-lib-instrument" section of the README.
/// `scripts/istanbul-diff.mjs` filters `name` out of its comparison so this
/// divergence does not surface there as a regression.
#[test]
fn fn_name_inference_is_intentional_superset() {
    // `const f = function () {}`: istanbul reports "(anonymous_0)".
    let result = instrument_js("const f = function(x) { return x; };", "t.js");
    assert_eq!(result.coverage_map.fn_map["0"].name, "f");

    let result = instrument_js("const arrowExpr = (x) => x;", "t.js");
    assert_eq!(result.coverage_map.fn_map["0"].name, "arrowExpr");

    // `class C { bar(x) {} }`: istanbul reports "(anonymous_0)".
    let result = instrument_js("class C { bar(x) { return x; } }", "t.js");
    assert_eq!(result.coverage_map.fn_map["0"].name, "bar");

    // A computed, non-literal key has no static name to inherit, so both
    // implementations fall back to the anonymous placeholder.
    let result = instrument_js("class C { [Symbol.iterator]() { return 1; } }", "t.js");
    assert!(
        result.coverage_map.fn_map["0"].name.starts_with("(anonymous_"),
        "computed non-literal keys should fall back to (anonymous_N), got {}",
        result.coverage_map.fn_map["0"].name
    );

    // Truly anonymous IIFE: no parent to inherit a name from.
    let result = instrument_js("(function() { return 1; })();", "t.js");
    assert!(
        result.coverage_map.fn_map["0"].name.starts_with("(anonymous_"),
        "bare anonymous fn should produce (anonymous_N), got {}",
        result.coverage_map.fn_map["0"].name
    );
}

/// Pinned divergence: the ES2021 logical-assignment operators (`??=`, `||=`,
/// `&&=`) are instrumented as `binary-expr` branches with two locations, the
/// always-reached left side and the conditionally-assigned right side.
/// istanbul-lib-instrument has no `AssignmentExpression` visitor entry and
/// emits no branch, so it misses a genuine short-circuit conditional. See the
/// "Differences from istanbul-lib-instrument" section of the README.
///
/// If istanbul adds support and produces a different shape (count or type
/// string), that surfaces here rather than as silent double-counting in a real
/// coverage run.
#[test]
fn logical_assignment_is_intentional_branch_superset() {
    // istanbul-lib-instrument's `visitor.js` has no `AssignmentExpression`
    // entry, so its reference count is zero for every operator.
    const ISTANBUL_EXPECTED: usize = 0;
    const OXC_EXPECTED: usize = 1;

    for (op, source) in &[
        ("??=", "function f(a, b) { a ??= b; }"),
        ("||=", "function f(a, b) { a ||= b; }"),
        ("&&=", "function f(a, b) { a &&= b; }"),
    ] {
        let result = instrument_js(source, &format!("logical-assignment-{op}.js"));
        let branches = &result.coverage_map.branch_map;
        assert_eq!(
            branches.len(),
            OXC_EXPECTED,
            "oxc should emit {OXC_EXPECTED} branch for `{op}` (got {}); istanbul emits {ISTANBUL_EXPECTED}",
            branches.len()
        );
        let entry = branches.values().next().unwrap();
        assert_eq!(entry.branch_type, "binary-expr", "`{op}` branch type");
        assert_eq!(entry.locations.len(), 2, "`{op}` branch should have 2 locations (left, right)");
    }
}

/// Istanbul v7 emits exactly `path`, `statementMap`, `fnMap`, `branchMap`,
/// `s`, `f` and `b`.
#[test]
fn conformance_exact_field_set() {
    let result = instrument_js("function f() { return 1; }", "test.js");
    let json = serde_json::to_value(&result.coverage_map).unwrap();
    let keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();

    for field in &["path", "statementMap", "fnMap", "branchMap", "s", "f", "b"] {
        assert!(keys.contains(field), "Missing Istanbul field: {field}");
    }

    assert!(
        json.get("_coverageSchema").is_none(),
        "Should not include _coverageSchema (Istanbul v7 doesn't)"
    );
    assert!(json.get("hash").is_none(), "Should not include hash (Istanbul v7 doesn't)");
    assert!(json.get("inputSourceMap").is_none(), "Should not include inputSourceMap");
}

#[test]
fn conformance_json_format() {
    for &(name, source, _, _, _, _) in ISTANBUL_REFERENCE {
        let result = instrument_js(source, &format!("{name}.js"));
        let json = serde_json::to_value(&result.coverage_map).unwrap();

        assert!(json["path"].is_string(), "{name}: missing path");
        assert!(json["statementMap"].is_object(), "{name}: missing statementMap");
        assert!(json["fnMap"].is_object(), "{name}: missing fnMap");
        assert!(json["branchMap"].is_object(), "{name}: missing branchMap");
        assert!(json["s"].is_object(), "{name}: missing s");
        assert!(json["f"].is_object(), "{name}: missing f");
        assert!(json["b"].is_object(), "{name}: missing b");

        // Serialized as `type`, not `branch_type`.
        for (id, entry) in json["branchMap"].as_object().unwrap() {
            assert!(entry["type"].is_string(), "{name}: branchMap[{id}] missing 'type'");
            assert!(entry["locations"].is_array(), "{name}: branchMap[{id}] missing 'locations'");
        }

        for (id, entry) in json["fnMap"].as_object().unwrap() {
            assert!(entry["name"].is_string(), "{name}: fnMap[{id}] missing 'name'");
            assert!(entry["line"].is_number(), "{name}: fnMap[{id}] missing 'line'");
            assert!(entry["decl"].is_object(), "{name}: fnMap[{id}] missing 'decl'");
            assert!(entry["loc"].is_object(), "{name}: fnMap[{id}] missing 'loc'");
        }
    }
}
