//! Integration tests for the public `instrument()` API: statement, function
//! and branch coverage, source maps, coverage-map ingestion and error
//! handling. Pragma handling lives in `pragma_test.rs`.

use rustc_hash::FxHashSet;

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn default_opts() -> InstrumentOptions {
    InstrumentOptions::default()
}

fn instrument_js(source: &str) -> oxc_coverage_instrument::InstrumentResult {
    instrument(source, "test.js", &default_opts()).unwrap()
}

const ECMASCRIPT_LINE_TERMINATORS: [(&str, &str); 5] =
    [("LF", "\n"), ("CRLF", "\r\n"), ("CR", "\r"), ("LS", "\u{2028}"), ("PS", "\u{2029}")];

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
fn statement_locations_follow_ecmascript_line_terminators() {
    for (name, terminator) in ECMASCRIPT_LINE_TERMINATORS {
        let source = format!("first();{terminator}second();");
        let result = instrument_js(&source);
        let second = &result.coverage_map.statement_map["1"];
        assert_eq!((second.start.line, second.start.column), (2, 0), "{name}");
    }
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
    // Only the `const x = 1` inside the block counts: blocks and empty
    // statements are containers.
    assert_eq!(result.coverage_map.statement_map.len(), 1);
}

// An inline `export const fn = <arrow/function>` hoists its per-declarator
// statement counter to a sibling of the enclosing statement, which is the
// `ExportNamedDeclaration` rather than the inner `VariableDeclaration`. Each
// form below must emit that counter before the `export` keyword; a counter
// targeting the inner declaration never matches and would leave `s[0]` at zero
// even though the initializer runs at module evaluation.
#[test]
fn statement_export_const_arrow_initializer_is_counted() {
    let result = instrument_js("export const fn = () => { return 1; };\n");
    assert_eq!(result.coverage_map.statement_map.len(), 2);
    let counter = result.code.find(".s[0];").expect("hoisted s[0] counter must be emitted");
    let export = result.code.find("export const fn").expect("export declaration present");
    assert!(counter < export, "statement counter must run before the export declaration");
}

#[test]
fn statement_export_const_function_initializer_is_counted() {
    let result = instrument_js("export const fn = function () { return 1; };\n");
    let counter = result.code.find(".s[0];").expect("hoisted s[0] counter must be emitted");
    let export = result.code.find("export const fn").expect("export declaration present");
    assert!(counter < export, "statement counter must run before the export declaration");
}

#[test]
fn statement_export_const_multi_declarator_both_counted() {
    let result = instrument_js("export const a = () => 1, b = () => 2;\n");
    let export = result.code.find("export const a").expect("export declaration present");
    // Both arrow initializers hoist their statement counters before the export.
    let s0 = result.code.find(".s[0];").expect("s[0] counter must be emitted");
    let s2 = result.code.find(".s[2];").expect("s[2] counter must be emitted");
    assert!(s0 < export && s2 < export, "both statement counters must run before the export");
}

