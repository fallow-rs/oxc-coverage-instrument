//! Tests for `InstrumentOptions::function_identity_overlay`. Validates that
//! the opt-in `x_fallow_functionMap` overlay carries stable, deterministic
//! ids covering every function shape (named, anonymous, same-line, class
//! method, object literal method, arrow) and that the TS-direct strip path
//! preserves the original TypeScript positions in the identity inputs.

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn opts_with_overlay() -> InstrumentOptions {
    InstrumentOptions { function_identity_overlay: true, ..InstrumentOptions::default() }
}

#[test]
fn overlay_absent_by_default() {
    let src = "function add(a, b) { return a + b; }\n";
    let result = instrument(src, "app.js", &InstrumentOptions::default()).expect("instrument");
    assert!(
        result.coverage_map.x_fallow_function_map.is_none(),
        "default output must not carry the overlay; Istanbul consumers see byte-identical shape"
    );
    assert!(
        !result.coverage_map_json.contains("x_fallow_functionMap"),
        "the JSON form must also omit the overlay key by default"
    );
}

#[test]
fn overlay_keys_align_with_fn_map() {
    let src = "function add(a, b) { return a + b; }\n\
              function sub(a, b) { return a - b; }\n";
    let result = instrument(src, "app.js", &opts_with_overlay()).expect("instrument");
    let overlay = result.coverage_map.x_fallow_function_map.expect("overlay present");
    assert_eq!(overlay.len(), result.coverage_map.fn_map.len());
    for key in result.coverage_map.fn_map.keys() {
        assert!(overlay.contains_key(key), "overlay missing key {key} present in fn_map");
    }
}

#[test]
fn overlay_id_is_deterministic_across_runs() {
    let src = "function handler(req) { return req.body; }\n";
    let r1 = instrument(src, "app.js", &opts_with_overlay()).expect("instrument 1");
    let r2 = instrument(src, "app.js", &opts_with_overlay()).expect("instrument 2");
    let id1 = &r1.coverage_map.x_fallow_function_map.as_ref().unwrap()["0"].id;
    let id2 = &r2.coverage_map.x_fallow_function_map.as_ref().unwrap()["0"].id;
    assert_eq!(id1, id2, "identical source must produce identical ids");
    assert!(id1.starts_with("fallow:fn:"), "id format must be fallow:fn:<hex>, got {id1}");
}

#[test]
fn overlay_id_changes_on_body_or_position_edit() {
    let original =
        instrument("function handler(req) { return req.body; }\n", "app.js", &opts_with_overlay())
            .expect("instrument original")
            .coverage_map
            .x_fallow_function_map
            .unwrap()["0"]
            .id
            .clone();

    // Insert a blank line before the function so positions shift.
    let shifted = instrument(
        "\nfunction handler(req) { return req.body; }\n",
        "app.js",
        &opts_with_overlay(),
    )
    .expect("instrument shifted")
    .coverage_map
    .x_fallow_function_map
    .unwrap()["0"]
        .id
        .clone();

    assert_ne!(
        original, shifted,
        "shifting the function down a line must change the identity hash"
    );
}

#[test]
fn overlay_same_line_functions_get_distinct_ids() {
    let src = "function a() {}; function b() {}\n";
    let result = instrument(src, "app.js", &opts_with_overlay()).expect("instrument");
    let overlay = result.coverage_map.x_fallow_function_map.expect("overlay");
    assert_eq!(overlay.len(), 2, "expected two functions on one line");
    let id_a = &overlay["0"].id;
    let id_b = &overlay["1"].id;
    assert_ne!(id_a, id_b, "two same-line functions must hash to distinct ids");
}

