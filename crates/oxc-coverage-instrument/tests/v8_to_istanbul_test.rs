//! Integration tests for the V8-to-Istanbul converter.
//!
//! Each test feeds a small JS snippet plus a hand-crafted V8 coverage report
//! (mirroring what `node --experimental-vm-modules --coverage` or
//! `@vitest/coverage-v8` would produce) and checks that the resulting Istanbul
//! `FileCoverage` carries the right per-statement / per-function hit counts.

use oxc_coverage_instrument::{
    V8CoverageRange, V8FunctionCoverage, V8ToIstanbulError, v8_to_istanbul,
    v8_to_istanbul_with_loader,
};

fn range(start: u32, end: u32, count: u32) -> V8CoverageRange {
    V8CoverageRange { start_offset: start, end_offset: end, count }
}

fn function(name: &str, ranges: Vec<V8CoverageRange>, is_block: bool) -> V8FunctionCoverage {
    V8FunctionCoverage { function_name: name.to_string(), ranges, is_block_coverage: is_block }
}

const REAL_V8_IF_FUNCTION_SOURCE: &str =
    "function f(x) { if (x) { return 1; } else { return -1; } }";
const REAL_V8_FUNCTION_END: u32 = 58;
const REAL_V8_THEN_START: u32 = 23;
const REAL_V8_THEN_END: u32 = 36;
const REAL_V8_ELSE_START: u32 = 36;
const REAL_V8_ELSE_END: u32 = 56;

fn assert_real_v8_if_counts(calls: &str, ranges: Vec<V8CoverageRange>, expected: &[u32]) {
    assert_real_v8_if_functions(calls, &[function("f", ranges, true)], expected);
}

fn assert_real_v8_if_functions(calls: &str, functions: &[V8FunctionCoverage], expected: &[u32]) {
    let source = format!("{REAL_V8_IF_FUNCTION_SOURCE} {calls}\n");
    assert_if_counts(&source, functions, expected);
}

