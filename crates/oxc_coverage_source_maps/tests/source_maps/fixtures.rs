//! Source maps and `FileCoverage` shapes shared by the test modules.

use std::collections::BTreeMap;

use oxc_coverage_types::{
    BranchEntry, FileCoverage, FnEntry, FunctionIdentity, Location, Position,
};
use srcmap_generator::SourceMapGenerator;

pub const SRC_PATH: &str = "src/app.ts";

/// Build a `Location` from 1-based lines and 0-based columns.
pub fn loc(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Location {
    Location {
        start: Position { line: start_line, column: start_col },
        end: Position { line: end_line, column: end_col },
    }
}

/// `AAAA;AACA;AACA` is the canonical three-line identity map: line 1 maps to
/// line 1 of `src/app.ts`, line 2 to line 2, line 3 to line 3. The
/// `srcmap-sourcemap` parser expects the names array even when empty.
pub fn identity_three_line_map(source_root: Option<&str>) -> serde_json::Value {
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
pub fn full_shape_file_coverage(input_source_map: Option<serde_json::Value>) -> FileCoverage {
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

/// A map whose generated line `n` maps to `sources[n]` at its listed original
/// line, so one generated file fans out across several original sources.
pub fn generated_map(path: &str, sources: &[(&str, u32)]) -> serde_json::Value {
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

/// Coverage for the first two generated lines of `path`, populating every
/// section including `bT` and the function identity overlay. `count_base`
/// offsets the hit counts so two fixtures can be told apart after a merge.
pub fn mapped_full_shape_coverage(
    path: &str,
    map: serde_json::Value,
    count_base: u32,
) -> FileCoverage {
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

/// Single-line identity map: line 1 of the generated file maps to line 1 of
/// `src/app.ts`. Positions on line 2+ have no mapping and
/// `original_position_for` returns `None` for them, which is the trigger the
/// `RemapOptions::drop_unmapped` tests need.
pub fn one_line_identity_map() -> serde_json::Value {
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
pub fn mixed_mapped_file_coverage() -> FileCoverage {
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

/// Two segments on one generated line over `src/app.ts`:
/// `gen(1,0) -> orig(1,0)` and `gen(1,6) -> orig(1,10)`. Original line 1 is
/// `state.someVeryLongPropertyName1;`, UTF-16 length 32, so an end past the
/// last segment exercises the end-of-line clamp. With `with_content` false the
/// map carries no `sourcesContent` and the clamp falls back to the mappings.
pub fn two_segment_map(with_content: bool) -> serde_json::Value {
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
pub fn single_statement_coverage(
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
