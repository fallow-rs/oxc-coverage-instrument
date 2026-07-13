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
    PositionRemapper, RemapOptions, SourceMapStore, remap_coverage, remap_coverage_map_with_loader,
    remap_coverage_map_with_options, remap_coverage_to_map_with_options,
    remap_coverage_with_loader, remap_coverage_with_options,
};
use oxc_coverage_types::{
    BranchEntry, FileCoverage, FnEntry, FunctionIdentity, Location, Position, parse_coverage_map,
};
use srcmap_generator::SourceMapGenerator;

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

fn generated_map(path: &str, sources: &[(&str, u32)]) -> serde_json::Value {
    let mut generator = SourceMapGenerator::new(Some(path.to_string()));
    for (generated_line, (source_path, original_line)) in sources.iter().enumerate() {
        let source = generator.add_source(source_path);
        generator.set_source_content(source, "0123456789\n0123456789\n0123456789\n");
        generator.add_mapping(
            u32::try_from(generated_line).expect("fixture line fits u32"),
            0,
            source,
            *original_line,
            0,
        );
    }
    serde_json::from_str(&generator.to_json()).expect("generated source map is valid JSON")
}

fn mapped_full_shape_coverage(path: &str, map: serde_json::Value, count_base: u32) -> FileCoverage {
    let mut statement_map = BTreeMap::new();
    let mut fn_map = BTreeMap::new();
    let mut branch_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let mut f = BTreeMap::new();
    let mut b = BTreeMap::new();
    let mut b_t = BTreeMap::new();
    let mut overlay = BTreeMap::new();

    for generated_line in 1..=2 {
        let key = (generated_line - 1).to_string();
        let entry_loc = loc(generated_line, 0, generated_line, 5);
        statement_map.insert(key.clone(), entry_loc.clone());
        fn_map.insert(
            key.clone(),
            FnEntry {
                name: format!("f{generated_line}"),
                line: generated_line,
                decl: loc(generated_line, 0, generated_line, 1),
                loc: entry_loc.clone(),
            },
        );
        branch_map.insert(
            key.clone(),
            BranchEntry {
                loc: entry_loc.clone(),
                line: generated_line,
                branch_type: "if".to_string(),
                locations: vec![
                    loc(generated_line, 0, generated_line, 2),
                    loc(generated_line, 3, generated_line, 5),
                ],
            },
        );
        s.insert(key.clone(), count_base + generated_line);
        f.insert(key.clone(), count_base + generated_line + 10);
        b.insert(key.clone(), vec![count_base + generated_line, count_base + generated_line + 1]);
        b_t.insert(
            key.clone(),
            vec![count_base + generated_line + 2, count_base + generated_line + 3],
        );
        overlay.insert(
            key,
            FunctionIdentity {
                id: format!("fallow:fn:{generated_line:08x}"),
                name: format!("f{generated_line}"),
                path: "src/identity.ts".to_string(),
                decl: loc(generated_line, 0, generated_line, 1),
                loc: entry_loc,
            },
        );
    }

    FileCoverage {
        path: path.to_string(),
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t: Some(b_t),
        input_source_map: Some(map),
        x_fallow_function_map: Some(overlay),
    }
}

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
fn map_remap_merges_same_source_collisions_by_location() {
    let first_map = generated_map("dist/first.js", &[("src/shared.ts", 0), ("src/shared.ts", 1)]);
    let second_map = generated_map("dist/second.js", &[("src/shared.ts", 0), ("src/shared.ts", 2)]);
    let first = mapped_full_shape_coverage("dist/first.js", first_map, 0);
    let second = mapped_full_shape_coverage("dist/second.js", second_map, 20);
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(first.path.clone(), first);
    coverage_map.insert(second.path.clone(), second);

    let remapped =
        remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
    let shared = remapped.get("src/shared.ts").expect("shared source survives");

    let expected_ids = vec!["0", "1", "2"];
    assert_eq!(shared.statement_map.keys().map(String::as_str).collect::<Vec<_>>(), expected_ids);
    assert_eq!(shared.s.keys().map(String::as_str).collect::<Vec<_>>(), expected_ids);
    assert_eq!(shared.fn_map.keys().map(String::as_str).collect::<Vec<_>>(), expected_ids);
    assert_eq!(shared.f.keys().map(String::as_str).collect::<Vec<_>>(), expected_ids);
    assert_eq!(shared.branch_map.keys().map(String::as_str).collect::<Vec<_>>(), expected_ids);
    assert_eq!(shared.b.keys().map(String::as_str).collect::<Vec<_>>(), expected_ids);
    assert_eq!(
        shared.b_t.as_ref().expect("bT preserved").keys().map(String::as_str).collect::<Vec<_>>(),
        expected_ids,
    );
    assert_eq!(
        shared
            .x_fallow_function_map
            .as_ref()
            .expect("overlay preserved")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected_ids,
    );
    assert_eq!(shared.statement_map.len(), 3, "shared and distinct statements survive");
    assert_eq!(shared.fn_map.len(), 3, "shared and distinct functions survive");
    assert_eq!(shared.branch_map.len(), 3, "shared and distinct branches survive");
    let shared_statement_id = shared
        .statement_map
        .iter()
        .find_map(|(id, location)| (location.start.line == 1).then_some(id))
        .expect("shared statement exists");
    assert_eq!(shared.s[shared_statement_id], 22, "shared statement counts sum");
    let shared_function_id = shared
        .fn_map
        .iter()
        .find_map(|(id, function)| (function.loc.start.line == 1).then_some(id))
        .expect("shared function exists");
    assert_eq!(shared.f[shared_function_id], 42, "shared function counts sum");
    let shared_branch_id = shared
        .branch_map
        .iter()
        .find_map(|(id, branch)| (branch.loc.start.line == 1).then_some(id))
        .expect("shared branch exists");
    assert_eq!(shared.b[shared_branch_id], vec![22, 24], "branch counts sum by arm");
    assert_eq!(
        shared.b_t.as_ref().expect("bT preserved")[shared_branch_id],
        vec![26, 28],
        "truthy branch counts sum by arm",
    );
}

