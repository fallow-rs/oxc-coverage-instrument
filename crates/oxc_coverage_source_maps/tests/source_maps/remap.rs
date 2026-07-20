//! The single-file and coverage-map remap entry points, plus the loader
//! fallback for entries with no embedded `inputSourceMap`.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{
    RemapOptions, remap_coverage, remap_coverage_map_with_loader, remap_coverage_with_loader,
    remap_coverage_with_options,
};
use oxc_coverage_types::{Location, Position};

use crate::fixtures::{
    SRC_PATH, full_shape_file_coverage, identity_three_line_map, single_statement_coverage,
};

#[test]
fn empty_single_source_coverage_still_remaps() {
    let mut empty = full_shape_file_coverage(Some(identity_three_line_map(None)));
    empty.statement_map.clear();
    empty.fn_map.clear();
    empty.branch_map.clear();
    empty.s.clear();
    empty.f.clear();
    empty.b.clear();
    assert!(remap_coverage(&empty).is_some(), "empty single-source coverage still remaps");
}

#[test]
fn fully_pruned_single_source_coverage_returns_its_source_file() {
    let fully_unmapped = single_statement_coverage(identity_three_line_map(None), 4, 0, 4, 5);
    let remapped =
        remap_coverage_with_options(&fully_unmapped, RemapOptions { drop_unmapped: true })
            .expect("fully pruned single-source coverage still returns its source file");
    assert!(remapped.statement_map.is_empty());
    assert!(remapped.s.is_empty());
}

#[test]
fn remap_coverage_rewrites_fn_and_branch_entries() {
    let fc = full_shape_file_coverage(Some(identity_three_line_map(None)));
    let remapped = remap_coverage(&fc).expect("identity remap succeeds");

    assert_eq!(remapped.path, SRC_PATH);
    assert!(remapped.input_source_map.is_none(), "consumed map is cleared");

    let fn0 = remapped.fn_map.get("0").expect("fnMap entry survives remap");
    assert_eq!(fn0.loc.start.line, 2, "function loc keeps its original line");
    assert_eq!(fn0.line, 2, "FnEntry.line tracks loc.start.line after remap");
    assert_eq!(fn0.decl.start.line, 2);

    let br0 = remapped.branch_map.get("0").expect("branchMap entry survives remap");
    assert_eq!(br0.loc.start.line, 3, "branch loc keeps its original line");
    assert_eq!(br0.line, 3, "BranchEntry.line tracks loc.start.line after remap");
    assert_eq!(br0.locations.len(), 2, "both arm locations survive");
    assert_eq!(br0.locations[0].start.line, 3, "arm 0 maps back to line 3");
}

#[test]
fn loader_fallback_supplies_external_source_map() {
    let mut fc = full_shape_file_coverage(None);
    fc.path = "intermediate.js".to_string();
    let map_json = identity_three_line_map(None).to_string();

    let remapped = remap_coverage_with_loader(&fc, |path| {
        assert_eq!(path, "intermediate.js", "loader is called with FileCoverage.path");
        Some(map_json.clone())
    })
    .expect("loader-supplied map should drive a successful remap");
    assert_eq!(remapped.path, SRC_PATH);
}

#[test]
fn loader_returning_none_falls_back_to_none() {
    let fc = full_shape_file_coverage(None);
    assert!(remap_coverage(&fc).is_none(), "no embedded map and no loader means no remap");
    assert!(
        remap_coverage_with_loader(&fc, |_| None).is_none(),
        "loader that yields None must not invent a map",
    );
}

#[test]
fn remap_coverage_map_with_loader_preserves_unmapped_entries() {
    let with_map = full_shape_file_coverage(Some(identity_three_line_map(None)));
    let without_map = full_shape_file_coverage(None);

    let mut coverage_map = BTreeMap::new();
    coverage_map.insert("with-map.js".to_string(), with_map);
    coverage_map.insert("no-map.js".to_string(), without_map);

    let out = remap_coverage_map_with_loader(&coverage_map, |_| None);

    assert!(out.contains_key(SRC_PATH), "successfully remapped entry is rekeyed by source path");
    assert!(
        out.contains_key("no-map.js"),
        "entry without a source map and no loader fallback passes through unchanged",
    );
}

#[test]
fn empty_sources_array_blocks_remap() {
    let empty_sources: serde_json::Value =
        serde_json::from_str(r#"{"version":3,"sources":[""],"mappings":"","names":[]}"#).unwrap();
    let fc = full_shape_file_coverage(Some(empty_sources));
    assert!(
        remap_coverage(&fc).is_none(),
        "empty `sources[0]` should bail rather than invent a path",
    );
}

#[test]
fn source_root_joins_with_separator_when_missing() {
    // `sourceRoot` without trailing slash plus a relative first source: the
    // helper must insert `/` between them so the resulting path matches what
    // istanbul-lib-source-maps would have produced.
    let map = identity_three_line_map(Some("dist"));
    let fc = full_shape_file_coverage(Some(map));
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    assert_eq!(remapped.path, format!("dist/{SRC_PATH}"));
}

#[test]
fn remap_skips_positions_with_unknown_line() {
    // A `Position { line: 0, column: _ }` signals "unknown" in Istanbul; the
    // remap helper must leave those untouched rather than feed `-1` into the
    // source-map lookup.
    let mut fc = full_shape_file_coverage(Some(identity_three_line_map(None)));
    fc.statement_map.insert(
        "blank".to_string(),
        Location { start: Position { line: 0, column: 0 }, end: Position { line: 0, column: 0 } },
    );
    fc.s.insert("blank".to_string(), 0);

    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let blank = &remapped.statement_map["blank"];
    assert_eq!(blank.start.line, 0, "unknown line stays unknown");
    assert_eq!(blank.end.line, 0);
}
