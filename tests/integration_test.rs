//! Integration tests for oxc-coverage-instrument.
//!
//! Tests the public `instrument()` API across all coverage dimensions:
//! statements, functions, branches, pragmas, source maps, and edge cases.

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn default_opts() -> InstrumentOptions {
    InstrumentOptions::default()
}

fn instrument_js(source: &str) -> oxc_coverage_instrument::InstrumentResult {
    instrument(source, "test.js", &default_opts()).unwrap()
}

// ---------------------------------------------------------------------------
// Statement coverage
// ---------------------------------------------------------------------------

#[test]
fn statement_simple_variable_declaration() {
    let result = instrument_js("const x = 1;");
    assert_eq!(result.coverage_map.statement_map.len(), 1);
    assert!(result.code.contains("++") && result.code.contains(".s[0]"));
}

#[test]
fn statement_multiple_statements() {
    let result = instrument_js("const x = 1;\nconst y = 2;\nconst z = x + y;");
    assert_eq!(result.coverage_map.statement_map.len(), 3);
}

#[test]
fn statement_return_throw_expression() {
    let result = instrument_js(
        "function f() { const x = 1; return x; }\nfunction g() { throw new Error(); }",
    );
    // f: function decl stmt + const + return = 3 in f, g: function decl stmt + throw = 2 in g
    // Plus the two function declaration statements themselves
    assert!(result.coverage_map.statement_map.len() >= 4);
}

#[test]
fn statement_empty_and_block_not_counted() {
    let result = instrument_js(";;; { const x = 1; }");
    // Only the const x = 1 inside the block should be counted (blocks/empty skipped)
    assert_eq!(result.coverage_map.statement_map.len(), 1);
}

// ---------------------------------------------------------------------------
// Function coverage
// ---------------------------------------------------------------------------

#[test]
fn function_declaration() {
    let result = instrument_js("function add(a, b) { return a + b; }");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "add");
    assert!(result.code.contains(".f[0]"));
}

#[test]
fn function_expression() {
    let result = instrument_js("const add = function(a, b) { return a + b; };");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "add");
}

#[test]
fn arrow_function_expression_body() {
    let result = instrument_js("const double = (x) => x * 2;");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "double");
    // Arrow with expression body should be converted to block with return
    assert!(result.code.contains("return"));
    assert!(result.code.contains(".f[0]"));
}

#[test]
fn arrow_function_block_body() {
    let result = instrument_js("const add = (a, b) => { return a + b; };");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "add");
}

#[test]
fn class_method() {
    let result =
        instrument_js("class Calc { add(a, b) { return a + b; } sub(a, b) { return a - b; } }");
    assert_eq!(result.coverage_map.fn_map.len(), 2);
    assert_eq!(result.coverage_map.fn_map["0"].name, "add");
    assert_eq!(result.coverage_map.fn_map["1"].name, "sub");
}

#[test]
fn anonymous_function() {
    let result = instrument_js("setTimeout(function() { console.log('hi'); }, 100);");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert!(
        result.coverage_map.fn_map["0"]
            .name
            .starts_with("(anonymous_")
    );
}

#[test]
fn multiple_functions() {
    let result = instrument_js(
        "function a() {} function b() {} const c = () => 1; const d = function() {};",
    );
    assert_eq!(result.coverage_map.fn_map.len(), 4);
}

// ---------------------------------------------------------------------------
// Branch coverage: if/else
// ---------------------------------------------------------------------------

#[test]
fn branch_if_else() {
    let result = instrument_js("if (true) { console.log('yes'); } else { console.log('no'); }");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(result.coverage_map.branch_map["0"].branch_type, "if");
    assert_eq!(result.coverage_map.branch_map["0"].locations.len(), 2);
    assert!(result.code.contains(".b[0][0]"));
    assert!(result.code.contains(".b[0][1]"));
}

#[test]
fn branch_if_without_else() {
    let result = instrument_js("if (true) { console.log('yes'); }");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(result.coverage_map.branch_map["0"].locations.len(), 2);
    // Consequent should have counter, alternate should be empty span
    assert!(result.code.contains(".b[0][0]"));
}

// ---------------------------------------------------------------------------
// Branch coverage: ternary
// ---------------------------------------------------------------------------

#[test]
fn branch_ternary() {
    let result = instrument_js("const x = true ? 1 : 0;");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(result.coverage_map.branch_map["0"].branch_type, "cond-expr");
    // Ternary branches use comma operator wrapping
    assert!(result.code.contains(".b[0][0]"));
    assert!(result.code.contains(".b[0][1]"));
}

// ---------------------------------------------------------------------------
// Branch coverage: switch
// ---------------------------------------------------------------------------

#[test]
fn branch_switch() {
    let result = instrument_js(
        "switch(x) { case 1: console.log('one'); break; case 2: console.log('two'); break; default: console.log('other'); }",
    );
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(result.coverage_map.branch_map["0"].branch_type, "switch");
    assert_eq!(result.coverage_map.branch_map["0"].locations.len(), 3);
}

