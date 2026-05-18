//! Convert V8 byte-range coverage into Istanbul `FileCoverage`.
//!
//! V8's inspector protocol reports coverage as `[startOffset, endOffset, count]`
//! ranges grouped by function. Istanbul reporters consume per-statement,
//! per-function, and per-branch hit counts keyed by (line, column). The bridge
//! is to walk the same AST that `instrument()` walks, recover each location's
//! byte range, and intersect with the V8 ranges to assign counts.
//!
//! Status: v1 covers statement and function counts from `isBlockCoverage`
//! output. Branch counts and inline-`sourceMappingURL` extraction are pinned
//! follow-ups. Composing through a separate `inputSourceMap` should be done
//! by feeding the result into [`crate::source_maps::remap_coverage`].
//!
//! ## CJS wrapper offset
//!
//! Node wraps every CommonJS module in `(function(exports,require,module,...){`
//! before V8 sees it. V8 byte offsets are relative to that wrapped source. Pass
//! the wrapper length (62 by default on stock Node CJS) so this module can
//! shift offsets back into the user's source. ESM modules and bare `eval`
//! sources have a wrapper length of zero.

use serde::{Deserialize, Serialize};

use crate::types::{FileCoverage, Location};
use crate::{InstrumentOptions, instrument};

/// A function's coverage data as reported by the V8 inspector.
///
/// Serializes to / from the same JSON shape as
/// [`node:inspector`'s `Profiler.FunctionCoverage`](https://nodejs.org/api/inspector.html)
/// so callers can hand the V8 inspector's output straight through.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V8FunctionCoverage {
    /// Function name as reported by V8 (may be empty for anonymous functions
    /// or the implicit top-level module function).
    #[serde(rename = "functionName")]
    pub function_name: String,
    /// One or more byte ranges. With `is_block_coverage = false` there is
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
    /// Byte offset of the range start (inclusive) in the V8-visible source.
    #[serde(rename = "startOffset")]
    pub start_offset: u32,
    /// Byte offset of the range end (exclusive).
    #[serde(rename = "endOffset")]
    pub end_offset: u32,
    /// Hit count. Zero means the range was reachable but never executed.
    pub count: u32,
}

/// Errors produced by the V8-to-Istanbul conversion.
#[derive(Debug)]
pub enum V8ToIstanbulError {
    /// The source could not be parsed.
    Parse(String),
}

impl std::fmt::Display for V8ToIstanbulError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for V8ToIstanbulError {}

/// Convert V8 function coverage into Istanbul `FileCoverage`.
///
/// `wrapper_length` accounts for Node's CJS module wrapper prefix
/// (`(function(exports,require,module,__filename,__dirname){`). Pass 0 for ESM.
///
/// Statement and function counts are populated from the smallest V8 range
/// containing each location. Branch counts are emitted at zero in v1; richer
/// branch resolution against block-coverage ranges is a follow-up.
pub fn v8_to_istanbul(
    source: &str,
    filename: &str,
    functions: &[V8FunctionCoverage],
    wrapper_length: u32,
) -> Result<FileCoverage, V8ToIstanbulError> {
    // TODO(v2): swap for a visit-only AST pass that collects locations
    // without emitting the instrumented code + preamble we throw away.
    let instrumented = instrument(source, filename, &InstrumentOptions::default())
        .map_err(|e| V8ToIstanbulError::Parse(e.to_string()))?;

    let mut file_coverage = instrumented.coverage_map;
    let line_offsets = compute_line_offsets(source);
    let ranges: Vec<V8CoverageRange> =
        functions.iter().flat_map(|f| f.ranges.iter().copied()).collect();

    for (id, loc) in &file_coverage.statement_map {
        let count = count_for_location(source, loc, &line_offsets, &ranges, wrapper_length);
        if let Some(slot) = file_coverage.s.get_mut(id) {
            *slot = count;
        }
    }
    for (id, fn_entry) in &file_coverage.fn_map {
        let count =
            count_for_location(source, &fn_entry.loc, &line_offsets, &ranges, wrapper_length);
        if let Some(slot) = file_coverage.f.get_mut(id) {
            *slot = count;
        }
    }

    Ok(file_coverage)
}

