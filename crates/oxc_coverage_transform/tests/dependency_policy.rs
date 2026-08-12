use std::{collections::BTreeSet, fs, path::Path, process::Command};

fn workspace_metadata() -> serde_json::Value {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps", "--manifest-path"])
        .arg(manifest)
        .output()
        .expect("run cargo metadata");
    assert!(output.status.success(), "cargo metadata failed");
    serde_json::from_slice(&output.stdout).expect("parse cargo metadata")
}

fn package<'a>(metadata: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    metadata["packages"]
        .as_array()
        .and_then(|packages| packages.iter().find(|package| package["name"] == name))
        .unwrap_or_else(|| panic!("missing workspace package {name}"))
}

#[test]
fn normal_dependencies_stay_inside_the_ast_kernel_boundary() {
    let metadata = workspace_metadata();
    let package = package(&metadata, "oxc_coverage_transform");
    let dependencies = package["dependencies"]
        .as_array()
        .expect("package dependencies")
        .iter()
        .filter(|dependency| dependency["kind"].is_null())
        .map(|dependency| dependency["name"].as_str().expect("dependency name"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::from([
        "oxc_allocator",
        "oxc_ast",
        "oxc_semantic",
        "oxc_span",
        "oxc_syntax",
        "oxc_traverse",
    ]);

    assert_eq!(dependencies, expected);
}

#[test]
fn satellite_compatibility_is_opt_in_from_the_instrument_adapter() {
    let metadata = workspace_metadata();
    let kernel = package(&metadata, "oxc_coverage_transform");
    assert_eq!(kernel["features"]["default"], serde_json::json!([]));
    assert_eq!(kernel["features"]["satellite-eager-compose"], serde_json::json!([]));

    let instrument = package(&metadata, "oxc_coverage_instrument");
    let transform_dependency = instrument["dependencies"]
        .as_array()
        .and_then(|dependencies| {
            dependencies.iter().find(|dependency| dependency["name"] == "oxc_coverage_transform")
        })
        .expect("instrument transform dependency");
    assert_eq!(transform_dependency["features"], serde_json::json!(["satellite-eager-compose"]));
}

#[test]
fn kernel_source_does_not_import_satellite_layers() {
    const FORBIDDEN: &[&str] = &[
        "oxc_coverage_types",
        "oxc_coverage_source_maps",
        "oxc_coverage_v8",
        "oxc_coverage_report",
        "serde_json",
        "sha2",
        "oxc_parser",
        "oxc_codegen",
        "oxc_transformer",
    ];

    fn inspect(path: &Path) {
        for entry in fs::read_dir(path).expect("read kernel source directory") {
            let path = entry.expect("read kernel source entry").path();
            if path.is_dir() {
                inspect(&path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                let source = fs::read_to_string(&path).expect("read kernel source file");
                for forbidden in FORBIDDEN {
                    assert!(
                        !source.contains(forbidden),
                        "{} imports forbidden satellite dependency {forbidden}",
                        path.display(),
                    );
                }
            }
        }
    }

    inspect(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
}