fn assert_if_counts(source: &str, functions: &[V8FunctionCoverage], expected: &[u32]) {
    let fc = v8_to_istanbul(source, "real-inspector-if.js", functions, 0).unwrap();
    let arm_counts = fc
        .branch_map
        .iter()
        .find(|(_, branch)| branch.branch_type == "if")
        .and_then(|(id, _)| fc.b.get(id))
        .expect("if branch must appear in branchMap");
    assert_eq!(arm_counts, expected);
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
fn assigns_branch_arm_counts_from_block_coverage() {
    // V8 block coverage emits one inner range per `{ ... }` BlockStatement,
    // not per inner statement. The arm resolver matches both BlockStatement
    // ranges against the collected consequent / alternate body spans, even
    // though istanbul records `branchMap.locations[0]` as the whole
    // `IfStatement` span (see istanbul-lib-instrument's coverIfBranches).
    // arm[0] therefore reflects "number of times the predicate was truthy",
    // which equals the then-block's hit count.
    let source = "function f(x) {\n  if (x) {\n    a();\n  } else {\n    b();\n  }\n}\nf(true);\n";
    let module_end = source.len() as u32;
    let then_start = source.find("if (x) {").unwrap() as u32 + 7;
    let then_end = source.find("} else").unwrap() as u32 + 1;
    let else_start = source.find("else {").unwrap() as u32 + 5;
    let else_end = source.rfind("\n  }").unwrap() as u32 + 4;

    let functions = vec![function(
        "f",
        vec![
            range(0, module_end, 1),
            range(then_start, then_end, 1),
            range(else_start, else_end, 0),
        ],
        true,
    )];

    let fc = v8_to_istanbul(source, "ifelse.js", &functions, 0).unwrap();
    let (_, arm_counts) = fc
        .branch_map
        .iter()
        .find(|(_, b)| b.branch_type == "if")
        .map(|(id, _b)| (id.clone(), fc.b.get(id).cloned().unwrap_or_default()))
        .expect("if branch must appear in branchMap");
    assert_eq!(arm_counts.len(), 2, "if has two arms");
    assert_eq!(
        arm_counts[0], 1,
        "arm[0] resolves through the collected then-block body span; predicate truthy once"
    );
    assert_eq!(arm_counts[1], 0, "else arm should report zero hits");
}

#[test]
fn branch_arm_count_picks_tight_inner_range_over_enclosing_outer() {
    // V8 emits ranges outermost-first. If the arm-resolver returned the first
    // tolerance match, a naive walk would prefer the enclosing block (with the
    // function's or module's higher count) over the actual arm block. Pin the
    // tightest-match rule: when the else range itself reports a non-zero hit
    // (e.g., the function ran 5 times and the else arm was taken 2 times),
    // arm[1] must reflect the else range's count, not the outer's.
    let source = "function f(x) {\n  if (x) {\n    a();\n  } else {\n    b();\n  }\n}\n";
    let module_end = source.len() as u32;
    let then_start = source.find("if (x) {").unwrap() as u32 + 7;
    let then_end = source.find("} else").unwrap() as u32 + 1;
    let else_start = source.find("else {").unwrap() as u32 + 5;
    let else_end = source.rfind("\n  }").unwrap() as u32 + 4;

    // Outer function ran 5 times; else arm taken 2 of those; then arm taken 3.
    let functions = vec![function(
        "f",
        vec![
            range(0, module_end, 5),
            range(then_start, then_end, 3),
            range(else_start, else_end, 2),
        ],
        true,
    )];

    let fc = v8_to_istanbul(source, "tight.js", &functions, 0).unwrap();
    let (_, arm_counts) = fc
        .branch_map
        .iter()
        .find(|(_, b)| b.branch_type == "if")
        .map(|(id, _b)| (id.clone(), fc.b.get(id).cloned().unwrap_or_default()))
        .expect("if branch must appear in branchMap");
    assert_eq!(
        arm_counts[1], 2,
        "tightest match must win: else arm should resolve to else range count (2), not outer (5)"
    );
}

#[test]
fn ternary_arms_report_zero_when_no_block_range_matches() {
    // Ternary expressions (`cond-expr`) have no `{ ... }` BlockStatement, so
    // V8 emits no inner range for either arm. Falling back to the enclosing
    // function count would report both arms as executed N times when only
    // one arm runs per evaluation; that trips coverage gates silently.
    // The tolerance-based arm resolver returns 0 instead, which is honest.
    let source = "function f(x) { return x ? a() : b(); }\nf(true);\n";
    let end = source.len() as u32;
    // Function has count = 1; no inner block ranges (no `{ ... }` inside
    // the ternary). istanbul still records both arms in branchMap.
    let functions = vec![function("f", vec![range(0, end, 1)], true)];

    let fc = v8_to_istanbul(source, "ternary.js", &functions, 0).unwrap();
    let (_, arm_counts) = fc
        .branch_map
        .iter()
        .find(|(_, b)| b.branch_type == "cond-expr")
        .map(|(id, _b)| (id.clone(), fc.b.get(id).cloned().unwrap_or_default()))
        .expect("cond-expr branch must appear in branchMap");
    assert_eq!(arm_counts.len(), 2, "ternary has two arms");
    assert!(
        arm_counts.iter().all(|&c| c == 0),
        "both ternary arms must report 0, not the function count: {arm_counts:?}"
    );
}

#[test]
fn corrupted_inline_source_map_silently_skipped() {
    // A malformed base64 payload should be silently ignored rather than
    // erroring or panicking. The function returns FileCoverage with no
    // inputSourceMap attached, and the rest of the conversion proceeds as
    // if no inline map were present. Pinning this prevents future drift
    // toward fail-loud behavior that would break otherwise-valid coverage
    // on a single bad map line.
    let source =
        "const x = 1;\n//# sourceMappingURL=data:application/json;base64,!!!not-base64!!!\n";
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let fc = v8_to_istanbul(source, "bad-map.js", &functions, 0).unwrap();
    assert!(fc.input_source_map.is_none(), "corrupt inline map must not produce inputSourceMap");
    assert!(fc.s.values().any(|&c| c == 1), "rest of coverage still resolves");
}

#[test]
fn extracts_inline_urlsafe_base64_source_map() {
    // esbuild emits inline maps using the URL-safe base64 alphabet (RFC 4648 §5,
    // `-` and `_` instead of `+` and `/`). Decoder must accept both alphabets
    // so esbuild-bundled JS does not silently drop its inline map.
    let original_map_json = r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["const x: number = 1;"],"mappings":"AAAA","names":[]}"#;
    // Encode standard then transliterate `+/` to `-_` to mimic URL-safe output.
    let base64 = encode_base64(original_map_json.as_bytes()).replace('+', "-").replace('/', "_");
    let source =
        format!("const x = 1;\n//# sourceMappingURL=data:application/json;base64,{base64}\n");
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let fc = v8_to_istanbul(&source, "app.js", &functions, 0).unwrap();
    let attached = fc.input_source_map.expect("URL-safe inline map should attach");
    assert_eq!(attached["sources"][0], "src/app.ts");
}

