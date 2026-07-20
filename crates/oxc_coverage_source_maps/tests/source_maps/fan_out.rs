//! Fanning one generated file out across several original sources.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{
    RemapOptions, SourceMapStore, remap_coverage, remap_coverage_map_with_options,
    remap_coverage_to_map_with_options, remap_coverage_with_options,
};
use oxc_coverage_types::FileCoverage;

use crate::fixtures::{generated_map, loc, mapped_full_shape_coverage};

#[test]
fn map_remap_fans_one_generated_file_out_by_original_source() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(coverage.path.clone(), coverage);

    let remapped =
        remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });

    let a = remapped.get("src/a.ts").expect("source a coverage survives");
    let b = remapped.get("src/b.ts").expect("source b coverage survives");
    assert_eq!(a.statement_map.len(), 1);
    assert_eq!(b.statement_map.len(), 1);
    assert_eq!(a.fn_map.len(), 1);
    assert_eq!(b.fn_map.len(), 1);
    assert_eq!(a.branch_map.len(), 1);
    assert_eq!(b.branch_map.len(), 1);
    assert_eq!(a.b_t.as_ref().expect("bT preserved").len(), 1);
    assert_eq!(b.b_t.as_ref().expect("bT preserved").len(), 1);
    assert_eq!(a.x_fallow_function_map.as_ref().expect("overlay preserved").len(), 1);
    assert_eq!(b.x_fallow_function_map.as_ref().expect("overlay preserved").len(), 1);
}

#[test]
fn multi_source_map_has_no_single_file_result() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);

    assert!(remap_coverage(&coverage).is_none(), "single-result API cannot discard a source");
}

#[test]
fn multi_source_map_api_assigns_contiguous_ids() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);

    let remapped =
        remap_coverage_to_map_with_options(&coverage, RemapOptions { drop_unmapped: true })
            .expect("usable multi-source map");
    for file in remapped.values() {
        assert_eq!(file.statement_map.keys().collect::<Vec<_>>(), vec!["0"]);
        assert_eq!(file.s.keys().collect::<Vec<_>>(), vec!["0"]);
        assert_eq!(file.fn_map.keys().collect::<Vec<_>>(), vec!["0"]);
        assert_eq!(file.f.keys().collect::<Vec<_>>(), vec!["0"]);
        assert_eq!(file.branch_map.keys().collect::<Vec<_>>(), vec!["0"]);
        assert_eq!(file.b.keys().collect::<Vec<_>>(), vec!["0"]);
        assert!(file.input_source_map.is_none());
    }
}

#[test]
fn branch_ownership_falls_back_to_first_retained_arm_when_umbrella_is_unmapped() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 1)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);
    coverage.statement_map.clear();
    coverage.s.clear();
    coverage.fn_map.clear();
    coverage.f.clear();
    coverage.x_fallow_function_map = None;
    coverage.branch_map.retain(|id, _| id == "0");
    coverage.b.retain(|id, _| id == "0");
    coverage.b_t.as_mut().expect("bT").retain(|id, _| id == "0");
    let branch = coverage.branch_map.get_mut("0").expect("branch");
    branch.loc = loc(3, 0, 3, 5);
    branch.locations = vec![loc(1, 0, 1, 2), loc(1, 3, 1, 5)];

    let remapped =
        remap_coverage_to_map_with_options(&coverage, RemapOptions { drop_unmapped: true })
            .expect("usable map");
    let a = &remapped["src/a.ts"];
    let branch = a.branch_map.values().next().expect("branch survives");
    assert_eq!(branch.loc.start.line, branch.locations[0].start.line);
    assert_eq!(branch.loc.start.column, branch.locations[0].start.column);
    assert_eq!(branch.loc.end.line, branch.locations[0].end.line);
    assert_eq!(branch.loc.end.column, branch.locations[0].end.column);
    assert_eq!(branch.line, branch.loc.start.line);
    assert_eq!(a.b.keys().collect::<Vec<_>>(), a.branch_map.keys().collect::<Vec<_>>());
    assert_eq!(
        a.b_t.as_ref().expect("bT").keys().collect::<Vec<_>>(),
        a.branch_map.keys().collect::<Vec<_>>()
    );
}

