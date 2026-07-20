//! LCOV `tracefile` reporter.
//!
//! Produces the same record layout as istanbul-reports' `lcovonly` output:
//! one `SF:`/`end_of_record` block per file, with `FN:`/`FNDA:`/`FNF:`/`FNH:`
//! for function coverage, `DA:`/`LF:`/`LH:` for line coverage, and
//! `BRDA:`/`BRF:`/`BRH:` for branch coverage.
//!
//! Compatibility constraints:
//!
//! - `SF:` paths are relative to a caller-supplied `root_dir` (the `--root` CLI
//!   flag, defaulting to the cwd). Absolute paths break Codecov self-hosted
//!   runners and the GitLab MR widget; consumers expect repo-relative paths.
//! - `BRDA:<line>,<block>,<branch>,<taken>` sets `<block>` from the
//!   [`BranchEntry`][be] map key (the istanbul branch ID), not the line number.
//!   Putting every arm in `block=0` makes Codecov merge branches across
//!   unrelated `if` statements that share a line.
//! - Both `DA:` (line hits) AND `BRDA:` (branch hits) are emitted. Coveralls
//!   ignores `BRDA:` entirely and reports 0% branch coverage if a tracefile
//!   ships branches but no lines.
//! - `FN:` / `FNDA:` names are stripped of surrounding parens, so
//!   `(anonymous_0)` becomes `anonymous_0`. Codecov's function-coverage parser
//!   rejects names containing parens.
//! - When a branch's block was never entered (the sum of all arm counts is
//!   zero), every arm emits `-` for `<taken>` rather than `0`. `0` means
//!   "block entered, arm not taken", which is nonsensical for an `if` whose
//!   condition never evaluated.
//!
//! The emitter never panics on coverage data; missing or malformed entries are
//! treated as zero hits.
//!
//! [be]: oxc_coverage_types::BranchEntry

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use cow_utils::CowUtils as _;
use oxc_coverage_report::{NodeKind, ReportNode, Visitor, walk};
use oxc_coverage_types::{FileCoverage, FnEntry};

use crate::projection::relative_source_path;

/// Write an LCOV report to `out`. `root_dir` is used to relativize `SF:` paths.
///
/// # Errors
/// Returns [`io::Error`] if `out` fails to accept a write.
pub fn write<W: io::Write>(root: &ReportNode, root_dir: &Path, out: &mut W) -> io::Result<()> {
    let mut emitter = LcovEmitter { out, root_dir };
    walk(root, &mut emitter)
}

struct LcovEmitter<'a, W: io::Write> {
    out: &'a mut W,
    root_dir: &'a Path,
}

