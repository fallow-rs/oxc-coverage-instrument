use oxc_coverage_instrument::{CompatProfile, InstrumentOptions, instrument};

fn istanbul(source: &str) -> oxc_coverage_instrument::InstrumentResult {
    let options =
        InstrumentOptions { compat: Some(CompatProfile::Istanbul), ..InstrumentOptions::default() };
    instrument(source, "compat.js", &options).expect("instrument with Istanbul profile")
}

#[test]
fn istanbul_profile_skips_logical_assignment_branches() {
    for source in ["let x; x ??= 1;", "let x; x ||= 1;", "let x = 1; x &&= 2;"] {
        let result = istanbul(source);
        assert!(result.coverage_map.branch_map.is_empty(), "unexpected branch for {source}");
    }
}

#[test]
fn istanbul_profile_uses_anonymous_names_for_inferred_functions() {
    let result = istanbul(
        "const f = function () {}; const g = () => 1; class C { method() {} } const o = { run() {} };",
    );
    let names =
        result.coverage_map.fn_map.values().map(|entry| entry.name.as_str()).collect::<Vec<_>>();

    assert_eq!(names, ["(anonymous_0)", "(anonymous_1)", "(anonymous_2)", "(anonymous_3)"]);
}

#[test]
fn istanbul_profile_keeps_anonymous_default_export_anonymous() {
    let result = istanbul("export default function () {}");

    assert_eq!(result.coverage_map.fn_map["0"].name, "(anonymous_0)");
}

#[test]
fn istanbul_profile_keeps_explicit_function_names() {
    let result = istanbul("function declared() {} const value = function explicit() {};");

    assert_eq!(result.coverage_map.fn_map["0"].name, "declared");
    assert_eq!(result.coverage_map.fn_map["1"].name, "explicit");
}

#[test]
fn istanbul_profile_overrides_callback_name_extension() {
    let options = InstrumentOptions {
        compat: Some(CompatProfile::Istanbul),
        name_callback_arguments: true,
        ..InstrumentOptions::default()
    };
    let result = instrument("items.map(() => 1);", "compat.js", &options).expect("instrument");

    assert_eq!(result.coverage_map.fn_map["0"].name, "(anonymous_0)");
}

#[test]
fn istanbul_profile_truncates_method_declaration_spans() {
    let result = istanbul("class C { method() {} }");

    let entry = &result.coverage_map.fn_map["0"];
    assert_eq!(entry.decl.end.column, entry.decl.start.column + 1);
}

#[test]
fn istanbul_profile_anchors_object_method_declarations_at_the_key() {
    let source = "const o = { execute() {} };";
    let result = istanbul(source);
    let entry = &result.coverage_map.fn_map["0"];
    let expected_start = u32::try_from(source.find("execute").expect("method key")).unwrap();

    assert_eq!(entry.decl.start.column, expected_start);
    assert_eq!(entry.decl.end.column, expected_start + 1);
}

#[test]
fn istanbul_profile_leaves_optional_chains_native() {
    let result = istanbul("const value = object?.nested?.value;");

    assert!(result.coverage_map.branch_map.is_empty());
    assert!(!result.code.contains("_oc("));
}

#[test]
fn istanbul_profile_emits_empty_coordinates_for_synthetic_else() {
    let result = istanbul("if (value) work();");
    let value: serde_json::Value =
        serde_json::from_str(&result.coverage_map_json).expect("coverage JSON");
    let alternate = &value["branchMap"]["0"]["locations"][1];

    assert_eq!(alternate["start"], serde_json::json!({}));
    assert_eq!(alternate["end"], serde_json::json!({}));
}

#[test]
fn default_profile_keeps_oxc_extensions() {
    let result = instrument(
        "let x; x ??= 1; const f = function () {}; class C { method() {} } const value = object?.x;",
        "default.js",
        &InstrumentOptions::default(),
    )
    .expect("instrument with defaults");

    assert!(
        result.coverage_map.branch_map.values().any(|entry| entry.branch_type == "binary-expr")
    );
    assert!(
        result.coverage_map.branch_map.values().any(|entry| entry.branch_type == "optional-chain")
    );
    assert_eq!(result.coverage_map.fn_map["0"].name, "f");
    assert_eq!(result.coverage_map.fn_map["1"].name, "method");
    assert!(
        result.coverage_map.fn_map["1"].decl.end.column
            > result.coverage_map.fn_map["1"].decl.start.column + 1
    );
}