/// Precompute byte offsets for the start of each line in `source`.
/// `line_offsets[N]` is the byte offset of the (0-based) Nth line's first
/// character. `line_offsets.len()` equals the line count plus one (sentinel
/// at the end of the source so the last line's range is also bounded).
fn compute_line_offsets(source: &str) -> Vec<u32> {
    let mut offsets = vec![0u32];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            let next = u32::try_from(i + 1).unwrap_or(u32::MAX);
            offsets.push(next);
        }
    }
    let end = u32::try_from(source.len()).unwrap_or(u32::MAX);
    offsets.push(end);
    offsets
}

/// Byte offset of an Istanbul `(line, column)` inside `source`.
///
/// Istanbul columns are UTF-16 code units (Babel + `istanbul-lib-instrument`
/// convention). srcmap is byte-based. For ASCII the two collapse, but for
/// non-ASCII source the byte position must be computed by walking the line
/// and consuming `col_utf16` UTF-16 code units. The walk is bounded by the
/// `line_offsets` sentinel so a column past end-of-line clamps to end-of-line.
fn position_to_byte_offset(
    source: &str,
    line_1based: u32,
    col_utf16: u32,
    line_offsets: &[u32],
) -> u32 {
    if line_1based == 0 {
        return 0;
    }
    let line_idx = (line_1based - 1) as usize;
    if line_idx >= line_offsets.len() - 1 {
        return *line_offsets.last().unwrap_or(&0);
    }
    let line_start = line_offsets[line_idx] as usize;
    let line_end = line_offsets[line_idx + 1] as usize;
    let line_bytes = source.get(line_start..line_end).unwrap_or("");

    let mut utf16_remaining = col_utf16;
    let mut byte_in_line = 0usize;
    for ch in line_bytes.chars() {
        if utf16_remaining == 0 {
            break;
        }
        let units = ch.len_utf16() as u32;
        if units > utf16_remaining {
            break;
        }
        utf16_remaining -= units;
        byte_in_line += ch.len_utf8();
    }

    u32::try_from(line_start + byte_in_line).unwrap_or(u32::MAX)
}

fn count_for_location(
    source: &str,
    loc: &Location,
    line_offsets: &[u32],
    ranges: &[V8CoverageRange],
    wrapper_length: u32,
) -> u32 {
    let start = position_to_byte_offset(source, loc.start.line, loc.start.column, line_offsets)
        + wrapper_length;
    let end = position_to_byte_offset(source, loc.end.line, loc.end.column, line_offsets)
        + wrapper_length;
    smallest_containing_range_count(start, end, ranges)
}

/// Pick the count of the smallest V8 range that fully contains `[start, end)`.
/// Smaller ranges represent inner blocks (with their own counts under
/// `isBlockCoverage`) and override the outer function-level count.
///
/// Both V8 ranges and the statement byte span use the half-open convention
/// (`endOffset` / `end` are exclusive). The containment predicate is therefore
/// `r.start <= start && r.end >= end`: a range whose exclusive end is equal
/// to the statement's exclusive end is the smallest possible exact container.
fn smallest_containing_range_count(start: u32, end: u32, ranges: &[V8CoverageRange]) -> u32 {
    let mut best: Option<V8CoverageRange> = None;
    for r in ranges {
        if r.start_offset <= start && r.end_offset >= end {
            let width = r.end_offset.saturating_sub(r.start_offset);
            match best {
                None => best = Some(*r),
                Some(prev) => {
                    let prev_width = prev.end_offset.saturating_sub(prev.start_offset);
                    if width < prev_width {
                        best = Some(*r);
                    }
                }
            }
        }
    }
    best.map_or(0, |r| r.count)
}