#[test]
fn pending_statement_counters_preserve_source_order() {
    let source = "export const fn = () => 1;\nwhile (ready) tick();\nbar();\n";
    let result = instrument(source, "pending-order.js", &InstrumentOptions::default()).unwrap();
    let first = result
        .code
        .find(".s[0];")
        .unwrap_or_else(|| panic!("first counter must be emitted:\n{}", result.code));
    let export = result.code.find("export const fn").expect("export statement must be emitted");
    let while_counter = result.code.find(".s[2];").expect("while counter must be emitted");
    let loop_counter = result.code.find(".s[3];").expect("loop-child counter must be emitted");
    let while_stmt = result.code.find("while (ready)").expect("while statement must be emitted");
    let tick = result.code.find("tick();").expect("loop-child statement must be emitted");
    let final_counter = result.code.find(".s[4];").expect("final counter must be emitted");
    let bar = result.code.find("bar();").expect("final statement must be emitted");

    assert!(first < export, "export counter must precede its statement");
    assert!(export < while_counter && while_counter < while_stmt);
    assert!(
        while_stmt < loop_counter && loop_counter < tick,
        "loop-child counter must remain inside the loop"
    );
    assert!(tick < final_counter && final_counter < bar);
    assert!(
        first < while_counter && while_counter < loop_counter && loop_counter < final_counter,
        "counter IDs must remain in source order"
    );
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
    // Both paths need executable counters; otherwise the false path is
    // permanently reported as uncovered.
    assert!(result.code.contains(".b[0][0]"));
    assert!(result.code.contains(".b[0][1]"));

    let json = serde_json::to_value(&result.coverage_map).unwrap();
    // The synthetic else arm anchors as a zero-width location at the
    // consequent's end. Both `start` and `end` carry real line/column
    // fields so downstream consumers (`istanbul-reports`, dashboards) do
    // not trip over `start.line` access on an empty placeholder.
    let synthetic = &json["branchMap"]["0"]["locations"][1];
    assert!(
        synthetic["start"].get("line").is_some(),
        "synthetic else arm must have a real start line: {synthetic}"
    );
    assert_eq!(synthetic["start"], synthetic["end"], "synthetic arm is zero-width");
}

#[test]
fn optional_chain_link_tracked_as_branch() {
    // Each `?.` link surfaces as an `optional-chain` branch with two arms:
    // arm 0 fires when the observed value is nullish (the link short-circuits),
    // arm 1 fires when the link continues.
    let r = instrument_js("function f(e) { return e?.stderr?.replace(/x/, 'y'); }");
    let oc_entries: Vec<_> =
        r.coverage_map.branch_map.values().filter(|e| e.branch_type == "optional-chain").collect();
    assert_eq!(oc_entries.len(), 2, "two `?.` links should produce two optional-chain branches");
    for entry in oc_entries {
        assert_eq!(entry.locations.len(), 2, "each optional-chain branch has two arms");
    }
    // The runtime helper must appear in the preamble when at least one
    // optional-chain branch is recorded.
    assert!(
        r.code.contains("_oc(val, id)"),
        "optional-chain helper missing from preamble:\n{}",
        r.code
    );
}

#[test]
fn optional_call_tracked_as_branch() {
    // `cb?.()` is also an optional-chain link (on the callee).
    let r = instrument_js("function f(cb) { return cb?.(1); }");
    let entries: Vec<_> =
        r.coverage_map.branch_map.values().filter(|e| e.branch_type == "optional-chain").collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].locations.len(), 2);
}

#[test]
fn optional_member_call_preserves_receiver_reference() {
    let r = instrument_js("function f(value) { return value?.items?.flat?.(); }");
    let entry_count =
        r.coverage_map.branch_map.values().filter(|e| e.branch_type == "optional-chain").count();

    assert_eq!(entry_count, 2, "member links stay tracked while the bound call stays native");
    assert!(
        r.code.contains("?.flat?.()"),
        "the method reference must remain the optional call callee:\n{}",
        r.code
    );
}

#[test]
fn track_optional_chain_false_leaves_chain_native() {
    // With tracking off, member, computed and call optional links emit no
    // `optional-chain` branch and no `_oc` helper, matching
    // istanbul-lib-instrument.
    let source = "function f(e, cb) { return e?.stderr?.['k']?.replace(/x/, 'y') ?? cb?.(1); }";
    let opts = InstrumentOptions { track_optional_chain: false, ..InstrumentOptions::default() };
    let r = instrument(source, "test.js", &opts).unwrap();

    let oc_count =
        r.coverage_map.branch_map.values().filter(|e| e.branch_type == "optional-chain").count();
    assert_eq!(oc_count, 0, "no optional-chain branches when tracking is disabled");
    assert!(!r.code.contains("_oc("), "the `_oc` helper must not be emitted:\n{}", r.code);
    // The `??` is still a logical branch: only `?.` tracking is suppressed.
    assert!(
        r.coverage_map.branch_map.values().any(|e| e.branch_type == "binary-expr"),
        "non-optional-chain branches (the `??`) are still tracked",
    );
}

