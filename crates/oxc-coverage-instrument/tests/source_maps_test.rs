//! Integration tests for the `source_maps::remap_coverage` API.
//!
//! Covers the Vitest istanbul-reporter path: instrument code that came from an
//! upstream transform (e.g. TypeScript), then walk the resulting `FileCoverage`
//! through its embedded `inputSourceMap` so coverage positions point back at
//! the original source the developer wrote.

use std::collections::BTreeMap;

use oxc_coverage_instrument::{
    FileCoverage, InstrumentOptions, RemapOptions, SourceMapStore, instrument, parse_coverage_map,
    remap_coverage, remap_coverage_map, remap_coverage_map_with_loader, remap_coverage_with_loader,
    remap_coverage_with_options,
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

// ---------------------------------------------------------------------------
// Eager composition (issue #100): `InstrumentOptions::compose_input_source_map`
// folds the embedded `inputSourceMap` into the coverage map at instrument time
// instead of leaving it for a downstream `remap_coverage` / `remapCoverageMap`.
// ---------------------------------------------------------------------------

#[test]
fn compose_input_source_map_rewrites_path_and_positions() {
    let (_, intermediate, input_sm) = three_line_inputs();
    let opts = InstrumentOptions {
        input_source_map: Some(input_sm),
        compose_input_source_map: true,
        ..InstrumentOptions::default()
    };
    let result = instrument(&intermediate, "intermediate.js", &opts).unwrap();

    // Path and positions are already in the original source, and no input map
    // is left embedded (the composition consumed it).
    assert_eq!(result.coverage_map.path, "src/app.ts");
    assert!(
        result.coverage_map.input_source_map.is_none(),
        "eager compose clears the consumed inputSourceMap"
    );
    let mut lines: Vec<u32> =
        result.coverage_map.statement_map.values().map(|loc| loc.start.line).collect();
    lines.sort_unstable();
    assert_eq!(lines, vec![1, 2, 3], "statementMap lines after eager compose");
}

#[test]
fn compose_input_source_map_equals_instrument_then_remap() {
    // The whole contract: eager compose must be bit-for-bit equal to
    // instrument-without-compose followed by `remap_coverage_with_options` with
    // `drop_unmapped: true` (issue #105: the eager path drops unmapped positions
    // unconditionally because there is no later remap opportunity). If this
    // holds, `remapCoverageMap` on the eager result is provably a no-op. Note
    // every position in `three_line_inputs` maps cleanly, so no entry is
    // actually dropped here; the drop behaviour is exercised by
    // `compose_input_source_map_drops_unmapped_positions` below.
    let (_, intermediate, input_sm) = three_line_inputs();

    let eager = instrument(
        &intermediate,
        "intermediate.js",
        &InstrumentOptions {
            input_source_map: Some(input_sm.clone()),
            compose_input_source_map: true,
            ..InstrumentOptions::default()
        },
    )
    .unwrap();

    let lazy_instrumented = instrument(
        &intermediate,
        "intermediate.js",
        &InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() },
    )
    .unwrap();
    let lazy = remap_coverage_with_options(
        &lazy_instrumented.coverage_map,
        RemapOptions { drop_unmapped: true },
    )
    .expect("lazy remap succeeds");

    assert_eq!(
        serde_json::to_value(&eager.coverage_map).unwrap(),
        serde_json::to_value(&lazy).unwrap(),
        "eager compose must equal instrument-then-remap with drop_unmapped"
    );
}

/// Intermediate JS whose first statement maps back to `src/app.ts` but whose
/// remaining statements have no mapping (the source map only carries a `mappings`
/// segment for generated line 1, mirroring `one_line_identity_map` in the
/// source-maps edge tests). The original source is a single line, so any entry
/// kept at its generated line 2+ coordinate would land past end-of-file. This is
/// the minimal stand-in for the Vue-SFC case in issue #105 where compiler
/// boilerplate at the tail of the chunk has no mapping back to the `.vue`.
fn partial_map_inputs() -> (String, String) {
    let original_ts = "export const a = 1;\n"; // one line
    let intermediate_js = "const a = 1;\nconst b = 2;\nconst c = 3;\n"; // three statements
    let input_sm = format!(
        r#"{{"version":3,"sources":["src/app.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA","names":[]}}"#,
    );
    (intermediate_js.to_string(), input_sm)
}

