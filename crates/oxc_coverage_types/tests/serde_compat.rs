//! Serde compatibility with the `null` shapes Istanbul producers emit, and the
//! hand-written `Position` serialization.

use oxc_coverage_types::{FileCoverage, Location, Position, parse_coverage_map};

/// A `FileCoverage` whose nullable fields all carry `null`.
const NULL_HEAVY: &str = r#"{
    "path": null,
    "statementMap": {"0": {"start": {"line": null, "column": null}, "end": {}}},
    "fnMap": {"0": {"name": null, "line": null, "decl": {"start": {}, "end": {}}, "loc": {"start": {}, "end": {}}}},
    "branchMap": {"0": {"loc": {"start": {}, "end": {}}, "line": null, "type": null, "locations": []}},
    "s": {"0": null},
    "f": {"0": null},
    "b": {"0": null}
}"#;

#[test]
fn null_string_fields_deserialize_as_empty() {
    let coverage = FileCoverage::from_json(NULL_HEAVY).expect("null-heavy coverage parses");

    assert_eq!(coverage.path, "");
    assert_eq!(coverage.fn_map["0"].name, "");
    assert_eq!(coverage.branch_map["0"].branch_type, "");
}

#[test]
fn null_positions_deserialize_as_zero() {
    let coverage = FileCoverage::from_json(NULL_HEAVY).expect("null-heavy coverage parses");
    let statement = &coverage.statement_map["0"];

    assert_eq!((statement.start.line, statement.start.column), (0, 0));
    assert_eq!((statement.end.line, statement.end.column), (0, 0));
    assert_eq!(coverage.fn_map["0"].line, 0);
    assert_eq!(coverage.branch_map["0"].line, 0);
}

#[test]
fn null_scalar_counters_deserialize_as_zero() {
    let coverage = FileCoverage::from_json(NULL_HEAVY).expect("null-heavy coverage parses");

    assert_eq!(coverage.s["0"], 0);
    assert_eq!(coverage.f["0"], 0);
}

#[test]
fn null_branch_counter_array_deserializes_as_empty_vec() {
    let coverage = FileCoverage::from_json(NULL_HEAVY).expect("null-heavy coverage parses");

    assert_eq!(coverage.b["0"], Vec::<u32>::new());
}

#[test]
fn null_branch_counter_element_deserializes_as_zero() {
    let json = r#"{
        "path": "a.js",
        "statementMap": {}, "fnMap": {},
        "branchMap": {"0": {"loc": {"start": {}, "end": {}}, "line": 1, "type": "if", "locations": []}},
        "s": {}, "f": {},
        "b": {"0": [1, null, 3]}
    }"#;

    let coverage = FileCoverage::from_json(json).expect("branch counters parse");

    assert_eq!(coverage.b["0"], vec![1, 0, 3]);
}

#[test]
fn absent_and_null_branch_truthy_map_both_deserialize_as_none() {
    let absent = r#"{"path": "a.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}}"#;
    let null = r#"{"path": "a.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}, "bT": null}"#;

    assert!(FileCoverage::from_json(absent).expect("absent bT parses").b_t.is_none());
    assert!(FileCoverage::from_json(null).expect("null bT parses").b_t.is_none());
}

#[test]
fn present_branch_truthy_map_applies_both_null_coercions() {
    let json = r#"{
        "path": "a.js",
        "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {},
        "bT": {"0": null, "1": [null, 2]}
    }"#;

    let b_t = FileCoverage::from_json(json).expect("bT parses").b_t.expect("bT is present");

    assert_eq!(b_t["0"], Vec::<u32>::new());
    assert_eq!(b_t["1"], vec![0, 2]);
}

#[test]
fn unknown_position_serializes_as_empty_object() {
    let position = Position { line: 0, column: 0 };

    assert_eq!(serde_json::to_string(&position).expect("position serializes"), "{}");
}

#[test]
fn line_zero_with_non_zero_column_serializes_both_fields() {
    let position = Position { line: 0, column: 4 };

    assert_eq!(
        serde_json::to_string(&position).expect("position serializes"),
        r#"{"line":0,"column":4}"#
    );
}

#[test]
fn location_of_two_unknown_positions_round_trips_byte_identically() {
    let json = r#"{"start":{},"end":{}}"#;

    let location: Location = serde_json::from_str(json).expect("location parses");

    assert_eq!(serde_json::to_string(&location).expect("location serializes"), json);
}

#[test]
fn coverage_map_parses_every_file_entry() {
    let json = r#"{
        "a.js": {"path": "a.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}},
        "b.js": {"path": "b.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}}
    }"#;

    let map = parse_coverage_map(json).expect("coverage map parses");

    assert_eq!(map.keys().collect::<Vec<_>>(), vec!["a.js", "b.js"]);
}

#[test]
fn coverage_map_missing_a_required_key_is_an_error() {
    let json = r#"{"a.js": {"path": "a.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}}}"#;

    assert!(parse_coverage_map(json).is_err());
}
