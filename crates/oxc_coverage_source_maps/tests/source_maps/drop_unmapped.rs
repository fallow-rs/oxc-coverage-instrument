//! `RemapOptions::drop_unmapped` pruning of entries the source map cannot
//! resolve, and the counter realignment that follows it.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{
    RemapOptions, SourceMapStore, remap_coverage, remap_coverage_map_with_options,
    remap_coverage_with_options,
};
use oxc_coverage_types::{FunctionIdentity, Location, Position};

use crate::fixtures::{SRC_PATH, loc, mixed_mapped_file_coverage, one_line_identity_map};

#[test]
fn drop_unmapped_default_keeps_generated_positions() {
    // Without `drop_unmapped`, unmapped positions keep their generated-output
    // coordinates and nothing is pruned.
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
    // The overlay shares the fn-id keyspace, so it has to stay 1:1 with `fnMap`.
    // Otherwise a consumer joining on `fnMap` keys finds an orphan identity for
    // a function that no longer exists.
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
fn drop_unmapped_falls_back_to_first_mapped_arm_when_outer_loc_fails() {
    let fc = mixed_mapped_file_coverage();
    let opts = RemapOptions { drop_unmapped: true };
    let embedded = remap_coverage_with_options(&fc, opts).expect("embedded remap succeeds");

    let mut stored_fc = fc;
    let map = stored_fc.input_source_map.take().expect("fixture has an embedded map");
    let mut store = SourceMapStore::new();
    store.add_map(stored_fc.path.clone(), &map);
    let stored =
        store.transform_coverage_with_options(&stored_fc, opts).expect("stored remap succeeds");

    for remapped in [&embedded, &stored] {
        let branch =
            remapped.branch_map.get("drop_outer").expect("mapped arms preserve the branch");
        let first_arm = &branch.locations[0];
        assert_eq!(branch.loc.start.line, first_arm.start.line);
        assert_eq!(branch.loc.start.column, first_arm.start.column);
        assert_eq!(branch.loc.end.line, first_arm.end.line);
        assert_eq!(branch.loc.end.column, first_arm.end.column);
        assert_eq!(branch.line, first_arm.start.line);
        assert_eq!(remapped.b["drop_outer"], vec![8, 9]);
        assert_eq!(remapped.b_t.as_ref().expect("bT")["drop_outer"], vec![14, 15]);
    }
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
    assert_eq!(entry.fn_map.keys().collect::<Vec<_>>(), vec!["0"]);
    assert_eq!(entry.f.keys().collect::<Vec<_>>(), vec!["0"]);
}

#[test]
fn drop_unmapped_applies_through_store() {
    let fc = mixed_mapped_file_coverage();
    let mut store = SourceMapStore::new();
    store.add_map("intermediate.js", &one_line_identity_map());

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