#[test]
fn compose_input_source_map_drops_unmapped_positions() {
    // Issue #105: the eager compose path must drop entries whose positions have
    // no mapping, instead of stranding them at generated coordinates past the
    // end of the original file. This is the non-vacuous counterpart to
    // `compose_input_source_map_equals_instrument_then_remap`: here the input map
    // only maps generated line 1, so the line 2+ statements have no original
    // position and must be pruned.
    let (intermediate, input_sm) = partial_map_inputs();
    let original_line_count = 1; // src/app.ts is a single line.

    // Baseline: plain generated map has all three statements.
    let plain =
        instrument(&intermediate, "intermediate.js", &InstrumentOptions::default()).unwrap();
    assert_eq!(
        plain.coverage_map.statement_map.len(),
        3,
        "the generated map has a statement per line before composition"
    );

    // Eager compose drops the two unmapped statements.
    let eager = instrument(
        &intermediate,
        "intermediate.js",
        &InstrumentOptions {
            input_source_map: Some(input_sm.clone()),
            compose_input_source_map: true,
            ..InstrumentOptions::default()
        },
    )
    .unwrap();
    assert_eq!(eager.coverage_map.path, "src/app.ts");
    assert_eq!(
        eager.coverage_map.statement_map.len(),
        1,
        "eager compose drops the unmapped line 2+ statements"
    );
    assert!(
        eager.coverage_map.statement_map.values().all(|loc| loc.start.line <= original_line_count),
        "no surviving statement lands past the end of the original file"
    );
    // The matching hit-count slots drop with their statements.
    assert_eq!(eager.coverage_map.s.len(), 1, "the `s` slots realign to the kept statement");

    // The bug shape, made explicit: WITHOUT dropping (the lazy default), the
    // unmapped statements are kept at their generated line numbers, which exceed
    // the original file's line count.
    let lazy_kept =
        remap_coverage(&plain_with_map(&intermediate, &input_sm)).expect("lazy remap succeeds");
    assert!(
        lazy_kept.statement_map.values().any(|loc| loc.start.line > original_line_count),
        "lazy keep-default strands unmapped statements past end-of-file (the #105 bug)"
    );

    // And the eager result equals the lazy path WITH drop_unmapped, proving the
    // two paths now agree on the drop policy.
    let lazy_dropped = remap_coverage_with_options(
        &plain_with_map(&intermediate, &input_sm),
        RemapOptions { drop_unmapped: true },
    )
    .expect("lazy drop remap succeeds");
    assert_eq!(
        serde_json::to_value(&eager.coverage_map).unwrap(),
        serde_json::to_value(&lazy_dropped).unwrap(),
        "eager compose must equal instrument-then-remap with drop_unmapped"
    );
}

/// Instrument `intermediate` with `input_sm` embedded but without eager compose,
/// returning the `FileCoverage` that still carries its `inputSourceMap` so the
/// lazy remap helpers can be exercised against it.
fn plain_with_map(intermediate: &str, input_sm: &str) -> FileCoverage {
    instrument(
        intermediate,
        "intermediate.js",
        &InstrumentOptions {
            input_source_map: Some(input_sm.to_string()),
            ..InstrumentOptions::default()
        },
    )
    .unwrap()
    .coverage_map
}

