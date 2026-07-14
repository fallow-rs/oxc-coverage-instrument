use oxc_coverage_report::{NodeKind, ReportNode};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

const INDEX_FILE: &str = "index.html";
const ROOT_ASSETS: [&str; 3] = ["base.css", "coverage-tokens.css", "base.js"];
// Common target filesystems allow 255 bytes or ASCII code units per component.
// Keep headroom while retaining readable collision suffixes.
const MAX_COMPONENT_LEN: usize = 240;
const MAX_ENCODED_PREFIX_LEN: usize = MAX_COMPONENT_LEN + 1;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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

#[derive(Clone, Debug)]
struct PortableLeaf {
    value: String,
    unchanged: bool,
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
    let preferences: BTreeMap<NodeKey, PortableLeaf> = children
        .iter()
        .map(|child| {
            let (preferred, suffix) = match &child.kind {
                NodeKind::Folder { .. } => (child.name.clone(), ""),
                NodeKind::File { .. } => (format!("{}.html", child.name), ".html"),
            };
            (NodeKey::from_node(child), portable_leaf(&preferred, suffix))
        })
        .collect();
    let protected_preferred: BTreeSet<String> =
        preferences.values().map(|preferred| ascii_case_key(&preferred.value)).collect();
    let protected_ordinary: BTreeSet<String> = preferences
        .values()
        .filter(|preferred| preferred.unchanged)
        .map(|preferred| ascii_case_key(&preferred.value))
        .collect();

    for child in children.iter().filter(|child| matches!(child.kind, NodeKind::Folder { .. })) {
        let preferred = &preferences[&NodeKey::from_node(child)];
        let leaf = allocate_leaf(
            &mut occupied,
            &protected_preferred,
            &protected_ordinary,
            preferred,
            "dir",
            "",
        );
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
        let preferred = &preferences[&NodeKey::from_node(child)];
        let leaf = allocate_leaf(
            &mut occupied,
            &protected_preferred,
            &protected_ordinary,
            preferred,
            "file",
            ".html",
        );
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
    protected_ordinary: &BTreeSet<String>,
    preferred: &PortableLeaf,
    kind: &str,
    suffix: &str,
) -> String {
    let preferred_key = ascii_case_key(&preferred.value);
    if (preferred.unchanged || !protected_ordinary.contains(&preferred_key))
        && occupied.insert(preferred_key)
    {
        return preferred.value.clone();
    }

    for attempt in 1u32.. {
        let candidate = collision_leaf(&preferred.value, kind, suffix, attempt);
        let key = ascii_case_key(&candidate);
        if protected_preferred.contains(&key) || occupied.contains(&key) {
            continue;
        }
        occupied.insert(key);
        return candidate;
    }
    unreachable!("u32 collision suffix space is sufficient for a finite report tree")
}

fn portable_leaf(preferred: &str, suffix: &str) -> PortableLeaf {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let disarm_device = is_windows_device_name(preferred);
    let trailing_alias_start = preferred.trim_end_matches(['.', ' ']).len();
    let mut encoded = String::with_capacity(preferred.len().min(MAX_ENCODED_PREFIX_LEN));
    let mut encoded_len = 0usize;
    let mut hash = FNV_OFFSET_BASIS;
    let mut changed = false;
    for (index, byte) in preferred.bytes().enumerate() {
        hash = update_name_hash(hash, byte);
        let escape = !is_portable_ascii_byte(byte)
            || index >= trailing_alias_start
            || (disarm_device && index == 0);
        if escape {
            let escaped =
                [b'_', b'x', HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0x0f)], b'_'];
            push_encoded_prefix(&mut encoded, &escaped);
            encoded_len = encoded_len.saturating_add(escaped.len());
            changed = true;
        } else {
            push_encoded_prefix(&mut encoded, &[byte]);
            encoded_len = encoded_len.saturating_add(1);
        }
    }

    let value = if encoded_len <= MAX_COMPONENT_LEN {
        encoded
    } else {
        bound_component(&encoded, suffix, hash)
    };
    PortableLeaf { unchanged: !changed && encoded_len <= MAX_COMPONENT_LEN, value }
}

