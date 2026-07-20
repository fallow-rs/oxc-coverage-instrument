//! V8 inspector coverage to Istanbul `FileCoverage` conversion.
//!
//! V8's inspector protocol reports coverage as `[startOffset, endOffset, count]`
//! ranges grouped by function. Istanbul reporters consume per-statement,
//! per-function, and per-branch hit counts keyed by (line, column). This crate
//! takes a pre-built `FileCoverage` (typically produced by an AST-traversal
//! pass in an instrumenter) and fills in its hit-count vectors from V8 ranges.
//!
//! ## Position semantics
//!
//! Istanbul's `Position` is 1-based line + 0-based UTF-16 column. V8 ranges are
//! absolute UTF-16 code-unit offsets into the V8-visible source. Oxc branch
//! body spans use UTF-8 byte offsets, so this crate converts source-side
//! coordinates to absolute UTF-16 offsets before intersecting V8 ranges.
//!
//! ## Wrapper base
//!
//! Node wraps every CommonJS module in `(function(exports,require,module,...){`.
//! Some coverage producers report offsets relative to a wrapped source. Pass
//! their wrapper prefix length in UTF-16 code units so source-side coordinates
//! use the same base. Node inspector coverage is normally source-relative, so
//! its wrapper length is zero.
//!
//! ## Companion helpers for inline / external source maps
//!
//! V8 coverage data does not carry source-map information; the source itself
//! does, via a `//# sourceMappingURL=` trailer. [`extract_inline_source_map`]
//! decodes the inline `data:application/json` form and
//! [`extract_external_source_mapping_url`] reports the trailer URL when it is
//! not a data URL. Callers can attach the result as `FileCoverage.input_source_map`
//! and chain through `oxc_coverage_source_maps::remap_coverage` to resolve
//! positions back to the original source.

mod source_index;
mod source_map_url;

use std::collections::BTreeMap;

use oxc_coverage_types::{FileCoverage, Location};
use serde::{Deserialize, Serialize};

use crate::source_index::SourceIndex;

pub use crate::source_map_url::{extract_external_source_mapping_url, extract_inline_source_map};

/// A function's coverage data as reported by the V8 inspector.
///
/// Serializes to / from the same JSON shape as the Chrome DevTools Protocol's
/// [`Profiler.FunctionCoverage`](https://chromedevtools.github.io/devtools-protocol/tot/Profiler/#type-FunctionCoverage)
/// so callers can hand the V8 inspector's output straight through.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V8FunctionCoverage {
    /// Function name as reported by V8 (may be empty for anonymous functions
    /// or the implicit top-level module function).
    #[serde(rename = "functionName")]
    pub function_name: String,
    /// One or more UTF-16 code-unit ranges. With `is_block_coverage = false` there is
    /// exactly one range (the whole function); with `is_block_coverage = true`
    /// the outermost range covers the function and inner ranges cover blocks.
    pub ranges: Vec<V8CoverageRange>,
    /// When true, `ranges` includes block-level subdivisions. When false, the
    /// only count is at function granularity.
    #[serde(rename = "isBlockCoverage")]
    pub is_block_coverage: bool,
}

/// A single V8 coverage range.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct V8CoverageRange {
    /// UTF-16 code-unit offset of the range start (inclusive).
    #[serde(rename = "startOffset")]
    pub start_offset: u32,
    /// UTF-16 code-unit offset of the range end (exclusive).
    #[serde(rename = "endOffset")]
    pub end_offset: u32,
    /// Hit count. Zero means the range was reachable but never executed.
    pub count: u32,
}

#[derive(Clone, Copy)]
struct CoverageRangeWithMode {
    range: V8CoverageRange,
    is_block_coverage: bool,
}

struct CoverageContext<'a> {
    source_index: &'a SourceIndex,
    ranges: &'a [V8CoverageRange],
    arm_ranges: &'a [V8CoverageRange],
    inheritance_ranges: &'a [CoverageRangeWithMode],
    wrapper_length: u32,
}