#[test]
fn multi_source_single_file_api_returns_none_and_map_api_is_contiguous() {
    let map = generated_map("dist/bundle.js", &[("src/a.ts", 0), ("src/b.ts", 0)]);
    let coverage = mapped_full_shape_coverage("dist/bundle.js", map, 0);

    assert!(remap_coverage(&coverage).is_none(), "single-result API cannot discard a source");
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
fn single_source_api_keeps_empty_and_fully_pruned_results_compatible() {
    let mut empty = full_shape_file_coverage(Some(identity_three_line_map(None)));
    empty.statement_map.clear();
    empty.fn_map.clear();
    empty.branch_map.clear();
    empty.s.clear();
    empty.f.clear();
    empty.b.clear();
    assert!(remap_coverage(&empty).is_some(), "empty single-source coverage still remaps");

    let fully_unmapped = single_statement_coverage(identity_three_line_map(None), 4, 0, 4, 5);
    let remapped =
        remap_coverage_with_options(&fully_unmapped, RemapOptions { drop_unmapped: true })
            .expect("fully pruned single-source coverage still returns its source file");
    assert!(remapped.statement_map.is_empty());
    assert!(remapped.s.is_empty());
}

#[test]
fn collision_count_addition_saturates_at_u32_max() {
    let first_map = generated_map("dist/first.js", &[("src/shared.ts", 0), ("src/shared.ts", 1)]);
    let second_map = generated_map("dist/second.js", &[("src/shared.ts", 0), ("src/shared.ts", 2)]);
    let mut first = mapped_full_shape_coverage("dist/first.js", first_map, 0);
    let mut second = mapped_full_shape_coverage("dist/second.js", second_map, 0);
    first.s.insert("0".to_string(), u32::MAX);
    second.s.insert("0".to_string(), 1);
    first.f.insert("0".to_string(), u32::MAX);
    second.f.insert("0".to_string(), 1);
    first.b.insert("0".to_string(), vec![u32::MAX, u32::MAX]);
    second.b.insert("0".to_string(), vec![1, 1]);
    first.b_t.as_mut().expect("bT").insert("0".to_string(), vec![u32::MAX, u32::MAX]);
    second.b_t.as_mut().expect("bT").insert("0".to_string(), vec![1, 1]);
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(first.path.clone(), first);
    coverage_map.insert(second.path.clone(), second);

    let remapped =
        remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
    let shared = &remapped["src/shared.ts"];
    let id = shared
        .statement_map
        .iter()
        .find_map(|(id, location)| (location.start.line == 1).then_some(id))
        .expect("shared statement exists");
    assert_eq!(shared.s[id], u32::MAX);
    let function_id = shared
        .fn_map
        .iter()
        .find_map(|(id, function)| (function.decl.start.line == 1).then_some(id))
        .expect("shared function exists");
    assert_eq!(shared.f[function_id], u32::MAX);
    let branch_id = shared
        .branch_map
        .iter()
        .find_map(|(id, branch)| (branch.locations[0].start.line == 1).then_some(id))
        .expect("shared branch exists");
    assert_eq!(shared.b[branch_id], vec![u32::MAX, u32::MAX]);
    assert_eq!(shared.b_t.as_ref().expect("bT")[branch_id], vec![u32::MAX, u32::MAX]);
}

#[test]
fn conflicting_function_overlays_drop_the_optional_overlay() {
    let first_map = generated_map("dist/first.js", &[("src/shared.ts", 0), ("src/shared.ts", 1)]);
    let second_map = generated_map("dist/second.js", &[("src/shared.ts", 0), ("src/shared.ts", 2)]);
    let first = mapped_full_shape_coverage("dist/first.js", first_map, 0);
    let mut second = mapped_full_shape_coverage("dist/second.js", second_map, 20);
    second.x_fallow_function_map.as_mut().expect("overlay").get_mut("0").expect("identity").id =
        "fallow:fn:conflict".to_string();
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(first.path.clone(), first);
    coverage_map.insert(second.path.clone(), second);

    let remapped =
        remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
    assert!(
        remapped["src/shared.ts"].x_fallow_function_map.is_none(),
        "an optional overlay is dropped instead of choosing a conflicting identity",
    );
}

#[test]
fn function_and_branch_merge_identities_match_istanbul() {
    let first_map = generated_map("dist/first.js", &[("src/shared.ts", 0), ("src/shared.ts", 1)]);
    let second_map = generated_map("dist/second.js", &[("src/shared.ts", 0), ("src/shared.ts", 2)]);
    let first = mapped_full_shape_coverage("dist/first.js", first_map, 0);
    let mut second = mapped_full_shape_coverage("dist/second.js", second_map, 20);
    let function = second.fn_map.get_mut("0").expect("function");
    function.name = "different".to_string();
    function.loc = loc(2, 0, 2, 5);
    let branch = second.branch_map.get_mut("0").expect("branch");
    branch.branch_type = "cond-expr".to_string();
    branch.loc = loc(2, 0, 2, 5);
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(first.path.clone(), first);
    coverage_map.insert(second.path.clone(), second);

    let remapped =
        remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
    let shared = &remapped["src/shared.ts"];
    assert_eq!(shared.statement_map.len(), 3, "locations alone merge statements");
    assert_eq!(shared.fn_map.len(), 3, "function declarations define identity");
    assert_eq!(shared.branch_map.len(), 3, "ordered branch arms define identity");

    let function_id = shared
        .fn_map
        .iter()
        .find_map(|(id, function)| (function.decl.start.line == 1).then_some(id))
        .expect("merged function exists");
    assert_eq!(shared.fn_map[function_id].name, "f1", "first function metadata wins");
    assert_eq!(shared.f[function_id], 42);
    let branch_id = shared
        .branch_map
        .iter()
        .find_map(|(id, branch)| (branch.locations[0].start.line == 1).then_some(id))
        .expect("merged branch exists");
    assert_eq!(shared.branch_map[branch_id].branch_type, "if", "first branch metadata wins");
    assert_eq!(shared.b[branch_id], vec![22, 24]);
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
fn statement_only_collision_does_not_erase_complete_function_overlay() {
    for (full_path, statement_path) in [
        ("dist/a-functions.js", "dist/z-statements.js"),
        ("dist/z-functions.js", "dist/a-statements.js"),
    ] {
        let map = generated_map(full_path, &[("src/shared.ts", 0), ("src/shared.ts", 1)]);
        let full = mapped_full_shape_coverage(full_path, map, 0);
        let map = generated_map(statement_path, &[("src/shared.ts", 0), ("src/shared.ts", 1)]);
        let mut statement_only = mapped_full_shape_coverage(statement_path, map, 20);
        statement_only.fn_map.clear();
        statement_only.f.clear();
        statement_only.x_fallow_function_map = None;
        let mut coverage_map = BTreeMap::new();
        coverage_map.insert(full.path.clone(), full);
        coverage_map.insert(statement_only.path.clone(), statement_only);

        let remapped =
            remap_coverage_map_with_options(&coverage_map, RemapOptions { drop_unmapped: true });
        let shared = &remapped["src/shared.ts"];
        assert_eq!(
            shared
                .x_fallow_function_map
                .as_ref()
                .expect("complete overlay survives")
                .keys()
                .collect::<Vec<_>>(),
            shared.fn_map.keys().collect::<Vec<_>>(),
        );
    }
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
    store.add_map("intermediate.js", &identity_three_line_map(None));

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
/// and falls back to its first mapped arm.
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
    store.add_map("intermediate.js", &identity_three_line_map(None));
    let remapped = store.transform_coverage(fc).expect("store-driven remap");
    assert_eq!(remapped.path, SRC_PATH);
}

// --- Issue #111: getMapping range-remap semantics ---------------------------
//
// A two-segment generated line over `src/app.ts`:
//   gen(line1,col0)  -> orig(line1,col0)
//   gen(line1,col6)  -> orig(line1,col10)
// Original line 1 is `state.someVeryLongPropertyName1;` (UTF-16 length 32).
// This lets the tests exercise: an exclusive end that falls *between* segments
// (direct lookup truncates backward; getMapping widens to the next segment),
// a 1-char span that balloons to its enclosing segment, and the end-of-line
// clamp for an end past the last segment.
fn two_segment_map(with_content: bool) -> serde_json::Value {
    let content = if with_content {
        r#","sourcesContent":["state.someVeryLongPropertyName1;\n"]"#
    } else {
        ""
    };
    serde_json::from_str(&format!(
        r#"{{"version":3,"sources":["{SRC_PATH}"],"mappings":"AAAA,MAAU","names":[]{content}}}"#,
    ))
    .unwrap()
}

/// A single-statement `FileCoverage` over `intermediate.js` whose one statement
/// spans the given generated coordinates, used to read back the remapped span.
fn single_statement_coverage(
    map: serde_json::Value,
    sl: u32,
    sc: u32,
    el: u32,
    ec: u32,
) -> FileCoverage {
    let mut statement_map = BTreeMap::new();
    statement_map.insert(
        "0".to_string(),
        Location {
            start: Position { line: sl, column: sc },
            end: Position { line: el, column: ec },
        },
    );
    let mut s = BTreeMap::new();
    s.insert("0".to_string(), 1);
    FileCoverage {
        path: "intermediate.js".to_string(),
        statement_map,
        fn_map: BTreeMap::new(),
        branch_map: BTreeMap::new(),
        s,
        f: BTreeMap::new(),
        b: BTreeMap::new(),
        b_t: None,
        input_source_map: Some(map),
        x_fallow_function_map: None,
    }
}

#[test]
fn get_mapping_widens_end_that_falls_between_segments() {
    // Generated end col 4 sits between segment starts 0 and 6. A direct lookup
    // snaps it backward to the previous segment (orig col 0); getMapping widens
    // it to the next original segment (orig col 10).
    let fc = single_statement_coverage(two_segment_map(true), 1, 0, 1, 4);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 0), "start resolves to orig (1,0)");
    assert_eq!(
        (loc.end.line, loc.end.column),
        (1, 10),
        "end widens to the next original segment, not the truncated previous one",
    );
}

