//! Integration tests for the V8-to-Istanbul converter.
//!
//! Each test feeds a small JS snippet plus a hand-crafted V8 coverage report
//! (mirroring what `node --experimental-vm-modules --coverage` or
//! `@vitest/coverage-v8` would produce) and checks that the resulting Istanbul
//! `FileCoverage` carries the right per-statement / per-function hit counts.

use cow_utils::CowUtils;

use oxc_coverage_instrument::{
    V8CoverageRange, V8FunctionCoverage, V8ToIstanbulError, v8_to_istanbul,
    v8_to_istanbul_with_loader,
};

fn range(start: u32, end: u32, count: u32) -> V8CoverageRange {
    V8CoverageRange { start_offset: start, end_offset: end, count }
}

/// Byte length of `source` as a V8 offset.
fn byte_len(source: &str) -> u32 {
    u32::try_from(source.len()).unwrap()
}

/// UTF-16 code-unit length of `source`, the unit V8 reports offsets in.
fn utf16_len(source: &str) -> u32 {
    u32::try_from(source.encode_utf16().count()).unwrap()
}

/// Byte offset of the first occurrence of `needle` in `source`.
fn byte_offset_of(source: &str, needle: &str) -> u32 {
    u32::try_from(source.find(needle).expect("needle occurs in source")).unwrap()
}

/// Byte offset of the last occurrence of `needle` in `source`.
fn last_byte_offset_of(source: &str, needle: &str) -> u32 {
    u32::try_from(source.rfind(needle).expect("needle occurs in source")).unwrap()
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
    let end = byte_len(source);
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
    let module_end = byte_len(source);
    // Locate the `if` body block; for the test we just need a range tightly
    // around the `const y = 2;` statement.
    let inner_start = byte_offset_of(source, "    const y");
    let inner_end = byte_offset_of(source, "  }");
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
fn applies_utf16_wrapper_base_for_shifted_producers() {
    let source = "const marker = '😀é';\r\nconst y = 2;\n";
    let module_end = utf16_len(source);
    // Inspector coverage is normally source-relative. Some other producers
    // report a wrapper-shifted range and pass that UTF-16 base explicitly.
    let wrapper_length = 62;
    let functions =
        vec![function("", vec![range(wrapper_length, wrapper_length + module_end, 1)], false)];

    let fc = v8_to_istanbul(source, "cjs.js", &functions, wrapper_length).unwrap();
    let s_values: Vec<u32> = fc.s.values().copied().collect();
    assert!(s_values.iter().all(|&c| c == 1), "wrapper base must be applied: {s_values:?}");
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
    let module_end = byte_len(source);
    let then_start = byte_offset_of(source, "if (x) {") + 7;
    let then_end = byte_offset_of(source, "} else") + 1;
    let else_start = byte_offset_of(source, "else {") + 5;
    let else_end = last_byte_offset_of(source, "\n  }") + 4;

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
    let module_end = byte_len(source);
    let then_start = byte_offset_of(source, "if (x) {") + 7;
    let then_end = byte_offset_of(source, "} else") + 1;
    let else_start = byte_offset_of(source, "else {") + 5;
    let else_end = last_byte_offset_of(source, "\n  }") + 4;

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
    let end = byte_len(source);
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
    let end = byte_len(source);
    let functions = vec![function("", vec![range(0, end, 1)], false)];

    let fc = v8_to_istanbul(source, "bad-map.js", &functions, 0).unwrap();
    assert!(fc.input_source_map.is_none(), "corrupt inline map must not produce inputSourceMap");
    assert!(fc.s.values().any(|&c| c == 1), "rest of coverage still resolves");
}

#[test]
fn extracts_inline_urlsafe_base64_source_map() {
    // The decoder must accept the URL-safe base64 alphabet (RFC 4648 §5, `-`
    // and `_` in place of `+` and `/`) as well as the standard one, or an
    // inline map written that way is silently dropped.
    let original_map_json = r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["const x: number = 1;"],"mappings":"AAAA","names":[]}"#;
    let base64 = encode_base64(original_map_json.as_bytes())
        .cow_replace('+', "-")
        .cow_replace('/', "_")
        .into_owned();
    let source =
        format!("const x = 1;\n//# sourceMappingURL=data:application/json;base64,{base64}\n");
    let end = byte_len(&source);
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
    let end = byte_len(&source);
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
    // Istanbul and V8 report UTF-16 positions; srcmap's generated columns use
    // UTF-16 while Oxc spans use bytes. A
    // statement that follows a non-ASCII character must still land inside
    // the V8 range that contains it. `π` is 2 bytes UTF-8 / 1 UTF-16 unit,
    // so the byte position of `const y` shifts by an extra byte per π.
    let source = "const π = 1;\nconst y = π + 1;\n";
    let end = utf16_len(source);
    let functions = vec![function("", vec![range(0, end, 7)], false)];

    let fc = v8_to_istanbul(source, "greek.js", &functions, 0).unwrap();
    let s_values: Vec<u32> = fc.s.values().copied().collect();
    assert!(
        s_values.iter().all(|&c| c == 7),
        "non-ASCII statements must still resolve into the V8 range, got: {s_values:?}"
    );
}

