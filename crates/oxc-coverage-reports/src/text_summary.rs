//! Compact four-line rollup reporter.
//!
//! Output (header followed by one line per metric):
//!
//! ```text
//! =============================== Coverage summary ===============================
//! Statements   : 50% ( 1/2 )
//! Branches     : 50% ( 1/2 )
//! Functions    : 100% ( 1/1 )
//! Lines        : 50% ( 1/2 )
//! ================================================================================
//! ```
//!
//! Designed for piping to a PR comment, Slack notification, or CI step summary.
//! ANSI color codes are intentionally omitted; consumers wrap output as needed.

use oxc_coverage_report::{Metric, ReportNode};
use std::io;

const HEADER: &str =
    "=============================== Coverage summary ===============================";
const FOOTER: &str =
    "================================================================================";

/// Write a text-summary report to `out`.
pub fn write<W: io::Write>(root: &ReportNode, out: &mut W) -> io::Result<()> {
    let s = &root.summary;
    writeln!(out, "{HEADER}")?;
    write_metric(out, "Statements", &s.statements)?;
    write_metric(out, "Branches", &s.branches)?;
    write_metric(out, "Functions", &s.functions)?;
    write_metric(out, "Lines", &s.lines)?;
    writeln!(out, "{FOOTER}")?;
    Ok(())
}

fn write_metric<W: io::Write>(out: &mut W, label: &str, m: &Metric) -> io::Result<()> {
    writeln!(out, "{:<12} : {} ( {}/{} )", label, format_pct(m.pct), m.covered, m.total)
}

fn format_pct(pct: f64) -> String {
    // Mirror istanbul-reports text-summary: drop trailing `.00` for clean integer cases,
    // otherwise keep two decimal places.
    if (pct - pct.round()).abs() < f64::EPSILON {
        format!("{}%", pct.round() as i64)
    } else {
        format!("{pct:.2}%")
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
    fn empty_map_renders_all_one_hundred_percent() {
        let out = render("{}");
        assert!(out.contains("Statements   : 100% ( 0/0 )"));
        assert!(out.contains("Lines        : 100% ( 0/0 )"));
    }

    #[test]
    fn header_and_footer_bracket_the_metrics() {
        let out = render(
            r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#,
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.first().copied(), Some(HEADER));
        assert_eq!(lines.last().copied(), Some(FOOTER));
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn fractional_pct_keeps_two_decimals() {
        // 1/3 statements covered = 33.33%
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":3,"column":0},"end":{"line":3,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":0,"2":0},"f":{},"b":{}}}"#;
        let out = render(json);
        assert!(out.contains("Statements   : 33.33% ( 1/3 )"), "got: {out}");
    }
}
