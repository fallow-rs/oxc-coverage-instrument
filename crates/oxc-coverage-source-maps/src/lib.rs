//! Remap coverage data through embedded `inputSourceMap` to original sources.
//!
//! Istanbul's coverage data carries an `inputSourceMap` on each `FileCoverage`
//! when the instrumented input was already a transform output (e.g. TypeScript
//! emitted via `tsc` and then instrumented). Downstream coverage reporters
//! (nyc, `@vitest/coverage-istanbul`, monocart) call into
//! `istanbul-lib-source-maps` to walk that source map and rewrite every
//! coverage position back to the original source. This crate covers both
//! upstream usage modes:
//!
//! - **Mode A (remap-at-report-time)**: [`remap_coverage`] /
//!   [`remap_coverage_map`] walk a single `FileCoverage` (or every entry of a
//!   coverage-final.json shaped map) through the embedded `inputSourceMap`.
//!   When the input map is not embedded on the `FileCoverage`,
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
//!
//! Surviving positions are resolved through an istanbul `getMapping`-equivalent
//! range remap (issue #111), not a direct per-position lookup: starts resolve
//! with greatest-lower-bound and ends resolve to the next original segment (or
//! end of line), matching `istanbul-lib-source-maps`'s
//! `createSourceMapStore().transformCoverage` byte-for-byte.

use std::collections::BTreeMap;

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location, Position};

/// Options for the remap helpers. The default value preserves the legacy
/// keep-generated-position behaviour for positions whose source-map lookup
/// returns `None`, so existing callers that pass [`RemapOptions::default`] (or
/// rely on the parameterless [`remap_coverage`] / [`remap_coverage_map`]
/// helpers) see no change.
#[derive(Debug, Clone, Copy, Default)]
pub struct RemapOptions {
    /// When `true`, statement / function / branch entries whose positions
    /// cannot be looked up in the source map are pruned, along with their
    /// matching `s` / `f` / `b` / `bT` hit-count slots.
    ///
    /// Drop semantics mirror `istanbul-lib-source-maps`'s
    /// `transformer.js`:
    /// - **statement**: dropped when either `start` or `end` fails to remap.
    /// - **function**: dropped when any of `decl.start`, `decl.end`,
    ///   `loc.start`, `loc.end` fails to remap. A matching
    ///   `x_fallow_functionMap` overlay entry, if present, drops with it so the
    ///   overlay stays 1:1 with `fnMap`.
    /// - **branch**: per-arm prune when either arm endpoint fails to remap;
    ///   the whole branch is dropped when no arms survive, or when the
    ///   umbrella `loc` start/end fails to remap.
    ///
    /// Defaults to `false` so existing callers see no change.
    pub drop_unmapped: bool,
}

/// A position-remap predicate over a parsed `inputSourceMap`.
///
/// Lets the instrument crate decide, at AST-transform time, whether a coverage
/// point's positions remap through the input source map, without depending on
/// `srcmap-sourcemap` internals. The predicate exists exactly when eager
/// composition would succeed: [`PositionRemapper::from_json`] returns `None`
/// under the same conditions that make `apply_source_map` bail (unparsable
/// map, or a map that declares no usable source), so a gated transform and the
/// later compose agree by construction.
pub struct PositionRemapper {
    sm: srcmap_sourcemap::SourceMap,
}

impl PositionRemapper {
    /// Parse a source map from JSON. Returns `None` when the JSON fails to
    /// parse or when the map declares no usable source (mirrors the
    /// `resolve_primary_source` bail in `apply_source_map`), so the predicate
    /// exists exactly when eager compose would succeed.
    #[must_use]
    pub fn from_json(input_sm_json: &str) -> Option<Self> {
        let sm = srcmap_sourcemap::SourceMap::from_json(input_sm_json).ok()?;
        resolve_primary_source(&sm)?;
        Some(Self { sm })
    }

    /// Whether the istanbul position (`line` is 1-based, `column` is 0-based
    /// UTF-16) remaps through the source map. Returns `true` for the `line == 0`
    /// "unknown" sentinel, matching `try_remap_position` exactly.
    #[must_use]
    pub fn maps(&self, line: u32, column: u32) -> bool {
        if line == 0 {
            return true;
        }
        self.sm.original_position_for(line - 1, column).is_some()
    }
}

