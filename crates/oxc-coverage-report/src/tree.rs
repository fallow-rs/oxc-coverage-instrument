//! Folder/file tree built from a [`CoverageMap`][cm].
//!
//! [cm]: oxc_coverage_types::FileCoverage

use crate::summary::CoverageSummary;
use oxc_coverage_types::FileCoverage;

/// A node in the report tree: either a folder containing child nodes or a file
/// carrying the originating [`FileCoverage`].
#[derive(Debug, Clone)]
pub struct ReportNode {
    /// Short display name (last path component, or `""` for the root).
    pub name: String,
    /// Path relative to the tree root, joined with forward slashes.
    pub relative_path: String,
    /// Aggregated summary for this node (rolled up for folders).
    pub summary: CoverageSummary,
    /// Whether this node is a folder or a file.
    pub kind: NodeKind,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    Folder {
        children: Vec<ReportNode>,
    },
    /// `FileCoverage` is ~232 bytes (eight `BTreeMap`s plus path/source-map metadata).
    /// Boxing keeps `NodeKind` slim so a `Vec<ReportNode>` does not waste memory on
    /// the variant tag for every folder child.
    File {
        coverage: Box<FileCoverage>,
    },
}

impl ReportNode {
    pub fn is_folder(&self) -> bool {
        matches!(self.kind, NodeKind::Folder { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self.kind, NodeKind::File { .. })
    }

    pub fn children(&self) -> &[ReportNode] {
        match &self.kind {
            NodeKind::Folder { children } => children,
            NodeKind::File { .. } => &[],
        }
    }

    pub fn file_coverage(&self) -> Option<&FileCoverage> {
        match &self.kind {
            NodeKind::File { coverage } => Some(coverage.as_ref()),
            NodeKind::Folder { .. } => None,
        }
    }
}
