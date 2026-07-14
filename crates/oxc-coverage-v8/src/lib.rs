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
//! Node wraps every CommonJS module in `(function(exports,require,module,...){`
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

#[derive(Clone, Copy)]
struct NonAsciiSpan {
    byte_start: u32,
    byte_end: u32,
    utf16_start: u32,
    utf16_end: u32,
}

struct SourceIndex {
    line_starts_utf16: Vec<u32>,
    line_ends_utf16: Vec<u32>,
    non_ascii_spans: Vec<NonAsciiSpan>,
    byte_len: u32,
    utf16_len: u32,
}

impl SourceIndex {
    fn new(source: &str) -> Self {
        let mut line_starts_utf16 = vec![0];
        let mut line_ends_utf16 = Vec::new();
        let mut non_ascii_spans = Vec::new();
        let mut utf16_offset = 0u32;
        let mut previous_was_carriage_return = false;

        for (byte_start, ch) in source.char_indices() {
            let byte_start = u32::try_from(byte_start).unwrap_or(u32::MAX);
            let byte_width = ch.len_utf8() as u32;
            let utf16_width = ch.len_utf16() as u32;
            if byte_width != utf16_width {
                non_ascii_spans.push(NonAsciiSpan {
                    byte_start,
                    byte_end: byte_start.saturating_add(byte_width),
                    utf16_start: utf16_offset,
                    utf16_end: utf16_offset.saturating_add(utf16_width),
                });
            }
            if ch == '\n' {
                line_ends_utf16
                    .push(utf16_offset.saturating_sub(u32::from(previous_was_carriage_return)));
            }
            utf16_offset = utf16_offset.saturating_add(utf16_width);
            if ch == '\n' {
                line_starts_utf16.push(utf16_offset);
            }
            previous_was_carriage_return = ch == '\r';
        }
        line_ends_utf16.push(utf16_offset);

        Self {
            line_starts_utf16,
            line_ends_utf16,
            non_ascii_spans,
            byte_len: u32::try_from(source.len()).unwrap_or(u32::MAX),
            utf16_len: utf16_offset,
        }
    }

    fn byte_to_utf16(&self, byte_offset: u32) -> u32 {
        let byte_offset = byte_offset.min(self.byte_len);
        let index = self.non_ascii_spans.partition_point(|span| span.byte_end <= byte_offset);
        if let Some(span) = self.non_ascii_spans.get(index)
            && byte_offset > span.byte_start
            && byte_offset < span.byte_end
        {
            return span.utf16_start;
        }
        let byte_utf16_delta = index
            .checked_sub(1)
            .and_then(|previous| self.non_ascii_spans.get(previous))
            .map_or(0, |span| span.byte_end.saturating_sub(span.utf16_end));
        byte_offset.saturating_sub(byte_utf16_delta).min(self.utf16_len)
    }

    fn position_to_utf16(&self, line_1based: u32, column_utf16: u32) -> u32 {
        if line_1based == 0 {
            return 0;
        }
        let line_index = (line_1based - 1) as usize;
        let Some(line_start) = self.line_starts_utf16.get(line_index).copied() else {
            return self.utf16_len;
        };
        let line_end = self.line_ends_utf16.get(line_index).copied().unwrap_or(self.utf16_len);
        line_start.saturating_add(column_utf16.min(line_end.saturating_sub(line_start)))
    }
}

struct CoverageInput<'a> {
    source: &'a str,
    functions: &'a [V8FunctionCoverage],
    wrapper_length: u32,
    arm_body_byte_spans: &'a BTreeMap<String, Vec<(u32, u32)>>,
}

impl CoverageContext<'_> {
    fn count_for_location(&self, loc: &Location) -> u32 {
        let start = self
            .source_index
            .position_to_utf16(loc.start.line, loc.start.column)
            .saturating_add(self.wrapper_length);
        let end = self
            .source_index
            .position_to_utf16(loc.end.line, loc.end.column)
            .saturating_add(self.wrapper_length);
        smallest_containing_range_count(start, end, self.ranges)
    }

    // Expression arms need a tight V8 block range because an enclosing count
    // cannot distinguish them. Concrete if bodies inherit the smallest
    // enclosing range when V8 omits a child range with the same count.
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
            Some((start, end)) if !(start == 0 && end == 0) => (
                self.source_index.byte_to_utf16(start).saturating_add(self.wrapper_length),
                self.source_index.byte_to_utf16(end).saturating_add(self.wrapper_length),
            ),
            _ => self.location_utf16_span(arm_loc),
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
/// `wrapper_length` is an explicit UTF-16 code-unit base for producers that
/// report wrapper-shifted ranges. Pass 0 for source-relative inspector output.
pub fn apply_v8_coverage(
    file_coverage: &mut FileCoverage,
    source: &str,
    functions: &[V8FunctionCoverage],
    wrapper_length: u32,
    arm_body_byte_spans: &BTreeMap<String, Vec<(u32, u32)>>,
) {
    let input = CoverageInput { source, functions, wrapper_length, arm_body_byte_spans };
    apply_v8_coverage_inner(file_coverage, &input);
}

