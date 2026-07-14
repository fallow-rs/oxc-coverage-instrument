use oxc_coverage_report::{NodeKind, ReportNode};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

const INDEX_FILE: &str = "index.html";
const ROOT_ASSETS: [&str; 3] = ["base.css", "coverage-tokens.css", "base.js"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum NodeType {
    Folder,
    File,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NodeKey {
    relative_path: String,
    node_type: NodeType,
}

#[derive(Clone, Debug)]
struct PhysicalNode {
    output_path: PathBuf,
    href_from_parent: String,
}

pub(super) struct PhysicalPaths {
    nodes: BTreeMap<NodeKey, PhysicalNode>,
}

impl NodeKey {
    fn from_node(node: &ReportNode) -> Self {
        let node_type = match &node.kind {
            NodeKind::Folder { .. } => NodeType::Folder,
            NodeKind::File { .. } => NodeType::File,
        };
        Self { relative_path: node.relative_path.clone(), node_type }
    }
}

impl PhysicalPaths {
    pub(super) fn build(root: &ReportNode) -> io::Result<Self> {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeKey::from_node(root),
            PhysicalNode {
                output_path: PathBuf::from(INDEX_FILE),
                href_from_parent: INDEX_FILE.to_owned(),
            },
        );
        allocate_children(&mut nodes, Path::new(""), root.children(), true)?;
        validate_unique_outputs(&nodes)?;
        Ok(Self { nodes })
    }

    pub(super) fn output_path(&self, node: &ReportNode) -> &Path {
        &self.node(node).output_path
    }

    pub(super) fn href_from_parent(&self, node: &ReportNode) -> &str {
        &self.node(node).href_from_parent
    }

    fn node(&self, node: &ReportNode) -> &PhysicalNode {
        self.nodes
            .get(&NodeKey::from_node(node))
            .expect("physical path mapper was built from a different report tree")
    }
}

fn allocate_children(
    nodes: &mut BTreeMap<NodeKey, PhysicalNode>,
    parent_output_dir: &Path,
    children: &[ReportNode],
    is_root: bool,
) -> io::Result<()> {
    let mut occupied = BTreeSet::from([ascii_case_key(INDEX_FILE)]);
    if is_root {
        occupied.extend(ROOT_ASSETS.map(ascii_case_key));
    }
    let protected_preferred: BTreeSet<String> = children
        .iter()
        .map(|child| match &child.kind {
            NodeKind::Folder { .. } => child.name.clone(),
            NodeKind::File { .. } => format!("{}.html", child.name),
        })
        .map(|preferred| ascii_case_key(&preferred))
        .collect();

    for child in children.iter().filter(|child| matches!(child.kind, NodeKind::Folder { .. })) {
        let leaf = allocate_leaf(&mut occupied, &protected_preferred, &child.name, "dir", "");
        let output_dir = parent_output_dir.join(&leaf);
        nodes.insert(
            NodeKey::from_node(child),
            PhysicalNode {
                output_path: output_dir.join(INDEX_FILE),
                href_from_parent: format!("{}/{INDEX_FILE}", percent_encode_segment(&leaf)),
            },
        );
        allocate_children(nodes, &output_dir, child.children(), false)?;
    }

    for child in children.iter().filter(|child| matches!(child.kind, NodeKind::File { .. })) {
        let preferred = format!("{}.html", child.name);
        let leaf = allocate_leaf(&mut occupied, &protected_preferred, &preferred, "file", ".html");
        nodes.insert(
            NodeKey::from_node(child),
            PhysicalNode {
                output_path: parent_output_dir.join(&leaf),
                href_from_parent: percent_encode_segment(&leaf),
            },
        );
    }
    Ok(())
}

fn allocate_leaf(
    occupied: &mut BTreeSet<String>,
    protected_preferred: &BTreeSet<String>,
    preferred: &str,
    kind: &str,
    suffix: &str,
) -> String {
    if occupied.insert(ascii_case_key(preferred)) {
        return preferred.to_owned();
    }

    let stem = preferred.strip_suffix(suffix).unwrap_or(preferred);
    for attempt in 1u32.. {
        let candidate = format!("{stem}.oxc-{kind}-{attempt}{suffix}");
        let key = ascii_case_key(&candidate);
        if protected_preferred.contains(&key) || occupied.contains(&key) {
            continue;
        }
        occupied.insert(key);
        return candidate;
    }
    unreachable!("u32 collision suffix space is sufficient for a finite report tree")
}

fn validate_unique_outputs(nodes: &BTreeMap<NodeKey, PhysicalNode>) -> io::Result<()> {
    let mut outputs = BTreeSet::new();
    for asset in ROOT_ASSETS {
        outputs.insert(ascii_case_key(asset));
    }
    for physical in nodes.values() {
        validate_safe_output_path(&physical.output_path)?;
        if !outputs.insert(output_comparison_key(&physical.output_path)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "HTML report output collision remained after path mapping: {}",
                    physical.output_path.display(),
                ),
            ));
        }
    }
    Ok(())
}

fn ascii_case_key(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn output_comparison_key(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(component) => component.to_str(),
            _ => None,
        })
        .map(ascii_case_key)
        .collect::<Vec<_>>()
        .join("/")
}

fn percent_encode_segment(segment: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

fn validate_safe_output_path(path: &Path) -> io::Result<()> {
    let safe = path.components().all(|component| match component {
        std::path::Component::Normal(component) => {
            component.to_str().is_some_and(super::is_safe_report_path_component)
        }
        _ => false,
    });
    if safe {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("unsafe mapped HTML report path: {}", path.display()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_report::{ReportNode, summarize};
    use oxc_coverage_types::parse_coverage_map;
    use std::path::Path;

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

        assert_eq!(
            paths.output_path(root_file),
            Path::new("root #?% ü.js.html"),
            "safe physical leaves must remain unchanged",
        );
        assert_eq!(paths.href_from_parent(root_file), "root%20%23%3F%25%20%C3%BC.js.html",);
        assert_eq!(paths.href_from_parent(folder), "folder%20%23%3F%25%20%C3%BC/index.html",);
        assert_eq!(paths.href_from_parent(nested_file), "nested%20%23%3F%25%20%C3%BC.js.html",);
    }
}
