//! Mapping from report-tree nodes to the files the report actually
//! writes: collision-free leaf allocation, the percent-encoded hrefs
//! that link a parent page to its children, and the uniqueness and
//! safety checks on the allocated paths.

mod portable;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Component, Path, PathBuf};

use cow_utils::CowUtils as _;
use oxc_coverage_report::{NodeKind, ReportNode};

use portable::{
    MAX_COMPONENT_LEN, PortableLeaf, collision_leaf, is_portable_ascii_byte,
    is_windows_device_name, portable_leaf,
};

const INDEX_FILE: &str = "index.html";

/// Names the report root writes itself. A node preferring one of these is
/// moved aside so the stylesheet and script survive.
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

/// Where each report node is written and how its parent links to it.
///
/// A logical report path is not usable as a filesystem path: it can collide
/// with `index.html` or a root asset, differ from a sibling only by ASCII
/// case, name a Windows device, or exceed a component-length limit. The map
/// is built once up front so a page and every href pointing at it agree.
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
    /// Allocate an output path for every node in the tree rooted at `root`.
    ///
    /// # Errors
    /// Returns [`io::Error`] with kind `InvalidInput` if an allocated path
    /// is still unsafe or still collides, which would mean the mapping
    /// rules below failed rather than the input being hostile.
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

    /// Path `node`'s page is written to, relative to the report root.
    pub(super) fn output_path(&self, node: &ReportNode) -> &Path {
        &self.node(node).output_path
    }

    /// Percent-encoded href linking `node`'s parent index page to it.
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
        .filter(|preferred| preferred.is_verbatim)
        .map(|preferred| ascii_case_key(&preferred.value))
        .collect();

    for child in children.iter().filter(|child| matches!(child.kind, NodeKind::Folder { .. })) {
        let leaf = allocate_leaf(LeafRequest {
            occupied: &mut occupied,
            protected_preferred: &protected_preferred,
            protected_ordinary: &protected_ordinary,
            preferred: &preferences[&NodeKey::from_node(child)],
            kind: "dir",
            suffix: "",
        });
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
        let leaf = allocate_leaf(LeafRequest {
            occupied: &mut occupied,
            protected_preferred: &protected_preferred,
            protected_ordinary: &protected_ordinary,
            preferred: &preferences[&NodeKey::from_node(child)],
            kind: "file",
            suffix: ".html",
        });
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

/// Everything [`allocate_leaf`] needs to pick a name inside one directory.
struct LeafRequest<'a> {
    /// Case-folded names already taken in this directory.
    occupied: &'a mut BTreeSet<String>,
    /// Case-folded preferred name of every sibling. A mangled candidate
    /// never takes one, so a sibling later in the order still gets its
    /// own name.
    protected_preferred: &'a BTreeSet<String>,
    /// The subset of `protected_preferred` a sibling can claim verbatim.
    /// A node whose own name needed mangling yields to those siblings.
    protected_ordinary: &'a BTreeSet<String>,
    preferred: &'a PortableLeaf,
    /// `"dir"` or `"file"`, the discriminator in a `.oxc-{kind}-{n}` suffix.
    kind: &'a str,
    /// Extension the discriminator is inserted before, or empty.
    suffix: &'a str,
}

/// Pick a free leaf name for `request.preferred`, marking it occupied.
fn allocate_leaf(request: LeafRequest<'_>) -> String {
    let LeafRequest { occupied, protected_preferred, protected_ordinary, preferred, kind, suffix } =
        request;
    let preferred_key = ascii_case_key(&preferred.value);
    if (preferred.is_verbatim || !protected_ordinary.contains(&preferred_key))
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

/// Fold a leaf name to the key a case-insensitive filesystem would collide
/// on. ASCII-only: NTFS and APFS fold non-ASCII by Unicode rules this does
/// not model, and [`portable_leaf`] has already escaped every non-ASCII
/// byte out of the names reaching here.
fn ascii_case_key(value: &str) -> String {
    value.cow_to_ascii_lowercase().into_owned()
}

fn output_comparison_key(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(component) => component.to_str(),
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
        Component::Normal(component) => component.to_str().is_some_and(|component| {
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
mod tests;
