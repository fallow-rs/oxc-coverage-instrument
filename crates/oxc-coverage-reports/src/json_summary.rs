//! `coverage-summary.json` reporter.
//!
//! Matches the istanbul-reports `json-summary` shape field for field:
//!
//! ```json
//! {
//!   "total":   { "lines": {...}, "statements": {...}, "functions": {...}, "branches": {...} },
//!   "path/a.js": { "lines": {...}, ... },
//!   "path/b.js": { "lines": {...}, ... }
//! }
//! ```
//!
//! Each per-metric object has `total`, `covered`, `skipped`, `pct`. `pct` is a
//! JSON number rounded to two decimal places (Codecov / Vitest treat `pct` as
//! a float and threshold against it; integers or unrounded floats break
//! consumers).
//!
//! Output ordering is stable: `total` first, then files in alphabetical order
//! (via `BTreeMap`). Folder rollups are intentionally omitted; istanbul-reports
//! json-summary lists files only, not folders.

use oxc_coverage_report::{CoverageSummary, Metric, NodeKind, ReportNode, Visitor, walk};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io;

/// Write a json-summary report to `out`.
///
/// The outer object's keys are emitted in this fixed order to match
/// istanbul-reports: `total` first, then files in tree-walk order (which is
/// alphabetical because the [`oxc_coverage_report`] summarizer uses
/// `BTreeMap`). Default `serde_json::Map` is backed by `BTreeMap` and would
/// alphabetize the outer object, placing `"src/a.js"` before `"total"`; we
/// hand-assemble the outer object to preserve the consumer-expected ordering.
pub fn write<W: io::Write>(root: &ReportNode, out: &mut W) -> io::Result<()> {
    write_inner(out, JsonSummaryInput { root, pretty: false })
}

/// Write a pretty-printed json-summary report. Useful for snapshot tests and
/// human inspection; CI consumers should prefer [`write()`] for compactness.
pub fn write_pretty<W: io::Write>(root: &ReportNode, out: &mut W) -> io::Result<()> {
    write_inner(out, JsonSummaryInput { root, pretty: true })
}

#[derive(Clone, Copy)]
struct JsonSummaryInput<'a> {
    root: &'a ReportNode,
    pretty: bool,
}

fn write_inner<W: io::Write>(out: &mut W, input: JsonSummaryInput<'_>) -> io::Result<()> {
    let JsonSummaryInput { root, pretty } = input;
    let total = SummaryOut::from(&root.summary);
    let mut collector = FileCollector::default();
    walk(root, &mut collector).expect("FileCollector cannot fail");

    let total_bytes = serialize_summary(&total, pretty)?;
    if pretty {
        out.write_all(b"{\n  \"total\": ")?;
        write_indented(out, &total_bytes, 2)?;
        for (path, summary) in &collector.files {
            out.write_all(b",\n  ")?;
            serde_json::to_writer(&mut *out, path).map_err(io::Error::other)?;
            out.write_all(b": ")?;
            let bytes = serialize_summary(summary, true)?;
            write_indented(out, &bytes, 2)?;
        }
        out.write_all(b"\n}")?;
    } else {
        out.write_all(b"{\"total\":")?;
        out.write_all(&total_bytes)?;
        for (path, summary) in &collector.files {
            out.write_all(b",")?;
            serde_json::to_writer(&mut *out, path).map_err(io::Error::other)?;
            out.write_all(b":")?;
            let bytes = serialize_summary(summary, false)?;
            out.write_all(&bytes)?;
        }
        out.write_all(b"}")?;
    }
    Ok(())
}

fn serialize_summary(summary: &SummaryOut, pretty: bool) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    if pretty {
        serde_json::to_writer_pretty(&mut buf, summary).map_err(io::Error::other)?;
    } else {
        serde_json::to_writer(&mut buf, summary).map_err(io::Error::other)?;
    }
    Ok(buf)
}