#[test]
fn get_mapping_balloons_one_char_span_to_enclosing_segment() {
    // A 1-char generated span (col 6..7) on the second segment. getMapping
    // resolves the start with greatest-lower-bound (orig col 10) and the end to
    // end-of-line (no segment follows), ballooning the marker to its enclosing
    // span rather than leaving a 1-char span.
    let fc = single_statement_coverage(two_segment_map(true), 1, 6, 1, 7);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 10), "start snaps to enclosing segment");
    assert_eq!(loc.end.column, 32, "end balloons to the original line's UTF-16 length");
}

#[test]
fn get_mapping_clamps_end_of_line_from_sources_content() {
    // End col 20 is past the last segment with nothing after it: istanbul returns
    // column Infinity, which we clamp to the original line's UTF-16 length (32).
    let fc = single_statement_coverage(two_segment_map(true), 1, 6, 1, 20);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 10));
    assert_eq!(loc.end.column, 32, "Infinity end clamps to the line's UTF-16 length");
}

#[test]
fn get_mapping_end_of_line_without_content_falls_back_to_last_segment() {
    // Same map without sourcesContent: the Infinity end clamps to the rightmost
    // original column mapped on the line (10) instead of the true line length.
    let fc = single_statement_coverage(two_segment_map(false), 1, 0, 1, 20);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 0));
    assert_eq!(
        loc.end.column, 10,
        "without sourcesContent the end clamps to the last mapped original column",
    );
}

