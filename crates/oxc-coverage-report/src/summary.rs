//! Per-metric coverage aggregates.
//!
//! Mirrors istanbul-lib-coverage's `CoverageSummary` shape so consumers can
//! round-trip through json-summary without field translation.

use oxc_coverage_types::FileCoverage;
use std::collections::BTreeSet;

/// Coverage counts for one of the four Istanbul metrics.
///
/// `skipped` carries the count of intentionally uninstrumented entries (e.g.,
/// statements behind `/* istanbul ignore */` pragmas). The current
/// [`oxc_coverage_types`] deserializer coerces `null` hit counts to `0`, so the
/// skipped signal is lost on ingestion and this field is always `0` in
/// summaries produced from a parsed `coverage-final.json` today. The field is
/// kept for compatibility with istanbul-reports consumers that type-check the
/// json-summary schema.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metric {
    pub total: u32,
    pub covered: u32,
    pub skipped: u32,
    pub pct: f64,
}

impl Default for Metric {
    /// Zero counts with `pct = 100.0`, matching istanbul-lib-coverage's
    /// zero-total convention (a file with no statements is "fully covered").
    fn default() -> Self {
        Self { total: 0, covered: 0, skipped: 0, pct: 100.0 }
    }
}

impl Metric {
    /// Build a metric from a total + covered count, rounding `pct` to two
    /// decimal places. A zero-total metric reports `pct = 100.0`, matching
    /// istanbul-lib-coverage.
    pub fn new(total: u32, covered: u32) -> Self {
        Self { total, covered, skipped: 0, pct: pct(covered, total) }
    }

    /// Add another metric into this one, recomputing `pct` from the new totals.
    pub fn merge(&mut self, other: &Self) {
        self.total += other.total;
        self.covered += other.covered;
        self.skipped += other.skipped;
        self.pct = pct(self.covered, self.total);
    }
}

/// The four-metric per-file or per-folder coverage summary.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CoverageSummary {
    pub lines: Metric,
    pub statements: Metric,
    pub functions: Metric,
    pub branches: Metric,
}

impl CoverageSummary {
    /// Compute the summary for a single [`FileCoverage`].
    pub fn from_file(file: &FileCoverage) -> Self {
        Self {
            lines: lines_metric(file),
            statements: statements_metric(file),
            functions: functions_metric(file),
            branches: branches_metric(file),
        }
    }

    /// Merge another summary into this one (used to roll folder summaries up).
    pub fn merge(&mut self, other: &Self) {
        self.lines.merge(&other.lines);
        self.statements.merge(&other.statements);
        self.functions.merge(&other.functions);
        self.branches.merge(&other.branches);
    }
}

fn pct(covered: u32, total: u32) -> f64 {
    if total == 0 {
        return 100.0;
    }
    let raw = f64::from(covered) * 100.0 / f64::from(total);
    (raw * 100.0).round() / 100.0
}

fn statements_metric(file: &FileCoverage) -> Metric {
    let total = file.statement_map.len() as u32;
    let covered = file.s.values().filter(|&&hits| hits > 0).count() as u32;
    Metric::new(total, covered)
}

fn functions_metric(file: &FileCoverage) -> Metric {
    let total = file.fn_map.len() as u32;
    let covered = file.f.values().filter(|&&hits| hits > 0).count() as u32;
    Metric::new(total, covered)
}

fn branches_metric(file: &FileCoverage) -> Metric {
    let mut total: u32 = 0;
    let mut covered: u32 = 0;
    for arms in file.b.values() {
        total += arms.len() as u32;
        covered += arms.iter().filter(|&&hits| hits > 0).count() as u32;
    }
    Metric::new(total, covered)
}

fn lines_metric(file: &FileCoverage) -> Metric {
    let mut all_lines: BTreeSet<u32> = BTreeSet::new();
    let mut hit_lines: BTreeSet<u32> = BTreeSet::new();
    for (id, loc) in &file.statement_map {
        let line = loc.start.line;
        if line == 0 {
            continue;
        }
        all_lines.insert(line);
        if file.s.get(id).copied().unwrap_or(0) > 0 {
            hit_lines.insert(line);
        }
    }
    Metric::new(all_lines.len() as u32, hit_lines.len() as u32)
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "pct values are deterministically rounded to two decimals; exact comparison is intentional"
)]
mod tests {
    use super::*;
    use oxc_coverage_types::parse_coverage_map;

    fn parse_single(json: &str) -> FileCoverage {
        let map = parse_coverage_map(json).unwrap();
        map.into_values().next().unwrap()
    }

    #[test]
    fn pct_is_one_hundred_for_zero_total() {
        let m = Metric::new(0, 0);
        assert_eq!(m.pct, 100.0);
    }

    #[test]
    fn pct_rounds_to_two_decimals() {
        // 1/3 = 33.333... -> 33.33
        let m = Metric::new(3, 1);
        assert_eq!(m.pct, 33.33);
        // 2/3 = 66.666... -> 66.67
        let m = Metric::new(3, 2);
        assert_eq!(m.pct, 66.67);
    }

    #[test]
    fn statements_count_nonzero_hits() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":5}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":5}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":0},"f":{},"b":{}}}"#;
        let f = parse_single(json);
        let s = CoverageSummary::from_file(&f);
        assert_eq!(s.statements, Metric { total: 2, covered: 1, skipped: 0, pct: 50.0 });
    }

    #[test]
    fn branches_sum_arms() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{},"f":{},"b":{"0":[1,0]}}}"#;
        let f = parse_single(json);
        let s = CoverageSummary::from_file(&f);
        assert_eq!(s.branches, Metric { total: 2, covered: 1, skipped: 0, pct: 50.0 });
    }

    #[test]
    fn lines_dedupe_by_start_line() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":5}},"1":{"start":{"line":1,"column":6},"end":{"line":1,"column":10}},"2":{"start":{"line":2,"column":0},"end":{"line":2,"column":5}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":0,"2":0},"f":{},"b":{}}}"#;
        let f = parse_single(json);
        let s = CoverageSummary::from_file(&f);
        // Two distinct lines (1, 2); line 1 has a hit (statement 0), so covered = 1.
        assert_eq!(s.lines, Metric { total: 2, covered: 1, skipped: 0, pct: 50.0 });
    }

    #[test]
    fn merge_sums_counts_and_recomputes_pct() {
        let mut a = Metric::new(10, 5);
        let b = Metric::new(10, 8);
        a.merge(&b);
        assert_eq!(a, Metric { total: 20, covered: 13, skipped: 0, pct: 65.0 });
    }
}
