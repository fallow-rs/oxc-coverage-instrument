//! Tests for `InstrumentOptions::strip_typescript`. Verifies that raw
//! TypeScript source can be instrumented in one shot: the output is valid
//! JavaScript (type annotations removed), and statementMap / branchMap
//! positions still refer to the original TypeScript source offsets.

use oxc_coverage_instrument::{InstrumentError, InstrumentOptions, instrument};

fn ts_opts() -> InstrumentOptions {
    InstrumentOptions { source_map: true, strip_typescript: true, ..InstrumentOptions::default() }
}

#[test]
fn ts_direct_output_is_valid_js() {
    let src = "const x: number = 1;\nconsole.log(x);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    assert!(
        !result.code.contains(": number"),
        "type annotation must be stripped, got: {}",
        result.code
    );
    assert!(
        !result.code.contains("interface "),
        "interface declarations must be stripped, got: {}",
        result.code
    );
    assert!(
        result.code.contains("const x ="),
        "executable JS variable declaration expected, got: {}",
        result.code
    );
}

#[test]
fn ts_direct_statement_map_positions_are_ts_positions() {
    let src = "const x: number = 1;\nconst y: string = \"hi\";\nconsole.log(x, y);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    let map = &result.coverage_map.statement_map;
    assert_eq!(map.len(), 3, "expected 3 statements, got {}: {map:#?}", map.len());

    let s0 = &map["0"];
    assert_eq!(s0.start.line, 1, "statement 0 should be on line 1, got {s0:?}");
    let s1 = &map["1"];
    assert_eq!(s1.start.line, 2, "statement 1 should be on line 2, got {s1:?}");
    let s2 = &map["2"];
    assert_eq!(s2.start.line, 3, "statement 2 should be on line 3, got {s2:?}");
}

#[test]
fn ts_direct_source_map_sources_and_content() {
    let src = "const x: number = 1;\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    let sm: serde_json::Value =
        serde_json::from_str(result.source_map.as_deref().expect("source map")).expect("json");
    assert_eq!(sm["sources"][0], "app.ts");
    let content = sm["sourcesContent"][0].as_str().expect("sourcesContent[0]");
    assert!(
        content.contains(": number"),
        "sourcesContent must retain TS annotations, got: {content}"
    );
}

#[test]
fn ts_direct_type_only_nodes_produce_no_counters() {
    let src = "interface Foo { a: number; }\ntype Bar = string;\ndeclare const baz: number;\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    assert_eq!(
        result.coverage_map.statement_map.len(),
        0,
        "type-only nodes must not produce statement counters: {:#?}",
        result.coverage_map.statement_map
    );
    assert_eq!(result.coverage_map.fn_map.len(), 0);
    assert_eq!(result.coverage_map.branch_map.len(), 0);
}

#[test]
fn ts_direct_interface_is_skipped_in_position_map() {
    let src = "const x: number = 1;\nconst y: string = \"hi\";\ninterface Foo { a: number }\nconsole.log(x, y);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    let lines: Vec<u32> =
        result.coverage_map.statement_map.values().map(|loc| loc.start.line).collect();
    assert!(!lines.contains(&3), "interface on line 3 must not appear in statement_map: {lines:?}");
    assert!(lines.contains(&1));
    assert!(lines.contains(&2));
    assert!(lines.contains(&4));
}

#[test]
fn ts_direct_typescript_function_gets_counter() {
    let src = "function add(a: number, b: number): number {\n  return a + b;\n}\nadd(1, 2);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    assert!(!result.code.contains(": number"), "param annotations stripped");
    assert_eq!(result.coverage_map.fn_map.len(), 1, "function counter expected");
    let f = &result.coverage_map.fn_map["0"];
    assert_eq!(f.name, "add");
    assert_eq!(f.decl.start.line, 1);
}

#[test]
fn ts_direct_parse_error_returns_parse_error() {
    let src = "const x: = ;\n";
    let err = instrument(src, "app.ts", &ts_opts()).expect_err("parse must fail");
    assert!(matches!(err, InstrumentError::ParseError(_)), "expected ParseError, got {err:?}");
}

#[test]
fn ts_direct_default_off_preserves_existing_behavior() {
    let src = "const x: number = 1;\n";
    let opts = InstrumentOptions::default();
    assert!(!opts.strip_typescript);
    let result = instrument(src, "app.ts", &opts).expect("instrument");
    assert!(
        result.code.contains(": number"),
        "with strip_typescript=false the TS annotation must remain, got: {}",
        result.code
    );
}
