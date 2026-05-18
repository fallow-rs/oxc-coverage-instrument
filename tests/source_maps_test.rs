//! Integration tests for the `source_maps::remap_coverage` API.
//!
//! Covers the Vitest istanbul-reporter path: instrument code that came from an
//! upstream transform (e.g. TypeScript), then walk the resulting `FileCoverage`
//! through its embedded `inputSourceMap` so coverage positions point back at
//! the original source the developer wrote.

use std::collections::BTreeMap;

use oxc_coverage_instrument::{
    FileCoverage, InstrumentOptions, SourceMapStore, instrument, parse_coverage_map,
    remap_coverage, remap_coverage_map, remap_coverage_map_with_loader, remap_coverage_with_loader,
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

#[test]
fn remap_coverage_with_loader_falls_back_to_external_map() {
    // No embedded inputSourceMap; the loader supplies the map JSON keyed
    // by the FileCoverage path. Matches nyc's sourceStore disk-read flow.
    let intermediate_js = "const x = 1;\nconst y = 2;\nconst z = 3;\n";
    let original_ts = "const x: number = 1;\nconst y: number = 2;\nconst z: number = 3;\n";
    let input_sm = format!(
        r#"{{"version":3,"sources":["src/app.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA;AACA;AACA","names":[]}}"#,
    );

    // Instrument WITHOUT input_source_map so the resulting FileCoverage has
    // no embedded map. The loader must fill the gap.
    let result =
        instrument(intermediate_js, "intermediate.js", &InstrumentOptions::default()).unwrap();
    assert!(
        result.coverage_map.input_source_map.is_none(),
        "precondition: no inputSourceMap on the FileCoverage"
    );

    let remapped = remap_coverage_with_loader(&result.coverage_map, |path| {
        if path == "intermediate.js" { Some(input_sm.clone()) } else { None }
    })
    .expect("loader supplies the missing map");

    assert_eq!(remapped.path, "src/app.ts");
    let mut lines: Vec<u32> = remapped.statement_map.values().map(|loc| loc.start.line).collect();
    lines.sort_unstable();
    assert_eq!(lines, vec![1, 2, 3], "statementMap lines after loader-supplied remap");
}

#[test]
fn remap_coverage_with_loader_prefers_embedded_map() {
    // When the FileCoverage already carries an embedded inputSourceMap, the
    // loader is not consulted. This matches istanbul-lib-source-maps's
    // semantics: the sourceStore only fires when the map is missing.
    let intermediate_js = "const x = 1;\nconst y = 2;\nconst z = 3;\n";
    let original_ts = "const x: number = 1;\nconst y: number = 2;\nconst z: number = 3;\n";
    let input_sm = format!(
        r#"{{"version":3,"sources":["src/app.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA;AACA;AACA","names":[]}}"#,
    );
    let opts =
        InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() };
    let result = instrument(intermediate_js, "intermediate.js", &opts).unwrap();

    let remapped =
        remap_coverage_with_loader(&result.coverage_map, |_| panic!("loader must not run"))
            .expect("embedded map is used");
    assert_eq!(remapped.path, "src/app.ts");
}

#[test]
fn remap_coverage_map_with_loader_handles_mixed_entries() {
    // Mix of entries: one embedded, one needing the loader, one with neither
    // (pass-through). All three behaviors in a single call.
    let intermediate_js = "const x = 1;\n";
    let ts_map = r#"{"version":3,"sources":["src/embedded.ts"],"mappings":"AAAA","names":[]}"#;
    let loader_map = r#"{"version":3,"sources":["src/loader.ts"],"mappings":"AAAA","names":[]}"#;

    let embedded_result = instrument(
        intermediate_js,
        "embedded.js",
        &InstrumentOptions {
            input_source_map: Some(ts_map.to_string()),
            ..InstrumentOptions::default()
        },
    )
    .unwrap();
    let needs_loader =
        instrument(intermediate_js, "needs-loader.js", &InstrumentOptions::default()).unwrap();
    let plain = instrument(intermediate_js, "plain.js", &InstrumentOptions::default()).unwrap();

    let mut coverage: BTreeMap<String, FileCoverage> = BTreeMap::new();
    coverage.insert("embedded.js".to_string(), embedded_result.coverage_map);
    coverage.insert("needs-loader.js".to_string(), needs_loader.coverage_map);
    coverage.insert("plain.js".to_string(), plain.coverage_map);

    let remapped = remap_coverage_map_with_loader(&coverage, |path| {
        if path == "needs-loader.js" { Some(loader_map.to_string()) } else { None }
    });

    assert!(remapped.contains_key("src/embedded.ts"), "embedded path remapped");
    assert!(remapped.contains_key("src/loader.ts"), "loader-supplied path remapped");
    assert!(remapped.contains_key("plain.js"), "no-map entry passes through");
}