#[test]
fn normalizes_real_inspector_utf16_offsets_before_matching() {
    let source = r"const prefix = '😀😀😀😀😀😀é';
function unicodeBranch(value) {
  if (value) {
    return '✓';
  } else {
    return 'miss';
  }
}
unicodeBranch(true);
unicodeBranch(true);
//# sourceURL=oxc-coverage-instrument://unicode-offsets.js
";
    // Captured from Node's Profiler.takePreciseCoverage. These offsets are
    // UTF-16 code units, including the astral and BMP characters above.
    let functions = vec![
        function("", vec![range(0, 232, 1)], true),
        function("unicodeBranch", vec![range(32, 130, 2), range(98, 128, 0)], true),
    ];

    let fc = v8_to_istanbul(source, "inspector-unicode.js", &functions, 0).unwrap();
    let function_count = fc
        .fn_map
        .iter()
        .find(|(_, entry)| entry.name == "unicodeBranch")
        .and_then(|(id, _)| fc.f.get(id))
        .copied();
    let branch_counts = fc
        .branch_map
        .iter()
        .find(|(_, entry)| entry.branch_type == "if")
        .and_then(|(id, entry)| fc.b.get(id).map(|counts| (entry, counts)))
        .expect("if branch must appear in branchMap");

    assert_eq!(function_count, Some(2));
    assert_eq!(branch_counts.1, &[2, 0]);
    assert_eq!(branch_counts.1.len(), branch_counts.0.locations.len());
}

#[test]
fn preserves_utf16_offsets_across_crlf_and_mixed_newlines() {
    let source = "const astral = '😀';\r\nconst bmp = 'é';\nconst value = astral + bmp;\r\n";
    let source_end = utf16_len(source);
    let expression_start = utf16_len(&source[..source.rfind("astral + bmp").unwrap()]);
    let expression_end = expression_start + utf16_len("astral + bmp");
    let functions = vec![function(
        "",
        vec![range(0, source_end, 1), range(expression_start, expression_end, 9)],
        true,
    )];

    let fc = v8_to_istanbul(source, "mixed-newlines.js", &functions, 0).unwrap();
    let line_three_id = fc
        .statement_map
        .iter()
        .find(|(_, location)| location.start.line == 3)
        .map(|(id, _)| id)
        .expect("third-line expression must be tracked");

    assert_eq!(fc.s.get(line_three_id), Some(&9));
}

#[test]
fn unicode_prior_line_preserves_nested_function_arm_inheritance() {
    let source = "const marker = '😀é';\r\nif (true) { function g() {} }\r\ng(); g();\n";
    let source_end = utf16_len(source);
    let function_start = utf16_len(&source[..source.find("function g").unwrap()]);
    let function_end = function_start + utf16_len("function g() {}");
    let functions = [
        function("", vec![range(0, source_end, 1)], true),
        function("g", vec![range(function_start, function_end, 2)], true),
    ];

    assert_if_counts(source, &functions, &[1, 0]);
}