#[test]
fn branch_ownership_uses_mapped_arms_when_umbrella_maps_to_another_source() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 2)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);
    coverage.statement_map.clear();
    coverage.s.clear();
    coverage.fn_map.clear();
    coverage.f.clear();
    coverage.x_fallow_function_map = None;
    coverage.branch_map.retain(|id, _| id == "0");
    coverage.b.retain(|id, _| id == "0");
    coverage.b_t.as_mut().expect("bT").retain(|id, _| id == "0");
    let branch = coverage.branch_map.get_mut("0").expect("branch");
    branch.loc = loc(2, 0, 2, 5);
    branch.locations = vec![loc(1, 0, 1, 2), loc(1, 3, 1, 5)];

    let remapped =
        remap_coverage_to_map_with_options(&coverage, RemapOptions { drop_unmapped: true })
            .expect("usable map");
    let a = &remapped["src/a.ts"];
    let branch = a.branch_map.values().next().expect("branch survives under arm source");
    assert_eq!(branch.loc.start.line, 3, "mapped umbrella metadata remains available");
    assert_eq!(branch.locations[0].start.line, 1);
}

#[test]
fn single_result_api_preserves_sparse_nonzero_ids_for_multi_source_maps() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 1)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map.clone(), 0);
    coverage.statement_map.retain(|id, _| id == "0");
    coverage.s.retain(|id, _| id == "0");
    coverage.fn_map.retain(|id, _| id == "0");
    coverage.f.retain(|id, _| id == "0");
    coverage.branch_map.retain(|id, _| id == "0");
    coverage.b.retain(|id, _| id == "0");
    coverage.b_t.as_mut().expect("bT").retain(|id, _| id == "0");
    coverage.x_fallow_function_map.as_mut().expect("overlay").retain(|id, _| id == "0");

    let statement = coverage.statement_map.remove("0").expect("statement");
    let statement_hits = coverage.s.remove("0").expect("statement hits");
    coverage.statement_map.insert("7".to_string(), statement);
    coverage.s.insert("7".to_string(), statement_hits);
    let function = coverage.fn_map.remove("0").expect("function");
    let function_hits = coverage.f.remove("0").expect("function hits");
    let identity =
        coverage.x_fallow_function_map.as_mut().expect("overlay").remove("0").expect("identity");
    coverage.fn_map.insert("9".to_string(), function);
    coverage.f.insert("9".to_string(), function_hits);
    coverage.x_fallow_function_map.as_mut().expect("overlay").insert("9".to_string(), identity);
    let branch = coverage.branch_map.remove("0").expect("branch");
    let branch_hits = coverage.b.remove("0").expect("branch hits");
    let branch_truthy = coverage.b_t.as_mut().expect("bT").remove("0").expect("truthy hits");
    coverage.branch_map.insert("11".to_string(), branch);
    coverage.b.insert("11".to_string(), branch_hits);
    coverage.b_t.as_mut().expect("bT").insert("11".to_string(), branch_truthy);

    let assert_sparse_ids = |remapped: &FileCoverage| {
        assert_eq!(remapped.path, "src/a.ts");
        assert_eq!(remapped.statement_map.keys().collect::<Vec<_>>(), vec!["7"]);
        assert_eq!(remapped.s.keys().collect::<Vec<_>>(), vec!["7"]);
        assert_eq!(remapped.fn_map.keys().collect::<Vec<_>>(), vec!["9"]);
        assert_eq!(remapped.f.keys().collect::<Vec<_>>(), vec!["9"]);
        assert_eq!(remapped.branch_map.keys().collect::<Vec<_>>(), vec!["11"]);
        assert_eq!(remapped.b.keys().collect::<Vec<_>>(), vec!["11"]);
        assert_eq!(remapped.b_t.as_ref().expect("bT").keys().collect::<Vec<_>>(), vec!["11"]);
        assert_eq!(
            remapped.x_fallow_function_map.as_ref().expect("overlay").keys().collect::<Vec<_>>(),
            vec!["9"]
        );
    };

    let remapped = remap_coverage_with_options(&coverage, RemapOptions { drop_unmapped: true })
        .expect("one surviving source returns coverage");
    assert_sparse_ids(&remapped);

    let mut stored_coverage = coverage;
    stored_coverage.input_source_map = None;
    let mut store = SourceMapStore::new();
    store.add_map("dist/bundle.js", &map);
    let remapped = store
        .transform_coverage_with_options(&stored_coverage, RemapOptions { drop_unmapped: true })
        .expect("stored multi-source map returns one surviving source");
    assert_sparse_ids(&remapped);
}

