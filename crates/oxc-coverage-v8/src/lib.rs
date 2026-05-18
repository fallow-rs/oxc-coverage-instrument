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
//! byte offsets into the V8-visible source. This crate walks each location's
//! UTF-16 column into a byte offset, then intersects against the V8 ranges.
//!
//! ## CJS wrapper offset
//!
//! Node wraps every CommonJS module in `(function(exports,require,module,...){`
//! before V8 sees it. V8 byte offsets are relative to that wrapped source. Pass
//! the wrapper length (62 by default on stock Node CJS) so this crate can shift
//! offsets back into the user's source. ESM modules and bare `eval` sources
//! have a wrapper length of zero.
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

use std::collections::BTreeMap;

use oxc_coverage_types::{FileCoverage, Location};
use serde::{Deserialize, Serialize};

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

struct CoverageContext<'a> {
    source: &'a str,
    line_offsets: &'a [u32],
    ranges: &'a [V8CoverageRange],
    wrapper_length: u32,
}

impl CoverageContext<'_> {
    fn count_for_location(&self, loc: &Location) -> u32 {
        let start =
            self.position_to_byte_offset(loc.start.line, loc.start.column) + self.wrapper_length;
        let end = self.position_to_byte_offset(loc.end.line, loc.end.column) + self.wrapper_length;
        smallest_containing_range_count(start, end, self.ranges)
    }

    // Branch arms need a tight V8 block range. Falling back to an enclosing
    // function/module range would over-report uncovered ternary and logical arms.
    fn arm_count_for_arm(&self, arm_loc: &Location, body_byte_span: Option<(u32, u32)>) -> u32 {
        const TOLERANCE: u32 = 4;

        let (arm_start, arm_end) = match body_byte_span {
            Some((start, end)) if !(start == 0 && end == 0) => {
                (start + self.wrapper_length, end + self.wrapper_length)
            }
            _ => (
                self.position_to_byte_offset(arm_loc.start.line, arm_loc.start.column)
                    + self.wrapper_length,
                self.position_to_byte_offset(arm_loc.end.line, arm_loc.end.column)
                    + self.wrapper_length,
            ),
        };

        let mut best: Option<(V8CoverageRange, u32)> = None;
        for r in self.ranges {
            let dist_start = r.start_offset.abs_diff(arm_start);
            let dist_end = r.end_offset.abs_diff(arm_end);
            if dist_start > TOLERANCE || dist_end > TOLERANCE {
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
        best.map_or(0, |(r, _)| r.count)
    }

    // Istanbul columns are UTF-16 code units, while V8 ranges are byte offsets.
    fn position_to_byte_offset(&self, line_1based: u32, col_utf16: u32) -> u32 {
        if line_1based == 0 {
            return 0;
        }
        let line_idx = (line_1based - 1) as usize;
        if line_idx >= self.line_offsets.len() - 1 {
            return *self.line_offsets.last().unwrap_or(&0);
        }
        let line_start = self.line_offsets[line_idx] as usize;
        let line_end = self.line_offsets[line_idx + 1] as usize;
        let line_bytes = self.source.get(line_start..line_end).unwrap_or("");

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
}

/// Apply V8 coverage ranges to a pre-built `FileCoverage` by filling in its
/// statement, function, and branch hit-count vectors.
///
/// The caller is responsible for constructing `file_coverage` (typically via
/// an instrumenter's AST-traversal pass) and supplying `arm_body_byte_spans`
/// for branches. `arm_body_byte_spans["<branch_id>"][<arm_idx>]` is the
/// `(start, end)` byte range of the arm body when known, or `(0, 0)` when the
/// body span is synthetic / unknown (e.g. a synthesized else-arm). See the
/// `arm_count_for_arm` documentation for how this side-table resolves the
/// if-arm 0 case that istanbul's whole-IfStatement convention puts at
/// `locations[0]`.
///
/// `wrapper_length` accounts for Node's CJS module wrapper prefix
/// (`(function(exports,require,module,__filename,__dirname){`). Pass 0 for
/// ESM.
pub fn apply_v8_coverage(
    file_coverage: &mut FileCoverage,
    source: &str,
    functions: &[V8FunctionCoverage],
    wrapper_length: u32,
    arm_body_byte_spans: &BTreeMap<String, Vec<(u32, u32)>>,
) {
    let line_offsets = compute_line_offsets(source);
    let ranges: Vec<V8CoverageRange> =
        functions.iter().flat_map(|f| f.ranges.iter().copied()).collect();
    let context =
        CoverageContext { source, line_offsets: &line_offsets, ranges: &ranges, wrapper_length };

    for (id, loc) in &file_coverage.statement_map {
        let count = context.count_for_location(loc);
        if let Some(slot) = file_coverage.s.get_mut(id) {
            *slot = count;
        }
    }
    for (id, fn_entry) in &file_coverage.fn_map {
        let count = context.count_for_location(&fn_entry.loc);
        if let Some(slot) = file_coverage.f.get_mut(id) {
            *slot = count;
        }
    }
    for (id, branch_entry) in &file_coverage.branch_map {
        let body_spans = arm_body_byte_spans.get(id);
        let arm_counts: Vec<u32> = branch_entry
            .locations
            .iter()
            .enumerate()
            .map(|(arm_idx, loc)| {
                context.arm_count_for_arm(
                    loc,
                    body_spans.and_then(|spans| spans.get(arm_idx).copied()),
                )
            })
            .collect();
        if let Some(slot) = file_coverage.b.get_mut(id) {
            *slot = arm_counts;
        }
    }
}

/// Pull a trailing `//# sourceMappingURL=<url>` comment from the tail of
/// `source` and return the URL when it is NOT a `data:` URI. Returns `None`
/// when no trailer is present or when the trailer is an inline data URL (the
/// inline form is handled by [`extract_inline_source_map`]).
#[must_use]
pub fn extract_external_source_mapping_url(source: &str) -> Option<&str> {
    const NEEDLE: &str = "//# sourceMappingURL=";
    let idx = source.rfind(NEEDLE)?;
    let after = &source[idx + NEEDLE.len()..];
    let url = after.lines().next()?.trim();
    if url.is_empty() || url.starts_with("data:") {
        return None;
    }
    Some(url)
}

/// Pull an inline `//# sourceMappingURL=data:application/json;base64,...`
/// comment from the tail of `source` and decode the embedded source map.
///
/// Only the data-URL form (the dominant case for ESM bundles emitted by Vite,
/// esbuild, swc, and tsc) is supported here. External URLs are handled via
/// [`extract_external_source_mapping_url`] + a caller-supplied loader.
#[must_use]
pub fn extract_inline_source_map(source: &str) -> Option<serde_json::Value> {
    const NEEDLE: &str = "//# sourceMappingURL=data:application/json";
    let idx = source.rfind(NEEDLE)?;
    let line = source[idx..].lines().next()?;

    let comma = line.find(',')?;
    let payload = &line[comma + 1..];
    let is_base64 = line[..comma].contains(";base64");
    let json = if is_base64 {
        let bytes = decode_base64(payload).ok()?;
        String::from_utf8(bytes).ok()?
    } else {
        urlencoding_decode(payload).ok()?
    };
    serde_json::from_str(&json).ok()
}

/// Tiny base64 decoder (standard + URL-safe alphabets, no padding required).
/// Kept in-crate to avoid pulling a base64 dep just for this one site.
fn decode_base64(input: &str) -> Result<Vec<u8>, ()> {
    fn value(c: u8) -> Result<u8, ()> {
        // Accepts both the standard (RFC 4648 §4) and URL-safe (RFC 4648 §5)
        // alphabets so esbuild-emitted inline maps (which use the URL-safe
        // alphabet in some output modes) decode without a silent miss.
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' | b'-' => Ok(62),
            b'/' | b'_' => Ok(63),
            _ => Err(()),
        }
    }
    let trimmed: Vec<u8> =
        input.bytes().filter(|b| *b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(trimmed.len() * 3 / 4);
    for chunk in trimmed.chunks(4) {
        let n0 = value(chunk[0])?;
        let n1 = value(chunk[1])?;
        out.push((n0 << 2) | (n1 >> 4));
        if let Some(&c2) = chunk.get(2) {
            let n2 = value(c2)?;
            out.push((n1 << 4) | (n2 >> 2));
            if let Some(&c3) = chunk.get(3) {
                let n3 = value(c3)?;
                out.push((n2 << 6) | n3);
            }
        }
    }
    Ok(out)
}

/// Decode percent-encoded URL payload for non-base64 inline source maps.
fn urlencoding_decode(input: &str) -> Result<String, ()> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16).ok_or(())? as u8;
            let lo = (bytes[i + 2] as char).to_digit(16).ok_or(())? as u8;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
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
