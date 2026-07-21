//! Canonicalizing remap assigns output ids in ascending numeric order of the
//! incoming ids, matching how `istanbul-lib-source-maps` enumerates the
//! `statementMap` / `fnMap` / `branchMap` object keys. With 12 entries the
//! string order of the ids ("0", "1", "10", "11", "2", ...) and their numeric
//! order disagree, so a string-order renumbering pairs ids with the wrong
//! locations across a tool boundary.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{RemapOptions, remap_coverage_to_map_with_options};
use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry};
use srcmap_generator::SourceMapGenerator;

use crate::fixtures::{SRC_PATH, loc};

const ENTRIES: u32 = 12;

/// Identity line map over a single source: generated line `i` maps to line `i`
/// of `src/app.ts`, for `1..=ENTRIES`.
fn identity_line_map() -> serde_json::Value {
    let mut generator = SourceMapGenerator::new(Some("intermediate.js".to_string()));
    let source = generator.add_source(SRC_PATH);
    let content = "0123456789\n".repeat(ENTRIES as usize);
    generator.set_source_content(source, &content);
    for line in 0..ENTRIES {
        generator.add_mapping(line, 0, source, line, 0);
    }
    serde_json::from_str(&generator.to_json()).expect("generated source map is valid JSON")
}

/// A `FileCoverage` whose statement / function / branch id `j` (0-based, decimal
/// string) sits on line `j + 1`, so the numeric order of the ids matches the
/// ascending line order and any string-order renumbering scrambles the two.
fn line_indexed_coverage() -> FileCoverage {
    let mut statement_map = BTreeMap::new();
    let mut fn_map = BTreeMap::new();
    let mut branch_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let mut f = BTreeMap::new();
    let mut b = BTreeMap::new();

    for index in 0..ENTRIES {
        let id = index.to_string();
        let line = index + 1;
        statement_map.insert(id.clone(), loc(line, 0, line, 5));
        fn_map.insert(
            id.clone(),
            FnEntry {
                name: format!("f{line}"),
                line,
                decl: loc(line, 0, line, 1),
                loc: loc(line, 0, line, 5),
            },
        );
        branch_map.insert(
            id.clone(),
            BranchEntry {
                loc: loc(line, 0, line, 5),
                line,
                branch_type: "if".to_string(),
                locations: vec![loc(line, 0, line, 2), loc(line, 3, line, 5)],
            },
        );
        s.insert(id.clone(), line);
        f.insert(id.clone(), line + 100);
        b.insert(id, vec![line, line + 1]);
    }

    FileCoverage {
        path: "intermediate.js".to_string(),
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t: None,
        input_source_map: Some(identity_line_map()),
        x_fallow_function_map: None,
    }
}

fn remapped() -> FileCoverage {
    let coverage = line_indexed_coverage();
    let mut remapped =
        remap_coverage_to_map_with_options(&coverage, RemapOptions { drop_unmapped: true })
            .expect("single-source identity map remaps");
    assert_eq!(remapped.len(), 1, "identity map resolves to one source");
    remapped.remove(SRC_PATH).expect("remapped src/app.ts")
}

/// Numeric id `k` must point at line `k + 1` for every entry.
fn assert_numeric_order(lines: impl Iterator<Item = (String, u32)>) {
    let mut by_numeric: Vec<(u32, u32)> = lines
        .map(|(id, line)| (id.parse::<u32>().expect("canonical ids are decimal"), line))
        .collect();
    by_numeric.sort_by_key(|(id, _)| *id);
    let expected: Vec<(u32, u32)> = (0..ENTRIES).map(|id| (id, id + 1)).collect();
    assert_eq!(by_numeric, expected, "numeric id k must point at line k + 1");
}

#[test]
fn canonical_statement_ids_follow_numeric_order() {
    let file = remapped();
    assert_eq!(file.statement_map.len(), ENTRIES as usize, "every statement survives");
    assert_numeric_order(
        file.statement_map.iter().map(|(id, location)| (id.clone(), location.start.line)),
    );
    assert_eq!(
        file.s.keys().collect::<std::collections::BTreeSet<_>>(),
        file.statement_map.keys().collect::<std::collections::BTreeSet<_>>(),
        "counters stay aligned with the renumbered statements",
    );
}

#[test]
fn canonical_function_ids_follow_numeric_order() {
    let file = remapped();
    assert_eq!(file.fn_map.len(), ENTRIES as usize, "every function survives");
    assert_numeric_order(
        file.fn_map.iter().map(|(id, function)| (id.clone(), function.loc.start.line)),
    );
    assert_eq!(
        file.f.keys().collect::<std::collections::BTreeSet<_>>(),
        file.fn_map.keys().collect::<std::collections::BTreeSet<_>>(),
        "counters stay aligned with the renumbered functions",
    );
}

#[test]
fn canonical_branch_ids_follow_numeric_order() {
    let file = remapped();
    assert_eq!(file.branch_map.len(), ENTRIES as usize, "every branch survives");
    assert_numeric_order(
        file.branch_map.iter().map(|(id, branch)| (id.clone(), branch.loc.start.line)),
    );
    assert_eq!(
        file.b.keys().collect::<std::collections::BTreeSet<_>>(),
        file.branch_map.keys().collect::<std::collections::BTreeSet<_>>(),
        "counters stay aligned with the renumbered branches",
    );
}