#[test]
fn extracts_inline_base64_source_map() {
    // Vite, esbuild, swc, tsc all emit `//# sourceMappingURL=data:...;base64,...`
    // trailers. v8_to_istanbul must decode and attach that map as
    // inputSourceMap so a downstream remap_coverage chains cleanly.
    let original_map_json = r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["const x: number = 1;"],"mappings":"AAAA","names":[]}"#;
    let base64 = encode_base64(original_map_json.as_bytes());
    let source =
        format!("const x = 1;\n//# sourceMappingURL=data:application/json;base64,{base64}\n");
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let fc = v8_to_istanbul(&source, "app.js", &functions, 0).unwrap();
    let attached = fc.input_source_map.expect("inline map should attach");
    assert_eq!(attached["sources"][0], "src/app.ts");
    assert_eq!(attached["version"], 3);
}

/// Test helper: encode a byte slice as base64 using the standard alphabet.
fn encode_base64(bytes: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(ALPHA[(b0 >> 2) as usize] as char);
        out.push(ALPHA[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHA[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHA[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[test]
fn handles_non_ascii_source_columns() {
    // Istanbul reports UTF-16 columns; srcmap and V8 work in bytes. A
    // statement that follows a non-ASCII character must still land inside
    // the V8 range that contains it. `π` is 2 bytes UTF-8 / 1 UTF-16 unit,
    // so the byte position of `const y` shifts by an extra byte per π.
    let source = "const π = 1;\nconst y = π + 1;\n";
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 7)], false)];

    let fc = v8_to_istanbul(source, "greek.js", &functions, 0).unwrap();
    let s_values: Vec<u32> = fc.s.values().copied().collect();
    assert!(
        s_values.iter().all(|&c| c == 7),
        "non-ASCII statements must still resolve into the V8 range, got: {s_values:?}"
    );
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

#[test]
fn if_arm_zero_reports_then_block_count_with_mixed_arms() {
    // The fix for issue #43 task 2: arm[0] of an `if` branch reports
    // "number of times the predicate evaluated truthy", derived from the
    // collected consequent-body byte span and matched against V8's then
    // BlockStatement range. Calling `f` five times with three truthy and
    // two falsy inputs gives arm[0] == 3 and arm[1] == 2, not [0, 2] as
    // the pre-fix behavior would have produced.
    let source = "function f(x) {\n  if (x) {\n    a();\n  } else {\n    b();\n  }\n}\n";
    let module_end = source.len() as u32;
    let then_start = source.find("if (x) {").unwrap() as u32 + 7;
    let then_end = source.find("} else").unwrap() as u32 + 1;
    let else_start = source.find("else {").unwrap() as u32 + 5;
    let else_end = source.rfind("\n  }").unwrap() as u32 + 4;

    let functions = vec![function(
        "f",
        vec![
            range(0, module_end, 5),
            range(then_start, then_end, 3),
            range(else_start, else_end, 2),
        ],
        true,
    )];

    let fc = v8_to_istanbul(source, "ifelse-counts.js", &functions, 0).unwrap();
    let (_, arm_counts) = fc
        .branch_map
        .iter()
        .find(|(_, b)| b.branch_type == "if")
        .map(|(id, _b)| (id.clone(), fc.b.get(id).cloned().unwrap_or_default()))
        .expect("if branch must appear in branchMap");
    assert_eq!(
        arm_counts,
        vec![3, 2],
        "arm[0] = then-block count (3), arm[1] = else-block count (2)"
    );
}

#[test]
fn if_arm_zero_resolves_when_alternate_is_missing() {
    // No `else` clause; istanbul still records two arms with locations[1]
    // pointing at the synthesized empty alternate. arm[0] must still resolve
    // through the collected then-block span; arm[1] is honestly zero.
    let source = "function f(x) {\n  if (x) {\n    a();\n  }\n}\n";
    let module_end = source.len() as u32;
    let then_start = source.find("if (x) {").unwrap() as u32 + 7;
    let then_end = source.rfind("\n  }").unwrap() as u32 + 4;

    let functions =
        vec![function("f", vec![range(0, module_end, 4), range(then_start, then_end, 4)], true)];

    let fc = v8_to_istanbul(source, "if-only.js", &functions, 0).unwrap();
    let (_, arm_counts) = fc
        .branch_map
        .iter()
        .find(|(_, b)| b.branch_type == "if")
        .map(|(id, _b)| (id.clone(), fc.b.get(id).cloned().unwrap_or_default()))
        .expect("if branch must appear in branchMap");
    assert_eq!(arm_counts.len(), 2, "if without else still has two arms");
    assert_eq!(arm_counts[0], 4, "arm[0] reflects then-block count");
    assert_eq!(arm_counts[1], 0, "synthetic else-arm honestly reports zero");
}

#[test]
fn real_v8_if_inherits_then_count_when_only_else_has_a_child_range() {
    assert_real_v8_if_counts(
        "f(true); f(true);",
        vec![range(0, REAL_V8_FUNCTION_END, 2), range(REAL_V8_ELSE_START, REAL_V8_ELSE_END, 0)],
        &[2, 0],
    );
}

#[test]
fn real_v8_if_inherits_else_count_when_only_then_has_a_child_range() {
    assert_real_v8_if_counts(
        "f(false); f(false);",
        vec![range(0, REAL_V8_FUNCTION_END, 2), range(REAL_V8_THEN_START, REAL_V8_THEN_END, 0)],
        &[0, 2],
    );
}

#[test]
fn real_v8_if_matches_explicit_then_and_else_ranges() {
    assert_real_v8_if_counts(
        "f(true); f(false); f(true);",
        vec![
            range(0, REAL_V8_FUNCTION_END, 3),
            range(REAL_V8_THEN_START, REAL_V8_THEN_END, 2),
            range(REAL_V8_ELSE_START, REAL_V8_ELSE_END, 1),
        ],
        &[2, 1],
    );
}

#[test]
fn function_only_v8_coverage_does_not_infer_if_arm_counts() {
    let calls = "f(true); f(true);";
    let source = format!("{REAL_V8_IF_FUNCTION_SOURCE} {calls}\n");
    assert_real_v8_if_functions(
        calls,
        &[
            function("", vec![range(0, source.len() as u32, 1)], true),
            function("f", vec![range(0, REAL_V8_FUNCTION_END, 2)], false),
        ],
        &[0, 0],
    );
}

#[test]
fn real_v8_if_ignores_uncalled_nested_function_outer_range() {
    let source = "if (true) { function g() {} }\n";
    let functions = [
        function("", vec![range(0, source.len() as u32, 1)], true),
        function("g", vec![range(12, 27, 0)], false),
    ];
    assert_if_counts(source, &functions, &[1, 0]);
}

#[test]
fn real_v8_if_ignores_called_nested_function_outer_range() {
    let source = "if (true) { function g() {} }\ng(); g();\n";
    let functions = [
        function("", vec![range(0, source.len() as u32, 1)], true),
        function("g", vec![range(12, 27, 2)], true),
    ];
    assert_if_counts(source, &functions, &[1, 0]);
}

#[test]
fn real_v8_if_uses_strict_parent_for_annex_b_function_arm() {
    let source = "if (true) function g() {}\ng(); g();\n";
    let functions = [
        function("", vec![range(0, source.len() as u32, 1)], true),
        function("g", vec![range(10, 25, 2)], true),
    ];
    assert_if_counts(source, &functions, &[1, 0]);
}

#[test]
fn external_source_mapping_url_invokes_loader() {
    // When the source carries a non-data sourceMappingURL trailer, the loader
    // is consulted with the URL string and the returned JSON is parsed and
    // attached as inputSourceMap on the result.
    let map_json =
        r#"{"version":3,"sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#.to_string();
    let source = "const x = 1;\n//# sourceMappingURL=app.js.map\n";
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let seen_urls = std::cell::RefCell::new(Vec::<String>::new());
    let fc = v8_to_istanbul_with_loader(source, "app.js", &functions, 0, |url| {
        seen_urls.borrow_mut().push(url.to_string());
        if url == "app.js.map" { Some(map_json.clone()) } else { None }
    })
    .unwrap();

    assert_eq!(
        seen_urls.into_inner(),
        vec!["app.js.map".to_string()],
        "loader sees the trailer URL"
    );
    let attached = fc.input_source_map.expect("external map should be attached");
    assert_eq!(attached["sources"][0], "src/app.ts");
}

#[test]
fn external_source_mapping_url_loader_returning_none_leaves_map_unset() {
    // A loader that returns None (disk read failed, no map next to file)
    // must not panic or attach garbage. The result must look like the
    // pre-loader behavior: no inputSourceMap, rest of the conversion intact.
    let source = "const x = 1;\n//# sourceMappingURL=missing.map\n";
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let fc = v8_to_istanbul_with_loader(source, "missing.js", &functions, 0, |_| None).unwrap();
    assert!(fc.input_source_map.is_none(), "no map attached when loader returns None");
    assert!(fc.s.values().any(|&c| c == 1), "rest of coverage still resolves");
}

#[test]
fn inline_source_map_takes_precedence_over_external_loader() {
    // When both forms could match (inline data URL is rfind-found before any
    // other comment), the inline form wins and the loader is never called.
    // A simple way to surface that: a panicking loader that would fail the
    // test if invoked.
    let original_map_json =
        r#"{"version":3,"sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
    let base64 = encode_base64(original_map_json.as_bytes());
    let source =
        format!("const x = 1;\n//# sourceMappingURL=data:application/json;base64,{base64}\n");
    let end = source.len() as u32;
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let fc = v8_to_istanbul_with_loader(&source, "app.js", &functions, 0, |_| {
        panic!("loader must not be called when an inline map is present")
    })
    .unwrap();
    let attached = fc.input_source_map.expect("inline map should be attached");
    assert_eq!(attached["sources"][0], "src/app.ts");
}

#[test]
fn parse_error_propagates_as_v8_to_istanbul_error() {
    // Source that the Oxc parser refuses (stray `}` at the top level). Without
    // a Parse-error path, the converter would silently produce an empty
    // FileCoverage and downstream reporters would show 0 lines instead of
    // surfacing the underlying syntax error.
    let invalid = "function () { }}}\nconst x = ;\n";
    let functions: Vec<V8FunctionCoverage> = vec![];

    let err = v8_to_istanbul(invalid, "broken.js", &functions, 0)
        .expect_err("invalid source must return a Parse error");
    let V8ToIstanbulError::Parse(message) = err;
    assert!(
        !message.is_empty(),
        "parse error must carry the underlying diagnostic, got empty string",
    );
}

#[test]
fn v8_to_istanbul_error_display_includes_diagnostic() {
    // The `Display` impl is the user-facing surface in CLI error messages; if
    // it ever stops prefixing with `parse error:` the CLI output silently
    // changes shape. Pin both the prefix and the inner message.
    let err = V8ToIstanbulError::Parse("unexpected token".to_string());
    let rendered = err.to_string();
    assert!(rendered.starts_with("parse error: "), "got: {rendered}");
    assert!(rendered.contains("unexpected token"), "got: {rendered}");

    // `std::error::Error` is implemented so downstream callers can use `?`
    // through their own error enums; trivial smoke check that the trait
    // method resolves.
    let as_err: &dyn std::error::Error = &err;
    assert!(as_err.source().is_none());
}
