//! Tests for the physical-path mapping: output paths, collision suffixes,
//! portable component rewriting, and the hrefs that link the pages together.

use super::*;
use oxc_coverage_report::{ReportNode, summarize};
use oxc_coverage_types::parse_coverage_map;
use std::path::Path;

const PORTABLE_COMPONENT_LIMIT: usize = 240;

fn tree(json: &str) -> ReportNode {
    let map = parse_coverage_map(json).unwrap();
    summarize(&map)
}

fn find<'a>(node: &'a ReportNode, relative_path: &str, node_type: NodeType) -> &'a ReportNode {
    find_optional(node, relative_path, node_type).expect("requested report node exists")
}

fn find_optional<'a>(
    node: &'a ReportNode,
    relative_path: &str,
    node_type: NodeType,
) -> Option<&'a ReportNode> {
    let current_type = if node.is_folder() { NodeType::Folder } else { NodeType::File };
    if node.relative_path == relative_path && current_type == node_type {
        return Some(node);
    }
    node.children().iter().find_map(|child| find_optional(child, relative_path, node_type))
}

fn coverage_entry(path: &str) -> String {
    format!(
        r#""{path}":{{"path":"{path}","statementMap":{{}},"fnMap":{{}},"branchMap":{{}},"s":{{}},"f":{{}},"b":{{}}}}"#,
    )
}

fn assert_portable_outputs(paths: &PhysicalPaths) {
    for physical in paths.nodes.values() {
        for component in physical.output_path.components() {
            let component = component.as_os_str().to_str().unwrap();
            assert!(component.is_ascii(), "non-ASCII output component: {component:?}");
            assert!(
                component.len() <= PORTABLE_COMPONENT_LIMIT,
                "overlong output component: {} bytes",
                component.len(),
            );
            assert!(
                !component
                    .bytes()
                    .any(|byte| { byte < b' ' || byte == 0x7f || b"<>:\"/\\|?*".contains(&byte) }),
                "nonportable output component: {component:?}",
            );
            assert!(!component.ends_with(['.', ' ']), "trailing alias: {component:?}");

            let stem = ascii_case_key(component.split('.').next().unwrap());
            let reserved = matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
                || stem.strip_prefix("com").or_else(|| stem.strip_prefix("lpt")).is_some_and(
                    |digit| digit.len() == 1 && matches!(digit.as_bytes()[0], b'1'..=b'9'),
                );
            assert!(!reserved, "Windows device output component: {component:?}");
        }
    }
}

