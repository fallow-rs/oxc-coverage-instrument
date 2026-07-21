use std::{collections::BTreeSet, fs, path::Path};

#[test]
fn normal_dependencies_stay_inside_the_ast_kernel_boundary() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split_once("[dependencies]")
        .expect("dependencies section")
        .1
        .split_once("[dev-dependencies]")
        .expect("dev-dependencies section")
        .0
        .lines()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name.trim()))
        .filter(|name| !name.is_empty())
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

    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(source_root).expect("read kernel source directory") {
        let path = entry.expect("read kernel source entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
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
