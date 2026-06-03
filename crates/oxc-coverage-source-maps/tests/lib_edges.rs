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
    RemapOptions, SourceMapStore, remap_coverage, remap_coverage_map_with_loader,
    remap_coverage_map_with_options, remap_coverage_with_loader, remap_coverage_with_options,
};
use oxc_coverage_types::{
    BranchEntry, FileCoverage, FnEntry, FunctionIdentity, Location, Position, parse_coverage_map,
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
        x_fallow_function_map: None,
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

/// Single-line identity map: line 1 of the generated file maps to line 1 of
/// `src/app.ts`. Positions on line 2+ have no mapping and
/// `original_position_for` returns `None` for them, which is the trigger we
/// need to exercise `RemapOptions::drop_unmapped`.
fn one_line_identity_map() -> serde_json::Value {
    serde_json::from_str(&format!(
        r#"{{"version":3,"sources":["{SRC_PATH}"],"mappings":"AAAA","names":[]}}"#,
    ))
    .unwrap()
}

/// `FileCoverage` whose statement / function / branch entries straddle a
/// "mapped line 1 / unmapped line 2+" boundary so the `drop_unmapped` tests
/// can assert which entries survive and which get pruned.
///
/// Statement `keep` sits at line 1 (mapped), `drop` at line 2 (unmapped).
/// Function `keep` has both `decl` and `loc` on line 1; function `drop` has
/// `decl` on line 1 but `loc` on line 2, exercising the "any of decl/loc
/// fails" rule. Branch `keep` has its umbrella `loc` on line 1 with two arms,
/// one on line 1 (kept) and one on line 2 (pruned); branch `drop_no_arms`
/// has its umbrella `loc` on line 1 but every arm on line 2 (drop because
/// no arms survive); branch `drop_outer` has its umbrella `loc` on line 2
/// (drop even though arms map).
fn mixed_mapped_file_coverage() -> FileCoverage {
    let pos = |line: u32, col: u32| Position { line, column: col };
    let loc =
        |sl: u32, sc: u32, el: u32, ec: u32| Location { start: pos(sl, sc), end: pos(el, ec) };

    let mut statement_map = BTreeMap::new();
    statement_map.insert("keep".to_string(), loc(1, 0, 1, 10));
    statement_map.insert("drop".to_string(), loc(2, 0, 2, 10));

    let mut fn_map = BTreeMap::new();
    fn_map.insert(
        "keep".to_string(),
        FnEntry { name: "k".to_string(), line: 1, decl: loc(1, 0, 1, 1), loc: loc(1, 0, 1, 10) },
    );
    fn_map.insert(
        "drop".to_string(),
        FnEntry { name: "d".to_string(), line: 1, decl: loc(1, 0, 1, 1), loc: loc(2, 0, 2, 10) },
    );

    let mut branch_map = BTreeMap::new();
    branch_map.insert(
        "keep".to_string(),
        BranchEntry {
            loc: loc(1, 0, 1, 8),
            line: 1,
            branch_type: "if".to_string(),
            // arm 0 maps (line 1); arm 1 does not (line 2).
            locations: vec![loc(1, 4, 1, 5), loc(2, 6, 2, 7)],
        },
    );
    branch_map.insert(
        "drop_no_arms".to_string(),
        BranchEntry {
            loc: loc(1, 0, 1, 8),
            line: 1,
            branch_type: "if".to_string(),
            // every arm unmapped.
            locations: vec![loc(2, 4, 2, 5), loc(2, 6, 2, 7)],
        },
    );
    branch_map.insert(
        "drop_outer".to_string(),
        BranchEntry {
            loc: loc(2, 0, 2, 8),
            line: 2,
            branch_type: "if".to_string(),
            // arms are technically mappable, but umbrella loc is not.
            locations: vec![loc(1, 4, 1, 5), loc(1, 6, 1, 7)],
        },
    );

    let mut s = BTreeMap::new();
    s.insert("keep".to_string(), 7);
    s.insert("drop".to_string(), 13);
    let mut f = BTreeMap::new();
    f.insert("keep".to_string(), 2);
    f.insert("drop".to_string(), 3);
    let mut b = BTreeMap::new();
    b.insert("keep".to_string(), vec![4, 5]);
    b.insert("drop_no_arms".to_string(), vec![6, 7]);
    b.insert("drop_outer".to_string(), vec![8, 9]);
    let mut b_t = BTreeMap::new();
    b_t.insert("keep".to_string(), vec![10, 11]);
    b_t.insert("drop_no_arms".to_string(), vec![12, 13]);
    b_t.insert("drop_outer".to_string(), vec![14, 15]);

    FileCoverage {
        path: "intermediate.js".to_string(),
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t: Some(b_t),
        input_source_map: Some(one_line_identity_map()),
        x_fallow_function_map: None,
    }
}

#[test]
fn drop_unmapped_default_keeps_generated_positions() {
    // Default options preserve current behaviour: nothing gets dropped, and
    // unmapped positions keep their generated-output coordinates.
    let fc = mixed_mapped_file_coverage();
    let remapped =
        remap_coverage(&fc).expect("remap with embedded map and default options succeeds");

    assert_eq!(remapped.statement_map.len(), 2, "default keeps both statements");
    assert_eq!(remapped.fn_map.len(), 2, "default keeps both functions");
    assert_eq!(remapped.branch_map.len(), 3, "default keeps all branches");
    assert_eq!(remapped.branch_map["keep"].locations.len(), 2, "default keeps both arms");
}

#[test]
fn drop_unmapped_prunes_statements_and_aligned_counters() {
    let fc = mixed_mapped_file_coverage();
    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_with_options(&fc, opts).expect("drop_unmapped remap succeeds");

    assert!(remapped.statement_map.contains_key("keep"));
    assert!(!remapped.statement_map.contains_key("drop"));
    assert!(remapped.s.contains_key("keep"), "matching `s` slot survives");
    assert_eq!(remapped.s["keep"], 7);
    assert!(!remapped.s.contains_key("drop"), "matching `s` slot drops with the statement");
}

#[test]
fn drop_unmapped_prunes_functions_when_decl_or_loc_fails() {
    let fc = mixed_mapped_file_coverage();
    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_with_options(&fc, opts).expect("drop_unmapped remap succeeds");

    assert!(remapped.fn_map.contains_key("keep"));
    assert!(!remapped.fn_map.contains_key("drop"), "function with unmapped loc drops");
    assert!(remapped.f.contains_key("keep"));
    assert!(!remapped.f.contains_key("drop"));
    let kept = &remapped.fn_map["keep"];
    assert_eq!(kept.line, kept.loc.start.line, "FnEntry.line tracks loc.start.line after remap");
}

#[test]
fn drop_unmapped_prunes_function_identity_overlay_with_its_function() {
    // When a function is dropped, its `x_fallow_functionMap` overlay entry (if
    // present, sharing the fn-id keyspace) must drop too, so the overlay stays
    // 1:1 with `fnMap`. Otherwise a consumer joining on `fnMap` keys finds an
    // orphan identity for a function that no longer exists.
    let pos = |line: u32, col: u32| Position { line, column: col };
    let loc =
        |sl: u32, sc: u32, el: u32, ec: u32| Location { start: pos(sl, sc), end: pos(el, ec) };
    let identity = |id: &str, name: &str, l: u32| FunctionIdentity {
        id: id.to_string(),
        name: name.to_string(),
        path: "src/app.ts".to_string(),
        decl: loc(l, 0, l, 1),
        loc: loc(l, 0, l, 10),
    };

    let mut fc = mixed_mapped_file_coverage();
    let mut overlay = BTreeMap::new();
    overlay.insert("keep".to_string(), identity("fallow:fn:keep", "k", 1));
    overlay.insert("drop".to_string(), identity("fallow:fn:drop", "d", 1));
    fc.x_fallow_function_map = Some(overlay);

    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_with_options(&fc, opts).expect("drop_unmapped remap succeeds");

    let overlay = remapped.x_fallow_function_map.expect("overlay survives drop_unmapped");
    assert!(overlay.contains_key("keep"), "kept function retains its overlay entry");
    assert!(!overlay.contains_key("drop"), "dropped function's overlay entry is pruned");
    assert_eq!(
        overlay.keys().collect::<Vec<_>>(),
        remapped.fn_map.keys().collect::<Vec<_>>(),
        "overlay stays 1:1 with fnMap after drop",
    );
}

#[test]
fn drop_unmapped_prunes_branch_arms_and_realigns_counters() {
    let fc = mixed_mapped_file_coverage();
    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_with_options(&fc, opts).expect("drop_unmapped remap succeeds");

    // Per-arm prune: arm 0 (line 1) survives, arm 1 (line 2) drops.
    let kept =
        remapped.branch_map.get("keep").expect("branch with at least one mapped arm survives");
    assert_eq!(kept.locations.len(), 1, "only the mapped arm survives");
    assert_eq!(kept.locations[0].start.line, 1, "the surviving arm maps to line 1");

    // Counter vectors realign so positions still line up with the kept arms.
    assert_eq!(remapped.b["keep"], vec![4], "b realigns to the kept arm");
    let b_t = remapped.b_t.as_ref().expect("bT survives when drop preserves the branch");
    assert_eq!(b_t["keep"], vec![10], "bT realigns to the kept arm");
}

#[test]
fn drop_unmapped_drops_whole_branch_when_no_arms_survive() {
    let fc = mixed_mapped_file_coverage();
    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_with_options(&fc, opts).expect("drop_unmapped remap succeeds");

    assert!(
        !remapped.branch_map.contains_key("drop_no_arms"),
        "branch drops when every arm fails to remap",
    );
    assert!(!remapped.b.contains_key("drop_no_arms"), "matching `b` slot drops with the branch");
    let b_t = remapped.b_t.as_ref().expect("bT exists for the surviving branches");
    assert!(!b_t.contains_key("drop_no_arms"), "matching `bT` slot drops with the branch");
}

#[test]
fn drop_unmapped_drops_branch_when_outer_loc_fails() {
    let fc = mixed_mapped_file_coverage();
    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_with_options(&fc, opts).expect("drop_unmapped remap succeeds");

    assert!(
        !remapped.branch_map.contains_key("drop_outer"),
        "branch drops when the umbrella `loc` fails to remap, even if arms would map",
    );
    assert!(!remapped.b.contains_key("drop_outer"));
    let b_t = remapped.b_t.as_ref().expect("bT exists for the surviving branches");
    assert!(!b_t.contains_key("drop_outer"));
}

#[test]
fn drop_unmapped_keeps_unknown_line_zero_positions() {
    // Istanbul uses `line: 0` as the "unknown" sentinel; the drop path must
    // treat it as a successful no-op rather than pruning it.
    let mut fc = mixed_mapped_file_coverage();
    fc.statement_map.insert(
        "blank".to_string(),
        Location { start: Position { line: 0, column: 0 }, end: Position { line: 0, column: 0 } },
    );
    fc.s.insert("blank".to_string(), 0);

    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_with_options(&fc, opts).expect("drop_unmapped remap succeeds");
    assert!(
        remapped.statement_map.contains_key("blank"),
        "`line: 0` sentinel positions survive `drop_unmapped`",
    );
}

#[test]
fn drop_unmapped_applies_to_coverage_map_helper() {
    let fc = mixed_mapped_file_coverage();
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert("intermediate.js".to_string(), fc);

    let opts = RemapOptions { drop_unmapped: true };
    let remapped = remap_coverage_map_with_options(&coverage_map, opts);
    let entry = remapped.get(SRC_PATH).expect("entry is rekeyed by source path");
    assert_eq!(entry.statement_map.len(), 1, "drop_unmapped flows through coverage-map helper");
    assert!(entry.fn_map.contains_key("keep"));
    assert!(!entry.fn_map.contains_key("drop"));
}

#[test]
fn drop_unmapped_applies_through_store() {
    let fc = mixed_mapped_file_coverage();
    let mut store = SourceMapStore::new();
    store.add_map("intermediate.js", one_line_identity_map());

    let opts = RemapOptions { drop_unmapped: true };
    let remapped = store
        .transform_coverage_with_options(&fc, opts)
        .expect("store-driven drop_unmapped remap succeeds");
    assert_eq!(remapped.statement_map.len(), 1);

    let mut coverage_map = BTreeMap::new();
    coverage_map.insert("intermediate.js".to_string(), fc);
    let remapped_map = store.transform_coverage_map_with_options(&coverage_map, opts);
    let entry = remapped_map.get(SRC_PATH).expect("store map-level helper rekeys by source path");
    assert_eq!(entry.statement_map.len(), 1);
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

// Issue #107: a coverage object can reach the remap pipeline already carrying an
// orphan counter, an `s`/`f`/`b` key whose location-map entry is absent. This
// happens when an upstream instrumenter emitted `++cov.s[id]` for a slot that
// was later pruned from the map: at runtime `undefined + 1 = NaN`, which
// serializes back as `null` and (via `deserialize_null_as_zero_map`) reappears
// as an orphan `s` key. Passing it through unchanged crashes
// `istanbul-lib-coverage`'s `CoverageMap.merge`. Every remap path must drop it.
#[test]
fn remap_drops_preexisting_orphan_statement_counter() {
    // Mapped statement "0" survives; orphan "1" has an `s` slot but no
    // statementMap entry. The embedded identity map drives a no-drop remap, so
    // nothing here is pruned by mapping failure: the orphan can only be removed
    // by the issue #107 invariant pass.
    let mut fc = full_shape_file_coverage(Some(identity_three_line_map(None)));
    fc.s.insert("1".to_string(), 0);
    assert!(!fc.statement_map.contains_key("1"), "precondition: orphan has no statementMap entry");

    let remapped = remap_coverage(&fc).expect("identity remap succeeds");
    assert!(
        !remapped.s.contains_key("1"),
        "orphan `s` counter must be dropped so istanbul-lib-coverage can merge the result",
    );
    for key in remapped.s.keys() {
        assert!(
            remapped.statement_map.contains_key(key),
            "every surviving `s` key must have a statementMap entry (key {key})",
        );
    }
}

#[test]
fn passthrough_entry_without_map_still_drops_orphan_counter() {
    // An already-composed entry has no embedded `inputSourceMap`, so the
    // map-level remap leaves it under its original key (the `None` branch). It
    // must still be reconciled: a runtime orphan here would otherwise survive a
    // `remapCoverageMap` round-trip untouched.
    let mut fc = full_shape_file_coverage(None);
    fc.f.insert("9".to_string(), 0); // orphan function counter, no fnMap["9"]
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(fc.path.clone(), fc);

    let out = remap_coverage_map_with_loader(&coverage_map, |_| None);
    let entry = out.get("intermediate.js").expect("entry passes through under original key");
    assert!(!entry.f.contains_key("9"), "orphan `f` counter dropped on passthrough");
    for key in entry.f.keys() {
        assert!(entry.fn_map.contains_key(key), "every surviving `f` key has an fnMap entry");
    }
}

#[test]
fn prune_orphan_counters_matches_issue_107_shape() {
    // The exact malformed shape from issue #107: statementMap is missing "1"
    // while `s["1"]` is present (and serialized as `null`, coerced to 0 on
    // ingest). prune_orphan_counters must remove it; the same rule covers
    // f/fnMap, b/branchMap, and bT/branchMap orphans. The x_fallow_functionMap
    // overlay (keyed by fn id) tracks fnMap, so an orphan overlay entry drops too.
    let json = r#"{
        "path": "/x/mod.ts",
        "statementMap": {"0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 12}}},
        "fnMap": {},
        "branchMap": {},
        "s": {"0": 1, "1": null},
        "f": {"2": 3},
        "b": {"4": [1, 0]},
        "bT": {"5": [0]},
        "x_fallow_functionMap": {"2": {"id": "fallow:fn:00000000", "name": "g", "path": "/x/mod.ts", "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 9}}}}
    }"#;
    let mut fc =
        oxc_coverage_types::FileCoverage::from_json(json).expect("parses with null-as-zero");
    assert_eq!(fc.s.get("1"), Some(&0), "null `s` value ingests as 0 before reconciliation");

    let removed = fc.prune_orphan_counters();
    assert_eq!(
        removed, 5,
        "all five orphan entries removed: counters s.1, f.2, b.4, bT.5 plus the overlay entry 2",
    );
    assert_eq!(fc.s.keys().collect::<Vec<_>>(), vec!["0"], "only the mapped statement survives");
    assert!(fc.f.is_empty() && fc.b.is_empty(), "orphan function/branch counters removed");
    assert!(fc.b_t.as_ref().is_none_or(BTreeMap::is_empty), "orphan bT counter removed");
    assert!(
        fc.x_fallow_function_map.as_ref().is_none_or(BTreeMap::is_empty),
        "orphan overlay entry (fn id absent from fnMap) is pruned with its counter",
    );
}
