//! Tests for the LCOV emitter: record layout, function-name sanitization,
//! and how malformed counters are read.

use super::*;
use oxc_coverage_report::summarize;
use oxc_coverage_types::parse_coverage_map;

const DAMAGED_BRANCH: &str = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}}},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{"0":1},"f":{},"b":{"0":[4,0,9]}}}"#;
const DAMAGED_BRANCH_MISSING: &str = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}}},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{"0":1},"f":{},"b":{}}}"#;

fn render(json: &str) -> String {
    let map = parse_coverage_map(json).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    write(&root, Path::new(""), &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn emits_tn_sf_and_end_of_record() {
    let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
    let out = render(json);
    assert!(out.starts_with("TN:\nSF:a.js"));
    assert!(out.contains("end_of_record"));
}

#[test]
fn branch_metadata_controls_emitted_arms() {
    let out = render(DAMAGED_BRANCH);
    assert!(out.contains("BRDA:1,0,0,4"));
    assert!(out.contains("BRDA:1,0,1,0"));
    assert!(out.contains("BRF:2"));
    assert!(out.contains("BRH:1"));
    assert!(!out.contains("BRDA:1,0,2,9"));
}

#[test]
fn missing_branch_array_emits_uncovered_metadata_arms() {
    let out = render(DAMAGED_BRANCH_MISSING);
    assert!(out.contains("BRDA:1,0,0,-"));
    assert!(out.contains("BRDA:1,0,1,-"));
    assert!(out.contains("BRF:2"));
    assert!(out.contains("BRH:0"));
}

#[test]
fn brda_block_uses_branch_entry_id_not_line() {
    // Two if-statements both starting on line 5 (id "0" at block 0, id "1"
    // at block 1). BRDA block field MUST differentiate the two so Codecov
    // does not merge them into a single 4-arm branch.
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {},
            "fnMap": {},
            "branchMap": {
                "0": {"loc": {"start": {"line": 5, "column": 0}, "end": {"line": 5, "column": 10}}, "line": 5, "type": "if", "locations": [{"start": {"line": 5, "column": 0}, "end": {"line": 5, "column": 5}}, {"start": {"line": 5, "column": 6}, "end": {"line": 5, "column": 10}}]},
                "1": {"loc": {"start": {"line": 5, "column": 11}, "end": {"line": 5, "column": 20}}, "line": 5, "type": "if", "locations": [{"start": {"line": 5, "column": 11}, "end": {"line": 5, "column": 15}}, {"start": {"line": 5, "column": 16}, "end": {"line": 5, "column": 20}}]}
            },
            "s": {},
            "f": {},
            "b": {"0": [1, 0], "1": [0, 1]}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("BRDA:5,0,0,1"), "got:\n{out}");
    assert!(out.contains("BRDA:5,0,1,0"), "got:\n{out}");
    assert!(out.contains("BRDA:5,1,0,0"), "got:\n{out}");
    assert!(out.contains("BRDA:5,1,1,1"), "got:\n{out}");
}

#[test]
fn brda_taken_is_dash_when_block_never_entered() {
    // All arms zero -> the if condition itself never ran -> emit `-`.
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {},
            "fnMap": {},
            "branchMap": {
                "0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}]}
            },
            "s": {},
            "f": {},
            "b": {"0": [0, 0]}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("BRDA:1,0,0,-"), "got:\n{out}");
    assert!(out.contains("BRDA:1,0,1,-"), "got:\n{out}");
}

#[test]
fn anonymous_function_name_has_parens_stripped() {
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {},
            "fnMap": {
                "0": {"name": "(anonymous_0)", "line": 3, "decl": {"start": {"line": 3, "column": 0}, "end": {"line": 3, "column": 3}}, "loc": {"start": {"line": 3, "column": 0}, "end": {"line": 5, "column": 0}}}
            },
            "branchMap": {},
            "s": {},
            "f": {"0": 2},
            "b": {}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("FN:3,anonymous_0"), "got:\n{out}");
    assert!(out.contains("FNDA:2,anonymous_0"), "got:\n{out}");
    assert!(!out.contains("(anonymous_0)"));
}

#[test]
fn both_da_and_brda_emitted() {
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {
                "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}
            },
            "fnMap": {},
            "branchMap": {
                "0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}]}
            },
            "s": {"0": 1},
            "f": {},
            "b": {"0": [1, 0]}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("DA:1,1"));
    assert!(out.contains("BRDA:1,0,0,1"));
}

#[test]
fn sf_is_relativized_against_root_dir() {
    let json = r#"{"/proj/src/a.js":{"path":"/proj/src/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
    let map = parse_coverage_map(json).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    write(&root, Path::new("/proj"), &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("SF:src/a.js"), "got:\n{out}");
    assert!(!out.contains("SF:/proj/src/a.js"));
}

#[test]
fn fn_name_with_internal_parens_is_neutralized() {
    // Asymmetric paren shapes ("(", ")") and names with internal parens
    // (minified output) must not survive into the FN: line because lcov 2.x
    // genhtml treats `(` / `)` as delimiters.
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {},
            "fnMap": {
                "0": {"name": "(", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 5, "column": 0}}},
                "1": {"name": "foo(x)bar", "line": 2, "decl": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 3}}, "loc": {"start": {"line": 2, "column": 0}, "end": {"line": 5, "column": 0}}}
            },
            "branchMap": {},
            "s": {},
            "f": {"0": 1, "1": 1},
            "b": {}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("FN:1,_"), "asymmetric paren must be replaced; got:\n{out}");
    assert!(out.contains("FN:2,foo_x_bar"), "internal parens must be replaced; got:\n{out}");
}

#[test]
fn brda_block_falls_back_to_index_for_non_numeric_id() {
    // Hand-crafted coverage maps can ship non-numeric branch ids (e.g.
    // "br_0" from a third-party emitter). The block field must still be
    // unique per branch; falling back to the iteration index achieves that.
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {},
            "fnMap": {},
            "branchMap": {
                "br_a": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}]},
                "br_b": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}]}
            },
            "s": {},
            "f": {},
            "b": {"br_a": [1, 0], "br_b": [0, 1]}
        }
    }"#;
    let out = render(json);
    // Both branches sit on line 1 but must produce DISTINCT block ids
    // (0 and 1 via enumeration) so Codecov treats them as separate ifs.
    assert!(out.contains("BRDA:1,0,0,1"), "got:\n{out}");
    assert!(out.contains("BRDA:1,1,0,0"), "got:\n{out}");
}

#[test]
fn lf_lh_counts_unique_lines() {
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {
                "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
                "1": {"start": {"line": 1, "column": 6}, "end": {"line": 1, "column": 10}},
                "2": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}}
            },
            "fnMap": {},
            "branchMap": {},
            "s": {"0": 1, "1": 0, "2": 0},
            "f": {},
            "b": {}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("LF:2"), "got:\n{out}");
    assert!(out.contains("LH:1"), "got:\n{out}");
}