impl CoverageContext<'_> {
    fn count_for_location(&self, loc: &Location) -> u32 {
        let (start, end) = self.location_utf16_span(loc);
        smallest_containing_range_count(start, end, self.ranges)
    }

    /// Resolve one branch arm's hit count.
    ///
    /// An expression arm needs a tight V8 block range because an enclosing
    /// count cannot distinguish it from its siblings. A concrete `if` body
    /// inherits the smallest enclosing range instead, because V8 omits a child
    /// range whose count equals its parent's.
    fn arm_count_for_arm(
        &self,
        arm_loc: &Location,
        body_byte_span: Option<(u32, u32)>,
        inherit_enclosing_count: bool,
    ) -> u32 {
        if body_byte_span.is_some_and(|(start, end)| start == end) {
            return 0;
        }
        let (arm_start, arm_end) = self.arm_utf16_span(arm_loc, body_byte_span);
        let (location_start, location_end) = self.location_utf16_span(arm_loc);
        self.best_arm_range_count(arm_start, arm_end)
            .or_else(|| self.best_arm_range_count(location_start, location_end))
            .unwrap_or_else(|| {
                let has_concrete_body = body_byte_span.is_some_and(|(start, end)| start < end);
                if inherit_enclosing_count && has_concrete_body {
                    self.smallest_inheritable_range_count(arm_start, arm_end)
                } else {
                    0
                }
            })
    }

    fn smallest_inheritable_range_count(&self, start: u32, end: u32) -> u32 {
        self.inheritance_ranges
            .iter()
            .find(|entry| {
                entry.range.start_offset <= start
                    && entry.range.end_offset >= end
                    && (entry.range.start_offset < start || entry.range.end_offset > end)
            })
            .filter(|entry| entry.is_block_coverage)
            .map_or(0, |entry| entry.range.count)
    }

    fn arm_utf16_span(&self, arm_loc: &Location, body_byte_span: Option<(u32, u32)>) -> (u32, u32) {
        match body_byte_span {
            Some((start, end)) => (
                self.source_index.byte_to_utf16(start).saturating_add(self.wrapper_length),
                self.source_index.byte_to_utf16(end).saturating_add(self.wrapper_length),
            ),
            None => self.location_utf16_span(arm_loc),
        }
    }

    fn location_utf16_span(&self, loc: &Location) -> (u32, u32) {
        (
            self.source_index
                .position_to_utf16(loc.start.line, loc.start.column)
                .saturating_add(self.wrapper_length),
            self.source_index
                .position_to_utf16(loc.end.line, loc.end.column)
                .saturating_add(self.wrapper_length),
        )
    }

    /// The count of the nearest V8 range whose start and end both sit within
    /// `TOLERANCE` code units of `[arm_start, arm_end)`, breaking ties towards
    /// the narrower range.
    fn best_arm_range_count(&self, arm_start: u32, arm_end: u32) -> Option<u32> {
        const TOLERANCE: u32 = 4;

        let mut best: Option<(V8CoverageRange, u32)> = None;
        let lower = arm_start.saturating_sub(TOLERANCE);
        let upper = arm_start.saturating_add(TOLERANCE);
        let start = self.arm_ranges.partition_point(|r| r.start_offset < lower);
        for r in &self.arm_ranges[start..] {
            if r.start_offset > upper {
                break;
            }
            let dist_start = r.start_offset.abs_diff(arm_start);
            let dist_end = r.end_offset.abs_diff(arm_end);
            if dist_end > TOLERANCE {
                continue;
            }
            let distance = dist_start + dist_end;
            match best {
                None => best = Some((*r, distance)),
                Some((prev, prev_distance)) => {
                    let prev_width = prev.end_offset.saturating_sub(prev.start_offset);
                    let this_width = r.end_offset.saturating_sub(r.start_offset);
                    if distance < prev_distance
                        || (distance == prev_distance && this_width < prev_width)
                    {
                        best = Some((*r, distance));
                    }
                }
            }
        }
        best.map(|(r, _)| r.count)
    }
}

