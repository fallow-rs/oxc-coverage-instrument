//! Per-source-line rollups of the branch and function counters, keyed by
//! the line the detail page renders them on. Statement counters roll up
//! through [`crate::projection::line_hits`], shared with the lcov and
//! cobertura emitters.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry};

use crate::escape::html_text;
use crate::projection::{aligned_branch_hits, branch_line, branched_lines};

/// Every branch arm reported on one source line, plus the arm totals the
/// row's `partial` class keys off.
pub(super) struct BranchSummary {
    /// Arms declared in `branchMap` for this line.
    pub(super) total: u32,
    /// Arms with a non-zero hit count.
    pub(super) covered: u32,
    arms: Vec<BranchArm>,
}

struct BranchArm {
    label: String,
    hits: u32,
}

impl BranchSummary {
    /// Per-arm markup for the row's branch note, or `None` when the line
    /// carries more arms than fit beside the source (a wide `switch`), in
    /// which case the caller falls back to a covered/total count.
    pub(super) fn detail_html(&self) -> Option<String> {
        if self.arms.is_empty() || self.arms.len() > 4 {
            return None;
        }
        let mut out = String::from("branches:");
        for arm in &self.arms {
            let mark = if arm.hits > 0 { "hit" } else { "miss" };
            let symbol = if arm.hits > 0 { "&#10003;" } else { "&#10007;" };
            let _ = write!(
                out,
                " <span class=\"branch-arm branch-arm--{mark}\">{symbol} {}={}</span>",
                html_text(&arm.label),
                arm.hits
            );
        }
        Some(out)
    }

    /// Spoken equivalent of [`BranchSummary::detail_html`], gated on the
    /// same arm count so the two stay in step.
    pub(super) fn detail_aria(&self) -> Option<String> {
        if self.arms.is_empty() || self.arms.len() > 4 {
            return None;
        }
        let mut parts = Vec::with_capacity(self.arms.len());
        for arm in &self.arms {
            let state = if arm.hits > 0 { "hit" } else { "missed" };
            parts.push(format!("{} arm {state} {} times", arm.label, arm.hits));
        }
        Some(parts.join(", "))
    }
}

/// The shared per-line arm totals, labelled with the arm names the detail
/// page renders beside the source.
pub(super) fn compute_branched_lines(file: &FileCoverage) -> BTreeMap<u32, BranchSummary> {
    let mut arms_by_line: BTreeMap<u32, Vec<BranchArm>> = BTreeMap::new();
    for (id, entry) in &file.branch_map {
        let line = branch_line(entry);
        if line == 0 {
            continue;
        }
        let hits = aligned_branch_hits(file, id, entry);
        arms_by_line.entry(line).or_default().extend(branch_arms(entry, &hits));
    }

    branched_lines(file)
        .into_iter()
        .map(|(line, (total, covered))| {
            let arms = arms_by_line.remove(&line).unwrap_or_default();
            (line, BranchSummary { total, covered, arms })
        })
        .collect()
}

fn branch_arms(entry: &BranchEntry, hits: &[u32]) -> Vec<BranchArm> {
    hits.iter()
        .enumerate()
        .map(|(idx, &hits)| BranchArm { label: branch_arm_label(entry, idx), hits })
        .collect()
}

fn branch_arm_label(entry: &BranchEntry, idx: usize) -> String {
    match entry.branch_type.as_str() {
        "if" => match idx {
            0 => "true".to_owned(),
            1 => "false".to_owned(),
            _ => format!("arm {}", idx + 1),
        },
        "cond-expr" => match idx {
            0 => "consequent".to_owned(),
            1 => "alternate".to_owned(),
            _ => format!("arm {}", idx + 1),
        },
        "switch" => format!("case {}", idx + 1),
        "binary-expr" => format!("operand {}", idx + 1),
        "default-arg" => "default".to_owned(),
        _ => format!("arm {}", idx + 1),
    }
}

pub(super) fn compute_fn_lines(file: &FileCoverage) -> BTreeMap<u32, u32> {
    let mut out: BTreeMap<u32, u32> = BTreeMap::new();
    for (id, entry) in &file.fn_map {
        let line = fn_line(entry);
        if line == 0 {
            continue;
        }
        let hits = file.f.get(id).copied().unwrap_or(0);
        out.entry(line).and_modify(|v| *v = (*v).max(hits)).or_insert(hits);
    }
    out
}

fn fn_line(entry: &FnEntry) -> u32 {
    if entry.line > 0 { entry.line } else { entry.decl.start.line }
}
