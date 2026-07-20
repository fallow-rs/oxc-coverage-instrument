//! `SourceMapStore`, the remap-during-collection container.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{RemapOptions, SourceMapStore};
use oxc_coverage_types::parse_coverage_map;

use crate::fixtures::{
    SRC_PATH, full_shape_file_coverage, generated_map, identity_three_line_map,
    mapped_full_shape_coverage,
};

#[test]
fn source_map_store_map_transform_fans_out_and_merges() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map.clone(), 0);
    coverage.input_source_map = None;
    let mut store = SourceMapStore::new();
    store.add_map("dist/bundle.js", &map);
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(coverage.path.clone(), coverage);

    let remapped = store
        .transform_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
    assert!(remapped.contains_key("src/a.ts"));
    assert!(remapped.contains_key("src/b.ts"));
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
    store.add_map(fc.path.clone(), &identity_three_line_map(None));
    assert!(!store.is_empty());
    assert_eq!(store.len(), 1);
    assert!(store.contains(&fc.path));

    let remapped =
        store.transform_coverage(&fc).expect("store entry must drive a successful remap");
    assert_eq!(remapped.path, SRC_PATH, "store entry must override the embedded inputSourceMap");
}

#[test]
fn source_map_store_add_map_json_remaps_without_value_round_trip() {
    let fc = full_shape_file_coverage(None);
    let map_json = identity_three_line_map(None).to_string();
    let mut store = SourceMapStore::new();

    store.add_map_json(fc.path.clone(), &map_json);

    let remapped =
        store.transform_coverage(&fc).expect("JSON string map should drive a successful remap");
    assert_eq!(remapped.path, SRC_PATH);
    assert!(remapped.input_source_map.is_none());
}

#[test]
fn source_map_store_add_map_json_keeps_invalid_maps_unusable() {
    let fc = full_shape_file_coverage(None);
    let mut store = SourceMapStore::new();

    store.add_map_json(fc.path.clone(), "{ not valid source map json");

    assert!(
        store.contains(&fc.path),
        "invalid maps are still registered so later inserts can replace them",
    );
    assert!(
        store.transform_coverage(&fc).is_none(),
        "invalid registered map must not fall back to a generated remap",
    );
}

#[test]
fn store_transform_map_rekeys_hits_and_preserves_misses() {
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
    store.add_map("intermediate.js", &identity_three_line_map(None));

    let out = store.transform_coverage_map(&coverage_map);
    assert!(out.contains_key(SRC_PATH), "hit branch rekeys by source path");
    assert!(out.contains_key("uncovered.js"), "miss branch keeps the original key");
}

#[test]
fn parse_coverage_map_round_trips_through_remap() {
    // `parse_coverage_map` is the typical caller entry point, so a full-shape
    // JSON has to survive a store-driven remap round trip.
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
    store.add_map("intermediate.js", &identity_three_line_map(None));
    let remapped = store.transform_coverage(fc).expect("store-driven remap");
    assert_eq!(remapped.path, SRC_PATH);
}
