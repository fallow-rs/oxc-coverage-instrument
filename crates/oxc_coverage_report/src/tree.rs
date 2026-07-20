//! Folder/file tree built from a [`CoverageMap`](crate::CoverageMap).

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

/// Either a folder with child nodes or a leaf file with its coverage data.
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// Child nodes: subfolders first, then files, each group in path order.
    Folder {
        /// Nodes contained directly by this folder.
        children: Vec<ReportNode>,
    },
    /// `FileCoverage` is large (the path plus nine map and metadata fields).
    /// Boxing it keeps `ReportNode` small, so folder nodes, which carry no
    /// coverage, do not pay for the file payload.
    File {
        /// Coverage data for the file this node represents.
        coverage: Box<FileCoverage>,
    },
}

impl ReportNode {
    /// `true` for a folder node.
    pub fn is_folder(&self) -> bool {
        matches!(self.kind, NodeKind::Folder { .. })
    }

    /// `true` for a file node.
    pub fn is_file(&self) -> bool {
        matches!(self.kind, NodeKind::File { .. })
    }

    /// Child nodes, or an empty slice for a file node.
    pub fn children(&self) -> &[ReportNode] {
        match &self.kind {
            NodeKind::Folder { children } => children,
            NodeKind::File { .. } => &[],
        }
    }

    /// Coverage data for a file node, or [`None`] for a folder node.
    pub fn file_coverage(&self) -> Option<&FileCoverage> {
        match &self.kind {
            NodeKind::File { coverage } => Some(coverage.as_ref()),
            NodeKind::Folder { .. } => None,
        }
    }
}