#[test]
fn track_optional_chain_default_on_matches_existing_behavior() {
    // The default (true) is unchanged: the same source tracked produces
    // optional-chain branches and the helper.
    let source = "function f(e) { return e?.stderr?.replace(/x/, 'y'); }";
    let tracked = instrument_js(source);
    let oc_count = tracked
        .coverage_map
        .branch_map
        .values()
        .filter(|e| e.branch_type == "optional-chain")
        .count();
    assert_eq!(oc_count, 2, "default still tracks each `?.` link");
    assert!(tracked.code.contains("_oc(val, id)"), "default still emits the helper");

    // Statement/function counts are identical with and without tracking; only
    // the optional-chain branches differ.
    let opts = InstrumentOptions { track_optional_chain: false, ..InstrumentOptions::default() };
    let untracked = instrument(source, "test.js", &opts).unwrap();
    assert_eq!(
        tracked.coverage_map.statement_map.len(),
        untracked.coverage_map.statement_map.len(),
        "statement coverage is unaffected by the optional-chain toggle",
    );
    assert_eq!(tracked.coverage_map.fn_map.len(), untracked.coverage_map.fn_map.len());
}

#[test]
fn class_field_initializer_keeps_function_name() {
    // Wrapping the value as `(++cov.s[N], function () {})` would defeat
    // NamedEvaluation and leave the function unnamed at runtime. The counter
    // goes on a synthetic sibling field instead, so the value stays a bare
    // function or class expression and `Function.name` is unchanged.
    let result = instrument_js(
        "class Foo { field1 = function () {}; static field2 = class {}; arrow = () => 1; }",
    );
    let counter_fields = result.code.matches("__cov_").count();
    assert!(
        counter_fields >= 3,
        "expected synthetic counter fields for each hoisted initializer:\n{}",
        result.code
    );
    assert!(
        !result.code.contains(", function ()") && !result.code.contains(", () =>"),
        "initializer must not be wrapped in a sequence expression:\n{}",
        result.code
    );
}

#[test]
fn fn_name_inference_matrix_outside_class_methods() {
    // Each row asserts that `fnMap[N].name` is the source-derived name for a
    // category of function, arrow or class expression that carries no `id` of
    // its own.
    let cases: &[(&str, &str)] = &[
        ("const o = { fn() {} };", "fn"),
        ("const o = { get prop() {} };", "get prop"),
        ("const o = { set prop(v) {} };", "set prop"),
        ("const o = { foo: function () {} };", "foo"),
        ("const o = { foo: () => 1 };", "foo"),
        ("var o = {}; o.bar = function () {};", "bar"),
        ("var o = {}; o['baz'] = function () {};", "baz"),
        ("function f({ y = function () {} } = {}) {}", "y"),
        ("function f(cb = () => 1) {}", "cb"),
        ("export default function () {}", "default"),
        ("export default () => 1;", "default"),
        ("export default class { ctor() {} }", "ctor"),
        ("class C { 0() {} }", "0"),
        ("class C { 'foo bar'() {} }", "foo bar"),
    ];
    for (source, expected) in cases {
        let result = instrument_js(source);
        let names: Vec<&str> =
            result.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(expected),
            "{source}: expected fnMap to include {expected:?}, got {names:?}"
        );
    }
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
    // Istanbul tracks loops with statement counters alone, never a branch entry.
    assert!(
        result.coverage_map.branch_map.is_empty(),
        "Loops should not produce branch entries (matching Istanbul)"
    );
}

