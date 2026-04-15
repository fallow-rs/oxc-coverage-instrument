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
    // Function declarations are not statements in Istanbul's model.
    // f: const x = 1 + return x = 2, g: throw = 1. Total = 3.
    assert_eq!(result.coverage_map.statement_map.len(), 3);
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
    assert!(result.coverage_map.fn_map["0"].name.starts_with("(anonymous_"));
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
    assert_eq!(result.coverage_map.branch_map["0"].branch_type, "binary-expr");
}

#[test]
fn branch_logical_or() {
    let result = instrument_js("const x = a || b;");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(result.coverage_map.branch_map["0"].branch_type, "binary-expr");
}

#[test]
fn branch_nullish_coalescing() {
    let result = instrument_js("const x = a ?? b;");
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    assert_eq!(result.coverage_map.branch_map["0"].branch_type, "binary-expr");
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
    let binary_branch_count =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "binary-expr").count();
    assert_eq!(binary_branch_count, 1);
}

#[test]
fn branch_logical_and_assignment() {
    let result = instrument_js("let x = 1; x &&= doSomething();");
    let binary_branch_count =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "binary-expr").count();
    assert_eq!(binary_branch_count, 1);
}

// ---------------------------------------------------------------------------
// Loops: no branch entries (matching Istanbul)
// ---------------------------------------------------------------------------

#[test]
fn loops_do_not_create_branch_entries() {
    let result = instrument_js(
        "for (let i = 0; i < 10; i++) { x(); } for (const k in o) { y(); } for (const v of a) { z(); } while (true) { break; } do { w(); } while (false);",
    );
    // Istanbul does NOT create branch entries for loops — only statement counters
    assert!(
        result.coverage_map.branch_map.is_empty(),
        "Loops should not produce branch entries (matching Istanbul)"
    );
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
    let fn_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();
    assert!(fn_names.contains(&"counted"));
    // 'ignored' should not have a counter
    assert!(!fn_names.contains(&"ignored"));
}

// ---------------------------------------------------------------------------
// Source map
// ---------------------------------------------------------------------------

#[test]
fn source_map_generation() {
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };
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
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };
    let result = instrument(source, "test.js", &opts).unwrap();
    let sm_json = result.source_map.as_ref().unwrap();
    let sm = oxc_sourcemap::SourceMap::from_json_string(sm_json).unwrap();

    // The preamble is 1 line. So the first mapping in the source map should
    // have a generated line >= 1 (0-indexed), not 0.
    // This verifies the preamble offset was applied.
    let tokens: Vec<_> = sm.get_tokens().collect();
    assert!(!tokens.is_empty(), "Source map should have at least one mapping");
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
    let keys: Vec<usize> =
        result.coverage_map.statement_map.keys().map(|k| k.parse::<usize>().unwrap()).collect();
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
    let if_branch_count =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "if").count();
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
    let result =
        instrument("function App() { return <div>Hello</div>; }", "test.jsx", &opts).unwrap();
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

// ---------------------------------------------------------------------------
// Nested arrows (bug #4 regression test)
// ---------------------------------------------------------------------------

#[test]
fn nested_arrow_functions_both_get_counters() {
    let result = instrument_js("const f = (x) => (y) => x + y;");
    assert_eq!(result.coverage_map.fn_map.len(), 2);
    // Both functions should have counter entries
    assert_eq!(result.coverage_map.f.len(), 2);
    // Instrumented code should contain both f[0] and f[1]
    assert!(result.code.contains(".f[0]"));
    assert!(result.code.contains(".f[1]"));
}

#[test]
fn deeply_nested_arrows() {
    let result = instrument_js("const f = (a) => (b) => (c) => a + b + c;");
    assert_eq!(result.coverage_map.fn_map.len(), 3);
    assert_eq!(result.coverage_map.f.len(), 3);
}

// ---------------------------------------------------------------------------
// Pragma: ignore next on arrow functions
// ---------------------------------------------------------------------------