#[test]
fn compose_input_source_map_bakes_original_positions_into_preamble() {
    // The point of composing eagerly is that the runtime `__coverage__` (the
    // `coverageData` literal in the preamble injected into `code`) already
    // carries the original-source path, so an E2E collector can dump it
    // verbatim. The preamble keys the coverage object by `coverage.path`.
    let (_, intermediate, input_sm) = three_line_inputs();

    let composed = instrument(
        &intermediate,
        "intermediate.js",
        &InstrumentOptions {
            input_source_map: Some(input_sm.clone()),
            compose_input_source_map: true,
            ..InstrumentOptions::default()
        },
    )
    .unwrap();
    assert!(
        composed.code.contains("var path = \"src/app.ts\""),
        "composed preamble keys runtime coverage by the original source path"
    );

    let lazy = instrument(
        &intermediate,
        "intermediate.js",
        &InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() },
    )
    .unwrap();
    assert!(
        lazy.code.contains("var path = \"intermediate.js\""),
        "without compose, the preamble keeps the generated path"
    );
}

#[test]
fn compose_input_source_map_no_op_without_input_map() {
    // compose_input_source_map is gated on input_source_map being set; with no
    // map there is nothing to compose, so the result is the plain generated map.
    let opts = InstrumentOptions { compose_input_source_map: true, ..InstrumentOptions::default() };
    let result = instrument("const x = 1;", "intermediate.js", &opts).unwrap();
    assert_eq!(result.coverage_map.path, "intermediate.js", "no map -> path unchanged");
    assert!(result.coverage_map.input_source_map.is_none());
}

#[test]
fn compose_input_source_map_off_keeps_embedded_map() {
    // Default (flag off) preserves the legacy behavior: the input map stays
    // embedded for downstream remap and positions remain generated.
    let (_, intermediate, input_sm) = three_line_inputs();
    let opts =
        InstrumentOptions { input_source_map: Some(input_sm), ..InstrumentOptions::default() };
    let result = instrument(&intermediate, "intermediate.js", &opts).unwrap();
    assert_eq!(result.coverage_map.path, "intermediate.js");
    assert!(
        result.coverage_map.input_source_map.is_some(),
        "with compose off, the inputSourceMap stays embedded"
    );
}

#[test]
fn compose_input_source_map_unusable_map_backs_off() {
    // A source map that declares no usable source makes `remap_coverage` return
    // None; eager compose must back off and leave the embedded map in place so
    // the lazy remap path is still available.
    let input_sm = r#"{"version":3,"sources":[],"mappings":"","names":[]}"#;
    let opts = InstrumentOptions {
        input_source_map: Some(input_sm.to_string()),
        compose_input_source_map: true,
        ..InstrumentOptions::default()
    };
    let result = instrument("const x = 1;", "intermediate.js", &opts).unwrap();
    assert_eq!(result.coverage_map.path, "intermediate.js", "unusable map -> path unchanged");
    assert!(
        result.coverage_map.input_source_map.is_some(),
        "unusable map -> embedded map retained for the lazy fallback"
    );
}

#[test]
fn compose_input_source_map_preserves_function_identity_overlay() {
    // The function-identity overlay is built before composition and the remap
    // pipeline does not rewrite it, so the eager overlay must equal the overlay
    // produced by instrument-then-remap. This keeps the `fallow:fn:<hex>` join
    // keys stable across the eager and lazy paths. The lazy side uses the same
    // `drop_unmapped: true` policy the eager path now applies (issue #105) so
    // the parity assertion stays symmetric; the identity map here maps every
    // position, so nothing is actually dropped on either side.
    let intermediate = "function add(a, b) {\n  return a + b;\n}\n";
    let original_ts = "function add(a: number, b: number): number {\n  return a + b;\n}\n";
    let input_sm = format!(
        r#"{{"version":3,"sources":["src/app.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA;AACA;AACA","names":[]}}"#,
    );

    let eager = instrument(
        intermediate,
        "intermediate.js",
        &InstrumentOptions {
            input_source_map: Some(input_sm.clone()),
            compose_input_source_map: true,
            function_identity_overlay: true,
            ..InstrumentOptions::default()
        },
    )
    .unwrap();

    let lazy_instrumented = instrument(
        intermediate,
        "intermediate.js",
        &InstrumentOptions {
            input_source_map: Some(input_sm),
            function_identity_overlay: true,
            ..InstrumentOptions::default()
        },
    )
    .unwrap();
    let lazy = remap_coverage_with_options(
        &lazy_instrumented.coverage_map,
        RemapOptions { drop_unmapped: true },
    )
    .expect("lazy remap succeeds");

    assert!(eager.coverage_map.x_fallow_function_map.is_some(), "overlay survives eager compose");
    assert_eq!(
        serde_json::to_value(&eager.coverage_map.x_fallow_function_map).unwrap(),
        serde_json::to_value(&lazy.x_fallow_function_map).unwrap(),
        "overlay must be identical across eager and lazy paths"
    );
}