#[test]
fn no_block_loop_bodies_emit_statement_counters() {
    let sources = [
        ("while", "function f() { let i = 0; while (i < 3) i++; return i; }"),
        (
            "for",
            "function f() { let total = 0; for (let i = 0; i < 3; i++) total++; return total; }",
        ),
        (
            "for-of",
            "function f(items) { let total = 0; for (const x of items) total += x; return total; }",
        ),
        (
            "for-in",
            "function f(obj) { let total = 0; for (const k in obj) total++; return total; }",
        ),
        ("do-while", "function f() { let i = 0; do i++; while (i < 3); return i; }"),
    ];

    for (name, source) in sources {
        let result = instrument_js(source);
        assert!(
            result.coverage_map.branch_map.is_empty(),
            "{name} should still use statement coverage rather than branch coverage"
        );
        assert_eq!(
            result.code.matches(".s[").count(),
            result.coverage_map.statement_map.len(),
            "{name} should emit one executable statement counter for every statementMap entry"
        );
    }
}

#[test]
fn no_block_statement_child_containers_emit_body_counters() {
    let sources = [
        ("with", "function f(obj) { with (obj) x++; return obj.x; }"),
        ("label", "function f() { let n = 0; label: n++; return n; }"),
        ("loop-label", "function f() { let n = 0; while (n < 3) label: n++; return n; }"),
        (
            "label-loop",
            "function f() { let n = 0; label: while (n < 3) { n++; continue label; } return n; }",
        ),
    ];

    for (name, source) in sources {
        let result = instrument_js(source);
        assert_eq!(
            result.code.matches(".s[").count(),
            result.coverage_map.statement_map.len(),
            "{name} should emit one executable statement counter for every statementMap entry"
        );
    }
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
    assert_eq!(sorted, vec![0, 1, 2]);
}

#[test]
fn positions_are_1_based_line_0_based_column() {
    // The only statement is the declarator init `1`, at line 1 column 10.
    let result = instrument_js("const x = 1;");
    let loc = &result.coverage_map.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 10));
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
    let cov_fn_name_from_preamble = |code: &str| -> String {
        let start = code.find("var cov_").unwrap() + 4;
        let end = code[start..].find(' ').unwrap() + start;
        code[start..end].to_string()
    };

    let first = instrument_js("const x = 1;");
    let second = instrument_js("const x = 1;");
    assert_eq!(
        cov_fn_name_from_preamble(&first.code),
        cov_fn_name_from_preamble(&second.code),
        "the same source and path must produce the same coverage function name"
    );

    // The name is derived from the path, which is what keeps two files in one
    // bundle from sharing a coverage object.
    let other_path =
        instrument("const x = 1;", "other.js", &default_opts()).expect("instrument other.js");
    assert_ne!(
        cov_fn_name_from_preamble(&first.code),
        cov_fn_name_from_preamble(&other_path.code),
        "a different path must produce a different coverage function name"
    );
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
// Nested arrows
// ---------------------------------------------------------------------------