// ---------------------------------------------------------------------------
// Branch coverage: logical expressions
// ---------------------------------------------------------------------------

#[test]
fn branch_logical_and() {
    let result = instrument_js("const x = a && b;");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(
        result.coverage_map.branch_map["0"].branch_type,
        "binary-expr"
    );
}

#[test]
fn branch_logical_or() {
    let result = instrument_js("const x = a || b;");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(
        result.coverage_map.branch_map["0"].branch_type,
        "binary-expr"
    );
}

#[test]
fn branch_nullish_coalescing() {
    let result = instrument_js("const x = a ?? b;");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(
        result.coverage_map.branch_map["0"].branch_type,
        "binary-expr"
    );
    assert!(result.code.contains(".b[0][0]"));
    assert!(result.code.contains(".b[0][1]"));
}

// ---------------------------------------------------------------------------
// Branch coverage: logical assignment
// ---------------------------------------------------------------------------

#[test]
fn branch_nullish_assignment() {
    let result = instrument_js("let x = null; x ??= 42;");
    let binary_branches: Vec<_> = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "binary-expr")
        .collect();
    assert_eq!(binary_branches.len(), 1);
    assert_eq!(binary_branches[0].locations.len(), 2);
}

#[test]
fn branch_logical_or_assignment() {
    let result = instrument_js("let x = 0; x ||= 'default';");
    let binary_branch_count = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "binary-expr")
        .count();
    assert_eq!(binary_branch_count, 1);
}

#[test]
fn branch_logical_and_assignment() {
    let result = instrument_js("let x = 1; x &&= doSomething();");
    let binary_branch_count = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "binary-expr")
        .count();
    assert_eq!(binary_branch_count, 1);
}

// ---------------------------------------------------------------------------
// Branch coverage: for loops
// ---------------------------------------------------------------------------

#[test]
fn branch_for_loop() {
    let result = instrument_js("for (let i = 0; i < 10; i++) { console.log(i); }");
    // Should have a "for" branch
    let for_branch_count = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "for")
        .count();
    assert_eq!(for_branch_count, 1);
}

#[test]
fn branch_for_in_loop() {
    let result = instrument_js("for (const key in obj) { console.log(key); }");
    let for_branch_count = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "for")
        .count();
    assert_eq!(for_branch_count, 1);
}

#[test]
fn branch_for_of_loop() {
    let result = instrument_js("for (const item of arr) { console.log(item); }");
    let for_branch_count = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "for")
        .count();
    assert_eq!(for_branch_count, 1);
}

// ---------------------------------------------------------------------------
// Pragma handling
// ---------------------------------------------------------------------------

#[test]
fn pragma_istanbul_ignore_file() {
    let result = instrument_js("/* istanbul ignore file */\nfunction f() { return 1; }");
    // Entire file ignored — no coverage
    assert!(result.coverage_map.fn_map.is_empty());
    assert!(result.coverage_map.statement_map.is_empty());
    assert!(result.coverage_map.branch_map.is_empty());
    // Code should be returned unmodified
    assert!(!result.code.contains("cov_"));
}

#[test]
fn pragma_v8_ignore_file() {
    let result = instrument_js("/* v8 ignore file */\nfunction f() { return 1; }");
    assert!(result.coverage_map.fn_map.is_empty());
}

#[test]
fn pragma_istanbul_ignore_next_function() {
    let result = instrument_js(
        "/* istanbul ignore next */\nfunction ignored() { return 1; }\nfunction counted() { return 2; }",
    );
    // Only 'counted' should be instrumented as a function
    let fn_names: Vec<&str> = result
        .coverage_map
        .fn_map
        .values()
        .map(|f| f.name.as_str())
        .collect();
    assert!(fn_names.contains(&"counted"));
    // 'ignored' should not have a counter
    assert!(!fn_names.contains(&"ignored"));
}

// ---------------------------------------------------------------------------
// Source map
// ---------------------------------------------------------------------------

#[test]
fn source_map_generation() {
    let opts = InstrumentOptions {
        source_map: true,
        ..InstrumentOptions::default()
    };
    let result = instrument("function f() { return 1; }", "test.js", &opts).unwrap();
    assert!(result.source_map.is_some());
    let sm = result.source_map.unwrap();
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&sm).unwrap();
    assert_eq!(parsed["version"], 3);
}

#[test]
fn source_map_disabled_by_default() {
    let result = instrument_js("function f() { return 1; }");
    assert!(result.source_map.is_none());
}

#[test]
fn source_map_accounts_for_preamble_offset() {
    let source = "function f() {\n  return 1;\n}";
    let opts = InstrumentOptions {
        source_map: true,
        ..InstrumentOptions::default()
    };
    let result = instrument(source, "test.js", &opts).unwrap();
    let sm_json = result.source_map.as_ref().unwrap();
    let sm = oxc_sourcemap::SourceMap::from_json_string(sm_json).unwrap();

    // The preamble is 1 line. So the first mapping in the source map should
    // have a generated line >= 1 (0-indexed), not 0.
    // This verifies the preamble offset was applied.
    let tokens: Vec<_> = sm.get_tokens().collect();
    assert!(
        !tokens.is_empty(),
        "Source map should have at least one mapping"
    );
    // First token's generated line should be >= 1 (after preamble)
    let first_gen_line = tokens[0].get_dst_line();
    assert!(
        first_gen_line >= 1,
        "First mapping should be on line >= 1 (after preamble), got line {first_gen_line}"
    );
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn parse_error_returns_err() {
    let result = instrument("function {{{", "bad.js", &default_opts());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("parse error"));
}

