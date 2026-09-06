//! Tabular text reporter.
//!
//! Prints a fixed-width table with one row per file and one row per non-root
//! folder, in tree order. Matches istanbul-reports `text` column layout:
//!
//! ```text
//! ------------|---------|----------|---------|---------|-------------------
//! File        | % Stmts | % Branch | % Funcs | % Lines | Uncovered Line #s
//! ------------|---------|----------|---------|---------|-------------------
//! All files   |   50.00 |    50.00 |  100.00 |   50.00 |
//!  src        |   50.00 |    50.00 |  100.00 |   50.00 |
//!   a.js      |  100.00 |   100.00 |  100.00 |  100.00 |
//!   b.js      |    0.00 |     0.00 |  100.00 |    0.00 | 1-3
//! ------------|---------|----------|---------|---------|-------------------
//! ```
//!
//! Uncovered line numbers are emitted for file rows when there are gaps; they
//! are collapsed into ranges (`1-3`, `7`, `10-12`).
//!
//! ANSI colour codes are omitted so the output is byte-stable for snapshot
//! tests and safe to pipe.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io;

use oxc_coverage_report::{ReportNode, Visitor, walk};
use oxc_coverage_types::FileCoverage;

const FILE_COL: usize = 50;
const PCT_COL: usize = 8;
const SEP: &str = " |";

/// Write a text report to `out`.
///
/// # Errors
/// Returns [`io::Error`] if `out` fails to accept a write.
pub fn write<W: io::Write>(root: &ReportNode, out: &mut W) -> io::Result<()> {
    let mut writer = TextWriter { out, depth: 0 };
    write_header(writer.out)?;
    walk(root, &mut writer)?;
    write_separator(writer.out)?;
    Ok(())
}

struct TextWriter<'a, W: io::Write> {
    out: &'a mut W,
    depth: usize,
}

impl<W: io::Write> Visitor for TextWriter<'_, W> {
    fn on_summary(&mut self, node: &ReportNode) -> io::Result<()> {
        let display = if self.depth == 0 { "All files".to_owned() } else { node.name.clone() };
        write_row(self.out, &display, self.depth, node, None)?;
        self.depth += 1;
        Ok(())
    }

    fn on_summary_end(&mut self, _node: &ReportNode) -> io::Result<()> {
        self.depth = self.depth.saturating_sub(1);
        Ok(())
    }

    fn on_detail(&mut self, node: &ReportNode) -> io::Result<()> {
        let uncovered = node.file_coverage().map(uncovered_lines);
        write_row(self.out, &node.name, self.depth, node, uncovered.as_deref())
    }
}

fn write_header<W: io::Write>(out: &mut W) -> io::Result<()> {
    write_separator(out)?;
    writeln!(
        out,
        "{file:<file_w$}{sep}{s:>pct_w$}{sep}{b:>pct_w$}{sep}{f:>pct_w$}{sep}{l:>pct_w$}{sep} Uncovered Line #s",
        file = "File",
        s = "% Stmts",
        b = "% Branch",
        f = "% Funcs",
        l = "% Lines",
        file_w = FILE_COL,
        pct_w = PCT_COL,
        sep = SEP,
    )?;
    write_separator(out)
}

fn write_separator<W: io::Write>(out: &mut W) -> io::Result<()> {
    let file_dashes = "-".repeat(FILE_COL);
    let pct_dashes = "-".repeat(PCT_COL + SEP.len());
    writeln!(out, "{file_dashes}{pct_dashes}{pct_dashes}{pct_dashes}{pct_dashes}{}", "-".repeat(20))
}

fn write_row<W: io::Write>(
    out: &mut W,
    name: &str,
    depth: usize,
    node: &ReportNode,
    uncovered: Option<&str>,
) -> io::Result<()> {
    let indent = " ".repeat(depth);
    let display = format!("{indent}{name}");
    let truncated = truncate(&display, FILE_COL);
    let s = &node.summary;
    let uncov = uncovered.unwrap_or("");
    writeln!(
        out,
        "{truncated:<file_w$}{sep}{stmts:>pct_w$}{sep}{branches:>pct_w$}{sep}{funcs:>pct_w$}{sep}{lines:>pct_w$}{sep} {uncov}",
        stmts = format_pct(s.statements.pct),
        branches = format_pct(s.branches.pct),
        funcs = format_pct(s.functions.pct),
        lines = format_pct(s.lines.pct),
        file_w = FILE_COL,
        pct_w = PCT_COL,
        sep = SEP,
    )
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_owned()
    } else {
        // Reserve one char for the ellipsis so the cell stays within width.
        let kept: String = s.chars().take(width.saturating_sub(1)).collect();
        format!("{kept}…")
    }
}

fn format_pct(pct: f64) -> String {
    format!("{pct:>.2}")
}

/// Collapse uncovered statement line numbers into a compact range list.
fn uncovered_lines(file: &FileCoverage) -> String {
    let mut lines: BTreeSet<u32> = BTreeSet::new();
    for (id, &hits) in &file.s {
        if hits > 0 {
            continue;
        }
        let Some(loc) = file.statement_map.get(id) else {
            continue;
        };
        let line = loc.start.line;
        if line != 0 {
            lines.insert(line);
        }
    }
    collapse_ranges(&lines)
}

fn collapse_ranges(lines: &BTreeSet<u32>) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut iter = lines.iter().copied();
    let first = iter.next().expect("non-empty checked above");
    let mut range_start = first;
    let mut range_end = first;
    for line in iter {
        if line != range_end + 1 {
            push_range(&mut out, range_start, range_end);
            range_start = line;
        }
        range_end = line;
    }
    push_range(&mut out, range_start, range_end);
    out
}

fn push_range(out: &mut String, start: u32, end: u32) {
    if !out.is_empty() {
        out.push(',');
    }
    if start == end {
        let _ = write!(out, "{start}");
    } else {
        let _ = write!(out, "{start}-{end}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_report::summarize;
    use oxc_coverage_types::parse_coverage_map;

    fn render(json: &str) -> String {
        let map = parse_coverage_map(json).unwrap();
        let root = summarize(&map);
        let mut buf = Vec::new();
        write(&root, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn uncovered_ranges_are_collapsed() {
        let mut s: BTreeSet<u32> = BTreeSet::new();
        s.extend([1, 2, 3, 7, 10, 11, 12, 20]);
        assert_eq!(collapse_ranges(&s), "1-3,7,10-12,20");
    }

    #[test]
    fn uncovered_column_lists_zero_hit_statement_lines() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":5,"column":0},"end":{"line":5,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":0,"2":0},"f":{},"b":{}}}"#;
        let out = render(json);
        assert!(out.contains("2,5"), "expected 2,5 in: {out}");
    }

    #[test]
    fn uncovered_column_ignores_statement_map_entries_without_counters() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":5}},"1":{"start":{"line":7,"column":0},"end":{"line":7,"column":5}}},"fnMap":{},"branchMap":{},"s":{"0":3},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        assert_eq!(uncovered_lines(&map["a.js"]), "");
    }
}