fn push_encoded_prefix(encoded: &mut String, bytes: &[u8]) {
    let remaining = MAX_ENCODED_PREFIX_LEN.saturating_sub(encoded.len());
    encoded.extend(bytes.iter().take(remaining).map(|byte| char::from(*byte)));
}

fn collision_leaf(preferred: &str, kind: &str, suffix: &str, attempt: u32) -> String {
    let stem = preferred.strip_suffix(suffix).unwrap_or(preferred);
    let discriminator = format!(".oxc-{kind}-{attempt}");
    let candidate = format!("{stem}{discriminator}{suffix}");
    if candidate.len() <= MAX_COMPONENT_LEN {
        return candidate;
    }

    let hash = stable_name_hash(preferred.as_bytes());
    let marker = format!("_h{hash:016x}");
    let prefix_len = MAX_COMPONENT_LEN - marker.len() - discriminator.len() - suffix.len();
    format!("{}{}{}{}", &stem[..stem.len().min(prefix_len)], marker, discriminator, suffix)
}

fn bound_component(value: &str, suffix: &str, hash: u64) -> String {
    if value.len() <= MAX_COMPONENT_LEN {
        return value.to_owned();
    }

    let stem = value.strip_suffix(suffix).unwrap_or(value);
    let marker = format!("_h{hash:016x}");
    let prefix_len = MAX_COMPONENT_LEN - marker.len() - suffix.len();
    format!("{}{}{}", &stem[..stem.len().min(prefix_len)], marker, suffix)
}

fn stable_name_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| update_name_hash(hash, *byte))
}

fn update_name_hash(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
}

fn is_portable_ascii_byte(byte: u8) -> bool {
    (b' '..=b'~').contains(&byte) && !b"<>:\"/\\|?*".contains(&byte)
}

fn is_windows_device_name(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or(value).trim_end_matches(' ');
    let stem = stem.to_ascii_lowercase();
    matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
        || stem
            .strip_prefix("com")
            .or_else(|| stem.strip_prefix("lpt"))
            .is_some_and(|digit| digit.len() == 1 && matches!(digit.as_bytes()[0], b'1'..=b'9'))
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
        std::path::Component::Normal(component) => component.to_str().is_some_and(|component| {
            super::is_safe_report_path_component(component)
                && component.len() <= MAX_COMPONENT_LEN
                && component.bytes().all(is_portable_ascii_byte)
                && !component.ends_with(['.', ' '])
                && !is_windows_device_name(component)
        }),
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
                    !component.bytes().any(|byte| {
                        byte < b' ' || byte == 0x7f || b"<>:\"/\\|?*".contains(&byte)
                    }),
                    "nonportable output component: {component:?}",
                );
                assert!(!component.ends_with(['.', ' ']), "trailing alias: {component:?}");

                let stem = component.split('.').next().unwrap().to_ascii_lowercase();
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
        let mut devices =
            vec!["CON".to_owned(), "prn".to_owned(), "AuX".to_owned(), "nul".to_owned()];
        for prefix in ["COM", "lPt"] {
            devices.extend((1..=9).map(|digit| format!("{prefix}{digit}")));
        }
        let mut entries =
            vec![coverage_entry("keep.js"), coverage_entry("CoN"), coverage_entry("PrN.txt")];
        entries.extend(devices.iter().map(|device| coverage_entry(&format!("{device}.txt/a.js"))));
        let root = tree(&format!("{{{}}}", entries.join(",")));
        let paths = PhysicalPaths::build(&root).unwrap();

        assert_portable_outputs(&paths);
        assert_eq!(
            paths.output_path(find(&root, "CoN", NodeType::File)),
            Path::new("_x43_oN.html"),
        );
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
        let first = allocate_leaf(
            &mut occupied,
            &protected_preferred,
            &protected_ordinary,
            &bounded,
            "dir",
            "",
        );
        let second = allocate_leaf(
            &mut occupied,
            &protected_preferred,
            &protected_ordinary,
            &bounded,
            "dir",
            "",
        );
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
        assert_eq!(
            paths.href_from_parent(nested_file),
            "nested%20%23_x3F_%25%20_xC3__xBC_.js.html",
        );
    }
}
