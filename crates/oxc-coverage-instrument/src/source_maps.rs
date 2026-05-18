//! Remap coverage data through embedded `inputSourceMap` to original sources.
//!
//! Istanbul's coverage data carries an `inputSourceMap` on each `FileCoverage`
//! when the instrumented input was already a transform output (e.g. TypeScript
//! emitted via `tsc` and then instrumented). Downstream coverage reporters
//! (nyc, `@vitest/coverage-istanbul`, monocart) call into
//! `istanbul-lib-source-maps` to walk that source map and rewrite every
//! coverage position back to the original source. This module covers both
//! upstream usage modes:
//!
//! - **Mode A (remap-at-report-time)**: [`remap_coverage`] /
//!   [`remap_coverage_map`] walk a single FileCoverage (or every entry of a
//!   coverage-final.json shaped map) through the embedded `inputSourceMap`.
//!   When the input map is not embedded on the FileCoverage,
//!   [`remap_coverage_with_loader`] / [`remap_coverage_map_with_loader`]
//!   accept a caller-supplied loader that reads the map JSON from disk or
//!   another source, matching `istanbul-lib-source-maps`'s `sourceStore`
//!   behavior used by nyc.
//! - **Mode B (continuous remap during collection)**: [`SourceMapStore`]
//!   exposes a stateful container that accumulates per-file maps via
//!   `add_map` and rewrites incoming `FileCoverage` objects via
//!   `transform_coverage`. Some runners (Jest with `transform`) consult the
//!   store as files are instrumented rather than once at report time.
//!
//! Position semantics: Istanbul's `Position` is 1-based line + 0-based UTF-16
//! column. `srcmap-sourcemap`'s `original_position_for` is 0-based for both.
//! Conversion happens at the lookup boundary.

use std::collections::BTreeMap;

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location, Position};

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
    remap_coverage_with_loader(coverage, |_| None)
}

/// Like [`remap_coverage`], but when the entry has no embedded
/// `inputSourceMap` the supplied `loader` is consulted with the
/// `FileCoverage.path` to fetch the source map JSON from disk or another
/// source. Matches `istanbul-lib-source-maps`'s `sourceStore` callback used
/// by nyc when the map sits next to the instrumented file rather than
/// embedded inside the coverage object.
///
/// Returning `None` from the loader (or supplying `|_| None`) makes this
/// behave identically to [`remap_coverage`].
pub fn remap_coverage_with_loader<L>(coverage: &FileCoverage, loader: L) -> Option<FileCoverage>
where
    L: Fn(&str) -> Option<String>,
{
    let input_sm_json = match coverage.input_source_map.as_ref() {
        Some(value) => serde_json::to_string(value).ok()?,
        None => loader(&coverage.path)?,
    };
    let sm = srcmap_sourcemap::SourceMap::from_json(&input_sm_json).ok()?;
    apply_source_map(coverage, &sm)
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
    coverage_map: &BTreeMap<String, FileCoverage>,
) -> BTreeMap<String, FileCoverage> {
    remap_coverage_map_with_loader(coverage_map, |_| None)
}

/// Like [`remap_coverage_map`], but with a disk-read fallback for entries
/// that lack an embedded `inputSourceMap`. The loader is called with the
/// `FileCoverage.path` of each entry that needs an external map.
pub fn remap_coverage_map_with_loader<L>(
    coverage_map: &BTreeMap<String, FileCoverage>,
    loader: L,
) -> BTreeMap<String, FileCoverage>
where
    L: Fn(&str) -> Option<String>,
{
    let mut out = BTreeMap::new();
    for (path, fc) in coverage_map {
        match remap_coverage_with_loader(fc, &loader) {
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

/// Apply a parsed `SourceMap` to a `FileCoverage`. Returns `None` when the
/// map declares no usable source (matches the bail in `resolve_primary_source`).
fn apply_source_map(
    coverage: &FileCoverage,
    sm: &srcmap_sourcemap::SourceMap,
) -> Option<FileCoverage> {
    let primary_source = resolve_primary_source(sm)?;

    let mut out = coverage.clone();
    out.path = primary_source;
    out.input_source_map = None;

    for loc in out.statement_map.values_mut() {
        remap_location(loc, sm);
    }
    for fn_entry in out.fn_map.values_mut() {
        remap_fn_entry(fn_entry, sm);
    }
    for branch_entry in out.branch_map.values_mut() {
        remap_branch_entry(branch_entry, sm);
    }

    Some(out)
}

/// Stateful map store for the Mode B "continuous remap during collection"
/// flow. Some runners (Jest with `transform`) instrument files incrementally
/// and want to record each file's source map as it is produced, then rewrite
/// `FileCoverage` objects on the fly rather than once at report time.
///
/// Example usage:
///
/// ```
/// use oxc_coverage_instrument::{SourceMapStore, instrument, InstrumentOptions};
///
/// let mut store = SourceMapStore::new();
/// let input_sm = r#"{"version":3,"sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
/// let opts = InstrumentOptions {
///     input_source_map: Some(input_sm.to_string()),
///     ..InstrumentOptions::default()
/// };
/// let result = instrument("const x = 1;", "intermediate.js", &opts).unwrap();
/// store.add_map("intermediate.js", serde_json::from_str(input_sm).unwrap());
///
/// // Later, when the runner finalizes its coverage map:
/// let remapped = store
///     .transform_coverage(&result.coverage_map)
///     .expect("store has a map for the file");
/// assert_eq!(remapped.path, "src/app.ts");
/// ```
#[derive(Debug, Default, Clone)]
pub struct SourceMapStore {
    maps: BTreeMap<String, serde_json::Value>,
}

impl SourceMapStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source map keyed by the file path. A later `add_map` with the
    /// same key replaces the earlier entry, matching
    /// `istanbul-lib-source-maps`'s `registerMap` semantics.
    pub fn add_map(&mut self, file: impl Into<String>, source_map: serde_json::Value) {
        self.maps.insert(file.into(), source_map);
    }

    /// Whether the store has a map registered for `file`.
    #[must_use]
    pub fn contains(&self, file: &str) -> bool {
        self.maps.contains_key(file)
    }

    /// Number of registered maps. Useful for tests and instrumentation.
    #[must_use]
    pub fn len(&self) -> usize {
        self.maps.len()
    }

    /// True when no maps have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
    }

    /// Remap a single `FileCoverage` using the stored map for its `path`. The
    /// store takes precedence over any embedded `inputSourceMap`: when the
    /// store has an entry for `coverage.path`, that map is used; otherwise
    /// the embedded map (if any) is used; otherwise `None`.
    ///
    /// Returns `None` when neither the store nor the embedded map yields a
    /// usable map, matching [`remap_coverage`]'s fallback semantics.
    #[must_use]
    pub fn transform_coverage(&self, coverage: &FileCoverage) -> Option<FileCoverage> {
        if let Some(value) = self.maps.get(&coverage.path) {
            let json = serde_json::to_string(value).ok()?;
            let sm = srcmap_sourcemap::SourceMap::from_json(&json).ok()?;
            return apply_source_map(coverage, &sm);
        }
        remap_coverage(coverage)
    }

    /// Remap every entry of a coverage map using the store. Entries whose
    /// `path` is not in the store and which carry no embedded `inputSourceMap`
    /// pass through unchanged under their original key.
    #[must_use]
    pub fn transform_coverage_map(
        &self,
        coverage_map: &BTreeMap<String, FileCoverage>,
    ) -> BTreeMap<String, FileCoverage> {
        let mut out = BTreeMap::new();
        for (path, fc) in coverage_map {
            match self.transform_coverage(fc) {
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