#[test]
fn pragma_ignore_next_arrow_function() {
    let result = instrument_js(
        "/* istanbul ignore next */\nconst ignored = () => 1;\nconst counted = () => 2;",
    );
    let fn_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();
    assert!(fn_names.contains(&"counted"));
    assert!(!fn_names.contains(&"ignored"));
}

// ---------------------------------------------------------------------------
// Pragma: ignore if/else effect verification
// ---------------------------------------------------------------------------

#[test]
fn pragma_ignore_if_skips_consequent_counter() {
    let result = instrument_js(
        "function f(x) {\n  /* istanbul ignore if */\n  if (x < 0) { throw new Error(); }\n  return x;\n}",
    );
    // Should still have a branch entry
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    // The if-branch counter (b[0][0]) should NOT be in the code
    assert!(!result.code.contains(".b[0][0]"));
    // The else-branch counter should still be absent (no else clause)
}

#[test]
fn pragma_ignore_else_skips_alternate_counter() {
    let result = instrument_js(
        "function f(x) {\n  /* istanbul ignore else */\n  if (x > 0) { return 'pos'; } else { return 'neg'; }\n}",
    );
    assert_eq!(result.coverage_map.branch_map.len(), 1);
    // The if-branch counter should be present
    assert!(result.code.contains(".b[0][0]"));
    // The else-branch counter should NOT be present
    assert!(!result.code.contains(".b[0][1]"));
}

// ---------------------------------------------------------------------------
// Pragma: unknown pragma → unhandled_pragmas
// ---------------------------------------------------------------------------

#[test]
fn unknown_pragma_populates_unhandled_pragmas() {
    let result = instrument_js("/* istanbul ignore banana */\nfunction f() { return 1; }");
    assert!(!result.unhandled_pragmas.is_empty());
    assert!(result.unhandled_pragmas[0].comment.contains("banana"));
    assert_eq!(result.unhandled_pragmas[0].line, 1);
}

#[test]
fn known_pragmas_not_in_unhandled() {
    let result = instrument_js("/* istanbul ignore next */\nfunction f() { return 1; }");
    assert!(result.unhandled_pragmas.is_empty());
}

// ---------------------------------------------------------------------------
// Pragma: v8/c8 variants for if/else/file
// ---------------------------------------------------------------------------

#[test]
fn pragma_v8_ignore_next() {
    let result =
        instrument_js("/* v8 ignore next */\nfunction ignored() {}\nfunction counted() {}");
    let fn_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();
    assert!(!fn_names.contains(&"ignored"));
    assert!(fn_names.contains(&"counted"));
}

#[test]
fn pragma_c8_ignore_file() {
    let result = instrument_js("/* c8 ignore file */\nfunction f() { return 1; }");
    assert!(result.coverage_map.fn_map.is_empty());
}

// ---------------------------------------------------------------------------
// Input source map
// ---------------------------------------------------------------------------

#[test]
fn input_source_map_stored_on_coverage() {
    let opts = InstrumentOptions {
        input_source_map: Some(
            r#"{"version":3,"sources":["test.ts"],"mappings":"AAAA"}"#.to_string(),
        ),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "test.js", &opts).unwrap();
    let json = serde_json::to_value(&result.coverage_map).unwrap();
    assert!(json["inputSourceMap"].is_object());
    assert_eq!(json["inputSourceMap"]["version"], 3);
}

#[test]
fn input_source_map_none_by_default() {
    let result = instrument_js("const x = 1;");
    let json = serde_json::to_value(&result.coverage_map).unwrap();
    assert!(json.get("inputSourceMap").is_none());
}

#[test]
fn source_map_composed_with_input_source_map() {
    let opts = InstrumentOptions {
        source_map: true,
        input_source_map: Some(
            r#"{"version":3,"sources":["original.ts"],"sourcesContent":["const x: number = 1;"],"mappings":"AAAA"}"#.to_string(),
        ),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "test.js", &opts).unwrap();
    assert!(result.source_map.is_some());
    let sm: serde_json::Value = serde_json::from_str(result.source_map.as_ref().unwrap()).unwrap();
    // The composed source map should reference the original TS file, not test.js
    let sources = sm["sources"].as_array().unwrap();
    let has_original = sources.iter().any(|s| s.as_str() == Some("original.ts"));
    assert!(has_original, "Composed source map should reference original.ts, got: {sources:?}");
}