#[test]
fn nested_arrow_functions_both_get_counters() {
    let result = instrument_js("const f = (x) => (y) => x + y;");
    assert_eq!(result.coverage_map.fn_map.len(), 2);
    assert_eq!(result.coverage_map.f.len(), 2);
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
fn composed_source_map_resolves_positions_to_original_source() {
    // Guard for the composition path: a non-trivial inputSourceMap that maps each generated
    // line back to a different line of an original TypeScript file. After instrumentation +
    // composition, the resulting source map must (a) still reference the original source and
    // (b) be able to resolve generated positions in the instrumented output back to the
    // expected original-source lines.
    //
    // The conformance suite exercises only the no-inputSourceMap path, so this
    // is the one place composition is checked end to end.
    use oxc_sourcemap::SourceMap as OxcSourceMap;

    let original_ts = "const x: number = 1;\nconst y: number = 2;\nconst z: number = 3;\n";
    let intermediate_js = "const x = 1;\nconst y = 2;\nconst z = 3;\n";

    // Identity-line mapping: each generated line maps to the same line in original_ts.
    // VLQ: "AAAA;AACA;AACA" = (gen 0:0 -> src[0] 0:0); next line src_line += 1; next line src_line += 1.
    let input_sm = format!(
        r#"{{"version":3,"sources":["original.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA;AACA;AACA","names":[]}}"#,
    );

    let opts = InstrumentOptions {
        source_map: true,
        input_source_map: Some(input_sm),
        ..InstrumentOptions::default()
    };
    let result = instrument(intermediate_js, "intermediate.js", &opts).unwrap();

    let raw = result.source_map.expect("output source map present");
    let composed = OxcSourceMap::from_json_string(&raw).expect("composed map parses");

    let sources: Vec<String> = composed.get_sources().map(ToString::to_string).collect();
    assert!(
        sources.iter().any(|s| s == "original.ts"),
        "composed map must reference original.ts after composition, got: {sources:?}"
    );

    let reached: FxHashSet<u32> = composed
        .get_source_view_tokens()
        .filter(|t| t.get_source_id().is_some())
        .map(|t| t.get_src_line())
        .collect();
    for original_line in 0u32..3 {
        assert!(
            reached.contains(&original_line),
            "composed map must reach original.ts line {original_line}; reached: {reached:?}"
        );
    }

    // The preamble embeds a JSON-encoded copy of the inputSourceMap's
    // sourcesContent, so the needles also appear in preamble lines. Pick the
    // LAST occurrence of each needle in the file: the actual instrumented
    // statement is always emitted after the preamble. This stays robust if
    // codegen ever splits the preamble across multiple lines.
    let all_lines: Vec<&str> = result.code.lines().collect();
    let lookup = composed.generate_lookup_table();
    for (needle, expected_src_line) in [("const x", 0u32), ("const y", 1), ("const z", 2)] {
        let (gen_line_idx, line) = all_lines
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.contains(needle))
            .unwrap_or_else(|| panic!("instrumented code lines must contain `{needle}`"));
        let gen_line = u32::try_from(gen_line_idx).unwrap();
        let gen_col = u32::try_from(line.find(needle).expect("substring column")).unwrap();
        let token = composed.lookup_token(&lookup, gen_line, gen_col).unwrap_or_else(|| {
            panic!("`{needle}` at {gen_line}:{gen_col} resolves in composed map")
        });
        assert_eq!(
            token.get_src_line(),
            expected_src_line,
            "`{needle}` at gen {gen_line}:{gen_col} must resolve to original.ts line {expected_src_line}, got {}",
            token.get_src_line()
        );
    }
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
    // The decl span runs from `async` at column 0 to past the identifier, so a
    // fixed offset from the `function` keyword would be wrong here.
    let decl = &result.coverage_map.fn_map["0"].decl;
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
    assert!(
        result.code.contains(".b[0][0]") && result.code.contains(".b[1][0]"),
        "Destructuring defaults must increment branch counters at runtime"
    );
}

