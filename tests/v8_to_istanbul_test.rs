//! Integration tests for the V8-to-Istanbul converter.
//!
//! Each test feeds a small JS snippet plus a hand-crafted V8 coverage report
//! (mirroring what `node --experimental-vm-modules --coverage` or
//! `@vitest/coverage-v8` would produce) and checks that the resulting Istanbul
//! `FileCoverage` carries the right per-statement / per-function hit counts.

use oxc_coverage_instrument::{V8CoverageRange, V8FunctionCoverage, v8_to_istanbul};

fn range(start: u32, end: u32, count: u32) -> V8CoverageRange {
    V8CoverageRange { start_offset: start, end_offset: end, count }
}

fn function(name: &str, ranges: Vec<V8CoverageRange>, is_block: bool) -> V8FunctionCoverage {
    V8FunctionCoverage { function_name: name.to_string(), ranges, is_block_coverage: is_block }
}

#[test]
fn assigns_function_count_from_outer_range() {
    // Three lines, one statement each. The V8 module-level range covers the
    // whole file with count = 1 (the module ran once).
    let source = "const x = 1;\nconst y = 2;\nconst z = 3;\n";
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let fc = v8_to_istanbul(source, "app.js", &functions, 0).unwrap();

    // Every statement falls inside the module range, so each gets count = 1.
    let s_values: Vec<u32> = fc.s.values().copied().collect();
    assert!(!s_values.is_empty(), "expected statementMap entries");
    assert!(s_values.iter().all(|&c| c == 1), "all statements should be covered: {s_values:?}");
}

#[test]
fn unexecuted_block_overrides_outer_count() {
    // Outer function range count = 1. Inner block range (the `if` body) has
    // count = 0. The "y = 2" statement sits inside that inner range and must
    // adopt count 0, not 1.
    let source = "function f() {\n  if (false) {\n    const y = 2;\n  }\n}\nf();\n";
    let module_end = source.len() as u32;
    // Locate the `if` body block; for the test we just need a range tightly
    // around the `const y = 2;` statement.
    let inner_start = source.find("    const y").unwrap() as u32;
    let inner_end = source.find("  }").unwrap() as u32;
    let functions =
        vec![function("f", vec![range(0, module_end, 1), range(inner_start, inner_end, 0)], true)];

    let fc = v8_to_istanbul(source, "branchy.js", &functions, 0).unwrap();

    // Find the statement entry for `const y = 2;` and verify its count is 0.
    let y_id = fc
        .statement_map
        .iter()
        .find(|(_, loc)| loc.start.line == 3)
        .map(|(id, _)| id.clone())
        .expect("statementMap should hold the inner declaration");
    assert_eq!(fc.s.get(&y_id).copied().unwrap_or(99), 0, "inner block count must override outer");
}

#[test]
fn returns_empty_when_no_coverage_ranges_apply() {
    // No ranges at all: every statement count should be 0.
    let source = "const x = 1;";
    let fc = v8_to_istanbul(source, "empty.js", &[], 0).unwrap();
    assert!(fc.s.values().all(|&c| c == 0), "no ranges, no hits");
}

#[test]
fn applies_wrapper_length_for_cjs_modules() {
    // Stock Node wraps CJS modules in `(function(...){`; V8 byte offsets are
    // shifted by that wrapper. The caller passes `wrapper_length` to undo
    // the shift on the source side.
    let source = "const x = 1;\nconst y = 2;\n";
    let module_end = source.len() as u32;
    // The wrapped (V8-visible) source is `<wrapper>const x = 1;\nconst y = 2;\n`.
    // V8 reports ranges in the WRAPPED source's offsets.
    let wrapper_length = 62;
    let functions =
        vec![function("", vec![range(wrapper_length, wrapper_length + module_end, 1)], false)];

    let fc = v8_to_istanbul(source, "cjs.js", &functions, wrapper_length).unwrap();
    let s_values: Vec<u32> = fc.s.values().copied().collect();
    assert!(s_values.iter().all(|&c| c == 1), "wrapper offset must be subtracted: {s_values:?}");
}

#[test]
fn function_counts_track_call_counts() {
    let source = "function add(a, b) { return a + b; }\nadd(1, 2);\n";
    let end = source.len() as u32;
    // V8 says the function ran 3 times. The module body ran 1 time. Two
    // distinct V8 entries.
    let functions = vec![
        function("", vec![range(0, end, 1)], false),
        function(
            "add",
            vec![range(
                source.find("function").unwrap() as u32,
                source.find('}').unwrap() as u32 + 1,
                3,
            )],
            false,
        ),
    ];

    let fc = v8_to_istanbul(source, "calls.js", &functions, 0).unwrap();

    let add_id = fc
        .fn_map
        .iter()
        .find(|(_, fe)| fe.name == "add")
        .map(|(id, _)| id.clone())
        .expect("fnMap must include `add`");
    assert_eq!(
        fc.f.get(&add_id).copied().unwrap_or(0),
        3,
        "function count should match V8 call count"
    );
}