/// Fill in the statement, function and branch hit counts of a pre-built
/// [`FileCoverage`] from V8's coverage ranges.
///
/// The caller builds `file_coverage`, typically from an instrumenter's
/// AST-traversal pass, and supplies `arm_body_byte_spans`:
/// `arm_body_byte_spans["<branch_id>"][<arm_idx>]` is the `(start, end)` UTF-8
/// byte range of that arm's body. A `(0, 0)` entry marks a synthetic body (a
/// synthesized else-arm, for instance) and resolves to a count of 0; a
/// non-empty body span is matched against V8's block ranges before falling back
/// to the arm's [`Location`].
///
/// `wrapper_length` is the UTF-16 code-unit base for producers that report
/// wrapper-shifted ranges. Pass 0 for source-relative inspector output.
pub fn apply_v8_coverage(
    file_coverage: &mut FileCoverage,
    source: &str,
    functions: &[V8FunctionCoverage],
    wrapper_length: u32,
    arm_body_byte_spans: &BTreeMap<String, Vec<(u32, u32)>>,
) {
    let source_index = SourceIndex::new(source);
    let mut ranges: Vec<V8CoverageRange> =
        functions.iter().flat_map(|f| f.ranges.iter().copied()).collect();
    ranges.sort_by_key(|r| r.end_offset.saturating_sub(r.start_offset));
    // The first range in every FunctionCoverage record is that function's outer
    // range, not a branch range. A nested function declaration can sit inside an
    // if arm, so only child ranges from block-coverage records are eligible for
    // tight arm matching.
    let mut arm_ranges: Vec<V8CoverageRange> = functions
        .iter()
        .filter(|function| function.is_block_coverage)
        .flat_map(|function| function.ranges.iter().skip(1).copied())
        .collect();
    arm_ranges.sort_by_key(|r| r.start_offset);
    let mut inheritance_ranges: Vec<CoverageRangeWithMode> = functions
        .iter()
        .flat_map(|function| {
            function.ranges.iter().copied().map(|range| CoverageRangeWithMode {
                range,
                is_block_coverage: function.is_block_coverage,
            })
        })
        .collect();
    inheritance_ranges
        .sort_by_key(|entry| entry.range.end_offset.saturating_sub(entry.range.start_offset));
    let context = CoverageContext {
        source_index: &source_index,
        ranges: &ranges,
        arm_ranges: &arm_ranges,
        inheritance_ranges: &inheritance_ranges,
        wrapper_length,
    };

    apply_statement_counts(file_coverage, &context);
    apply_function_counts(file_coverage, &context);
    apply_branch_counts(file_coverage, arm_body_byte_spans, &context);
}

fn apply_statement_counts(file_coverage: &mut FileCoverage, context: &CoverageContext<'_>) {
    for (id, loc) in &file_coverage.statement_map {
        let count = context.count_for_location(loc);
        if let Some(slot) = file_coverage.s.get_mut(id) {
            *slot = count;
        }
    }
}

fn apply_function_counts(file_coverage: &mut FileCoverage, context: &CoverageContext<'_>) {
    for (id, fn_entry) in &file_coverage.fn_map {
        let count = context.count_for_location(&fn_entry.loc);
        if let Some(slot) = file_coverage.f.get_mut(id) {
            *slot = count;
        }
    }
}

fn apply_branch_counts(
    file_coverage: &mut FileCoverage,
    arm_body_byte_spans: &BTreeMap<String, Vec<(u32, u32)>>,
    context: &CoverageContext<'_>,
) {
    for (id, branch_entry) in &file_coverage.branch_map {
        let Some(slot) = file_coverage.b.get_mut(id) else {
            continue;
        };
        let body_spans = arm_body_byte_spans.get(id);
        let inherit_enclosing_count = branch_entry.branch_type == "if";
        slot.clear();
        slot.reserve(branch_entry.locations.len());
        for (arm_idx, loc) in branch_entry.locations.iter().enumerate() {
            slot.push(context.arm_count_for_arm(
                loc,
                body_spans.and_then(|spans| spans.get(arm_idx).copied()),
                inherit_enclosing_count,
            ));
        }
    }
}

/// Pick the count of the smallest V8 range that fully contains `[start, end)`.
/// Smaller ranges represent inner blocks (with their own counts under
/// `isBlockCoverage`) and override the outer function-level count.
///
/// Both V8 ranges and the statement UTF-16 span use the half-open convention
/// (`endOffset` / `end` are exclusive). The containment predicate is therefore
/// `r.start <= start && r.end >= end`: a range whose exclusive end is equal
/// to the statement's exclusive end is the smallest possible exact container.
fn smallest_containing_range_count(start: u32, end: u32, ranges: &[V8CoverageRange]) -> u32 {
    for r in ranges {
        if r.start_offset <= start && r.end_offset >= end {
            return r.count;
        }
    }
    0
}