#[test]
fn branch_with_cross_source_arms_is_dropped() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);
    coverage.branch_map.get_mut("0").expect("branch").locations =
        vec![loc(1, 0, 1, 2), loc(2, 0, 2, 2)];
    coverage.branch_map.retain(|id, _| id == "0");
    coverage.b.retain(|id, _| id == "0");
    coverage.b_t.as_mut().expect("bT").retain(|id, _| id == "0");
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(coverage.path.clone(), coverage);

    let remapped =
        remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
    assert!(remapped.values().all(|file| file.branch_map.is_empty()));
}

#[test]
fn single_result_api_does_not_restore_cross_source_functions_or_branches() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);
    coverage.statement_map.retain(|id, _| id == "0");
    coverage.s.retain(|id, _| id == "0");
    coverage.fn_map.retain(|id, _| id == "0");
    coverage.f.retain(|id, _| id == "0");
    let function = coverage.fn_map.get_mut("0").expect("function");
    function.decl = loc(1, 0, 1, 1);
    function.loc = loc(2, 0, 2, 5);
    coverage.branch_map.retain(|id, _| id == "0");
    coverage.b.retain(|id, _| id == "0");
    coverage.b_t.as_mut().expect("bT").retain(|id, _| id == "0");
    coverage.branch_map.get_mut("0").expect("branch").locations =
        vec![loc(1, 0, 1, 2), loc(2, 0, 2, 2)];

    let remapped = remap_coverage_with_options(&coverage, RemapOptions { drop_unmapped: true })
        .expect("the source-a statement produces one output");
    assert_eq!(remapped.path, "src/a.ts");
    assert!(remapped.fn_map.is_empty(), "cross-source function stays dropped");
    assert!(remapped.branch_map.is_empty(), "cross-source branch stays dropped");
}

#[test]
fn multi_source_if_branch_keeps_implicit_else_arm() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);
    let implicit_else = loc(0, 0, 0, 0);
    coverage.branch_map.get_mut("0").expect("branch").locations =
        vec![loc(1, 0, 1, 2), implicit_else];
    coverage.branch_map.retain(|id, _| id == "0");
    coverage.b.retain(|id, _| id == "0");
    coverage.b_t.as_mut().expect("bT").retain(|id, _| id == "0");
    let remapped =
        remap_coverage_to_map_with_options(&coverage, RemapOptions { drop_unmapped: true })
            .expect("usable map");

    let branch = remapped["src/a.ts"].branch_map.values().next().expect("branch survives");
    assert_eq!(branch.locations.len(), 2, "implicit else stays aligned with its counters");
    assert_eq!(remapped["src/a.ts"].b.values().next().expect("counts"), &vec![1, 2]);
}

#[test]
fn ambiguous_unmapped_entries_drop_in_both_multi_source_modes() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let mut coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);
    coverage.statement_map.insert("unmapped".to_string(), loc(3, 0, 3, 5));
    coverage.s.insert("unmapped".to_string(), 99);

    for drop_unmapped in [false, true] {
        let remapped =
            remap_coverage_to_map_with_options(&coverage, RemapOptions { drop_unmapped })
                .expect("usable map");
        assert!(remapped.values().all(|file| file.s.values().all(|hits| *hits != 99)));
    }
}
