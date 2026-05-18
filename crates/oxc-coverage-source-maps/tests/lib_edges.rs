//! Direct unit-level coverage for the `oxc_coverage_source_maps` API.
//!
//! The integration suite in `oxc-coverage-instrument/tests/source_maps_test.rs`
//! drives the helpers via real instrumenter output, which always produces a
//! `FileCoverage` with the same shape: populated statementMap, empty fnMap and
//! branchMap (for the trivial fixtures it instruments). That leaves
//! `remap_fn_entry` / `remap_branch_entry` / the loader fallback path / the
//! `SourceMapStore` hit branches uncovered. This file exercises each of those
//! through the public surface so they survive future refactors.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{
    SourceMapStore, remap_coverage, remap_coverage_map_with_loader, remap_coverage_with_loader,
};
use oxc_coverage_types::{
    BranchEntry, FileCoverage, FnEntry, Location, Position, parse_coverage_map,
};

const SRC_PATH: &str = "src/app.ts";

/// `AAAA;AACA;AACA` is the canonical three-line identity map: line 1 maps to
/// line 1 of `src/app.ts`, line 2 to line 2, line 3 to line 3. The
/// `srcmap-sourcemap` parser expects the names array even when empty.
fn identity_three_line_map(source_root: Option<&str>) -> serde_json::Value {
    let root = source_root.map_or_else(String::new, |r| format!(r#","sourceRoot":"{r}""#));
    serde_json::from_str(&format!(
        r#"{{"version":3,"sources":["{SRC_PATH}"],"mappings":"AAAA;AACA;AACA","names":[]{root}}}"#,
    ))
    .unwrap()
}

/// Build a `FileCoverage` for `intermediate.js` whose statement / function /
/// branch maps each reference the same three identity-mapped lines. Hit-count
/// vectors are seeded with `0` so the remapped output round-trips a valid
/// shape.
fn full_shape_file_coverage(input_source_map: Option<serde_json::Value>) -> FileCoverage {
    let pos = |line: u32, col: u32| Position { line, column: col };
    let loc =
        |sl: u32, sc: u32, el: u32, ec: u32| Location { start: pos(sl, sc), end: pos(el, ec) };
    let mut statement_map = BTreeMap::new();
    statement_map.insert("0".to_string(), loc(1, 0, 1, 10));
    let mut fn_map = BTreeMap::new();
    fn_map.insert(
        "0".to_string(),
        FnEntry { name: "f".to_string(), line: 2, decl: loc(2, 0, 2, 1), loc: loc(2, 0, 2, 10) },
    );
    let mut branch_map = BTreeMap::new();
    branch_map.insert(
        "0".to_string(),
        BranchEntry {
            loc: loc(3, 0, 3, 8),
            line: 3,
            branch_type: "if".to_string(),
            locations: vec![loc(3, 4, 3, 5), loc(3, 6, 3, 7)],
        },
    );
    let mut s = BTreeMap::new();
    s.insert("0".to_string(), 0);
    let mut f = BTreeMap::new();
    f.insert("0".to_string(), 0);
    let mut b = BTreeMap::new();
    b.insert("0".to_string(), vec![0, 0]);

    FileCoverage {
        path: "intermediate.js".to_string(),
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t: None,
        input_source_map,
    }
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
    // Defaults to `remap_coverage`-style behaviour (matches `|_| None`).
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
fn source_map_store_hit_takes_precedence_over_embedded_map() {
    // Embedded map points the file to `wrong.ts`; the store's map points it to
    // `src/app.ts`. The store entry must win.
    let wrong_map: serde_json::Value = serde_json::from_str(
        r#"{"version":3,"sources":["wrong.ts"],"mappings":"AAAA","names":[]}"#,
    )
    .unwrap();
    let fc = full_shape_file_coverage(Some(wrong_map));

    let mut store = SourceMapStore::new();
    assert!(store.is_empty());
    store.add_map(fc.path.clone(), identity_three_line_map(None));
    assert!(!store.is_empty());
    assert_eq!(store.len(), 1);
    assert!(store.contains(&fc.path));

    let remapped =
        store.transform_coverage(&fc).expect("store entry must drive a successful remap");
    assert_eq!(remapped.path, SRC_PATH, "store entry must override the embedded inputSourceMap");
}

#[test]
fn source_map_store_transform_coverage_map_routes_both_branches() {
    let store_hit = full_shape_file_coverage(None);
    let store_miss = full_shape_file_coverage(None);

    let mut coverage_map = BTreeMap::new();
    coverage_map.insert("intermediate.js".to_string(), store_hit);
    coverage_map.insert("uncovered.js".to_string(), {
        let mut fc = store_miss;
        fc.path = "uncovered.js".to_string();
        fc
    });

    let mut store = SourceMapStore::new();
    store.add_map("intermediate.js", identity_three_line_map(None));

    let out = store.transform_coverage_map(&coverage_map);
    assert!(out.contains_key(SRC_PATH), "hit branch rekeys by source path");
    assert!(out.contains_key("uncovered.js"), "miss branch keeps the original key");
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

#[test]
fn parse_coverage_map_round_trips_through_remap() {
    // Sanity: the `oxc_coverage_types::parse_coverage_map` JSON entry point is
    // the typical caller path. Make sure a full-shape JSON survives a remap
    // round trip with a store-supplied map.
    let json = r#"{
        "intermediate.js": {
            "path": "intermediate.js",
            "statementMap": {"0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}},
            "fnMap": {},
            "branchMap": {},
            "s": {"0": 1},
            "f": {},
            "b": {}
        }
    }"#;
    let map = parse_coverage_map(json).unwrap();
    let fc = &map["intermediate.js"];

    let mut store = SourceMapStore::new();
    store.add_map("intermediate.js", identity_three_line_map(None));
    let remapped = store.transform_coverage(fc).expect("store-driven remap");
    assert_eq!(remapped.path, SRC_PATH);
}
