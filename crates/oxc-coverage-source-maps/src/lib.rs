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
/// under the same conditions that make `apply_source_map` bail (unparseable
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
        self.transform_coverage_with_options(coverage, RemapOptions::default())
    }

    /// Like [`SourceMapStore::transform_coverage`], but with a
    /// [`RemapOptions`] argument. See [`RemapOptions::drop_unmapped`] for the
    /// pruning semantics.
    #[must_use]
    pub fn transform_coverage_with_options(
        &self,
        coverage: &FileCoverage,
        options: RemapOptions,
    ) -> Option<FileCoverage> {
        if let Some(value) = self.maps.get(&coverage.path) {
            let json = serde_json::to_string(value).ok()?;
            let sm = srcmap_sourcemap::SourceMap::from_json(&json).ok()?;
            return apply_source_map(coverage, &sm, options);
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

/// Strict variant of [`remap_position`]: returns `true` when the position was
/// rewritten through the source map (or was the `line: 0` "unknown" sentinel,
/// which istanbul treats as a successful no-op), and `false` when the lookup
/// returned `None`. Used by [`prune_unmapped`] to decide which entries to
/// drop in `drop_unmapped` mode.
fn try_remap_position(pos: &mut Position, sm: &srcmap_sourcemap::SourceMap) -> bool {
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

fn remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) {
    remap_position(&mut loc.start, sm);
    remap_position(&mut loc.end, sm);
}

/// Strict variant of [`remap_location`]: returns `true` only when both
/// endpoints remap successfully (or were the `line: 0` "unknown" sentinel).
/// The location is rewritten in place regardless so partial-success diagnostic
/// inspection stays available to the caller; pruning paths discard the entry
/// when this returns `false`.
fn try_remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) -> bool {
    let start = try_remap_position(&mut loc.start, sm);
    let end = try_remap_position(&mut loc.end, sm);
    start && end
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