#[test]
fn input_source_map_invalid_json_ignored() {
    let opts = InstrumentOptions {
        input_source_map: Some("not valid json".to_string()),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "test.js", &opts).unwrap();
    let json = serde_json::to_value(&result.coverage_map).unwrap();
    assert!(json.get("inputSourceMap").is_none());
}

// ---------------------------------------------------------------------------
// Coverage variable validation
// ---------------------------------------------------------------------------

#[test]
fn invalid_coverage_variable_returns_error() {
    let opts = InstrumentOptions {
        coverage_variable: "it's_broken".to_string(),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "test.js", &opts);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("invalid coverage variable"));
}

#[test]
fn valid_coverage_variable_with_dollar() {
    let opts = InstrumentOptions {
        coverage_variable: "$coverage".to_string(),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "test.js", &opts);
    assert!(result.is_ok());
    assert!(result.unwrap().code.contains("$coverage"));
}

// ---------------------------------------------------------------------------
// Async function handling
// ---------------------------------------------------------------------------

#[test]
fn async_function_declaration() {
    let result = instrument_js("async function fetchData() { return await fetch('/api'); }");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "fetchData");
    // decl_span should NOT use hardcoded +8 — verify it covers "async function fetchData"
    let decl = &result.coverage_map.fn_map["0"].decl;
    // The declaration should span from "async" (col 0) to at least past "fetchData"
    assert!(decl.end.column > 8, "decl_span should extend past 'function' for async");
}

#[test]
fn async_arrow_function() {
    let result = instrument_js("const f = async (x) => { return await x; };");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "f");
}

// ---------------------------------------------------------------------------
// Destructuring defaults (AssignmentPattern branch)
// ---------------------------------------------------------------------------

#[test]
fn destructuring_default_creates_branch() {
    let result = instrument_js("const { x = 1, y = 2 } = obj;");
    let default_count =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "default-arg").count();
    assert_eq!(default_count, 2);
}

// ---------------------------------------------------------------------------
// Computed method keys
// ---------------------------------------------------------------------------

#[test]
fn computed_method_key_uses_anonymous_name() {
    let result = instrument_js("class C { [Symbol.iterator]() { return this; } }");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    // Computed key → anonymous name
    assert!(result.coverage_map.fn_map["0"].name.contains("anonymous"));
}

// ---------------------------------------------------------------------------
// Switch fall-through
// ---------------------------------------------------------------------------

#[test]
fn switch_fall_through_cases() {
    let result = instrument_js(
        "function f(x) { switch(x) { case 1: case 2: return 'a'; case 3: return 'b'; } }",
    );
    let switch_branches: Vec<_> =
        result.coverage_map.branch_map.values().filter(|b| b.branch_type == "switch").collect();
    assert_eq!(switch_branches.len(), 1);
    // 3 cases
    assert_eq!(switch_branches[0].locations.len(), 3);
}

// ---------------------------------------------------------------------------
// Unknown file extension fallback
// ---------------------------------------------------------------------------

#[test]
fn unknown_extension_treated_as_js() {
    let result = instrument("function f() { return 1; }", "test.coffee", &default_opts());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().coverage_map.fn_map.len(), 1);
}

// ---------------------------------------------------------------------------
// Source map + ignore file
// ---------------------------------------------------------------------------

#[test]
fn source_map_with_ignore_file() {
    let opts = InstrumentOptions { source_map: true, ..InstrumentOptions::default() };
    let result =
        instrument("/* istanbul ignore file */\nfunction f() { return 1; }", "test.js", &opts)
            .unwrap();
    // Ignored file returns no source map even when requested
    assert!(result.source_map.is_none());
}

// ---------------------------------------------------------------------------
// Multiple parse errors joined
// ---------------------------------------------------------------------------

#[test]
fn multiple_parse_errors_joined() {
    let result = instrument("function { const }", "bad.js", &default_opts());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("parse error"));
}