#[test]
fn source_map_store_transforms_via_registered_map() {
    // Mode B: caller registers a map per file as it's instrumented, then
    // applies the store to a FileCoverage at report time. Store takes
    // precedence over any embedded map and over the no-loader fallback.
    let intermediate_js = "const x = 1;\nconst y = 2;\nconst z = 3;\n";
    let original_ts = "const x: number = 1;\nconst y: number = 2;\nconst z: number = 3;\n";
    let input_sm_json = format!(
        r#"{{"version":3,"sources":["src/app.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA;AACA;AACA","names":[]}}"#,
    );

    let result =
        instrument(intermediate_js, "intermediate.js", &InstrumentOptions::default()).unwrap();

    let mut store = SourceMapStore::new();
    assert!(store.is_empty(), "freshly constructed store is empty");
    store.add_map(
        "intermediate.js",
        serde_json::from_str::<serde_json::Value>(&input_sm_json).unwrap(),
    );
    assert_eq!(store.len(), 1);
    assert!(store.contains("intermediate.js"));

    let remapped =
        store.transform_coverage(&result.coverage_map).expect("store-supplied map applies");
    assert_eq!(remapped.path, "src/app.ts");
}

#[test]
fn source_map_store_overrides_embedded_input_map() {
    // The store's map wins when both are present, mirroring
    // istanbul-lib-source-maps's `registerMap` precedence over per-file
    // embedded maps.
    let intermediate_js = "const x = 1;\n";
    let embedded_map_json =
        r#"{"version":3,"sources":["src/embedded.ts"],"mappings":"AAAA","names":[]}"#;
    let store_map_json = r#"{"version":3,"sources":["src/store.ts"],"mappings":"AAAA","names":[]}"#;
    let opts = InstrumentOptions {
        input_source_map: Some(embedded_map_json.to_string()),
        ..InstrumentOptions::default()
    };
    let result = instrument(intermediate_js, "intermediate.js", &opts).unwrap();

    let mut store = SourceMapStore::new();
    store.add_map("intermediate.js", serde_json::from_str(store_map_json).unwrap());

    let remapped = store.transform_coverage(&result.coverage_map).expect("store applies");
    assert_eq!(remapped.path, "src/store.ts", "store map wins over embedded map");
}

#[test]
fn source_map_store_falls_back_to_embedded_map_when_unregistered() {
    // If the store has no entry for a given path, the embedded map (if any)
    // is still applied, matching remap_coverage's behavior.
    let intermediate_js = "const x = 1;\n";
    let embedded_map_json =
        r#"{"version":3,"sources":["src/embedded.ts"],"mappings":"AAAA","names":[]}"#;
    let opts = InstrumentOptions {
        input_source_map: Some(embedded_map_json.to_string()),
        ..InstrumentOptions::default()
    };
    let result = instrument(intermediate_js, "intermediate.js", &opts).unwrap();

    let store = SourceMapStore::new();
    let remapped = store
        .transform_coverage(&result.coverage_map)
        .expect("embedded map is used when store has no entry");
    assert_eq!(remapped.path, "src/embedded.ts");
}

#[test]
fn source_map_store_passes_through_when_no_map_available() {
    // No embedded map, no store entry: transform_coverage returns None and
    // transform_coverage_map preserves the original key.
    let result = instrument("const x = 1;\n", "plain.js", &InstrumentOptions::default()).unwrap();
    let store = SourceMapStore::new();
    assert!(
        store.transform_coverage(&result.coverage_map).is_none(),
        "no map -> None mirrors remap_coverage"
    );

    let mut coverage: BTreeMap<String, FileCoverage> = BTreeMap::new();
    coverage.insert("plain.js".to_string(), result.coverage_map);
    let remapped = store.transform_coverage_map(&coverage);
    assert!(remapped.contains_key("plain.js"), "passthrough preserves key");
}