fn apply_v8_coverage_inner(file_coverage: &mut FileCoverage, input: &CoverageInput<'_>) {
    let source_index = SourceIndex::new(input.source);
    let mut ranges: Vec<V8CoverageRange> =
        input.functions.iter().flat_map(|f| f.ranges.iter().copied()).collect();
    ranges.sort_by_key(|r| r.end_offset.saturating_sub(r.start_offset));
    // The first range in every FunctionCoverage record is that function's
    // outer range, not a branch range. Nested function declarations can sit
    // inside an if arm, so only child ranges from block coverage records are
    // eligible for tight arm matching.
    let mut arm_ranges: Vec<V8CoverageRange> = input
        .functions
        .iter()
        .filter(|function| function.is_block_coverage)
        .flat_map(|function| function.ranges.iter().skip(1).copied())
        .collect();
    arm_ranges.sort_by_key(|r| r.start_offset);
    let mut inheritance_ranges: Vec<CoverageRangeWithMode> = input
        .functions
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
        wrapper_length: input.wrapper_length,
    };

    apply_statement_counts(file_coverage, &context);
    apply_function_counts(file_coverage, &context);
    apply_branch_counts(file_coverage, input, &context);
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
    input: &CoverageInput<'_>,
    context: &CoverageContext<'_>,
) {
    for (id, branch_entry) in &file_coverage.branch_map {
        let Some(slot) = file_coverage.b.get_mut(id) else {
            continue;
        };
        let body_spans = input.arm_body_byte_spans.get(id);
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

const SOURCE_MAPPING_URL_PREFIX: &str = "//# sourceMappingURL=";

fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn source_mapping_url_trailer(source: &str) -> Option<&str> {
    let source = source.trim_end();
    let line_start = source
        .char_indices()
        .rev()
        .find_map(|(index, ch)| is_line_terminator(ch).then_some(index + ch.len_utf8()))
        .unwrap_or(0);
    let line = source.get(line_start..)?.trim_start();
    let url = line.strip_prefix(SOURCE_MAPPING_URL_PREFIX)?.trim();
    if url.is_empty() || url.chars().any(|ch| matches!(ch, '\'' | '"' | '`')) {
        return None;
    }
    Some(url)
}

/// Pull a trailing `//# sourceMappingURL=<url>` comment from the final
/// non-whitespace physical line of `source` and return the URL when it is NOT
/// a `data:` URI. Returns `None` when no trailer is present or when the trailer
/// is an inline data URL (the inline form is handled by
/// [`extract_inline_source_map`]).
#[must_use]
pub fn extract_external_source_mapping_url(source: &str) -> Option<&str> {
    let url = source_mapping_url_trailer(source)?;
    (!url.starts_with("data:")).then_some(url)
}

/// Pull an inline `//# sourceMappingURL=data:application/json;base64,...`
/// comment from the final non-whitespace physical line of `source` and decode
/// the embedded source map.
///
/// Only the data-URL form (the dominant case for ESM bundles emitted by Vite,
/// esbuild, swc, and tsc) is supported here. External URLs are handled via
/// [`extract_external_source_mapping_url`] + a caller-supplied loader.
#[must_use]
pub fn extract_inline_source_map(source: &str) -> Option<serde_json::Value> {
    let url = source_mapping_url_trailer(source)?;
    let data = url.strip_prefix("data:application/json")?;
    let comma = data.find(',')?;
    let metadata = &data[..comma];
    let payload = &data[comma + 1..];
    let json = if metadata.split(';').any(|part| part == "base64") {
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

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0u8; 4];
    let mut len = 0usize;
    for byte in input.bytes() {
        if byte == b'=' || byte.is_ascii_whitespace() {
            continue;
        }
        chunk[len] = byte;
        len += 1;
        if len == 4 {
            let n0 = value(chunk[0])?;
            let n1 = value(chunk[1])?;
            let n2 = value(chunk[2])?;
            let n3 = value(chunk[3])?;
            out.push((n0 << 2) | (n1 >> 4));
            out.push((n1 << 4) | (n2 >> 2));
            out.push((n2 << 6) | n3);
            len = 0;
        }
    }
    if len == 1 {
        return Err(());
    }
    if len > 1 {
        let n0 = value(chunk[0])?;
        let n1 = value(chunk[1])?;
        out.push((n0 << 2) | (n1 >> 4));
        if len > 2 {
            let n2 = value(chunk[2])?;
            out.push((n1 << 4) | (n2 >> 2));
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