// ---------------------------------------------------------------------------
// Coverage map ingestion (parse_coverage_map / FileCoverage::from_json)
// ---------------------------------------------------------------------------

#[test]
fn parse_coverage_map_roundtrip() {
    use oxc_coverage_instrument::parse_coverage_map;

    let result = instrument_js("function f() { if (true) { return 1; } return 0; }");
    let mut root = std::collections::BTreeMap::new();
    root.insert(result.coverage_map.path.clone(), &result.coverage_map);
    let json = serde_json::to_string(&root).unwrap();

    let parsed = parse_coverage_map(&json).unwrap();
    assert!(parsed.contains_key("test.js"));
    assert_eq!(parsed["test.js"].fn_map.len(), result.coverage_map.fn_map.len());
}

#[test]
fn file_coverage_from_json_roundtrip() {
    use oxc_coverage_instrument::FileCoverage;

    let result = instrument_js("function f() { return 1; }");
    let json = serde_json::to_string(&result.coverage_map).unwrap();
    let parsed = FileCoverage::from_json(&json).unwrap();
    assert_eq!(parsed.path, "test.js");
    assert_eq!(parsed.fn_map.len(), result.coverage_map.fn_map.len());
}

#[test]
fn parse_coverage_map_invalid_json() {
    use oxc_coverage_instrument::parse_coverage_map;
    assert!(parse_coverage_map("not json").is_err());
}

#[test]
fn parse_coverage_map_null_hit_counts() {
    use oxc_coverage_instrument::parse_coverage_map;

    // Istanbul allows null in s/f/b hit count maps, null in position fields,
    // and even empty objects `{}` for positions (e.g., branch locations with
    // unknown spans). Real-world coverage files exercise all these variants.
    let json = r#"{
        "test.js": {
            "path": "test.js",
            "statementMap": {"0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": null}}},
            "fnMap": {"0": {"name": "f", "line": null, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": null}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": null}}}},
            "branchMap": {"0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": null}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": null}}, {"start": {}, "end": {}}]}},
            "s": {"0": null},
            "f": {"0": null},
            "b": {"0": [null, 1]}
        }
    }"#;

    let parsed = parse_coverage_map(json).unwrap();
    let file = &parsed["test.js"];
    assert_eq!(file.s["0"], 0, "null statement count should coerce to 0");
    assert_eq!(file.f["0"], 0, "null function count should coerce to 0");
    assert_eq!(file.b["0"], vec![0, 1], "null branch arm count should coerce to 0");
    assert_eq!(file.fn_map["0"].line, 0, "null fn line should coerce to 0");
    assert_eq!(file.statement_map["0"].end.column, 0, "null position column should coerce to 0");
    // Empty position objects `{}` should default both fields to 0
    let empty_pos = &file.branch_map["0"].locations[1].start;
    assert_eq!(empty_pos.line, 0, "missing line in empty position should default to 0");
    assert_eq!(empty_pos.column, 0, "missing column in empty position should default to 0");
}