#[test]
fn maps_root_and_nested_index_files_away_from_folder_indexes() {
    let root = tree(
        r#"{
          "index":{"path":"index","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "src/index":{"path":"src/index","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();
    let root_index_file = find(&root, "index", NodeType::File);
    let nested_index_file = find(&root, "src/index", NodeType::File);
    assert_eq!(paths.output_path(root_index_file), Path::new("index.oxc-file-1.html"));
    assert_eq!(paths.output_path(nested_index_file), Path::new("src/index.oxc-file-1.html"));
}

#[test]
fn preserves_ordinary_portable_files_and_folders() {
    let root = tree(
        r#"{
          "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "src/ordinary.js":{"path":"src/ordinary.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();

    assert_eq!(
        paths.output_path(find(&root, "keep.js", NodeType::File)),
        Path::new("keep.js.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "src", NodeType::Folder)),
        Path::new("src/index.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "src/ordinary.js", NodeType::File)),
        Path::new("src/ordinary.js.html"),
    );
}

#[test]
fn maps_ascii_case_variants_away_from_reserved_root_names() {
    let root = tree(
        r#"{
          "INDEX":{"path":"INDEX","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "BASE.CSS/a.js":{"path":"BASE.CSS/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();
    assert_eq!(
        paths.output_path(find(&root, "INDEX", NodeType::File)),
        Path::new("INDEX.oxc-file-1.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "BASE.CSS", NodeType::Folder)),
        Path::new("BASE.CSS.oxc-dir-1/index.html"),
    );
}

#[test]
fn maps_ascii_case_variants_away_from_nested_index_and_sibling_names() {
    let root = tree(
        r#"{
          "root.js":{"path":"root.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "src/INDEX":{"path":"src/INDEX","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "src/A.js":{"path":"src/A.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "src/a.js":{"path":"src/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();
    assert_eq!(
        paths.output_path(find(&root, "src/INDEX", NodeType::File)),
        Path::new("src/INDEX.oxc-file-1.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "src/A.js", NodeType::File)),
        Path::new("src/A.js.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "src/a.js", NodeType::File)),
        Path::new("src/a.js.oxc-file-1.html"),
    );
}

#[test]
fn maps_root_asset_named_folders_but_preserves_nested_and_ordinary_folders() {
    let root = tree(
        r#"{
          "root.js":{"path":"root.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "base.css/a.js":{"path":"base.css/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "base.js/a.js":{"path":"base.js/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "coverage-tokens.css/a.js":{"path":"coverage-tokens.css/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "ordinary/a.js":{"path":"ordinary/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "nested/base.css/a.js":{"path":"nested/base.css/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();
    assert_eq!(
        paths.output_path(find(&root, "base.css", NodeType::Folder)),
        Path::new("base.css.oxc-dir-1/index.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "base.js", NodeType::Folder)),
        Path::new("base.js.oxc-dir-1/index.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "coverage-tokens.css", NodeType::Folder)),
        Path::new("coverage-tokens.css.oxc-dir-1/index.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "ordinary", NodeType::Folder)),
        Path::new("ordinary/index.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "nested/base.css", NodeType::Folder)),
        Path::new("nested/base.css/index.html"),
    );
}

#[test]
fn resolves_secondary_collisions_deterministically() {
    let root = tree(
        r#"{
          "index":{"path":"index","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "index.oxc-file-1":{"path":"index.oxc-file-1","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let first = PhysicalPaths::build(&root).unwrap();
    let second = PhysicalPaths::build(&root).unwrap();
    let index = find(&root, "index", NodeType::File);
    let natural_candidate = find(&root, "index.oxc-file-1", NodeType::File);
    assert_eq!(first.output_path(index), Path::new("index.oxc-file-2.html"));
    assert_eq!(first.output_path(natural_candidate), Path::new("index.oxc-file-1.html"));
    assert_eq!(first.output_path(index), second.output_path(index));
    assert_eq!(first.output_path(natural_candidate), second.output_path(natural_candidate));
}

#[test]
fn maps_windows_illegal_and_control_bytes_to_portable_ascii() {
    let root = tree(
        r#"{
          "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "bad\u0001\u007f<name>:\"pipe|query?star*.js":{"path":"bad\u0001\u007f<name>:\"pipe|query?star*.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "nested?folder/a.js":{"path":"nested?folder/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();

    assert_portable_outputs(&paths);
    assert_eq!(
        paths.output_path(find(&root, "nested?folder", NodeType::Folder)),
        Path::new("nested_x3F_folder/index.html"),
    );
}

#[test]
fn maps_windows_device_stems_case_insensitively_with_extensions() {
    let mut devices = vec!["CON".to_owned(), "prn".to_owned(), "AuX".to_owned(), "nul".to_owned()];
    for prefix in ["COM", "lPt"] {
        devices.extend((1..=9).map(|digit| format!("{prefix}{digit}")));
    }
    let mut entries =
        vec![coverage_entry("keep.js"), coverage_entry("CoN"), coverage_entry("PrN.txt")];
    entries.extend(devices.iter().map(|device| coverage_entry(&format!("{device}.txt/a.js"))));
    let root = tree(&format!("{{{}}}", entries.join(",")));
    let paths = PhysicalPaths::build(&root).unwrap();

    assert_portable_outputs(&paths);
    assert_eq!(paths.output_path(find(&root, "CoN", NodeType::File)), Path::new("_x43_oN.html"),);
    assert_eq!(
        paths.output_path(find(&root, "PrN.txt", NodeType::File)),
        Path::new("_x50_rN.txt.html"),
    );
    for device in devices {
        let logical = format!("{device}.txt");
        let physical = paths.output_path(find(&root, &logical, NodeType::Folder));
        assert_ne!(physical.parent().unwrap(), Path::new(&logical));
    }
}

#[test]
fn maps_trailing_dot_and_space_aliases_away_from_siblings_and_assets() {
    let root = tree(
        r#"{
          "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "alias/a.js":{"path":"alias/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "alias./b.js":{"path":"alias./b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "alias /c.js":{"path":"alias /c.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "base.css./d.js":{"path":"base.css./d.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "base.css /e.js":{"path":"base.css /e.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();

    assert_portable_outputs(&paths);
    assert_eq!(
        paths.output_path(find(&root, "alias", NodeType::Folder)),
        Path::new("alias/index.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "alias.", NodeType::Folder)),
        Path::new("alias_x2E_/index.html"),
    );
    assert_eq!(
        paths.output_path(find(&root, "alias ", NodeType::Folder)),
        Path::new("alias_x20_/index.html"),
    );
}

#[test]
fn maps_unicode_normalization_and_case_aliases_to_distinct_ascii() {
    let names = ["é", "e\u{301}", "É"];
    let mut entries = vec![coverage_entry("keep.js")];
    entries.extend(names.iter().map(|name| coverage_entry(&format!("{name}/a.js"))));
    let root = tree(&format!("{{{}}}", entries.join(",")));
    let paths = PhysicalPaths::build(&root).unwrap();
    let outputs: BTreeSet<PathBuf> = names
        .iter()
        .map(|name| paths.output_path(find(&root, name, NodeType::Folder)).to_owned())
        .collect();

    assert_portable_outputs(&paths);
    assert_eq!(outputs.len(), names.len());
}

#[test]
fn portable_escape_looking_names_keep_their_leaves_during_collisions() {
    let root = tree(
        r#"{
          "bad?name.js":{"path":"bad?name.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "bad_x3F_name.js":{"path":"bad_x3F_name.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "bad_x3F_name.js.oxc-file-1":{"path":"bad_x3F_name.js.oxc-file-1","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "bad?dir/a.js":{"path":"bad?dir/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "bad_x3F_dir/b.js":{"path":"bad_x3F_dir/b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "bad_x3F_dir.oxc-dir-1/c.js":{"path":"bad_x3F_dir.oxc-dir-1/c.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let first = PhysicalPaths::build(&root).unwrap();
    let second = PhysicalPaths::build(&root).unwrap();

    assert_eq!(
        first.output_path(find(&root, "bad_x3F_name.js", NodeType::File)),
        Path::new("bad_x3F_name.js.html"),
    );
    assert_eq!(
        first.output_path(find(&root, "bad?name.js", NodeType::File)),
        Path::new("bad_x3F_name.js.oxc-file-2.html"),
    );
    assert_eq!(
        first.output_path(find(&root, "bad_x3F_dir", NodeType::Folder)),
        Path::new("bad_x3F_dir/index.html"),
    );
    assert_eq!(
        first.output_path(find(&root, "bad?dir", NodeType::Folder)),
        Path::new("bad_x3F_dir.oxc-dir-2/index.html"),
    );
    assert_eq!(first.nodes.keys().collect::<Vec<_>>(), second.nodes.keys().collect::<Vec<_>>());
    for key in first.nodes.keys() {
        assert_eq!(first.nodes[key].output_path, second.nodes[key].output_path);
    }
}

#[test]
fn bounds_encoded_preferred_and_collision_components() {
    let overlong_ascii = "a".repeat(400);
    let expanded_unicode = "é".repeat(100);
    let case_alias_lower = "b".repeat(235);
    let case_alias_upper = "B".repeat(235);
    let entries = [
        coverage_entry("keep.js"),
        coverage_entry(&format!("{overlong_ascii}/a.js")),
        coverage_entry(&format!("{expanded_unicode}/b.js")),
        coverage_entry(&format!("{case_alias_lower}/c.js")),
        coverage_entry(&format!("{case_alias_upper}/d.js")),
    ];
    let root = tree(&format!("{{{}}}", entries.join(",")));
    let paths = PhysicalPaths::build(&root).unwrap();

    assert_portable_outputs(&paths);
    assert_eq!(
        paths
            .output_path(find(&root, &case_alias_lower, NodeType::Folder))
            .parent()
            .unwrap()
            .as_os_str()
            .len(),
        PORTABLE_COMPONENT_LIMIT,
    );
}

#[test]
fn resolves_bounded_hash_and_identical_truncation_collisions() {
    let overlong = "z".repeat(400);
    let bounded = portable_leaf(&overlong, "");
    let first_fallback = collision_leaf(&bounded.value, "dir", "", 1);
    let entries = [
        coverage_entry("keep.js"),
        coverage_entry(&format!("{overlong}/a.js")),
        coverage_entry(&format!("{}/b.js", bounded.value)),
        coverage_entry(&format!("{first_fallback}/c.js")),
    ];
    let root = tree(&format!("{{{}}}", entries.join(",")));
    let paths = PhysicalPaths::build(&root).unwrap();

    assert_eq!(
        paths.output_path(find(&root, &bounded.value, NodeType::Folder)),
        Path::new(&bounded.value).join(INDEX_FILE),
    );
    assert_eq!(
        paths.output_path(find(&root, &first_fallback, NodeType::Folder)),
        Path::new(&first_fallback).join(INDEX_FILE),
    );
    assert_eq!(
        paths.output_path(find(&root, &overlong, NodeType::Folder)),
        Path::new(&collision_leaf(&bounded.value, "dir", "", 2)).join(INDEX_FILE),
    );
    assert_portable_outputs(&paths);

    let mut occupied = BTreeSet::new();
    let protected_preferred = BTreeSet::from([ascii_case_key(&bounded.value)]);
    let protected_ordinary = BTreeSet::new();
    let mut request = || {
        allocate_leaf(LeafRequest {
            occupied: &mut occupied,
            protected_preferred: &protected_preferred,
            protected_ordinary: &protected_ordinary,
            preferred: &bounded,
            kind: "dir",
            suffix: "",
        })
    };
    let first = request();
    let second = request();
    assert_eq!(first, bounded.value);
    assert_eq!(second, collision_leaf(&first, "dir", "", 1));
    assert_ne!(ascii_case_key(&first), ascii_case_key(&second));
    assert!(second.len() <= PORTABLE_COMPONENT_LIMIT);
}

#[test]
fn hrefs_percent_encode_each_physical_path_segment() {
    let root = tree(
        r#"{
          "root #?% ü.js":{"path":"root #?% ü.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
          "folder #?% ü/nested #?% ü.js":{"path":"folder #?% ü/nested #?% ü.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
        }"#,
    );
    let paths = PhysicalPaths::build(&root).unwrap();
    let root_file = find(&root, "root #?% ü.js", NodeType::File);
    let folder = find(&root, "folder #?% ü", NodeType::Folder);
    let nested_file = find(&root, "folder #?% ü/nested #?% ü.js", NodeType::File);

    assert_eq!(paths.output_path(root_file), Path::new("root #_x3F_% _xC3__xBC_.js.html"),);
    assert_eq!(paths.href_from_parent(root_file), "root%20%23_x3F_%25%20_xC3__xBC_.js.html",);
    assert_eq!(paths.href_from_parent(folder), "folder%20%23_x3F_%25%20_xC3__xBC_/index.html",);
    assert_eq!(paths.href_from_parent(nested_file), "nested%20%23_x3F_%25%20_xC3__xBC_.js.html",);
}
