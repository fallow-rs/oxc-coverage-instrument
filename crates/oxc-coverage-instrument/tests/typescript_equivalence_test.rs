//! Equivalence test for the `strip_typescript` path.
//!
//! Asserts that instrumenting a TypeScript source directly (with
//! `strip_typescript: true`) produces the same set of covered source
//! line numbers as instrumenting the JavaScript equivalent (with
//! `strip_typescript: false`). The pair is line-aligned: the JS
//! equivalent uses blank lines for the type-only declarations so the
//! executable statements live on the same line numbers in both inputs.
//!
//! Statement IDs typically differ between the two paths (synthesis
//! order is not stable across the strip pass), so we compare the
//! sorted set of line numbers, not IDs.

use oxc_coverage_instrument::{InstrumentOptions, instrument};
use std::collections::BTreeSet;

const TS_SOURCE: &str = include_str!("fixtures/typescript-direct-simple.ts");

// JS equivalent of `typescript-direct-simple.ts`. Line-aligned: the first
// six lines are blank so the function declaration lives on line 7 in
// both files, matching the TS source's interface/type-alias placement.
const JS_EQUIVALENT: &str = "\n\n\n\n\n\nfunction greet(user, status) {\n    if (status === \"active\") {\n        return `Hello, ${user.name}`;\n    }\n    return \"Please wait\";\n}\n\nconst u = { name: \"Alice\", age: 30 };\nconsole.log(greet(u, \"active\"));\n";

fn covered_lines(coverage_map: &oxc_coverage_types::FileCoverage) -> BTreeSet<u32> {
    let mut lines = BTreeSet::new();
    for loc in coverage_map.statement_map.values() {
        lines.insert(loc.start.line);
    }
    for entry in coverage_map.fn_map.values() {
        lines.insert(entry.loc.start.line);
    }
    for entry in coverage_map.branch_map.values() {
        lines.insert(entry.loc.start.line);
    }
    lines
}

#[test]
fn ts_direct_and_js_equivalent_cover_same_lines() {
    let ts_opts = InstrumentOptions { strip_typescript: true, ..InstrumentOptions::default() };
    let js_opts = InstrumentOptions::default();

    let ts_result = instrument(TS_SOURCE, "app.ts", &ts_opts).expect("instrument TS");
    let js_result = instrument(JS_EQUIVALENT, "app.js", &js_opts).expect("instrument JS");

    let ts_lines = covered_lines(&ts_result.coverage_map);
    let js_lines = covered_lines(&js_result.coverage_map);

    assert_eq!(
        ts_lines, js_lines,
        "TS-direct path and JS-equivalent path must cover the same set of source lines\nTS lines: {ts_lines:?}\nJS lines: {js_lines:?}\nTS source: {TS_SOURCE}\nJS source: {JS_EQUIVALENT}"
    );

    assert!(
        ts_lines.contains(&7),
        "function declaration on line 7 must be covered, got {ts_lines:?}"
    );
    assert!(ts_lines.contains(&8), "if statement on line 8 must be covered, got {ts_lines:?}");
}

#[test]
fn ts_direct_statement_count_matches_js_equivalent() {
    let ts_opts = InstrumentOptions { strip_typescript: true, ..InstrumentOptions::default() };
    let js_opts = InstrumentOptions::default();

    let ts_result = instrument(TS_SOURCE, "app.ts", &ts_opts).expect("instrument TS");
    let js_result = instrument(JS_EQUIVALENT, "app.js", &js_opts).expect("instrument JS");

    assert_eq!(
        ts_result.coverage_map.statement_map.len(),
        js_result.coverage_map.statement_map.len(),
        "statement counter count must match between TS-direct and JS-equivalent paths"
    );
    assert_eq!(
        ts_result.coverage_map.fn_map.len(),
        js_result.coverage_map.fn_map.len(),
        "function counter count must match"
    );
    assert_eq!(
        ts_result.coverage_map.branch_map.len(),
        js_result.coverage_map.branch_map.len(),
        "branch counter count must match"
    );
}

#[test]
fn ts_direct_emits_executable_js_without_type_annotations() {
    // Regression-strength assertion: the equivalence tests above compare line
    // sets, which the underlying CoverageTransform would produce identically
    // whether or not the strip pass ran (Span values survive either way).
    // This test asserts on the EMITTED CODE STRING: with strip_typescript on,
    // the output must NOT contain TS-only syntax. Mentally reverting
    // strip_typescript_pass to a no-op makes this test fail because the
    // codegen would emit the original TS annotations verbatim.
    let ts_opts = InstrumentOptions { strip_typescript: true, ..InstrumentOptions::default() };
    let result = instrument(TS_SOURCE, "app.ts", &ts_opts).expect("instrument TS");

    assert!(
        !result.code.contains(": string"),
        "type annotations must be stripped from output code: {}",
        result.code
    );
    assert!(
        !result.code.contains(": number"),
        "type annotations must be stripped from output code: {}",
        result.code
    );
    assert!(
        !result.code.contains("interface "),
        "interface declarations must be stripped from output code: {}",
        result.code
    );
    assert!(
        !result.code.contains("type Status"),
        "type alias declarations must be stripped from output code: {}",
        result.code
    );

    let no_strip_opts = InstrumentOptions::default();
    let no_strip_result = instrument(TS_SOURCE, "app.ts", &no_strip_opts).expect("instrument TS");
    assert!(
        no_strip_result.code.contains(": string"),
        "without strip_typescript the TS annotation must survive in output: {}",
        no_strip_result.code
    );
}

#[test]
fn transform_error_display_format() {
    use oxc_coverage_instrument::InstrumentError;
    let err = InstrumentError::TransformError(vec![
        "unsupported syntax at line 5".to_string(),
        "missing semicolon at line 8".to_string(),
    ]);
    let rendered = format!("{err}");
    assert!(rendered.starts_with("transform error: "), "got: {rendered}");
    assert!(rendered.contains("unsupported syntax at line 5"), "got: {rendered}");
    assert!(rendered.contains("missing semicolon at line 8"), "got: {rendered}");
    assert!(rendered.contains("; "), "diagnostics must be joined with '; ', got: {rendered}");
}

#[test]
fn transform_error_single_diagnostic_displays_without_separator() {
    use oxc_coverage_instrument::InstrumentError;
    let err = InstrumentError::TransformError(vec!["only one diagnostic".to_string()]);
    let rendered = format!("{err}");
    assert_eq!(rendered, "transform error: only one diagnostic");
}
