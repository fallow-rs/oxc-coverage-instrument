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
//! Designed for piping to a PR comment, chat notification, or CI step summary.
//! ANSI colour codes are omitted; consumers wrap output as needed.

use std::io;

use oxc_coverage_report::{Metric, ReportNode};

const HEADER: &str =
    "=============================== Coverage summary ===============================";
const FOOTER: &str =
    "================================================================================";

/// Write a text-summary report to `out`.
///
/// # Errors
/// Returns [`io::Error`] if `out` fails to accept a write.
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

/// istanbul-reports' text-summary drops a trailing `.00` on whole
/// percentages and keeps two decimals otherwise.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a percentage rounds to a whole number well inside i64"
)]
fn format_pct(pct: f64) -> String {
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
    fn fractional_pct_keeps_two_decimals() {
        // 1/3 statements covered = 33.33%
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":3,"column":0},"end":{"line":3,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":0,"2":0},"f":{},"b":{}}}"#;
        let out = render(json);
        assert!(out.contains("Statements   : 33.33% ( 1/3 )"), "got: {out}");
    }
}