impl<W: io::Write> Visitor for LcovEmitter<'_, W> {
    fn on_detail(&mut self, node: &ReportNode) -> io::Result<()> {
        if let NodeKind::File { coverage } = &node.kind {
            write_file_record(
                self.out,
                FileRecordInput { file: coverage, node, root_dir: self.root_dir },
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct FileRecordInput<'a> {
    file: &'a FileCoverage,
    node: &'a ReportNode,
    root_dir: &'a Path,
}

fn write_file_record<W: io::Write>(out: &mut W, input: FileRecordInput<'_>) -> io::Result<()> {
    let FileRecordInput { file, node, root_dir } = input;
    let sf = source_path(file, node, root_dir);
    writeln!(out, "TN:")?;
    writeln!(out, "SF:{sf}")?;

    write_functions(out, file)?;
    write_lines(out, file)?;
    write_branches(out, file)?;

    writeln!(out, "end_of_record")?;
    Ok(())
}

fn source_path(file: &FileCoverage, node: &ReportNode, root_dir: &Path) -> String {
    let raw = if file.path.is_empty() { node.relative_path.as_str() } else { file.path.as_str() };
    relative_source_path(raw, root_dir)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "istanbul counters are u32 on the wire, so a function count that does not fit is already outside the format"
)]
fn write_functions<W: io::Write>(out: &mut W, file: &FileCoverage) -> io::Result<()> {
    // istanbul-reports/lib/lcovonly emits FN x N, then FNF / FNH, then FNDA x N.
    // Codecov, Coveralls and genhtml accept any order; matching the reference
    // implementation keeps a diff against its output minimal.
    let entries: Vec<(&FnEntry, u32)> = file
        .fn_map
        .iter()
        .map(|(id, entry)| (entry, file.f.get(id).copied().unwrap_or(0)))
        .collect();

    let total = entries.len() as u32;
    let hit = entries.iter().filter(|(_, count)| *count > 0).count() as u32;

    for (entry, _count) in &entries {
        writeln!(out, "FN:{},{}", entry.decl.start.line, sanitize_fn_name(&entry.name))?;
    }
    writeln!(out, "FNF:{total}")?;
    writeln!(out, "FNH:{hit}")?;
    for (entry, count) in &entries {
        writeln!(out, "FNDA:{},{}", count, sanitize_fn_name(&entry.name))?;
    }
    Ok(())
}

fn sanitize_fn_name(name: &str) -> String {
    // Strip exactly one matched outer pair of parens. `trim_*_matches` would
    // eat every leading `(` and trailing `)` and could mangle names like
    // `(<anonymous>)` into `<anonymous>`, leaving angle brackets that some
    // LCOV parsers treat as delimiters.
    let stripped = name.strip_prefix('(').and_then(|s| s.strip_suffix(')')).unwrap_or(name);
    if stripped.is_empty() {
        return "anonymous".to_owned();
    }
    // Replace residual parens (asymmetric inputs like `(` alone, or names
    // containing internal parens from minified output) with `_`; lcov 2.x's
    // genhtml treats `(` and `)` as delimiters in FN: records.
    stripped.cow_replace(&['(', ')'][..], "_").into_owned()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "istanbul counters are u32 on the wire, so a line count that does not fit is already outside the format"
)]
fn write_lines<W: io::Write>(out: &mut W, file: &FileCoverage) -> io::Result<()> {
    let by_line = crate::projection::line_hits(file);

    let total = by_line.len() as u32;
    let hit = by_line.values().filter(|&&v| v > 0).count() as u32;
    for (line, count) in &by_line {
        writeln!(out, "DA:{line},{count}")?;
    }
    writeln!(out, "LF:{total}")?;
    writeln!(out, "LH:{hit}")?;
    Ok(())
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "istanbul counters are u32 on the wire, so a branch index that does not fit is already outside the format"
)]
fn write_branches<W: io::Write>(out: &mut W, file: &FileCoverage) -> io::Result<()> {
    let mut total: u32 = 0;
    let mut hit: u32 = 0;
    // The `block` field of `BRDA:` must be a stable per-file discriminator so
    // Codecov does not merge unrelated branches on the same line. Istanbul
    // ids are numeric strings, which keeps the diff against istanbul-reports
    // small. If any id is non-numeric or two strings parse to the same number,
    // use iteration indexes for the whole file so blocks cannot collide.
    let numeric_blocks: Option<Vec<u32>> =
        file.branch_map.keys().map(|id| id.parse::<u32>().ok()).collect();
    let numeric_blocks = numeric_blocks
        .filter(|blocks| blocks.iter().copied().collect::<BTreeSet<_>>().len() == blocks.len());
    for (block_idx, (id, entry)) in file.branch_map.iter().enumerate() {
        let block = numeric_blocks
            .as_ref()
            .and_then(|blocks| blocks.get(block_idx))
            .copied()
            .unwrap_or(block_idx as u32);
        let arms = crate::projection::aligned_branch_hits(file, id, entry);
        let block_entered = arms.iter().any(|&c| c > 0);
        for (idx, count) in arms.iter().enumerate() {
            total += 1;
            if *count > 0 {
                hit += 1;
            }
            let taken: String = if block_entered { count.to_string() } else { "-".to_owned() };
            // Fall back to the branch entry's reported line when individual arm
            // locations have unknown positions (line == 0). Some emitters omit
            // arm-level locations.
            let line = if entry.line > 0 {
                entry.line
            } else {
                entry.locations.get(idx).map_or(0, |loc| loc.start.line)
            };
            writeln!(out, "BRDA:{line},{block},{idx},{taken}")?;
        }
    }
    writeln!(out, "BRF:{total}")?;
    writeln!(out, "BRH:{hit}")?;
    Ok(())
}

#[cfg(test)]
mod tests;