// ---------------------------------------------------------------------------
// Istanbul format compliance
// ---------------------------------------------------------------------------

#[test]
fn coverage_map_has_required_fields() {
    let result = instrument_js("function f() { return 1; }");
    let json = serde_json::to_value(&result.coverage_map).unwrap();
    assert!(json["path"].is_string());
    assert!(json["statementMap"].is_object());
    assert!(json["fnMap"].is_object());
    assert!(json["branchMap"].is_object());
    assert!(json["s"].is_object());
    assert!(json["f"].is_object());
    assert!(json["b"].is_object());
}

#[test]
fn hit_counts_initialized_to_zero() {
    let result = instrument_js("function f() { return 1; }");
    for count in result.coverage_map.s.values() {
        assert_eq!(*count, 0);
    }
    for count in result.coverage_map.f.values() {
        assert_eq!(*count, 0);
    }
    for counts in result.coverage_map.b.values() {
        for count in counts {
            assert_eq!(*count, 0);
        }
    }
}

#[test]
fn statement_map_keys_are_sequential_strings() {
    let result = instrument_js("const a = 1;\nconst b = 2;\nconst c = 3;");
    let keys: Vec<usize> = result
        .coverage_map
        .statement_map
        .keys()
        .map(|k| k.parse::<usize>().unwrap())
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted);
    // Keys should be 0, 1, 2
    assert_eq!(sorted, vec![0, 1, 2]);
}

#[test]
fn positions_are_1_based_line_0_based_column() {
    let result = instrument_js("const x = 1;");
    let loc = &result.coverage_map.statement_map["0"];
    assert!(loc.start.line >= 1, "Line should be 1-based");
    // Column 0 is valid (0-based)
}

// ---------------------------------------------------------------------------
// Coverage variable name
// ---------------------------------------------------------------------------

#[test]
fn custom_coverage_variable() {
    let opts = InstrumentOptions {
        coverage_variable: "__custom_cov__".to_string(),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "test.js", &opts).unwrap();
    assert!(result.code.contains("__custom_cov__"));
}

// ---------------------------------------------------------------------------
// Deterministic output
// ---------------------------------------------------------------------------

#[test]
fn deterministic_cov_function_name() {
    let result1 = instrument_js("const x = 1;");
    let result2 = instrument_js("const x = 1;");
    // Same input → same function name
    // Extract function name from code
    let extract_name = |code: &str| -> String {
        let start = code.find("var cov_").unwrap() + 4;
        let end = code[start..].find(' ').unwrap() + start;
        code[start..end].to_string()
    };
    assert_eq!(extract_name(&result1.code), extract_name(&result2.code));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_source() {
    let result = instrument_js("");
    assert!(result.coverage_map.fn_map.is_empty());
    assert!(result.coverage_map.statement_map.is_empty());
    assert!(result.coverage_map.branch_map.is_empty());
}

#[test]
fn nested_functions() {
    let result =
        instrument_js("function outer() { function inner() { return 1; } return inner(); }");
    assert_eq!(result.coverage_map.fn_map.len(), 2);
}

#[test]
fn nested_if_else() {
    let result = instrument_js("if (a) { if (b) { x(); } else { y(); } } else { z(); }");
    // Should have 2 if-branches
    let if_branch_count = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "if")
        .count();
    assert_eq!(if_branch_count, 2);
}

#[test]
fn chained_logical_expressions() {
    let result = instrument_js("const x = a && b && c;");
    // Istanbul flattens a && b && c into 1 binary-expr branch with 3 locations
    let binary_branches: Vec<_> = result
        .coverage_map
        .branch_map
        .values()
        .filter(|b| b.branch_type == "binary-expr")
        .collect();
    assert_eq!(binary_branches.len(), 1);
    assert_eq!(binary_branches[0].locations.len(), 3);
}

#[test]
fn typescript_source() {
    let opts = InstrumentOptions::default();
    let result = instrument(
        "function add(a: number, b: number): number { return a + b; }",
        "test.ts",
        &opts,
    )
    .unwrap();
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "add");
}

#[test]
fn jsx_source() {
    let opts = InstrumentOptions::default();
    let result = instrument(
        "function App() { return <div>Hello</div>; }",
        "test.jsx",
        &opts,
    )
    .unwrap();
    assert_eq!(result.coverage_map.fn_map.len(), 1);
}

#[test]
fn coverage_map_json_roundtrip() {
    let result = instrument_js("function f() { if (true) { return 1; } return 0; }");
    let json = serde_json::to_string(&result.coverage_map).unwrap();
    // Should be valid JSON and deserializable
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed["path"], "test.js");
}