#[test]
fn get_mapping_degenerate_span_recomputes_end() {
    // A map whose generated line maps gen(0,0)->orig(0,0) and gen(0,2)->orig(0,5).
    // A backwards generated span (start col 2, end col 1) resolves to start
    // (1,5) and a Mapped end that also lands on (1,5): the degenerate-span guard
    // recomputes the end via LUB(genEnd) then steps one column left (col 4).
    let map: serde_json::Value = serde_json::from_str(&format!(
        r#"{{"version":3,"sources":["{SRC_PATH}"],"mappings":"AAAA,EAAK","names":[],"sourcesContent":["abcdefghij\n"]}}"#,
    ))
    .unwrap();
    let fc = single_statement_coverage(map, 1, 2, 1, 1);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 5), "start resolves to orig (1,5)");
    assert_eq!(
        (loc.end.line, loc.end.column),
        (1, 4),
        "degenerate guard recomputes end as LUB(genEnd) column minus one",
    );
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

// --- PositionRemapper::location_maps (eager AST-gate predicate, issue #122) ----

/// Build a `Location` from 1-based lines and 0-based columns.
fn loc(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Location {
    Location {
        start: Position { line: start_line, column: start_col },
        end: Position { line: end_line, column: end_col },
    }
}

#[test]
fn location_maps_matches_deferred_get_mapping_keep_decision() {
    // `AAAA;EACA`: gen(1,0)->orig(1,0) ; gen(2,2)->orig(2,0). Line 2's only
    // mapping sits at column 2, AFTER a statement that starts at column 0.
    let input_sm = r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["const a = 1;\nconst b = 2;\n"],"mappings":"AAAA;EACA","names":[]}"#;
    let remapper = PositionRemapper::from_json(input_sm).expect("usable map");

    // Line 1 maps cleanly at column 0.
    assert!(remapper.location_maps(&loc(1, 0, 1, 4)), "line-1 statement maps");

    // Issue #122: line-2 statement at column 0 has no mapping at-or-before on its
    // line (the only line-2 mapping is at column 2), but `getMapping`'s
    // least-upper-bound fallback resolves it forward and KEEPS it. The deferred
    // `drop_unmapped` path keeps it, so the eager gate must too.
    assert!(
        remapper.location_maps(&loc(2, 0, 2, 4)),
        "col-0 statement whose line maps at col 2 is kept (LUB fallback)",
    );

    // Line 3 has no mapping at all (the map only covers generated lines 1-2):
    // both GLB and LUB miss, so the span is genuinely unmapped and dropped.
    assert!(
        !remapper.location_maps(&loc(3, 0, 3, 4)),
        "a span on a generated line with no mappings is dropped",
    );

    // The `line == 0` "unknown" sentinel is kept as a no-op, matching
    // `try_remap_location`'s legacy-helper route.
    assert!(remapper.location_maps(&loc(0, 0, 0, 0)), "line-0 sentinel is a keep no-op");
}
