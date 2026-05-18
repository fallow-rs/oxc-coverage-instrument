//! Remap coverage data through embedded `inputSourceMap` to original sources.
//!
//! Istanbul's coverage data carries an `inputSourceMap` on each `FileCoverage`
//! when the instrumented input was already a transform output (e.g. TypeScript
//! emitted via `tsc` and then instrumented). Downstream coverage reporters
//! (nyc, `@vitest/coverage-istanbul`, monocart) call into
//! `istanbul-lib-source-maps` to walk that source map and rewrite every
//! coverage position back to the original source. This module is the Rust
//! equivalent for the "Mode A" (remap-at-report-time) path that Vitest uses;
//! the cold-start nyc disk-read path stays on the JS implementation for now.
//!
//! Position semantics: Istanbul's `Position` is 1-based line + 0-based UTF-16
//! column. `srcmap-sourcemap`'s `original_position_for` is 0-based for both.
//! Conversion happens at the lookup boundary.

use crate::types::{BranchEntry, FileCoverage, FnEntry, Location, Position};

/// Remap a single `FileCoverage` through its embedded `inputSourceMap`.
///
/// Returns `None` when the entry has no `inputSourceMap`, when that map fails
/// to parse, or when it declares no usable source. Callers should fall back to
/// the original coverage entry in those cases.
///
/// When the input map declares a `sourceRoot`, the resolved `path` is the
/// `sourceRoot` joined with the first entry in `sources` (matching
/// `istanbul-lib-source-maps` semantics).
pub fn remap_coverage(coverage: &FileCoverage) -> Option<FileCoverage> {
    let input_sm_value = coverage.input_source_map.as_ref()?;
    let input_sm_json = serde_json::to_string(input_sm_value).ok()?;
    let sm = srcmap_sourcemap::SourceMap::from_json(&input_sm_json).ok()?;

    let primary_source = resolve_primary_source(&sm)?;

    let mut out = coverage.clone();
    out.path = primary_source;
    out.input_source_map = None;

    for loc in out.statement_map.values_mut() {
        remap_location(loc, &sm);
    }
    for fn_entry in out.fn_map.values_mut() {
        remap_fn_entry(fn_entry, &sm);
    }
    for branch_entry in out.branch_map.values_mut() {
        remap_branch_entry(branch_entry, &sm);
    }

    Some(out)
}

/// Remap every `FileCoverage` in a coverage map. Entries without an
/// `inputSourceMap` pass through unchanged under their original key. Entries
/// with an `inputSourceMap` are rewritten and re-keyed by their resolved
/// original source path.
///
/// When two entries remap to the same original path (rare but possible for
/// bundled output where multiple instrumented chunks share a source), the
/// later entry replaces the earlier; richer merging belongs to a future
/// `istanbul-lib-coverage` successor and is out of scope here.
pub fn remap_coverage_map(
    coverage_map: &std::collections::BTreeMap<String, FileCoverage>,
) -> std::collections::BTreeMap<String, FileCoverage> {
    let mut out = std::collections::BTreeMap::new();
    for (path, fc) in coverage_map {
        match remap_coverage(fc) {
            Some(remapped) => {
                out.insert(remapped.path.clone(), remapped);
            }
            None => {
                out.insert(path.clone(), fc.clone());
            }
        }
    }
    out
}

fn resolve_primary_source(sm: &srcmap_sourcemap::SourceMap) -> Option<String> {
    let first = sm.sources.first()?;
    if first.is_empty() {
        return None;
    }
    let root = sm.source_root.as_deref().unwrap_or("");
    if root.is_empty() {
        return Some(first.clone());
    }

    // `srcmap-sourcemap::from_json` pre-joins `sourceRoot` and each source via
    // literal concatenation (spec-strict). `istanbul-lib-source-maps` inserts
    // a `/` separator when `sourceRoot` lacks one. Strip the literal prefix
    // and re-join with Istanbul's separator rule so coverage paths match what
    // existing reporters expect.
    let bare = first.strip_prefix(root).unwrap_or(first.as_str());
    if root.ends_with('/') || bare.starts_with('/') {
        Some(format!("{root}{bare}"))
    } else {
        Some(format!("{root}/{bare}"))
    }
}

fn remap_position(pos: &mut Position, sm: &srcmap_sourcemap::SourceMap) {
    if pos.line == 0 {
        return;
    }
    let gen_line = pos.line - 1;
    if let Some(orig) = sm.original_position_for(gen_line, pos.column) {
        pos.line = orig.line + 1;
        pos.column = orig.column;
    }
}

fn remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) {
    remap_position(&mut loc.start, sm);
    remap_position(&mut loc.end, sm);
}

fn remap_fn_entry(fn_entry: &mut FnEntry, sm: &srcmap_sourcemap::SourceMap) {
    remap_location(&mut fn_entry.decl, sm);
    remap_location(&mut fn_entry.loc, sm);
    fn_entry.line = fn_entry.loc.start.line;
}

fn remap_branch_entry(branch_entry: &mut BranchEntry, sm: &srcmap_sourcemap::SourceMap) {
    remap_location(&mut branch_entry.loc, sm);
    for loc in &mut branch_entry.locations {
        remap_location(loc, sm);
    }
    branch_entry.line = branch_entry.loc.start.line;
}
