//! End-to-end snapshot fixtures for every reporter on a shared two-file map.
//!
//! Each snapshot locks the byte-for-byte rendering. Updating an output format
//! intentionally requires accepting the snapshot via `cargo insta review`.

use oxc_coverage_report::summarize;
use oxc_coverage_reports::{Format, json_summary, text, text_summary};
use oxc_coverage_types::parse_coverage_map;

const FIXTURE: &str = r#"{
  "src/a.js": {
    "path": "src/a.js",
    "statementMap": {
      "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
      "1": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}},
      "2": {"start": {"line": 4, "column": 0}, "end": {"line": 4, "column": 5}}
    },
    "fnMap": {
      "0": {"name": "foo", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 5, "column": 0}}}
    },
    "branchMap": {
      "0": {"loc": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}}, "line": 2, "type": "if", "locations": [{"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 2}}, {"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 5}}]}
    },
    "s": {"0": 1, "1": 1, "2": 0},
    "f": {"0": 1},
    "b": {"0": [1, 0]}
  },
  "src/b.js": {
    "path": "src/b.js",
    "statementMap": {
      "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
      "1": {"start": {"line": 3, "column": 0}, "end": {"line": 3, "column": 5}}
    },
    "fnMap": {},
    "branchMap": {},
    "s": {"0": 0, "1": 0},
    "f": {},
    "b": {}
  }
}"#;

fn render(format: Format) -> String {
    let map = parse_coverage_map(FIXTURE).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    format.write(&root, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn text_snapshot() {
    let out = render(Format::Text);
    insta::assert_snapshot!(out);
}

#[test]
fn text_summary_snapshot() {
    let out = render(Format::TextSummary);
    insta::assert_snapshot!(out);
}

#[test]
fn json_summary_snapshot() {
    let map = parse_coverage_map(FIXTURE).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    json_summary::write_pretty(&root, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    insta::assert_snapshot!(out);
}

#[test]
fn format_dispatch_matches_module_writers() {
    let map = parse_coverage_map(FIXTURE).unwrap();
    let root = summarize(&map);

    let mut a = Vec::new();
    text::write(&root, &mut a).unwrap();
    let mut b = Vec::new();
    Format::Text.write(&root, &mut b).unwrap();
    assert_eq!(a, b);

    let mut a = Vec::new();
    text_summary::write(&root, &mut a).unwrap();
    let mut b = Vec::new();
    Format::TextSummary.write(&root, &mut b).unwrap();
    assert_eq!(a, b);

    let mut a = Vec::new();
    json_summary::write(&root, &mut a).unwrap();
    let mut b = Vec::new();
    Format::JsonSummary.write(&root, &mut b).unwrap();
    assert_eq!(a, b);
}

#[test]
fn unknown_format_returns_none() {
    assert_eq!(Format::parse("html"), None);
    assert_eq!(Format::parse("lcov"), None);
    assert_eq!(Format::parse(""), None);
}
