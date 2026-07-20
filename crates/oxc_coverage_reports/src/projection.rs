//! Per-file projections of the raw Istanbul counters that more than one
//! reporter needs, kept here so lcov, cobertura and html cannot drift apart
//! on how a malformed counter array is read.

use std::collections::BTreeMap;
use std::path::Path;

use cow_utils::CowUtils as _;
use oxc_coverage_types::{BranchEntry, FileCoverage};

/// Hit counts for `entry`, one per declared arm location.
///
/// The `b` array is authoritative only up to the arm count declared in
/// `branchMap`: a short array leaves the remaining arms at zero, and arms
/// beyond the declared locations are dropped. Reading the array length
/// instead would let a damaged coverage map change the branch denominator.
pub fn aligned_branch_hits(file: &FileCoverage, id: &str, entry: &BranchEntry) -> Vec<u32> {
    let stored = file.b.get(id).map(Vec::as_slice).unwrap_or_default();
    (0..entry.locations.len()).map(|index| stored.get(index).copied().unwrap_or(0)).collect()
}

/// `(total_arms, covered_arms)` across every branch in `file`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "istanbul counters are u32 on the wire, so an arm count that does not fit is already outside the format"
)]
pub fn branch_counts(file: &FileCoverage) -> (u32, u32) {
    file.branch_map.iter().fold((0, 0), |(total, covered), (id, entry)| {
        let hits = aligned_branch_hits(file, id, entry);
        (
            total + hits.len() as u32,
            covered + hits.iter().filter(|&&value| value > 0).count() as u32,
        )
    })
}

/// Hit counts keyed by the source line a statement starts on.
///
/// A line is covered when any statement starting on it ran, so overlapping
/// statements collapse to their maximum rather than their sum. Statements at
/// the `line == 0` unknown-position sentinel are skipped: they belong to no
/// source line.
pub fn line_hits(file: &FileCoverage) -> BTreeMap<u32, u32> {
    let mut by_line: BTreeMap<u32, u32> = BTreeMap::new();
    for (id, loc) in &file.statement_map {
        let line = loc.start.line;
        if line == 0 {
            continue;
        }
        let hits = file.s.get(id).copied().unwrap_or(0);
        by_line
            .entry(line)
            .and_modify(|existing| *existing = (*existing).max(hits))
            .or_insert(hits);
    }
    by_line
}

/// Source line a branch is reported on, falling back to the first arm
/// location for emitters that leave `BranchEntry::line` unset.
pub fn branch_line(entry: &BranchEntry) -> u32 {
    if entry.line > 0 {
        entry.line
    } else {
        entry.locations.first().map_or(0, |loc| loc.start.line)
    }
}

/// `(total_arms, covered_arms)` keyed by the source line a branch is
/// reported on.
///
/// Several branches can share a line (a chained ternary, a `switch` header),
/// so the arm counts sum per line. Branches at the `line == 0`
/// unknown-position sentinel are skipped: they belong to no source line.
#[expect(
    clippy::cast_possible_truncation,
    reason = "istanbul counters are u32 on the wire, so an arm count that does not fit is already outside the format"
)]
pub fn branched_lines(file: &FileCoverage) -> BTreeMap<u32, (u32, u32)> {
    let mut out: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
    for (id, entry) in &file.branch_map {
        let line = branch_line(entry);
        if line == 0 {
            continue;
        }
        let hits = aligned_branch_hits(file, id, entry);
        let total = hits.len() as u32;
        let covered = hits.iter().filter(|&&value| value > 0).count() as u32;
        let summary = out.entry(line).or_insert((0, 0));
        summary.0 += total;
        summary.1 += covered;
    }
    out
}

/// `path` relative to `root_dir` with `\` separators normalised to `/`.
///
/// A path that does not sit under `root_dir` is kept verbatim (minus the
/// separator normalisation): the reporters emit whatever the coverage map
/// recorded rather than inventing a relative path.
pub fn relative_source_path(path: &str, root_dir: &Path) -> String {
    let relative = Path::new(path)
        .strip_prefix(root_dir)
        .map_or_else(|_| path.to_owned(), |stripped| stripped.to_string_lossy().into());
    relative.cow_replace('\\', "/").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_types::parse_coverage_map;

    const BRANCH_FIXTURE: &str = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{},"f":{},"b":{"0":[4,0,9]}}}"#;

    #[test]
    fn alignment_defaults_missing_arms_and_ignores_extra_arms() {
        let map = parse_coverage_map(BRANCH_FIXTURE).unwrap();
        let file = &map["a.js"];
        let entry = &file.branch_map["0"];
        assert_eq!(aligned_branch_hits(file, "0", entry), vec![4, 0]);
        assert_eq!(branch_counts(file), (2, 1));
    }
}