/// Remap a single `FileCoverage` through its embedded `inputSourceMap`.
///
/// Returns `None` when the entry has no `inputSourceMap`, when that map fails
/// to parse, or when it declares no usable source. Callers should fall back to
/// the original coverage entry in those cases.
///
/// When the input map declares a `sourceRoot`, the resolved `path` is the
/// `sourceRoot` joined with the first entry in `sources` (matching
/// `istanbul-lib-source-maps` semantics).
///
/// The returned coverage always satisfies the Istanbul merge invariant
/// `keys(s) ⊆ keys(statementMap)` (and `f`/`fnMap`, `b`/`bT`/`branchMap`):
/// orphan counter slots that would crash `istanbul-lib-coverage`'s merge are
/// dropped via [`FileCoverage::prune_orphan_counters`] (issue #107).
pub fn remap_coverage(coverage: &FileCoverage) -> Option<FileCoverage> {
    remap_coverage_with_loader_and_options(coverage, |_| None, RemapOptions::default())
}

/// Like [`remap_coverage`], but with a [`RemapOptions`] argument. See
/// [`RemapOptions::drop_unmapped`] for the pruning semantics.
pub fn remap_coverage_with_options(
    coverage: &FileCoverage,
    options: RemapOptions,
) -> Option<FileCoverage> {
    remap_coverage_with_loader_and_options(coverage, |_| None, options)
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
    remap_coverage_with_loader_and_options(coverage, loader, RemapOptions::default())
}

/// Like [`remap_coverage_with_loader`], with a [`RemapOptions`] argument. See
/// [`RemapOptions::drop_unmapped`] for the pruning semantics.
pub fn remap_coverage_with_loader_and_options<L>(
    coverage: &FileCoverage,
    loader: L,
    options: RemapOptions,
) -> Option<FileCoverage>
where
    L: Fn(&str) -> Option<String>,
{
    let input_sm_json = match coverage.input_source_map.as_ref() {
        Some(value) => serde_json::to_string(value).ok()?,
        None => loader(&coverage.path)?,
    };
    let sm = srcmap_sourcemap::SourceMap::from_json(&input_sm_json).ok()?;
    apply_source_map(coverage, &sm, options)
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
    remap_coverage_map_with_loader_and_options(coverage_map, |_| None, RemapOptions::default())
}

/// Like [`remap_coverage_map`], with a [`RemapOptions`] argument. See
/// [`RemapOptions::drop_unmapped`] for the pruning semantics.
pub fn remap_coverage_map_with_options(
    coverage_map: &BTreeMap<String, FileCoverage>,
    options: RemapOptions,
) -> BTreeMap<String, FileCoverage> {
    remap_coverage_map_with_loader_and_options(coverage_map, |_| None, options)
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
    remap_coverage_map_with_loader_and_options(coverage_map, loader, RemapOptions::default())
}

