//! Convert V8 byte-range coverage into Istanbul `FileCoverage`.
//!
//! V8's inspector protocol reports coverage as `[startOffset, endOffset, count]`
//! ranges grouped by function. Istanbul reporters consume per-statement,
//! per-function, and per-branch hit counts keyed by (line, column). The bridge
//! is to walk the same AST that `instrument()` walks, recover each location's
//! byte range, and intersect with the V8 ranges to assign counts.
//!
//! v2 covers statement, function, and branch counts from `isBlockCoverage`
//! output. Inline `//# sourceMappingURL=` comments are extracted and attached
//! to the result as `inputSourceMap`, so feeding the output through
//! [`crate::source_maps::remap_coverage`] resolves coverage positions back to
//! original sources in one chained step.
//!
//! ## CJS wrapper offset
//!
//! Node wraps every CommonJS module in `(function(exports,require,module,...){`
//! before V8 sees it. V8 byte offsets are relative to that wrapped source. Pass
//! the wrapper length (62 by default on stock Node CJS) so this module can
//! shift offsets back into the user's source. ESM modules and bare `eval`
//! sources have a wrapper length of zero.

use serde::{Deserialize, Serialize};

use crate::instrument::collect_for_v8_to_istanbul;
use crate::types::{FileCoverage, Location};

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
/// Statement, function, and branch counts are each populated from the smallest
/// V8 range containing the corresponding location. When the source carries an
/// inline `//# sourceMappingURL=data:application/json;base64,...` comment, the
/// embedded map is decoded and attached as `inputSourceMap` so the result
/// chains cleanly into [`crate::source_maps::remap_coverage`].
///
/// To resolve an external `//# sourceMappingURL=foo.js.map` reference rather
/// than the inline data-URL form, use [`v8_to_istanbul_with_loader`] and
/// supply a loader that reads the map JSON from disk (or fetches it).
pub fn v8_to_istanbul(
    source: &str,
    filename: &str,
    functions: &[V8FunctionCoverage],
    wrapper_length: u32,
) -> Result<FileCoverage, V8ToIstanbulError> {
    v8_to_istanbul_with_loader(source, filename, functions, wrapper_length, |_| None)
}

/// Like [`v8_to_istanbul`], but with a loader for external `sourceMappingURL`
/// references.
///
/// When the source carries a `//# sourceMappingURL=` trailer that is not an
/// inline `data:application/json` URL, the loader is called with the URL as
/// reported by the trailer (e.g. `foo.js.map`, `https://cdn.example/x.js.map`).
/// Returning `Some(json)` attaches the parsed map as `inputSourceMap` on the
/// result so a subsequent [`crate::source_maps::remap_coverage`] resolves
/// positions back to the original source in one chained call. Returning
/// `None` leaves `inputSourceMap` unset.
///
/// The loader is sync and infallible: side channels (disk I/O errors, HTTP
/// failures) collapse to `None`. Caller-side URL resolution (relative paths,
/// http schemes, file:// URIs) is intentionally not handled here.
///
/// # Example
///
/// ```
/// use oxc_coverage_instrument::{V8CoverageRange, V8FunctionCoverage, v8_to_istanbul_with_loader};
///
/// let source = "const x = 1;\n//# sourceMappingURL=app.js.map\n";
/// let functions = vec![V8FunctionCoverage {
///     function_name: String::new(),
///     ranges: vec![V8CoverageRange { start_offset: 0, end_offset: source.len() as u32, count: 1 }],
///     is_block_coverage: false,
/// }];
///
/// // Loader keyed on the trailer URL; here we just keep one map in-memory,
/// // but a real caller would read from disk relative to `filename`.
/// let map_json = r#"{"version":3,"sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
/// let fc = v8_to_istanbul_with_loader(source, "app.js", &functions, 0, |url| {
///     if url == "app.js.map" { Some(map_json.to_string()) } else { None }
/// })
/// .unwrap();
///
/// // The loader-supplied map is attached and can be chained into remap_coverage.
/// assert!(fc.input_source_map.is_some());
/// ```
pub fn v8_to_istanbul_with_loader<L>(
    source: &str,
    filename: &str,
    functions: &[V8FunctionCoverage],
    wrapper_length: u32,
    loader: L,
) -> Result<FileCoverage, V8ToIstanbulError>
where
    L: Fn(&str) -> Option<String>,
{
    let collected = collect_for_v8_to_istanbul(source, filename)
        .map_err(|e| V8ToIstanbulError::Parse(e.to_string()))?;
    let mut file_coverage = collected.coverage_map;
    let arm_body_byte_spans = collected.arm_body_byte_spans;
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
    for (id, branch_entry) in &file_coverage.branch_map {
        let body_spans = arm_body_byte_spans.get(id);
        let arm_counts: Vec<u32> = branch_entry
            .locations
            .iter()
            .enumerate()
            .map(|(arm_idx, loc)| {
                arm_count_for_arm(
                    source,
                    loc,
                    body_spans.and_then(|spans| spans.get(arm_idx).copied()),
                    &line_offsets,
                    &ranges,
                    wrapper_length,
                )
            })
            .collect();
        if let Some(slot) = file_coverage.b.get_mut(id) {
            *slot = arm_counts;
        }
    }

    if file_coverage.input_source_map.is_none() {
        if let Some(inline_map) = extract_inline_source_map(source) {
            file_coverage.input_source_map = Some(inline_map);
        } else if let Some(url) = extract_external_source_mapping_url(source)
            && let Some(json) = loader(url)
        {
            file_coverage.input_source_map = serde_json::from_str::<serde_json::Value>(&json).ok();
        }
    }

    Ok(file_coverage)
}