#[test]
fn parse_coverage_map_null_string_fields() {
    use oxc_coverage_instrument::parse_coverage_map;

    // Istanbul-compatible tools may produce null for path, name, and type
    // fields during coverage merging or from non-standard instrumentation.
    let json = r#"{
        "test.js": {
            "path": null,
            "statementMap": {},
            "fnMap": {"0": {"name": null, "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}}}},
            "branchMap": {"0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}}, "line": 1, "type": null, "locations": []}},
            "s": {},
            "f": {"0": 0},
            "b": {}
        }
    }"#;

    let parsed = parse_coverage_map(json).unwrap();
    let file = &parsed["test.js"];
    assert_eq!(file.path, "", "null path should coerce to empty string");
    assert_eq!(file.fn_map["0"].name, "", "null fn name should coerce to empty string");
    assert_eq!(
        file.branch_map["0"].branch_type, "",
        "null branch type should coerce to empty string"
    );
}

#[test]
fn parse_coverage_map_missing_string_fields() {
    use oxc_coverage_instrument::parse_coverage_map;

    // Fields entirely absent from JSON (not just null) should also default.
    let json = r#"{
        "test.js": {
            "statementMap": {},
            "fnMap": {"0": {"line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}}}},
            "branchMap": {"0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 10}}, "line": 1, "locations": []}},
            "s": {},
            "f": {"0": 0},
            "b": {}
        }
    }"#;

    let parsed = parse_coverage_map(json).unwrap();
    let file = &parsed["test.js"];
    assert_eq!(file.path, "", "missing path should default to empty string");
    assert_eq!(file.fn_map["0"].name, "", "missing fn name should default to empty string");
    assert_eq!(
        file.branch_map["0"].branch_type, "",
        "missing branch type should default to empty string"
    );
}

// ---------------------------------------------------------------------------
// Source map composition fallback (invalid input source map)
// ---------------------------------------------------------------------------

#[test]
fn source_map_with_invalid_input_still_works() {
    let opts = InstrumentOptions {
        source_map: true,
        input_source_map: Some("not valid json".to_string()),
        ..InstrumentOptions::default()
    };
    let result = instrument("function f() { return 1; }", "test.js", &opts).unwrap();
    // Should still produce a source map (just not composed)
    assert!(result.source_map.is_some());
}

// ---------------------------------------------------------------------------
// Gap analysis: constructs that Istanbul instruments but we might miss
// ---------------------------------------------------------------------------

#[test]
fn gap_object_method_gets_function_counter() {
    // Istanbul creates function counters for object shorthand methods
    let result = instrument_js("const obj = { foo() { return 1; }, bar() { return 2; } };");
    // Should have 2 function entries (foo and bar)
    assert!(
        result.coverage_map.fn_map.len() >= 2,
        "Object methods should get function counters, got {} functions: {:?}",
        result.coverage_map.fn_map.len(),
        result.coverage_map.fn_map.values().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn gap_getter_setter_get_function_counter() {
    // Istanbul creates function counters for getter/setter in object literals
    let result = instrument_js("const obj = { get x() { return 1; }, set x(v) { this._x = v; } };");
    assert!(
        result.coverage_map.fn_map.len() >= 2,
        "Getters/setters should get function counters, got {} functions: {:?}",
        result.coverage_map.fn_map.len(),
        result.coverage_map.fn_map.values().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn class_property_initializer_gets_statement() {
    let result = instrument_js("class Foo { x = 1; y = computeDefault(); }");
    let stmt_count = result.coverage_map.statement_map.len();
    // Class declarations are containers, not statements. Only the two property
    // initializers are counted.
    assert_eq!(
        stmt_count, 2,
        "Class property initializers should get statement counters, got {stmt_count} statements",
    );
}

#[test]
fn private_class_property_initializer_gets_statement() {
    let result = instrument_js("class Foo { #x = computeDefault(); }");
    let stmt_count = result.coverage_map.statement_map.len();
    // Class declarations are containers. Only the private property initializer is counted.
    assert_eq!(
        stmt_count, 1,
        "Private class property initializers should get statement counters, got {stmt_count} statements",
    );
}

#[test]
fn class_property_initializer_wraps_value() {
    let result = instrument_js("class Foo {\n  x = 1;\n  y = computeDefault();\n}");
    // Initializer values should be wrapped: x = (++cov().s[N], value)
    assert!(result.code.contains(".s["), "Should contain statement counters in class body");
}

// ---------------------------------------------------------------------------
// ignoreClassMethods
// ---------------------------------------------------------------------------

#[test]
fn ignore_class_methods_skips_function_counter() {
    let opts = InstrumentOptions {
        ignore_class_methods: vec!["render".to_string(), "componentDidMount".to_string()],
        ..default_opts()
    };
    let result = instrument(
        "class App { render() { return 1; } update() { return 2; } componentDidMount() { return 3; } }",
        "test.js",
        &opts,
    ).unwrap();
    // Only 'update' should have a function counter; 'render' and 'componentDidMount' are skipped.
    assert_eq!(
        result.coverage_map.fn_map.len(),
        1,
        "Only non-ignored methods should get function counters"
    );
    assert_eq!(result.coverage_map.fn_map["0"].name, "update");
}

#[test]
fn ignore_class_methods_still_instruments_body() {
    let opts =
        InstrumentOptions { ignore_class_methods: vec!["render".to_string()], ..default_opts() };
    let result =
        instrument("class App { render() { const x = 1; return x; } }", "test.js", &opts).unwrap();
    // No function counter for render, but body statements are still counted
    assert_eq!(result.coverage_map.fn_map.len(), 0);
    // Class declaration is a container; only body statements count: const x = 1, return x.
    assert_eq!(result.coverage_map.statement_map.len(), 2);
}

#[test]
fn ignore_class_methods_empty_list_instruments_all() {
    let result = instrument_js("class App { render() { return 1; } update() { return 2; } }");
    assert_eq!(result.coverage_map.fn_map.len(), 2);
}

#[test]
fn ignore_class_methods_string_literal_key() {
    let opts =
        InstrumentOptions { ignore_class_methods: vec!["render".to_string()], ..default_opts() };
    // String-literal method key should also be matched
    let result = instrument(
        "class App { \"render\"() { return 1; } update() { return 2; } }",
        "test.js",
        &opts,
    )
    .unwrap();
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "update");
}

#[test]
fn ignore_class_methods_with_pragma_no_leak() {
    // When both ignoreClassMethods AND a pragma target the same method,
    // skip_next must not leak to the next statement after the method.
    let opts =
        InstrumentOptions { ignore_class_methods: vec!["render".to_string()], ..default_opts() };
    let result = instrument(
        "class App { /* istanbul ignore next */ render() { return 1; } update() { return 2; } }",
        "test.js",
        &opts,
    )
    .unwrap();
    // render is skipped (both by pragma and ignoreClassMethods)
    // update must NOT be affected by skip_next leaking
    assert_eq!(result.coverage_map.fn_map.len(), 1, "Only update should have a function counter");
    assert_eq!(result.coverage_map.fn_map["0"].name, "update");
}

// ---------------------------------------------------------------------------
// reportLogic (bT tracking)
// ---------------------------------------------------------------------------

#[test]
fn report_logic_adds_bt_field() {
    let opts = InstrumentOptions { report_logic: true, ..default_opts() };
    let result = instrument("const x = a && b;", "test.js", &opts).unwrap();
    assert!(result.coverage_map.b_t.is_some(), "bT should be present when report_logic is enabled");
    let b_t = result.coverage_map.b_t.unwrap();
    assert_eq!(b_t.len(), 1, "Should have 1 bT entry for the logical expression");
    // Each entry should have the same number of paths as the branch
    let branch_key = b_t.keys().next().unwrap();
    assert_eq!(b_t[branch_key].len(), 2, "bT entry should have 2 paths (a and b)");
}

#[test]
fn report_logic_disabled_no_bt_field() {
    let result = instrument_js("const x = a && b;");
    assert!(
        result.coverage_map.b_t.is_none(),
        "bT should not be present when report_logic is disabled"
    );
}

#[test]
fn report_logic_wraps_with_helper() {
    let opts = InstrumentOptions { report_logic: true, ..default_opts() };
    let result = instrument("const x = a || b;", "test.js", &opts).unwrap();
    // The code should contain calls to the truthy tracking helper
    assert!(result.code.contains("_bt("), "Should contain truthy tracking helper calls");
    // The preamble should declare the helper function and temp variable
    assert!(result.code.contains("_temp;"), "Should declare temp variable");
    assert!(result.code.contains("function "), "Should contain helper function definition");
    assert!(result.code.contains(".bT["), "Helper should reference bT counter");
    // The helper should use Istanbul's non-trivial truthy check:
    // empty arrays [] and empty objects {} are NOT counted as truthy
    assert!(
        result.code.contains("!Array.isArray("),
        "Should check if NOT an array (Istanbul's check)"
    );
    assert!(
        result.code.contains("Object.values("),
        "Should check Object.values length (Istanbul's check)"
    );
    assert!(
        result.code.contains("Object.getPrototypeOf("),
        "Should check prototype (Istanbul's check)"
    );
}

#[test]
fn report_logic_only_for_logical_expressions() {
    let opts = InstrumentOptions { report_logic: true, ..default_opts() };
    let result = instrument("if (x) { a(); } else { b(); }", "test.js", &opts).unwrap();
    // if/else branches should NOT create bT entries — only logical expressions do
    assert!(
        result.coverage_map.b_t.is_none() || result.coverage_map.b_t.as_ref().unwrap().is_empty(),
        "bT should not have entries for if/else branches"
    );
}

#[test]
fn report_logic_chained_logical() {
    let opts = InstrumentOptions { report_logic: true, ..default_opts() };
    let result = instrument("const x = a && b && c;", "test.js", &opts).unwrap();
    let b_t = result.coverage_map.b_t.unwrap();
    assert_eq!(b_t.len(), 1);
    let entry = b_t.values().next().unwrap();
    assert_eq!(entry.len(), 3, "Chained a && b && c should have 3 bT paths");
}

#[test]
fn report_logic_nullish_coalescing() {
    let opts = InstrumentOptions { report_logic: true, ..default_opts() };
    let result = instrument("const x = a ?? b;", "test.js", &opts).unwrap();
    let b_t = result.coverage_map.b_t.unwrap();
    assert_eq!(b_t.len(), 1, "Nullish coalescing should have bT entry");
}

// ---------------------------------------------------------------------------
// Counter hoisting for exports
// ---------------------------------------------------------------------------

#[test]
fn export_function_has_no_statement_counter() {
    // istanbul-lib-instrument doesn't emit a statement counter for a function
    // declaration (exported or not). Only the function counter and the body
    // statements are counted.
    let result = instrument_js("export function foo() { return 1; }");
    let export_pos = result.code.find("export").unwrap();
    assert!(
        !result.code[..export_pos].contains("++"),
        "Export function declarations should not get a hoisted statement counter"
    );
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "foo");
    // Exactly one statement: the return inside the body.
    assert_eq!(result.coverage_map.statement_map.len(), 1);
}

#[test]
fn export_const_arrow_gets_per_declarator_counter() {
    // istanbul-lib-instrument wraps the declarator init with a statement
    // counter: `export const add = (++cov().s[N], (a, b) => …)`.
    // The counter appears AFTER `export`, inline in the init expression.
    let result = instrument_js("export const add = (a, b) => a + b;");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "add");
    // Two statements: the declarator init wrapper and the arrow body's return.
    assert_eq!(result.coverage_map.statement_map.len(), 2);
}

/// Regression test: every declaration-container variant that istanbul-lib-instrument
/// skips must also be skipped by us. Covers the full skip list in
/// `enter_statement` so a wrongly-mapped `Statement` variant would surface here.
#[test]
fn declaration_containers_produce_no_statement_counters() {
    // Each input contains only container nodes (no executable statements). The
    // coverage map should contain zero statements. Functions and classes still
    // produce function counters, but no statement counters.
    let cases: &[(&str, &str, usize)] = &[
        ("bare_function", "function foo() {}", 1),
        ("bare_class", "class C {}", 0),
        ("export_function", "export function foo() {}", 1),
        ("export_class", "export class C {}", 0),
        ("export_default_function", "export default function foo() {}", 1),
        ("export_default_class", "export default class C {}", 0),
        ("export_all", "export * from './x';", 0),
        ("export_named_reexport", "export { x } from './x';", 0),
        ("import_decl", "import x from './x';", 0),
        ("ts_type_alias", "type X = number;", 0),
        ("ts_interface", "interface I {}", 0),
        ("ts_enum", "enum E { A, B }", 0),
        ("ts_module", "declare module 'x' {}", 0),
    ];
    for (name, src, expected_fns) in cases {
        let result = instrument(src, "test.ts", &default_opts())
            .unwrap_or_else(|e| panic!("{name} failed to parse: {e}"));
        assert_eq!(
            result.coverage_map.statement_map.len(),
            0,
            "{name}: expected 0 statement entries for {src:?}, got {}",
            result.coverage_map.statement_map.len()
        );
        assert_eq!(
            result.coverage_map.fn_map.len(),
            *expected_fns,
            "{name}: function count mismatch for {src:?}",
        );
    }
}

/// Regression test for `fnMap[*].decl` parity with istanbul-lib-instrument.
/// istanbul sets `decl` to the identifier span for named functions; prior to
/// v0.3.5 we were setting it to the span from the `function` keyword through
/// the identifier. Caught by the Vitest validator against v0.3.4.
#[test]
fn fn_decl_span_matches_istanbul() {
    // `export function sum(a, b) { return a + b }` — istanbul emits decl
    // column 16..19 (the identifier `sum`), matching `id.span`. Previously we
    // emitted 7..19 (from the `function` keyword through the identifier).
    let result = instrument_js("export function sum(a, b) { return a + b; }");
    let f = &result.coverage_map.fn_map["0"];
    assert_eq!(f.name, "sum");
    assert_eq!(f.decl.start.line, 1);
    assert_eq!(
        f.decl.start.column, 16,
        "decl.start should point at identifier, not `function` keyword"
    );
    assert_eq!(f.decl.end.column, 19);

    // Anonymous function expression: istanbul uses a 1-char span at the start
    // of the `function` keyword (where the name would go).
    let result = instrument_js("const f = function(a) { return a; };");
    let f = &result.coverage_map.fn_map["0"];
    assert_eq!(f.decl.start.column, 10);
    assert_eq!(f.decl.end.column, 11, "anon fn decl should be a 1-char marker");
}

/// Regression test: a module of simple function exports must produce the same
/// statement and function counts that istanbul-lib-instrument does. The Vitest
/// istanbul provider compares these directly in its `coverage-final.json`.
/// Before this fix we emitted a separate statement counter for each exported
/// `function` declaration, inflating statement coverage to 8/4 instead of 4/4.
#[test]
fn istanbul_parity_for_exported_function_module() {
    // Source mirrors vitest/test/coverage-test/fixtures/src/math.ts.
    let source = "export function sum(a, b) {\n  return a + b\n}\n\n\
         export function subtract(a, b) {\n  return a - b\n}\n\n\
         export function multiply(a, b) {\n  return a * b\n}\n\n\
         export function remainder(a, b) {\n  return a % b\n}\n";
    let result = instrument(source, "math.ts", &default_opts()).unwrap();

    // istanbul-lib-instrument for the same source produces:
    //   statements: 4 (one per `return` body)
    //   functions:  4
    //   branches:   0
    assert_eq!(
        result.coverage_map.statement_map.len(),
        4,
        "expected 4 statement entries (one per return), got {}",
        result.coverage_map.statement_map.len()
    );
    assert_eq!(result.coverage_map.fn_map.len(), 4);
    assert_eq!(result.coverage_map.branch_map.len(), 0);

    // Every statement span should point at a `return ...` inside a function body,
    // not at the enclosing `export function ...` declaration.
    for loc in result.coverage_map.statement_map.values() {
        assert_eq!(loc.start.column, 2, "return-statement column should be 2, got {loc:?}");
        assert_eq!(loc.end.column, 14, "return-statement end column should be 14, got {loc:?}");
    }

    // No hoisted statement counter should appear before an `export function`
    // — that was the original bug.
    let first_export = result.code.find("export").expect("instrumented code should still export");
    assert!(
        !result.code[..first_export].contains("++cov"),
        "exported function declarations must not produce hoisted statement counters"
    );
}

#[test]
fn report_logic_json_roundtrip() {
    let opts = InstrumentOptions { report_logic: true, ..default_opts() };
    let result = instrument("const x = a || b;", "test.js", &opts).unwrap();
    // Serialize and deserialize to verify bT survives JSON roundtrip
    let json = serde_json::to_string(&result.coverage_map).unwrap();
    assert!(json.contains("\"bT\""), "JSON should contain bT field");
    let parsed: oxc_coverage_instrument::FileCoverage = serde_json::from_str(&json).unwrap();
    assert!(parsed.b_t.is_some());
}