#[test]
fn unicode_offsets_do_not_change_attached_source_map_locations() {
    let map_json = r#"{"version":3,"sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
    let base64 = encode_base64(map_json.as_bytes());
    let source = format!(
        "const marker = '😀é';\r\nconst value = 1;\n//# sourceMappingURL=data:application/json;base64,{base64}\n"
    );
    let end = utf16_len(&source);
    let functions = vec![function("", vec![range(0, end, 3)], false)];

    let fc = v8_to_istanbul(&source, "unicode-map.js", &functions, 0).unwrap();
    let line_two = fc
        .statement_map
        .iter()
        .find(|(_, location)| location.start.line == 2)
        .expect("second-line statement must remain in generated-source coordinates");

    assert_eq!(fc.s.get(line_two.0), Some(&3));
    assert_eq!(line_two.1.start.line, 2);
    assert_eq!(fc.input_source_map.as_ref().unwrap()["sources"][0], "src/app.ts");
}

#[test]
fn function_counts_track_call_counts() {
    let source = "function add(a, b) { return a + b; }\nadd(1, 2);\n";
    let end = byte_len(source);
    // V8 says the function ran 3 times. The module body ran 1 time. Two
    // distinct V8 entries.
    let functions = vec![
        function("", vec![range(0, end, 1)], false),
        function(
            "add",
            vec![range(byte_offset_of(source, "function"), byte_offset_of(source, "}") + 1, 3)],
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
    // arm[0] reports how many times the predicate was truthy, resolved from
    // the consequent-body byte span against V8's then-block range. Five calls,
    // three truthy and two falsy, therefore give [3, 2].
    let source = "function f(x) {\n  if (x) {\n    a();\n  } else {\n    b();\n  }\n}\n";
    let module_end = byte_len(source);
    let then_start = byte_offset_of(source, "if (x) {") + 7;
    let then_end = byte_offset_of(source, "} else") + 1;
    let else_start = byte_offset_of(source, "else {") + 5;
    let else_end = last_byte_offset_of(source, "\n  }") + 4;

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
    let module_end = byte_len(source);
    let then_start = byte_offset_of(source, "if (x) {") + 7;
    let then_end = last_byte_offset_of(source, "\n  }") + 4;

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
            function("", vec![range(0, byte_len(&source), 1)], true),
            function("f", vec![range(0, REAL_V8_FUNCTION_END, 2)], false),
        ],
        &[0, 0],
    );
}

#[test]
fn real_v8_if_ignores_uncalled_nested_function_outer_range() {
    let source = "if (true) { function g() {} }\n";
    let functions = [
        function("", vec![range(0, byte_len(source), 1)], true),
        function("g", vec![range(12, 27, 0)], false),
    ];
    assert_if_counts(source, &functions, &[1, 0]);
}

#[test]
fn real_v8_if_ignores_called_nested_function_outer_range() {
    let source = "if (true) { function g() {} }\ng(); g();\n";
    let functions = [
        function("", vec![range(0, byte_len(source), 1)], true),
        function("g", vec![range(12, 27, 2)], true),
    ];
    assert_if_counts(source, &functions, &[1, 0]);
}

#[test]
fn real_v8_if_uses_strict_parent_for_annex_b_function_arm() {
    let source = "if (true) function g() {}\ng(); g();\n";
    let functions = [
        function("", vec![range(0, byte_len(source), 1)], true),
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
    let end = byte_len(source);
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
fn source_mapping_url_inside_template_does_not_invoke_loader() {
    let source = "const marker = `\n//# sourceMappingURL=ghost.map\n`;\n";
    let functions = vec![function("", vec![range(0, byte_len(source), 1)], false)];
    let seen = std::cell::RefCell::new(Vec::<String>::new());

    let coverage = v8_to_istanbul_with_loader(source, "app.js", &functions, 0, |url| {
        seen.borrow_mut().push(url.to_string());
        Some(r#"{"version":3,"sources":["ghost.ts"],"mappings":"AAAA","names":[]}"#.to_string())
    })
    .unwrap();

    assert!(seen.into_inner().is_empty(), "false trailer must not invoke the loader");
    assert!(coverage.input_source_map.is_none(), "false trailer must not attach a map");
}

#[test]
fn external_source_mapping_url_loader_returning_none_leaves_map_unset() {
    // A loader returning `None` (disk read failed, no map next to the file)
    // leaves `inputSourceMap` unset and does not disturb the rest of the
    // conversion.
    let source = "const x = 1;\n//# sourceMappingURL=missing.map\n";
    let end = byte_len(source);
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
    let end = byte_len(&source);
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
