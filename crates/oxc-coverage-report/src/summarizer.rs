//! Build a [`ReportNode`] tree from a coverage map.
//!
//! Algorithm matches istanbul-lib-report's `summarizers.pkg` shape:
//! 1. Find the longest common path prefix shared by every file (split on `/`).
//! 2. Build a nested folder structure rooted at that prefix; each leaf is a file.
//! 3. Bottom-up roll-up: every folder's summary is the merge of its children.
//!
//! Forward slashes are the only path separator handled. Windows-style `\\`
//! paths are not split; they remain a single segment, which is correct for
//! `coverage-final.json` produced by any Istanbul-compatible tool (all of them
//! emit forward slashes).

use crate::summary::CoverageSummary;
use crate::tree::{NodeKind, ReportNode};
use oxc_coverage_types::FileCoverage;
use std::collections::BTreeMap;

/// Alias for the deserialized `coverage-final.json` shape.
///
/// `BTreeMap<String, FileCoverage>` keyed by file path is what
/// [`oxc_coverage_types::parse_coverage_map`] returns.
pub type CoverageMap = BTreeMap<String, FileCoverage>;

/// Build a folder/file tree from a flat [`CoverageMap`].
///
/// The returned root has `name = ""` and `relative_path = ""`. If the input is
/// empty, the root is a folder with no children and a zero-everywhere summary
/// (with `pct = 100.0` per istanbul convention for zero-total metrics).
pub fn summarize(map: &CoverageMap) -> ReportNode {
    if map.is_empty() {
        return ReportNode {
            name: String::new(),
            relative_path: String::new(),
            summary: CoverageSummary::default(),
            kind: NodeKind::Folder { children: Vec::new() },
        };
    }

    let paths: Vec<Vec<&str>> = map.keys().map(|k| split_path(k)).collect();
    let prefix_len = common_prefix_len(&paths);

    let mut root = TempFolder::default();
    for (path_str, file) in map {
        let segments = split_path(path_str);
        let rel: &[&str] = &segments[prefix_len..];
        insert(&mut root, rel, file.clone());
    }

    finalize(String::new(), String::new(), root)
}

fn split_path(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

fn common_prefix_len(paths: &[Vec<&str>]) -> usize {
    if paths.is_empty() {
        return 0;
    }
    let first = &paths[0];
    // Leave at least the leaf segment so a single-file input still has a name.
    let max_prefix = first.len().saturating_sub(1);
    let mut len = 0;
    while len < max_prefix {
        let seg = first[len];
        if !paths.iter().all(|p| p.get(len).copied() == Some(seg)) {
            break;
        }
        len += 1;
    }
    len
}

#[derive(Default)]
struct TempFolder {
    subfolders: BTreeMap<String, TempFolder>,
    files: BTreeMap<String, FileCoverage>,
}

fn insert(folder: &mut TempFolder, segments: &[&str], file: FileCoverage) {
    match segments {
        [] => {}
        [leaf] => {
            folder.files.insert((*leaf).to_owned(), file);
        }
        [head, rest @ ..] => {
            let child = folder.subfolders.entry((*head).to_owned()).or_default();
            insert(child, rest, file);
        }
    }
}

fn join(parent: &str, segment: &str) -> String {
    if parent.is_empty() { segment.to_owned() } else { format!("{parent}/{segment}") }
}

fn finalize(name: String, rel: String, temp: TempFolder) -> ReportNode {
    let mut children: Vec<ReportNode> =
        Vec::with_capacity(temp.subfolders.len() + temp.files.len());
    let mut summary = CoverageSummary::default();

    for (sub_name, sub_temp) in temp.subfolders {
        let sub_rel = join(&rel, &sub_name);
        let child = finalize(sub_name, sub_rel, sub_temp);
        summary.merge(&child.summary);
        children.push(child);
    }
    for (file_name, coverage) in temp.files {
        let file_rel = join(&rel, &file_name);
        let file_summary = CoverageSummary::from_file(&coverage);
        summary.merge(&file_summary);
        children.push(ReportNode {
            name: file_name,
            relative_path: file_rel,
            summary: file_summary,
            kind: NodeKind::File { coverage: Box::new(coverage) },
        });
    }

    ReportNode { name, relative_path: rel, summary, kind: NodeKind::Folder { children } }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "pct values are deterministically rounded to two decimals; exact comparison is intentional"
)]
mod tests {
    use super::*;
    use oxc_coverage_types::parse_coverage_map;

    fn empty_file(path: &str, hit: bool) -> String {
        let s_val = u32::from(hit);
        format!(
            r#"{{"path":"{path}","statementMap":{{"0":{{"start":{{"line":1,"column":0}},"end":{{"line":1,"column":1}}}}}},"fnMap":{{}},"branchMap":{{}},"s":{{"0":{s_val}}},"f":{{}},"b":{{}}}}"#
        )
    }

    #[test]
    fn empty_map_returns_empty_root() {
        let map: CoverageMap = BTreeMap::new();
        let root = summarize(&map);
        assert!(root.is_folder());
        assert!(root.children().is_empty());
        assert_eq!(root.summary.statements.total, 0);
        assert_eq!(root.summary.statements.pct, 100.0);
    }

    #[test]
    fn single_file_has_no_extra_folder_wrapper() {
        let json = format!("{{\"a.js\":{}}}", empty_file("a.js", true));
        let map = parse_coverage_map(&json).unwrap();
        let root = summarize(&map);
        assert_eq!(root.children().len(), 1);
        let leaf = &root.children()[0];
        assert_eq!(leaf.name, "a.js");
        assert_eq!(leaf.relative_path, "a.js");
        assert!(leaf.is_file());
    }

    #[test]
    fn shared_prefix_is_stripped() {
        // Both files share `src/`; the prefix should be stripped from the tree.
        let json = format!(
            "{{\"src/a.js\":{},\"src/b.js\":{}}}",
            empty_file("src/a.js", true),
            empty_file("src/b.js", false)
        );
        let map = parse_coverage_map(&json).unwrap();
        let root = summarize(&map);
        let names: Vec<_> = root.children().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a.js", "b.js"]);
        // Rolled-up summary: 2 statements total, 1 covered.
        assert_eq!(root.summary.statements.total, 2);
        assert_eq!(root.summary.statements.covered, 1);
        assert_eq!(root.summary.statements.pct, 50.0);
    }

    #[test]
    fn divergent_paths_build_subfolders() {
        let json = format!(
            "{{\"src/a/x.js\":{},\"src/b/y.js\":{}}}",
            empty_file("src/a/x.js", true),
            empty_file("src/b/y.js", true)
        );
        let map = parse_coverage_map(&json).unwrap();
        let root = summarize(&map);
        // Common prefix is `src/`, so children are folders `a` and `b`.
        let names: Vec<_> = root.children().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
        assert!(root.children().iter().all(ReportNode::is_folder));
        // Each subfolder has one file.
        for child in root.children() {
            assert_eq!(child.children().len(), 1);
            assert!(child.children()[0].is_file());
        }
    }

    #[test]
    fn rollup_sums_across_subfolders() {
        let json = format!(
            "{{\"src/a/x.js\":{},\"src/b/y.js\":{}}}",
            empty_file("src/a/x.js", true),
            empty_file("src/b/y.js", false)
        );
        let map = parse_coverage_map(&json).unwrap();
        let root = summarize(&map);
        assert_eq!(root.summary.statements, crate::Metric::new(2, 1));
    }
}