#[test]
fn default_parameter_wraps_initializer_with_branch_counter() {
    let result = instrument_js("function f(x = 1) { return x; }");
    assert!(
        result.code.contains(".b[0][0]"),
        "Default parameter initializer must increment branch counter at runtime"
    );
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

#[test]
fn preamble_refreshes_stale_coverage_by_hash() {
    let first = instrument_js("function f() { return 1; }");
    let second = instrument_js("function f() { if (true) { return 1; } return 0; }");

    assert!(first.code.contains("coverageData.hash = hash;"));
    assert!(second.code.contains("coverage[gcv][path].hash !== hash"));
}

#[test]
fn preamble_invokes_setup_once_and_counters_use_cached_coverage() {
    let result = instrument_js("function f() { return 1; }");
    let cov_start = result.code.find("var cov_").unwrap() + 4;
    let cov_end = result.code[cov_start..].find(' ').unwrap() + cov_start;
    let cov_name = &result.code[cov_start..cov_end];
    assert!(
        result.code.contains("return actualCoverage; })();"),
        "coverage setup should be invoked once in the preamble"
    );
    assert!(
        !result.code.contains(&format!("{cov_name}().")),
        "counter sites should not call the coverage setup function"
    );
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
    // Initializer values should be wrapped: x = (++cov.s[N], value)
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
fn ignore_class_methods_skips_method_body() {
    let opts =
        InstrumentOptions { ignore_class_methods: vec!["render".to_string()], ..default_opts() };
    let result =
        instrument("class App { render() { const x = 1; return x; } }", "test.js", &opts).unwrap();
    // Istanbul's ignoreClassMethods skips the whole matched method body.
    assert_eq!(result.coverage_map.fn_map.len(), 0);
    assert_eq!(result.coverage_map.statement_map.len(), 0);
}

#[test]
fn ignore_class_methods_skips_named_function_expression_body() {
    let opts = InstrumentOptions {
        ignore_class_methods: vec!["testMethod".to_string()],
        ..default_opts()
    };
    let result = instrument(
        "function TestClass() {}\n\
         TestClass.prototype.testMethod = function testMethod(i) { return i; };\n\
         TestClass.prototype.goodMethod = function goodMethod(i) { return i; };\n\
         var testClass = new TestClass();\n\
         testClass.goodMethod();\n\
         testClass.testMethod(1);",
        "test.js",
        &opts,
    )
    .unwrap();

    let function_names: Vec<&str> =
        result.coverage_map.fn_map.values().map(|entry| entry.name.as_str()).collect();
    assert_eq!(function_names, vec!["TestClass", "goodMethod"]);

    let statement_lines: Vec<u32> =
        result.coverage_map.statement_map.values().map(|loc| loc.start.line).collect();
    assert_eq!(
        statement_lines,
        vec![2, 3, 3, 4, 5, 6],
        "ignored function expression body should not add a return statement"
    );
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
    // Only logical expressions get bT entries; an if/else branch never does.
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
    // A function-valued declarator init hoists its statement counter to a
    // sibling statement before the enclosing declaration (the sequence-wrap
    // `(++cov.s[N], fn)` would break Function.name inference). For an exported
    // declaration the sibling slot is before the `export` keyword;
    // `statement_export_const_arrow_initializer_is_counted` checks the emit.
    let result = instrument_js("export const add = (a, b) => a + b;");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "add");
    // Two statements: the declarator init and the arrow body's return.
    assert_eq!(result.coverage_map.statement_map.len(), 2);
    assert!(result.code.contains(".s[0];"), "declarator init counter must be emitted");
}

/// Every declaration-container variant istanbul-lib-instrument skips must be
/// skipped here too. Covers the full list in `is_container_statement`, so a
/// wrongly-classified `Statement` variant surfaces here.
#[test]
fn declaration_containers_produce_no_statement_counters() {
    // Each input holds container nodes only. Functions and classes still
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

/// if-branch `locations[0]` parity with istanbul, which passes `n.loc` (the
/// whole `IfStatement` span) as the consequent location rather than the
/// consequent block's narrower span. That is the range reporters highlight.
#[test]
fn if_branch_consequent_location_is_whole_if_span() {
    // The `if` statement below spans columns 16 to 60, and `locations[0]` is
    // that whole span.
    let result = instrument_js("function f(x) { if (x > 0) { return 1; } else { return -1; } }");
    let b = &result.coverage_map.branch_map["0"];
    assert_eq!(b.branch_type, "if");
    assert_eq!(b.locations[0].start.column, 16);
    assert_eq!(b.locations[0].end.column, 60);
    // locations[1] narrows to the alternate block (starts at `else`).
    assert!(b.locations[1].start.column >= 41, "locations[1] should start in the else region");
}

/// A class method's `decl` points at the key's identifier span, not at the
/// parameter list. istanbul truncates to columns 10-11 for `bar`; the full
/// identifier span is columns 10-13.
#[test]
fn class_method_decl_is_identifier_span() {
    let result = instrument_js("class C { bar(x) { return x; } }");
    let f = &result.coverage_map.fn_map["0"];
    assert_eq!(f.name, "bar");
    assert_eq!(f.decl.start.column, 10);
    assert_eq!(f.decl.end.column, 13, "decl should cover the full identifier `bar`");

    // Static-string method key: span comes from the string literal.
    let result = instrument_js("class C { \"my method\"() { return 1; } }");
    let f = &result.coverage_map.fn_map["0"];
    assert_eq!(f.name, "my method");
    // "my method" literal with quotes is col 10-21; end must be strictly > start.
    assert!(f.decl.end.column > f.decl.start.column);
}

/// `fnMap[*].decl` parity with istanbul-lib-instrument, which sets `decl` to
/// the identifier span for a named function.
#[test]
fn fn_decl_span_matches_istanbul() {
    // For `export function sum(a, b) { return a + b }` istanbul emits decl
    // column 16..19, the identifier `sum`, which is `id.span`.
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

/// A module of simple function exports must produce the statement and function
/// counts istanbul-lib-instrument does: the Vitest istanbul provider compares
/// them directly in its `coverage-final.json`. A statement counter emitted for
/// the `export function` declaration itself would inflate statements to 8/4.
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

    // No hoisted statement counter may appear before an `export function`.
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
    let json = serde_json::to_string(&result.coverage_map).unwrap();
    assert!(json.contains("\"bT\""), "JSON should contain bT field");
    let parsed: oxc_coverage_instrument::FileCoverage = serde_json::from_str(&json).unwrap();
    assert!(parsed.b_t.is_some());
}

// ---------------------------------------------------------------------------
// Non-ASCII column handling (Istanbul parity: UTF-16 code units, not bytes)
// ---------------------------------------------------------------------------

#[test]
fn non_ascii_columns_are_utf16_code_units() {
    // `π` is 2 UTF-8 bytes but 1 UTF-16 code unit. Istanbul/Babel report columns
    // as UTF-16 code units (JavaScript string indices), so the `1` init literal
    // must be at column 10, not 11 (its UTF-8 byte position).
    let result = instrument_js("const π = 1; const y = 2;");
    let stmt0 = &result.coverage_map.statement_map["0"];
    assert_eq!(stmt0.start.column, 10, "stmt 0 should start at UTF-16 col 10, got {stmt0:?}");
    assert_eq!(stmt0.end.column, 11, "stmt 0 should end at UTF-16 col 11, got {stmt0:?}");
    let stmt1 = &result.coverage_map.statement_map["1"];
    assert_eq!(stmt1.start.column, 23, "stmt 1 should start at UTF-16 col 23, got {stmt1:?}");
}

#[test]
fn emoji_columns_count_as_two_utf16_units() {
    // `😀` is one code point outside the BMP: 4 UTF-8 bytes, 2 UTF-16 code units
    // (one surrogate pair). Istanbul/Babel reflect the surrogate pair in columns.
    // "const a = '😀'; const b = 2;"
    //  c o n s t _ a _ = _ ' 😀😀 ' ;  _ c o n s t _ b _ = _ 2 ;
    //  0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
    // VariableDeclarator.init `2` is at UTF-16 cols 26..27.
    let result = instrument_js("const a = '😀'; const b = 2;");
    let stmt1 = &result.coverage_map.statement_map["1"];
    assert_eq!(stmt1.start.column, 26, "emoji should advance col by 2 UTF-16 units, got {stmt1:?}");
    assert_eq!(stmt1.end.column, 27);
}

// ---------------------------------------------------------------------------
// Pragma whitespace tolerance (Istanbul parity)
// ---------------------------------------------------------------------------

#[test]
fn private_class_method_does_not_add_function_counter() {
    let source = "class C {\n  #secret(x) { if (x) { return 1; } return 2; }\n  run(x) { return this.#secret(x); }\n}";
    let result = instrument_js(source);
    assert_eq!(
        result.coverage_map.fn_map.len(),
        1,
        "Istanbul instruments private method bodies but not private method function counters"
    );
    assert_eq!(result.coverage_map.branch_map.len(), 1);
}

#[test]
fn private_method_in_parameter_default_does_not_steal_enclosing_function_counter() {
    let result = instrument_js("function f(x = class { #m() {} }) {}");
    assert_eq!(result.coverage_map.fn_map.len(), 1);
    assert_eq!(result.coverage_map.fn_map["0"].name, "f");

    let counter = result.code.find(".f[0]").expect("function counter");
    let enclosing_body = result.code.rfind(")) {").expect("enclosing function body");
    assert!(
        counter > enclosing_body,
        "the counter for f must be emitted in f's body, not the nested private method: {}",
        result.code
    );
}

#[test]
fn private_accessors_in_parameter_defaults_do_not_steal_enclosing_function_counters() {
    for member in ["get #m() { return 1; }", "set #m(value) {}"] {
        let source = format!("function f(x = class {{ {member} }}) {{}}");
        let result = instrument_js(&source);
        assert_eq!(result.coverage_map.fn_map.len(), 1, "{member}");
        assert_eq!(result.coverage_map.fn_map["0"].name, "f", "{member}");

        let counter = result.code.find(".f[0]").expect("function counter");
        let enclosing_body = result.code.rfind(")) {").expect("enclosing function body");
        assert!(
            counter > enclosing_body,
            "the counter for f must stay in f's body for {member}: {}",
            result.code
        );
    }
}

// ---------------------------------------------------------------------------
// Source map composition: partial input maps
// ---------------------------------------------------------------------------

#[test]
fn source_map_composition_with_partial_input_map() {
    // Input map only maps the first line; later mappings in the output map
    // have no corresponding entry. The composed map must not misattribute those
    // unmapped positions to the wrong original source: the fallback emits a
    // position-only token instead of reusing the output map's source id.
    let opts = InstrumentOptions {
        source_map: true,
        input_source_map: Some(
            r#"{"version":3,"sources":["original.ts"],"sourcesContent":["const x: number = 1;\nconst y: number = 2;"],"mappings":"AAAA"}"#.to_string(),
        ),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;\nconst y = 2;", "test.js", &opts).unwrap();
    let sm_json = result.source_map.as_ref().unwrap();
    let sm: serde_json::Value = serde_json::from_str(sm_json).unwrap();
    let sources = sm["sources"].as_array().unwrap();
    // Composed map must reference the original source from the input map.
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].as_str(), Some("original.ts"));
    // The composed map must still decode, so the mappings string is non-empty.
    assert!(sm["mappings"].as_str().is_some_and(|m| !m.is_empty()));
}

#[test]
fn source_map_composition_preserves_input_names() {
    // Names referenced by an input source map's mappings must survive composition
    // so downstream reporters can resolve symbolicated identifiers. Matches
    // `@ampproject/remapping` semantics: only names that some mapping actually
    // references are propagated; unreferenced names are dropped (this prevents
    // unbounded growth of the names table through long transform chains).
    //
    // Input mappings (on intermediate source `const foo = bar;`):
    //   gen 0:0  -> src 0:0  name "foo" (idx 0)  -> "AAAAA"
    //   gen 0:6  -> src 0:6  name "bar" (idx 1)  -> delta [6,0,0,6,1] -> "MAAMC"
    let opts = InstrumentOptions {
        source_map: true,
        input_source_map: Some(
            r#"{"version":3,"sources":["original.ts"],"sourcesContent":["const foo = bar;"],"names":["foo","bar"],"mappings":"AAAAA,MAAMC"}"#
                .to_string(),
        ),
        ..InstrumentOptions::default()
    };
    let result = instrument("const foo = bar;", "test.js", &opts).unwrap();
    let sm: serde_json::Value = serde_json::from_str(result.source_map.as_ref().unwrap()).unwrap();
    let names: Vec<&str> =
        sm["names"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(names.contains(&"foo"), "names should include 'foo', got {names:?}");
    assert!(names.contains(&"bar"), "names should include 'bar', got {names:?}");
}

// ---------------------------------------------------------------------------
// Error `Display` impls: one case per enum arm, so a format string cannot drift.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Pragma parser edge cases: unmatched block pragmas, unsupported tool names,
// and comments between the pragma and the node it annotates.
// ---------------------------------------------------------------------------