/// Pull a trailing `//# sourceMappingURL=<url>` comment from the tail of
/// `source` and return the URL when it is NOT a `data:` URI. Returns `None`
/// when no trailer is present or when the trailer is an inline data URL (the
/// inline form is handled by [`extract_inline_source_map`]).
fn extract_external_source_mapping_url(source: &str) -> Option<&str> {
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
/// esbuild, swc, and tsc) is supported. External URLs are handled via the
/// loader-accepting [`v8_to_istanbul_with_loader`] API.
fn extract_inline_source_map(source: &str) -> Option<serde_json::Value> {
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

/// Tiny base64 decoder (standard alphabet, no padding required).
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

/// Resolve the V8 hit count for a branch arm.
///
/// Unlike statements and functions (which can correctly use a containing
/// scope's count, because being inside an executed function implies the
/// statement was reachable), branch arms need *arm-level* resolution. V8
/// block coverage only emits subdivision ranges for `BlockStatement` nodes,
/// so non-block branch shapes (ternaries, logical-expr right-hand operands,
/// `default-arg` expressions, switch cases without `{ ... }`) have no V8
/// range tight to the arm body. Falling back to the enclosing function /
/// module count there over-reports execution and trips CI coverage
/// thresholds. The honest answer is 0 ("V8 did not give us per-arm data for
/// this branch shape").
///
/// The 4-byte tolerance covers the typical newline + 2-space indent gap
/// between istanbul's reported arm location and V8's `BlockStatement` range.
/// Longer gaps (`else /* comment */ {`) intentionally return 0 because the
/// match is then ambiguous; under-reporting is preferable to over-reporting.
///
/// When multiple V8 ranges fall within tolerance of the same arm (nested
/// blocks whose `{` characters happen to be close together), the *tightest*
/// match wins: minimum sum of start-distance + end-distance, ties broken by
/// the narrower range. V8 emits ranges outermost-first, so a naive
/// first-match would prefer the enclosing block over the actual arm.
///
/// When `body_byte_span` is `Some`, the resolver uses that byte range
/// directly instead of computing one from `arm_loc`. This is the if-arm 0
/// path: istanbul's whole-IfStatement convention puts `locations[0]` at the
/// outer `if (...) ... else ...` span, which V8 does not match; the
/// collected consequent-body span does match V8's block range and yields
/// "number of times the predicate evaluated truthy".
fn arm_count_for_arm(
    source: &str,
    arm_loc: &Location,
    body_byte_span: Option<(u32, u32)>,
    line_offsets: &[u32],
    ranges: &[V8CoverageRange],
    wrapper_length: u32,
) -> u32 {
    const TOLERANCE: u32 = 4;

    let (arm_start, arm_end) = match body_byte_span {
        Some((start, end)) if !(start == 0 && end == 0) => {
            (start + wrapper_length, end + wrapper_length)
        }
        _ => (
            position_to_byte_offset(source, arm_loc.start.line, arm_loc.start.column, line_offsets)
                + wrapper_length,
            position_to_byte_offset(source, arm_loc.end.line, arm_loc.end.column, line_offsets)
                + wrapper_length,
        ),
    };

    let mut best: Option<(V8CoverageRange, u32)> = None;
    for r in ranges {
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