#[test]
fn overlay_covers_class_methods_and_arrow_and_object_method() {
    let src = "class C { foo() { return 1; } }\n\
              const obj = { bar() { return 2; } };\n\
              const baz = (x) => x + 1;\n";
    let result = instrument(src, "app.js", &opts_with_overlay()).expect("instrument");
    let overlay = result.coverage_map.x_fallow_function_map.expect("overlay");
    // The fn_map carries one entry per function form; the overlay must mirror it 1:1.
    assert_eq!(overlay.len(), result.coverage_map.fn_map.len());
    let names: Vec<&str> = overlay.values().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"foo"), "class method named foo must surface, got {names:?}");
    assert!(names.contains(&"bar"), "object literal method named bar must surface, got {names:?}");
    assert!(names.contains(&"baz"), "named arrow baz must surface, got {names:?}");
    for entry in overlay.values() {
        assert!(entry.id.starts_with("fallow:fn:"));
        assert_eq!(entry.path, "app.js");
    }
}

#[test]
fn overlay_anonymous_function_gets_id() {
    let src = "const handler = function () { return 1; };\n";
    let result = instrument(src, "app.js", &opts_with_overlay()).expect("instrument");
    let overlay = result.coverage_map.x_fallow_function_map.expect("overlay");
    assert_eq!(overlay.len(), 1);
    let entry = overlay.values().next().unwrap();
    assert!(entry.id.starts_with("fallow:fn:"), "anonymous function must still get a stable id");
}

#[test]
fn overlay_ts_direct_preserves_original_positions() {
    let src = "function add(a: number, b: number): number {\n  return a + b;\n}\n";
    let opts = InstrumentOptions {
        strip_typescript: true,
        function_identity_overlay: true,
        ..InstrumentOptions::default()
    };
    let result = instrument(src, "app.ts", &opts).expect("instrument");
    let overlay = result.coverage_map.x_fallow_function_map.expect("overlay");
    let entry = &overlay["0"];
    assert_eq!(entry.path, "app.ts");
    assert_eq!(entry.decl.start.line, 1, "decl must anchor on the original TS source line");
    assert!(
        entry.id.starts_with("fallow:fn:"),
        "id must be present even when the strip pass rewrites the AST"
    );
}

#[test]
fn overlay_path_change_changes_id() {
    let src = "function handler() { return 1; }\n";
    let id1 = instrument(src, "app.js", &opts_with_overlay())
        .unwrap()
        .coverage_map
        .x_fallow_function_map
        .unwrap()["0"]
        .id
        .clone();
    let id2 = instrument(src, "other.js", &opts_with_overlay())
        .unwrap()
        .coverage_map
        .x_fallow_function_map
        .unwrap()["0"]
        .id
        .clone();
    assert_ne!(id1, id2, "different paths must hash to distinct ids");
}

#[test]
fn overlay_handles_computed_key_method_with_special_chars_in_name() {
    // Smoke test for the end-to-end path: a computed-key method whose
    // name contains characters that would be ambiguous under a flat
    // string separator must still produce a valid `fallow:fn:<hex>` id.
    // The true collision-resistance regression test lives in the unit
    // tests under `crates/oxc-coverage-instrument/src/coverage_builder.rs`
    // where two synthetic FnEntry inputs are constructed to align under
    // the prior `|`-concat shape and prove they hash differently under
    // the JSON-encoded one. This integration test only proves "weird
    // names don't crash the integration".
    let src = "class C { ['x|y']() { return 1; } }\n";
    let result = instrument(src, "app.js", &opts_with_overlay()).expect("instrument");
    let overlay = result.coverage_map.x_fallow_function_map.expect("overlay");
    assert_eq!(overlay.len(), 1);
    let entry = overlay.values().next().unwrap();
    assert_eq!(entry.name, "x|y", "computed-key name must surface verbatim");
    assert!(entry.id.starts_with("fallow:fn:"), "id must be present for weird names");
}

#[test]
fn overlay_serializes_under_x_fallow_key() {
    let src = "function handler() { return 1; }\n";
    let result = instrument(src, "app.js", &opts_with_overlay()).expect("instrument");
    assert!(
        result.coverage_map_json.contains("\"x_fallow_functionMap\""),
        "serialized JSON must use the x_fallow_functionMap key so Istanbul consumers ignore it"
    );
    assert!(
        result.coverage_map_json.contains("fallow:fn:"),
        "serialized JSON must carry the fallow:fn: id prefix"
    );
}
