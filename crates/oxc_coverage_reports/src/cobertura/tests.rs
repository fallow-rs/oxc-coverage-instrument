//! Tests for the Cobertura emitter: document shape, DTD-required attributes,
//! rate formatting, and how malformed counters are read.

use super::*;
use oxc_coverage_report::summarize;
use oxc_coverage_types::parse_coverage_map;

const DAMAGED_BRANCH: &str = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}}},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{"0":1},"f":{},"b":{"0":[4,0,9]}}}"#;
const DAMAGED_BRANCH_MISSING: &str = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}}},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{"0":1},"f":{},"b":{}}}"#;

fn render(json: &str) -> String {
    let map = parse_coverage_map(json).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    write_with_timestamp(&root, Path::new(""), 0, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn root_carries_line_rate_branch_rate_timestamp() {
    let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
    let out = render(json);
    assert!(out.contains("line-rate=\""));
    assert!(out.contains("branch-rate=\""));
    assert!(out.contains("timestamp=\"0\""));
}

#[test]
fn branch_metadata_controls_rates() {
    let out = render(DAMAGED_BRANCH);
    assert!(out.contains("branches-valid=\"2\""));
    assert!(out.contains("branches-covered=\"1\""));
    assert!(out.contains("condition-coverage=\"50% (1/2)\""));
}

#[test]
fn missing_branch_array_counts_as_uncovered_in_cobertura() {
    let out = render(DAMAGED_BRANCH_MISSING);
    assert!(out.contains("branches-valid=\"2\""));
    assert!(out.contains("branches-covered=\"0\""));
    assert!(out.contains("condition-coverage=\"0% (0/2)\""));
}

#[test]
fn class_filename_is_relative() {
    let json = r#"{"/proj/src/a.js":{"path":"/proj/src/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
    let map = parse_coverage_map(json).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    write_with_timestamp(&root, Path::new("/proj"), 0, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("filename=\"src/a.js\""), "got:\n{out}");
    assert!(!out.contains("filename=\"/proj/src/a.js\""));
}

#[test]
fn methods_carry_complexity_zero() {
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {},
            "fnMap": {
                "0": {"name": "foo", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 3, "column": 0}}}
            },
            "branchMap": {},
            "s": {},
            "f": {"0": 1},
            "b": {}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("complexity=\"0\""), "method must carry complexity=\"0\"; got:\n{out}");
}

#[test]
fn no_missing_branches_element() {
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
    assert!(
        !out.contains("missing-branches"),
        "Azure DevOps rejects <missing-branches>; got:\n{out}"
    );
}

#[test]
fn xml_declaration_and_dtd_present() {
    let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
    let out = render(json);
    assert!(out.starts_with("<?xml version=\"1.0\" ?>"));
    assert!(out.contains("<!DOCTYPE coverage"));
}

#[test]
fn condition_coverage_attribute_on_branched_line() {
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
    assert!(out.contains("branch=\"true\" condition-coverage=\"50% (1/2)\""), "got:\n{out}");
}

#[test]
fn xml_illegal_control_chars_are_replaced() {
    // XML 1.0 forbids most control characters in document content;
    // strict parsers (Azure DevOps, libxml2 strict, Saxon) reject the
    // document when they appear. Verify NUL and `\x1F` are sanitized
    // out of attribute values rather than passed through unmodified.
    let json = "{
        \"a.js\": {
            \"path\": \"a.js\",
            \"statementMap\": {},
            \"fnMap\": {
                \"0\": {\"name\": \"foo\\u0000bar\\u001fbaz\", \"line\": 1, \"decl\": {\"start\": {\"line\": 1, \"column\": 0}, \"end\": {\"line\": 1, \"column\": 3}}, \"loc\": {\"start\": {\"line\": 1, \"column\": 0}, \"end\": {\"line\": 3, \"column\": 0}}}
            },
            \"branchMap\": {},
            \"s\": {},
            \"f\": {\"0\": 1},
            \"b\": {}
        }
    }";
    let out = render(json);
    assert!(!out.contains('\u{0000}'), "NUL must not survive into output");
    assert!(!out.contains('\u{001F}'), "control char must not survive");
    // Both should be replaced by U+FFFD (the Unicode replacement char).
    assert!(out.contains('\u{FFFD}'), "expected replacement char in output:\n{out}");
}

#[test]
fn complexity_attribute_present_on_class_and_package() {
    // Cobertura 0.4 DTD declares `complexity` as #REQUIRED on <coverage>,
    // <package>, <class>, and <method>. xmllint --valid rejects the
    // document if any are missing.
    let json = r#"{
        "src/a.js": {
            "path": "src/a.js",
            "statementMap": {"0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}},
            "fnMap": {},
            "branchMap": {},
            "s": {"0": 1},
            "f": {},
            "b": {}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("<package "), "expected <package> tag in:\n{out}");
    assert!(out.contains("<class "), "expected <class> tag in:\n{out}");
    // One each on <coverage>, <package> and <class>; the fixture has no
    // methods.
    let count = out.matches("complexity=\"0\"").count();
    assert!(
        count >= 3,
        "expected complexity=\"0\" on coverage, package, class; got {count}:\n{out}"
    );
}

#[test]
fn attribute_values_are_xml_escaped() {
    let json = r#"{
        "a.js": {
            "path": "a.js",
            "statementMap": {},
            "fnMap": {
                "0": {"name": "foo<bar>&baz", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 3, "column": 0}}}
            },
            "branchMap": {},
            "s": {},
            "f": {"0": 1},
            "b": {}
        }
    }"#;
    let out = render(json);
    assert!(out.contains("name=\"foo&lt;bar&gt;&amp;baz\""), "got:\n{out}");
}