/// Like [`remap_coverage_map_with_loader`], with a [`RemapOptions`] argument.
/// See [`RemapOptions::drop_unmapped`] for the pruning semantics.
pub fn remap_coverage_map_with_loader_and_options<L>(
    coverage_map: &BTreeMap<String, FileCoverage>,
    loader: L,
    options: RemapOptions,
) -> BTreeMap<String, FileCoverage>
where
    L: Fn(&str) -> Option<String>,
{
    let mut out = BTreeMap::new();
    for (path, fc) in coverage_map {
        match remap_coverage_with_loader_and_options(fc, &loader, options) {
            Some(remapped) => {
                out.insert(remapped.path.clone(), remapped);
            }
            None => {
                // Entry has no usable map (no embedded `inputSourceMap`, or an
                // unparsable one): it passes through under its original key. Still
                // enforce the Istanbul merge invariant (issue #107) so an
                // already-composed entry carrying a runtime orphan counter does not
                // slip through unchanged and crash a downstream `nyc` merge.
                let mut passthrough = fc.clone();
                passthrough.prune_orphan_counters();
                out.insert(path.clone(), passthrough);
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
    options: RemapOptions,
) -> Option<FileCoverage> {
    let primary_source = resolve_primary_source(sm)?;

    let mut out = coverage.clone();
    out.path = primary_source;
    out.input_source_map = None;

    if options.drop_unmapped {
        prune_unmapped(&mut out, sm);
    } else {
        for loc in out.statement_map.values_mut() {
            remap_location(loc, sm);
        }
        for fn_entry in out.fn_map.values_mut() {
            remap_fn_entry(fn_entry, sm);
        }
        for branch_entry in out.branch_map.values_mut() {
            remap_branch_entry(branch_entry, sm);
        }
    }

    // Enforce the Istanbul merge invariant on every coverage object we emit
    // (issue #107): drop any `s`/`f`/`b`/`bT` counter whose location-map entry
    // is absent. `prune_unmapped` already removes the counters it drops in
    // lockstep, so this is a no-op for the drop path; the no-drop path never
    // removes location entries, so a counter is only orphaned here if the INPUT
    // coverage already carried one (a runtime-collected `++cov.s[id]` against a
    // slot a prior tool pruned, which deserializes back as a `null`-valued
    // orphan). Passing such an orphan through unchanged would crash any
    // `istanbul-lib-coverage` / `nyc` consumer that merges the result.
    out.prune_orphan_counters();

    Some(out)
}

/// Drop statement / function / branch entries whose positions cannot be looked
/// up in the source map, taking matching `s` / `f` / `b` / `bT` slots with
/// them. Mirrors `istanbul-lib-source-maps`'s `transformer.js`. See
/// [`RemapOptions::drop_unmapped`] for the exact per-kind rules.
fn prune_unmapped(coverage: &mut FileCoverage, sm: &srcmap_sourcemap::SourceMap) {
    prune_statements(coverage, sm);
    prune_functions(coverage, sm);
    prune_branches(coverage, sm);
}

/// Statements: drop when either start or end fails to remap, taking the
/// matching `s` hit slot with the dropped entry.
fn prune_statements(coverage: &mut FileCoverage, sm: &srcmap_sourcemap::SourceMap) {
    let mut dropped_statements: Vec<String> = Vec::new();
    coverage.statement_map.retain(|key, loc| {
        if try_remap_location(loc, sm) {
            true
        } else {
            dropped_statements.push(key.clone());
            false
        }
    });
    for key in &dropped_statements {
        coverage.s.remove(key);
    }
}

/// Functions: drop when any of decl.start, decl.end, loc.start, loc.end fails
/// to remap. `FnEntry::line` is refreshed from the remapped `loc.start.line`
/// on the surviving entries; the matching `f` hit slot follows the drop.
fn prune_functions(coverage: &mut FileCoverage, sm: &srcmap_sourcemap::SourceMap) {
    let mut dropped_fns: Vec<String> = Vec::new();
    coverage.fn_map.retain(|key, fn_entry| {
        let decl_ok = try_remap_location(&mut fn_entry.decl, sm);
        let loc_ok = try_remap_location(&mut fn_entry.loc, sm);
        if decl_ok && loc_ok {
            fn_entry.line = fn_entry.loc.start.line;
            true
        } else {
            dropped_fns.push(key.clone());
            false
        }
    });
    for key in &dropped_fns {
        coverage.f.remove(key);
        // Keep the `x_fallow_functionMap` overlay 1:1 with `fnMap`: a dropped
        // function must not leave an orphan identity entry that consumers
        // joining on `fnMap` keys would never resolve. The overlay shares the
        // fn-id keyspace with `fnMap` (see `build_function_identity_map`).
        if let Some(fallow_map) = coverage.x_fallow_function_map.as_mut() {
            fallow_map.remove(key);
        }
    }
}

/// Branches: per-arm prune when either arm endpoint fails to remap; drop the
/// whole branch when no arms survive OR when the umbrella `loc` start/end fails
/// to remap. The `b` / `bT` arm vectors track surviving arms by position so
/// their hit counts stay aligned.
fn prune_branches(coverage: &mut FileCoverage, sm: &srcmap_sourcemap::SourceMap) {
    let mut dropped_branches: Vec<String> = Vec::new();
    let mut surviving_arms: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    coverage.branch_map.retain(|key, branch_entry| {
        let loc_ok = try_remap_location(&mut branch_entry.loc, sm);
        if !loc_ok {
            dropped_branches.push(key.clone());
            return false;
        }

        let mut kept_indices: Vec<usize> = Vec::new();
        let mut kept_locations: Vec<Location> = Vec::new();
        for (idx, arm) in branch_entry.locations.iter_mut().enumerate() {
            if try_remap_location(arm, sm) {
                kept_indices.push(idx);
                kept_locations.push(arm.clone());
            }
        }
        if kept_locations.is_empty() {
            dropped_branches.push(key.clone());
            return false;
        }

        branch_entry.locations = kept_locations;
        branch_entry.line = branch_entry.loc.start.line;
        surviving_arms.insert(key.clone(), kept_indices);
        true
    });
    for key in &dropped_branches {
        coverage.b.remove(key);
        if let Some(b_t) = coverage.b_t.as_mut() {
            b_t.remove(key);
        }
    }
    realign_arm_vec_map(&mut coverage.b, &surviving_arms);
    if let Some(b_t) = coverage.b_t.as_mut() {
        realign_arm_vec_map(b_t, &surviving_arms);
    }
}

/// Project each surviving-arm hit-count vector down to the kept indices.
/// Branches that disappeared from the map (already removed via `b.remove`)
/// are not in `surviving_arms` and are left untouched here.
fn realign_arm_vec_map(
    arm_counts: &mut BTreeMap<String, Vec<u32>>,
    surviving_arms: &BTreeMap<String, Vec<usize>>,
) {
    for (key, indices) in surviving_arms {
        let Some(existing) = arm_counts.get_mut(key) else {
            continue;
        };
        let trimmed = indices.iter().map(|&i| existing.get(i).copied().unwrap_or(0)).collect();
        *existing = trimmed;
    }
}

/// Stateful map store for the Mode B "continuous remap during collection"
/// flow. Some runners (Jest with `transform`) instrument files incrementally
/// and want to record each file's source map as it is produced, then rewrite
/// `FileCoverage` objects on the fly rather than once at report time.
///
/// Example usage:
///
/// ```
/// use oxc_coverage_source_maps::SourceMapStore;
/// use oxc_coverage_types::FileCoverage;
///
/// let mut store = SourceMapStore::new();
/// let input_sm = r#"{"version":3,"sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
/// store.add_map("intermediate.js", serde_json::from_str(input_sm).unwrap());
///
/// // Minimal FileCoverage to demonstrate the remap; the instrumenter produces
/// // this shape in real usage. Constructed inline here so the doctest does
/// // not reverse-depend on the instrumenter.
/// let fc = FileCoverage::from_json(
///     r#"{"path":"intermediate.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}"#,
/// ).unwrap();
///
/// let remapped = store
///     .transform_coverage(&fc)
///     .expect("store has a map for the file");
/// assert_eq!(remapped.path, "src/app.ts");
/// ```
#[derive(Debug, Default, Clone)]
pub struct SourceMapStore {
    maps: BTreeMap<String, Option<srcmap_sourcemap::SourceMap>>,
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
        let parsed = serde_json::to_string(&source_map)
            .ok()
            .and_then(|json| srcmap_sourcemap::SourceMap::from_json(&json).ok());
        self.maps.insert(file.into(), parsed);
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
        self.transform_coverage_with_options(coverage, RemapOptions::default())
    }

    /// Like [`SourceMapStore::transform_coverage`], but with a
    /// [`RemapOptions`] argument. See [`RemapOptions::drop_unmapped`] for the
    /// pruning semantics.
    ///
    /// A `Some` result is reconciled to the Istanbul merge invariant (orphan
    /// counters dropped, issue #107). A `None` result means "no usable map":
    /// the caller keeps the original `FileCoverage` and owns reconciliation in
    /// that case. The map-level [`SourceMapStore::transform_coverage_map`]
    /// reconciles its `None` passthrough entries for you.
    #[must_use]
    pub fn transform_coverage_with_options(
        &self,
        coverage: &FileCoverage,
        options: RemapOptions,
    ) -> Option<FileCoverage> {
        if let Some(sm) = self.maps.get(&coverage.path) {
            let sm = sm.as_ref()?;
            return apply_source_map(coverage, sm, options);
        }
        remap_coverage_with_options(coverage, options)
    }

    /// Remap every entry of a coverage map using the store. Entries whose
    /// `path` is not in the store and which carry no embedded `inputSourceMap`
    /// pass through unchanged under their original key.
    #[must_use]
    pub fn transform_coverage_map(
        &self,
        coverage_map: &BTreeMap<String, FileCoverage>,
    ) -> BTreeMap<String, FileCoverage> {
        self.transform_coverage_map_with_options(coverage_map, RemapOptions::default())
    }

    /// Like [`SourceMapStore::transform_coverage_map`], but with a
    /// [`RemapOptions`] argument. See [`RemapOptions::drop_unmapped`] for the
    /// pruning semantics.
    #[must_use]
    pub fn transform_coverage_map_with_options(
        &self,
        coverage_map: &BTreeMap<String, FileCoverage>,
        options: RemapOptions,
    ) -> BTreeMap<String, FileCoverage> {
        let mut out = BTreeMap::new();
        for (path, fc) in coverage_map {
            match self.transform_coverage_with_options(fc, options) {
                Some(remapped) => {
                    out.insert(remapped.path.clone(), remapped);
                }
                None => {
                    // Same invariant pass as the standalone map helper: an
                    // entry with no store/embedded map still gets its orphan
                    // counters dropped (issue #107) before passing through.
                    let mut passthrough = fc.clone();
                    passthrough.prune_orphan_counters();
                    out.insert(path.clone(), passthrough);
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

// --- istanbul `getMapping` range remap (issue #111) ---------------------------
//
// Surviving positions are resolved exactly as `istanbul-lib-source-maps`'s
// `lib/get-mapping.js` does, not by the direct `original_position_for` lookup the
// drop semantics (issue #105) layered on top of. Source maps carry segments only
// at token *starts*, so a direct lookup of an exclusive end snaps backward to the
// previous segment (truncating the span) and a span smaller than its enclosing
// segment never balloons. `getMapping` fixes both: the start resolves with
// greatest-lower-bound (so a sub-segment span snaps to its enclosing mapped span)
// and the end resolves to the *next* original segment after the end (or the end
// of the original line). This finishes the documented
// `createSourceMapStore().transformCoverage` equivalence the drop semantics
// already claim.
//
// Coordinate boundary: Istanbul `Position` is 1-based line + 0-based UTF-16
// column; `srcmap-sourcemap` is 0-based for both. Conversion happens here.

/// The original end resolved by [`original_end_position_for`]. Mirrors
/// `originalEndPositionFor`: either the start of the *next* original segment on
/// the same original line, or the end of that line (istanbul's `column:
/// Infinity`, which we cannot store in a `u32` and clamp at use).
enum EndResult {
    /// The next original segment after the end position; its column is the
    /// exclusive end of our span.
    Mapped { source: u32, line: u32, column: u32 },
    /// No segment follows on the same original line: the span extends to the
    /// end of the line (istanbul's `Infinity`). Clamped to the line's UTF-16
    /// length at use via [`original_line_end_column`].
    EndOfLine { source: u32, line: u32 },
}

/// `originalPositionTryBoth`: greatest-lower-bound first, falling back to
/// least-upper-bound. `line` and `column` are 0-based (srcmap space).
fn original_position_try_both(
    sm: &srcmap_sourcemap::SourceMap,
    line: u32,
    column: u32,
) -> Option<srcmap_sourcemap::OriginalLocation> {
    sm.original_position_for_with_bias(line, column, srcmap_sourcemap::Bias::GreatestLowerBound)
        .or_else(|| {
            sm.original_position_for_with_bias(
                line,
                column,
                srcmap_sourcemap::Bias::LeastUpperBound,
            )
        })
}

/// `allGeneratedPositionsFor({ ..., bias: LEAST_UPPER_BOUND })`: all generated
/// positions that map to the original `(source, line)` at the least-upper-bound
/// of `column` (the exact column when a segment exists there, otherwise the
/// next greater original column on that line).
///
/// `srcmap-sourcemap`'s `all_generated_positions_for` is exact-match on
/// `(source, line, column)`, NOT least-upper-bound like
/// `@jridgewell/trace-mapping`, so the matched column is computed on the
/// original side first (mirroring trace-mapping's `sliceGeneratedPositions`).
/// That scan is linear in the map's mapping count; resolving the matched column
/// this way (rather than via a generated-position round-trip) is what keeps the
/// result faithful when several mappings share a generated column. A
/// per-`(source, line)` original-column index would make it logarithmic if a
/// future profile shows it matters.
fn all_generated_positions_for_lub(
    sm: &srcmap_sourcemap::SourceMap,
    source: &str,
    line: u32,
    column: u32,
) -> Vec<srcmap_sourcemap::GeneratedLocation> {
    // Key the scan by name -> index (not the caller's raw mapping index): istanbul
    // and trace-mapping look up the source by name too, so for a map where the
    // same name appears at multiple indices this matches the library's first-match
    // behaviour. Well-formed maps have unique source names, so the two coincide.
    let Some(source_idx) = sm.source_index(source) else {
        return Vec::new();
    };
    // matchedColumn: the exact column when a segment exists there, else the
    // smallest original column strictly greater on this original line.
    let matched_column = sm
        .all_mappings()
        .iter()
        .filter(|m| {
            m.source == source_idx && m.original_line == line && m.original_column >= column
        })
        .map(|m| m.original_column)
        .min();
    let Some(matched_column) = matched_column else {
        return Vec::new();
    };
    sm.all_generated_positions_for(source, line, matched_column)
}

/// `originalEndPositionFor`: resolve the exclusive end of a generated range to
/// the start of the next original segment, or the end of the original line.
/// `gen_end_line` / `gen_end_col` are 0-based (srcmap space).
fn original_end_position_for(
    sm: &srcmap_sourcemap::SourceMap,
    gen_end_line: u32,
    gen_end_col: u32,
) -> Option<EndResult> {
    // beforeEnd = originalPositionTryBoth(line, column - 1). A column-0 exclusive
    // end would make istanbul evaluate `column - 1 === -1`, which
    // `@jridgewell/trace-mapping` rejects (it throws); we conservatively report
    // "no mapping" so the caller drops the entry (drop mode) or keeps the
    // generated position (no-drop mode) instead of panicking.
    let before_col = gen_end_col.checked_sub(1)?;
    let before = original_position_try_both(sm, gen_end_line, before_col)?;
    let source = sm.source(before.source);

    // afterEndMappings = allGeneratedPositionsFor(LUB) one column to the right;
    // map each back (GLB) and take the first that lands on the same original
    // line: that segment's start is our exclusive end.
    let after = all_generated_positions_for_lub(sm, source, before.line, before.column + 1);
    for gen_pos in &after {
        if let Some(orig) = sm.original_position_for_with_bias(
            gen_pos.line,
            gen_pos.column,
            srcmap_sourcemap::Bias::GreatestLowerBound,
        ) && orig.line == before.line
        {
            return Some(EndResult::Mapped {
                source: orig.source,
                line: orig.line,
                column: orig.column,
            });
        }
    }
    // Nothing follows on this original line: extend to end of line.
    Some(EndResult::EndOfLine { source: before.source, line: before.line })
}

/// The exclusive end column of original line `line` (0-based) in `source`.
/// Resolves istanbul's `column: Infinity` to a concrete `u32`: the UTF-16
/// length of the line from `sourcesContent`, or (when `sourcesContent` is
/// absent) the rightmost original column mapped on that line.
fn original_line_end_column(sm: &srcmap_sourcemap::SourceMap, source_idx: u32, line: u32) -> u32 {
    if let Some(Some(content)) = sm.sources_content.get(source_idx as usize)
        && let Some(text) = content.split('\n').nth(line as usize)
    {
        // Strip a trailing CR so a CRLF source measures the same UTF-16 length
        // istanbul's `\n`-split view of the line would.
        let text = text.strip_suffix('\r').unwrap_or(text);
        return u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX);
    }
    sm.all_mappings()
        .iter()
        .filter(|m| m.source == source_idx && m.original_line == line)
        .map(|m| m.original_column)
        .max()
        .unwrap_or(0)
}

/// Resolve a `Location` through istanbul `getMapping` semantics. Returns the
/// remapped original-space location, or `None` when `getMapping` would yield
/// `null` (start or end fails to map, or they map to different sources). The
/// returned positions are in Istanbul space (1-based line, 0-based column).
///
/// The `line: 0` "unknown" sentinel is handled by the callers, not here:
/// `getMapping` has no notion of it and `line - 1` would underflow.
fn get_mapping_location(loc: &Location, sm: &srcmap_sourcemap::SourceMap) -> Option<Location> {
    let start = original_position_try_both(sm, loc.start.line - 1, loc.start.column)?;
    let end = original_end_position_for(sm, loc.end.line - 1, loc.end.column)?;

    let (end_source, mut end_line, mut end_col, end_is_eol) = match end {
        EndResult::Mapped { source, line, column } => (source, line, column, false),
        EndResult::EndOfLine { source, line } => {
            (source, line, original_line_end_column(sm, source, line), true)
        }
    };

    // getMapping: both endpoints must carry a source and they must agree.
    if start.source != end_source {
        return None;
    }

    // Degenerate-span guard (get-mapping.js): a zero-area span at the same
    // line+column corrupts `keyFromLoc` merge dedup. Recompute the end via LUB
    // of the generated end, then step one column left. Skipped for the
    // end-of-line case: istanbul's `Infinity` can never equal `start.column`,
    // so the guard never fires there.
    if !end_is_eol && start.line == end_line && start.column == end_col {
        let lub = original_position_for_lub(sm, loc.end.line - 1, loc.end.column)?;
        end_line = lub.line;
        // get-mapping.js does `end.column -= 1` unconditionally; when LUB lands on
        // column 0 it yields a JS `-1`, which can never round-trip through our
        // `u32` (a negative column is an invalid Istanbul position anyway). We
        // saturate to 0 instead. This sub-case requires the degenerate guard to
        // fire AND the recomputed LUB to sit at column 0 on the same generated
        // line; istanbul itself marks the branch "edge case too hard to test for".
        end_col = lub.column.saturating_sub(1);
    }

    Some(Location {
        start: Position { line: start.line + 1, column: start.column },
        end: Position { line: end_line + 1, column: end_col },
    })
}

/// Least-upper-bound lookup used by the degenerate-span branch.
fn original_position_for_lub(
    sm: &srcmap_sourcemap::SourceMap,
    line: u32,
    column: u32,
) -> Option<srcmap_sourcemap::OriginalLocation> {
    sm.original_position_for_with_bias(line, column, srcmap_sourcemap::Bias::LeastUpperBound)
}

// --- legacy direct-lookup helpers (line-0 sentinel + no-drop fallback) --------

/// Direct per-position remap. Retained only as the fallback for the `line: 0`
/// "unknown" sentinel and for no-drop entries that `getMapping` cannot resolve
/// (where the historical behaviour is to keep the generated position).
fn legacy_remap_position(pos: &mut Position, sm: &srcmap_sourcemap::SourceMap) {
    if pos.line == 0 {
        return;
    }
    let gen_line = pos.line - 1;
    if let Some(orig) = sm.original_position_for(gen_line, pos.column) {
        pos.line = orig.line + 1;
        pos.column = orig.column;
    }
}

/// Strict variant of [`legacy_remap_position`]: `true` when the position
/// remapped (or was the `line: 0` sentinel), `false` when the lookup returned
/// `None`.
fn legacy_try_remap_position(pos: &mut Position, sm: &srcmap_sourcemap::SourceMap) -> bool {
    if pos.line == 0 {
        return true;
    }
    let gen_line = pos.line - 1;
    let Some(orig) = sm.original_position_for(gen_line, pos.column) else {
        return false;
    };
    pos.line = orig.line + 1;
    pos.column = orig.column;
    true
}

fn legacy_remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) {
    legacy_remap_position(&mut loc.start, sm);
    legacy_remap_position(&mut loc.end, sm);
}

fn legacy_try_remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) -> bool {
    let start = legacy_try_remap_position(&mut loc.start, sm);
    let end = legacy_try_remap_position(&mut loc.end, sm);
    start && end
}

// --- remap entry points (route through getMapping) ----------------------------

/// Remap a location through `getMapping` (no-drop mode). On the `line: 0`
/// sentinel or when `getMapping` cannot resolve the span, fall back to the
/// legacy direct lookup so existing callers see no change on entries istanbul
/// would have dropped.
fn remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) {
    if loc.start.line == 0 || loc.end.line == 0 {
        legacy_remap_location(loc, sm);
        return;
    }
    match get_mapping_location(loc, sm) {
        Some(remapped) => *loc = remapped,
        None => legacy_remap_location(loc, sm),
    }
}

/// Strict variant of [`remap_location`] for `drop_unmapped` mode: `true` (and
/// rewrites in place) when `getMapping` resolves the span, `false` (drop) when
/// it does not. The `line: 0` sentinel routes to the legacy helper, which keeps
/// it as a successful no-op.
fn try_remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) -> bool {
    if loc.start.line == 0 || loc.end.line == 0 {
        return legacy_try_remap_location(loc, sm);
    }
    match get_mapping_location(loc, sm) {
        Some(remapped) => {
            *loc = remapped;
            true
        }
        None => false,
    }
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
