//! Folding entries that land on the same original path.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{RemapOptions, remap_coverage_map_with_options};

use crate::fixtures::{generated_map, loc, mapped_full_shape_coverage};

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