#[test]
fn compose_input_source_map_prunes_overlay_for_dropped_functions() {
    // Issue #105 + the function-identity overlay: when eager compose drops an
    // unmapped function, its `x_fallow_functionMap` overlay entry must drop with
    // it so the overlay stays 1:1 with `fnMap` (no orphan identity for a
    // function that no longer exists). The input map only maps generated line 1,
    // so `drop` (declared on line 2) is pruned while `keep` (line 1) survives.
    let intermediate = "function keep() { return 1; }\nfunction drop() { return 2; }\n";
    let original_ts = "export function keep() { return 1; }\n"; // one line
    let input_sm = format!(
        r#"{{"version":3,"sources":["src/app.ts"],"sourcesContent":[{original_ts:?}],"mappings":"AAAA","names":[]}}"#,
    );

    let eager = instrument(
        intermediate,
        "intermediate.js",
        &InstrumentOptions {
            input_source_map: Some(input_sm),
            compose_input_source_map: true,
            function_identity_overlay: true,
            ..InstrumentOptions::default()
        },
    )
    .unwrap();

    let fn_names: Vec<&str> = eager.coverage_map.fn_map.values().map(|f| f.name.as_str()).collect();
    assert_eq!(fn_names, vec!["keep"], "the unmapped function is dropped from fnMap");

    let overlay = eager.coverage_map.x_fallow_function_map.expect("overlay survives eager compose");
    assert_eq!(
        overlay.keys().collect::<Vec<_>>(),
        eager.coverage_map.fn_map.keys().collect::<Vec<_>>(),
        "overlay stays 1:1 with fnMap; the dropped function leaves no orphan entry",
    );
}

#[test]
fn compose_input_source_map_does_not_double_compose_output_source_map() {
    // The coverage map and the output `source_map` are independent artifacts.
    // `compose_input_source_map` rewrites the COVERAGE map; the output source
    // map (instrumented JS -> original source) is composed once by
    // `finalize_source_map` regardless of the flag. So with `source_map: true`,
    // the emitted `source_map` must be byte-identical whether or not
    // `compose_input_source_map` is set: the flag must NOT feed the input map
    // into the source-map chain a second time.
    let (_, intermediate, input_sm) = three_line_inputs();

    let without_compose = instrument(
        &intermediate,
        "intermediate.js",
        &InstrumentOptions {
            source_map: true,
            input_source_map: Some(input_sm.clone()),
            ..InstrumentOptions::default()
        },
    )
    .unwrap();

    let with_compose = instrument(
        &intermediate,
        "intermediate.js",
        &InstrumentOptions {
            source_map: true,
            input_source_map: Some(input_sm),
            compose_input_source_map: true,
            ..InstrumentOptions::default()
        },
    )
    .unwrap();

    let sm_without = without_compose.source_map.expect("source_map emitted");
    let sm_with = with_compose.source_map.expect("source_map emitted");
    assert_eq!(
        sm_with, sm_without,
        "output source_map must not change when the coverage map is composed eagerly"
    );

    // And the output source map chains all the way back to the original source
    // (instrumented JS -> src/app.ts), not just to the intermediate JS.
    let parsed: serde_json::Value = serde_json::from_str(&sm_with).unwrap();
    let sources: Vec<&str> =
        parsed["sources"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
    assert!(
        sources.contains(&"src/app.ts"),
        "output source map must point back at the original source, got: {sources:?}"
    );
}
