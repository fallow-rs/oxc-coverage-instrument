//! Integration tests for the `source_maps::remap_coverage` API.
//!
//! Covers the Vitest istanbul-reporter path: instrument code that came from an
//! upstream transform (e.g. TypeScript), then walk the resulting `FileCoverage`
//! through its embedded `inputSourceMap` so coverage positions point back at
//! the original source the developer wrote.

use std::collections::BTreeMap;

use oxc_coverage_instrument::{
    FileCoverage, InstrumentOptions, instrument, parse_coverage_map, remap_coverage,
    remap_coverage_map,
};

/// Three-line TypeScript file post type-strip, with an identity-line source map.
/// Each line of intermediate JS maps back to the same line of original TS.
fn three_line_inputs() -> (String, String, String) {
    let original_ts = "const x: number = 1;\nconst y: number = 2;\nconst z: number = 3;\n";
    let intermediate_js = "const x = 1;\nconst y = 2;\nconst z = 3;\n";
    let input_sm = format!(
        r#"{{"version":3,"sources":["src/app.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA;AACA;AACA","names":[]}}"#,
    );
    (original_ts.to_string(), intermediate_js.to_string(), input_sm)
}

#[test]
fn remap_coverage_rewrites_path_to_original_source() {
    let (_, intermediate, input_sm) = three_line_inputs();
    let opts =
        InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() };
    let result = instrument(&intermediate, "intermediate.js", &opts).unwrap();

    let remapped = remap_coverage(&result.coverage_map)
        .expect("remap returns Some when inputSourceMap present");

    assert_eq!(remapped.path, "src/app.ts");
    assert!(
        remapped.input_source_map.is_none(),
        "remapped coverage clears the consumed inputSourceMap"
    );
}

#[test]
fn remap_coverage_resolves_statement_positions_to_original_lines() {
    let (_, intermediate, input_sm) = three_line_inputs();
    let opts =
        InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() };
    let result = instrument(&intermediate, "intermediate.js", &opts).unwrap();

    let remapped = remap_coverage(&result.coverage_map).expect("remap succeeds");

    // Each line of the intermediate JS holds one statement, and the input map
    // is an identity-line mapping back to src/app.ts. Statement lines on the
    // remapped output must therefore be 1, 2, 3 in some order.
    let mut lines: Vec<u32> = remapped.statement_map.values().map(|loc| loc.start.line).collect();
    lines.sort_unstable();
    assert_eq!(lines, vec![1, 2, 3], "statementMap lines after remap, got: {lines:?}");
}

#[test]
fn remap_coverage_applies_source_root() {
    // Input map declares a sourceRoot; the resolved path must join sourceRoot
    // with the first source entry. Matches `istanbul-lib-source-maps`.
    let input_sm = r#"{"version":3,"sourceRoot":"project/","sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
    let opts = InstrumentOptions {
        input_source_map: Some(input_sm.to_string()),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "intermediate.js", &opts).unwrap();
    let remapped = remap_coverage(&result.coverage_map).expect("remap succeeds");
    assert_eq!(remapped.path, "project/src/app.ts");
}

#[test]
fn remap_coverage_applies_source_root_without_trailing_slash() {
    // `istanbul-lib-source-maps` inserts a `/` when sourceRoot has none.
    let input_sm = r#"{"version":3,"sourceRoot":"project","sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
    let opts = InstrumentOptions {
        input_source_map: Some(input_sm.to_string()),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "intermediate.js", &opts).unwrap();
    let remapped = remap_coverage(&result.coverage_map).expect("remap succeeds");
    assert_eq!(remapped.path, "project/src/app.ts");
}

#[test]
fn remap_coverage_returns_none_without_input_source_map() {
    let result = instrument("const x = 1;", "test.js", &InstrumentOptions::default()).unwrap();
    assert!(
        remap_coverage(&result.coverage_map).is_none(),
        "remap returns None when no inputSourceMap is attached"
    );
}

#[test]
fn remap_coverage_returns_none_when_input_sources_empty() {
    // Source map with empty sources array is unusable for remapping; the
    // function must back off rather than emit a FileCoverage with an empty path.
    let input_sm = r#"{"version":3,"sources":[],"mappings":"","names":[]}"#;
    let opts = InstrumentOptions {
        input_source_map: Some(input_sm.to_string()),
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "test.js", &opts).unwrap();
    assert!(remap_coverage(&result.coverage_map).is_none());
}

#[test]
fn remap_coverage_map_rewrites_keys_to_original_paths() {
    let (_, intermediate, input_sm) = three_line_inputs();
    let opts =
        InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() };
    let result = instrument(&intermediate, "intermediate.js", &opts).unwrap();

    let mut coverage: BTreeMap<String, FileCoverage> = BTreeMap::new();
    coverage.insert("intermediate.js".to_string(), result.coverage_map);

    let remapped = remap_coverage_map(&coverage);

    assert!(
        remapped.contains_key("src/app.ts"),
        "remapped coverage map should be keyed by the resolved original path"
    );
    assert!(
        !remapped.contains_key("intermediate.js"),
        "remapped coverage map should not retain the intermediate path"
    );
}

#[test]
fn remap_coverage_map_passes_through_entries_without_input_map() {
    let result = instrument("const x = 1;", "plain.js", &InstrumentOptions::default()).unwrap();
    let mut coverage: BTreeMap<String, FileCoverage> = BTreeMap::new();
    coverage.insert("plain.js".to_string(), result.coverage_map);

    let remapped = remap_coverage_map(&coverage);

    assert!(remapped.contains_key("plain.js"), "passthrough preserves the key");
}

#[test]
fn remap_coverage_round_trips_through_parse_coverage_map() {
    // Simulate the wire shape: serialize to coverage-final.json, parse back,
    // remap. This is the path Vitest's istanbul reporter actually exercises.
    let (_, intermediate, input_sm) = three_line_inputs();
    let opts =
        InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() };
    let result = instrument(&intermediate, "intermediate.js", &opts).unwrap();
    let original_statement_count = result.coverage_map.statement_map.len();

    let coverage_json = format!(
        r#"{{"intermediate.js":{}}}"#,
        serde_json::to_string(&result.coverage_map).unwrap()
    );
    let parsed = parse_coverage_map(&coverage_json).expect("parse coverage-final.json shape");

    let remapped = remap_coverage_map(&parsed);
    let fc = remapped.get("src/app.ts").expect("remapped under original path");
    assert_eq!(
        fc.statement_map.len(),
        original_statement_count,
        "statementMap entries must survive serialize-parse-remap"
    );
    assert_eq!(fc.s.len(), original_statement_count, "hit-count slots must survive the round-trip");
    assert!(fc.input_source_map.is_none(), "inputSourceMap should be consumed during remap");
}