fn write_indented<W: io::Write>(out: &mut W, bytes: &[u8], extra_indent: usize) -> io::Result<()> {
    let text = std::str::from_utf8(bytes).expect("serde_json emits valid UTF-8");
    let pad = " ".repeat(extra_indent);
    let mut first = true;
    for line in text.split('\n') {
        if !first {
            out.write_all(b"\n")?;
            out.write_all(pad.as_bytes())?;
        }
        first = false;
        out.write_all(line.as_bytes())?;
    }
    Ok(())
}

#[derive(Serialize)]
struct MetricOut {
    total: u32,
    covered: u32,
    skipped: u32,
    pct: f64,
}

impl From<Metric> for MetricOut {
    fn from(m: Metric) -> Self {
        Self { total: m.total, covered: m.covered, skipped: m.skipped, pct: m.pct }
    }
}

#[derive(Serialize)]
struct SummaryOut {
    lines: MetricOut,
    statements: MetricOut,
    functions: MetricOut,
    branches: MetricOut,
}

impl From<&CoverageSummary> for SummaryOut {
    fn from(s: &CoverageSummary) -> Self {
        Self {
            lines: s.lines.into(),
            statements: s.statements.into(),
            functions: s.functions.into(),
            branches: s.branches.into(),
        }
    }
}

#[derive(Default)]
struct FileCollector {
    files: BTreeMap<String, SummaryOut>,
}

impl Visitor for FileCollector {
    fn on_detail(&mut self, node: &ReportNode) -> io::Result<()> {
        if let NodeKind::File { coverage } = &node.kind {
            let key = if coverage.path.is_empty() {
                node.relative_path.clone()
            } else {
                coverage.path.clone()
            };
            self.files.insert(key, SummaryOut::from(&node.summary));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_report::summarize;
    use oxc_coverage_types::parse_coverage_map;

    fn build_root(json: &str) -> ReportNode {
        let map = parse_coverage_map(json).unwrap();
        summarize(&map)
    }

    #[test]
    fn pct_is_a_float_with_two_decimal_places() {
        // 1/3 statements covered should serialize as `33.33`, not an int and
        // not the full-precision `33.333333333333336`.
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":3,"column":0},"end":{"line":3,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":0,"2":0},"f":{},"b":{}}}"#;
        let root = build_root(json);
        let mut buf = Vec::new();
        write(&root, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("\"pct\":33.33"), "got: {out}");
        assert!(!out.contains("33.33333"));
    }

    #[test]
    fn zero_total_pct_is_one_hundred_float() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let root = build_root(json);
        let mut buf = Vec::new();
        write(&root, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        // serde_json renders 100.0 as `100.0` (not `100`); both forms are valid
        // JSON numbers, but consumers expecting a float type must not see an
        // integer literal.
        assert!(out.contains("\"pct\":100.0") || out.contains("\"pct\":100"), "got: {out}");
    }

    #[test]
    fn total_appears_before_files() {
        let json = r#"{"src/a.js":{"path":"src/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/b.js":{"path":"src/b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let root = build_root(json);
        let mut buf = Vec::new();
        write(&root, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let total_pos = out.find("\"total\"").unwrap();
        let a_pos = out.find("\"src/a.js\"").unwrap();
        let b_pos = out.find("\"src/b.js\"").unwrap();
        assert!(total_pos < a_pos && a_pos < b_pos);
    }

    #[test]
    fn folders_are_not_included() {
        // Two files share `src/`; the tree builds a folder rollup, but
        // json-summary should only list files (matching istanbul-reports).
        let json = r#"{"src/a.js":{"path":"src/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/b.js":{"path":"src/b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let root = build_root(json);
        let mut buf = Vec::new();
        write(&root, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(!out.contains("\"src\":"));
    }

    #[test]
    fn all_four_metrics_present_per_entry() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let root = build_root(json);
        let mut buf = Vec::new();
        write(&root, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        for key in &["lines", "statements", "functions", "branches"] {
            assert!(out.contains(&format!("\"{key}\"")), "missing {key}");
        }
        for field in &["total", "covered", "skipped", "pct"] {
            assert!(out.contains(&format!("\"{field}\"")), "missing {field}");
        }
    }
}
