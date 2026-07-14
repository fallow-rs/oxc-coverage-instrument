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
//!   [`remap_coverage_to_map`] exposes the one-to-many form for callers that
//!   start with one generated file containing several original sources.
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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location, Position};

/// Options for the remap helpers. The default value preserves the legacy
/// keep-generated-position behaviour for positions whose source-map lookup
/// returns `None`, so existing callers that pass [`RemapOptions::default`] (or
/// rely on the parameterless [`remap_coverage`] helper) see no change for
/// single-source maps. In a multi-source map, an unmapped entry has no safe
/// original owner and is omitted by the map-returning helpers in both modes.
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
    ///   the whole branch is dropped when no arms survive or retained arms
    ///   resolve to different sources. An unmapped umbrella `loc` falls back
    ///   to the first retained arm.
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
    /// Lookup caches shared across every `location_maps` call for this map. The
    /// eager gate queries one node at a time on a hot traversal path; without a
    /// persistent context the per-`(source, line)` column index (issue #122
    /// getMapping) would be rebuilt and discarded on every node. `RefCell`
    /// because `location_maps` takes `&self` (the transform visits with `&self`).
    caches: RefCell<RemapCaches>,
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
        Some(Self { sm, caches: RefCell::new(RemapCaches::default()) })
    }

    /// Whether an istanbul `Location` survives `getMapping` resolution, i.e.
    /// whether the deferred `remap_coverage_with_options(.., { drop_unmapped:
    /// true })` path would KEEP this entry. The eager AST-level drop gate (issue
    /// #106) consults this so a gated transform and the later no-drop compose
    /// agree with the deferred prune (issue #105 / #111) by construction: both
    /// ask `get_mapping_location`, so a span that resolves for one resolves for
    /// the other. Issue #122: the previous per-position predicate resolved each
    /// endpoint with a single greatest-lower-bound lookup, which dropped a point
    /// whose generated column sat just before that line's first mapping even
    /// though `getMapping`'s least-upper-bound fallback keeps it.
    ///
    /// Mirrors `try_remap_location` exactly, including the `line == 0`
    /// "unknown" sentinel (routed to the legacy direct lookup, which keeps a
    /// sentinel endpoint as a no-op), but without mutating the location: the gate
    /// only needs the keep/drop decision; the position rewrite happens later in
    /// the no-drop `remap_coverage` pass.
    #[must_use]
    pub fn location_maps(&self, loc: &Location) -> bool {
        if loc.start.line == 0 || loc.end.line == 0 {
            return self.legacy_position_maps(&loc.start) && self.legacy_position_maps(&loc.end);
        }
        let mut caches = self.caches.borrow_mut();
        let mut ctx = RemapContext::new(&self.sm, &mut caches);
        get_mapping_location_cached(&mut ctx, loc).is_some()
    }

    /// Non-mutating mirror of the former `legacy_try_remap_position`: `true` when the
    /// istanbul position (1-based `line`, 0-based UTF-16 `column`) has a direct
    /// greatest-lower-bound mapping, or when it is the `line == 0` sentinel.
    fn legacy_position_maps(&self, pos: &Position) -> bool {
        pos.line == 0 || self.sm.original_position_for(pos.line - 1, pos.column).is_some()
    }
}

/// Remap a single `FileCoverage` through its embedded `inputSourceMap`.
///
/// Returns `None` when the entry has no `inputSourceMap`, when that map fails
/// to parse, when it declares no usable source, or when mappings resolve to
/// several original files. Callers that need a complete multi-source result
/// must use [`remap_coverage_to_map`].
///
/// When the input map declares a `sourceRoot`, the sole resolved output path
/// joins that root using `istanbul-lib-source-maps` semantics.
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
    let sm = source_map_with_loader(coverage, loader)?;
    if let Some(path) = sole_resolved_source_path(&sm) {
        return Some(apply_source_map_single(coverage, &sm, options, path));
    }
    let remapped = apply_source_map_to_map_internal(coverage, &sm, options, false)?;
    select_single_remap(remapped)
}

/// Remap one `FileCoverage` into every original source represented by its
/// mappings. Returns `None` only when no usable source map is available.
///
/// Unlike [`remap_coverage`], this API can represent a true multi-source
/// result. Each returned file has contiguous metadata ids, aligned counters,
/// and no `inputSourceMap`.
pub fn remap_coverage_to_map(coverage: &FileCoverage) -> Option<BTreeMap<String, FileCoverage>> {
    remap_coverage_to_map_with_loader_and_options(coverage, |_| None, RemapOptions::default())
}

/// Like [`remap_coverage_to_map`], with a [`RemapOptions`] argument.
pub fn remap_coverage_to_map_with_options(
    coverage: &FileCoverage,
    options: RemapOptions,
) -> Option<BTreeMap<String, FileCoverage>> {
    remap_coverage_to_map_with_loader_and_options(coverage, |_| None, options)
}

/// Like [`remap_coverage_to_map`], with a source-map loader fallback.
pub fn remap_coverage_to_map_with_loader<L>(
    coverage: &FileCoverage,
    loader: L,
) -> Option<BTreeMap<String, FileCoverage>>
where
    L: Fn(&str) -> Option<String>,
{
    remap_coverage_to_map_with_loader_and_options(coverage, loader, RemapOptions::default())
}

/// Like [`remap_coverage_to_map_with_loader`], with remap options.
pub fn remap_coverage_to_map_with_loader_and_options<L>(
    coverage: &FileCoverage,
    loader: L,
    options: RemapOptions,
) -> Option<BTreeMap<String, FileCoverage>>
where
    L: Fn(&str) -> Option<String>,
{
    let sm = source_map_with_loader(coverage, loader)?;
    apply_source_map_to_map(coverage, &sm, options)
}

fn source_map_with_loader<L>(
    coverage: &FileCoverage,
    loader: L,
) -> Option<srcmap_sourcemap::SourceMap>
where
    L: Fn(&str) -> Option<String>,
{
    let input_sm_json = match coverage.input_source_map.as_ref() {
        Some(value) => serde_json::to_string(value).ok()?,
        None => loader(&coverage.path)?,
    };
    srcmap_sourcemap::SourceMap::from_json(&input_sm_json).ok()
}

fn select_single_remap(mut remapped: BTreeMap<String, FileCoverage>) -> Option<FileCoverage> {
    if remapped.len() != 1 {
        return None;
    }
    remapped.pop_first().map(|(_, coverage)| coverage)
}

/// Remap every `FileCoverage` in a coverage map. Entries without an
/// `inputSourceMap` pass through unchanged under their original key. Entries
/// with an `inputSourceMap` are rewritten and re-keyed by their resolved
/// original source path.
///
/// Entries that fan into the same original path merge by remapped location.
/// Equivalent hit counts use saturating `u32` addition, matching the counter
/// representation without allowing debug and release overflow to diverge.
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
        match remap_coverage_to_map_with_loader_and_options(fc, &loader, options) {
            Some(remapped) => {
                for (_, coverage) in remapped {
                    insert_or_merge_coverage(&mut out, coverage);
                }
            }
            None => {
                // Entry has no usable map (no embedded `inputSourceMap`, or an
                // unparsable one): it passes through under its original key. Still
                // enforce the Istanbul merge invariant (issue #107) so an
                // already-composed entry carrying a runtime orphan counter does not
                // slip through unchanged and crash a downstream `nyc` merge.
                let mut passthrough = fc.clone();
                passthrough.prune_orphan_counters();
                passthrough.path.clone_from(path);
                insert_or_merge_coverage(&mut out, passthrough);
            }
        }
    }
    out
}

/// getMapping lookup caches. Kept separate from the `&SourceMap` borrow so they
/// can outlive a single [`RemapContext`]: [`PositionRemapper`] owns one set and
/// reuses it across every eager-gate call, while a file remap uses a fresh
/// set per coverage map.
#[derive(Default)]
struct RemapCaches {
    mapping_cache: BTreeMap<LocationKey, Option<MappedLocation>>,
    original_line_columns: BTreeMap<OriginalLineKey, Vec<u32>>,
    // Each cache belongs to one SourceMap, so source indices are stable keys.
    original_line_ends: BTreeMap<u32, OriginalLineEndCache>,
}

impl RemapCaches {
    fn original_line_end_column(
        &mut self,
        sm: &srcmap_sourcemap::SourceMap,
        source_idx: u32,
        line: u32,
    ) -> u32 {
        let content = usize::try_from(source_idx)
            .ok()
            .and_then(|source_idx| sm.sources_content.get(source_idx))
            .and_then(Option::as_deref);
        let cache = self.original_line_ends.entry(source_idx).or_default();
        if let Some(content) = content
            && let Some(column) = cache.content.line_end(content, line)
        {
            return column;
        }
        cache.mapped_line_end(sm, source_idx, line)
    }
}

#[derive(Default)]
struct OriginalLineEndCache {
    content: ContentLineEnds,
    mapped: Option<MappedLineEnds>,
}

impl OriginalLineEndCache {
    fn mapped_line_end(
        &mut self,
        sm: &srcmap_sourcemap::SourceMap,
        source_idx: u32,
        line: u32,
    ) -> u32 {
        self.mapped.get_or_insert_with(|| MappedLineEnds::from_source_map(sm, source_idx)).get(line)
    }
}

// Bound dense storage so an untrusted original line cannot force a large allocation.
const MAX_DENSE_LINE_ENDS: usize = 65_536;
// Dense storage is worthwhile only when line indices are close to the mapping count.
const MAX_DENSE_SPARSITY: usize = 4;

enum MappedLineEnds {
    Dense(Vec<u32>),
    Sparse(BTreeMap<u32, u32>),
}

impl MappedLineEnds {
    fn from_source_map(sm: &srcmap_sourcemap::SourceMap, source_idx: u32) -> Self {
        let mappings: Vec<(u32, u32)> = sm
            .all_mappings()
            .iter()
            .filter(|mapping| mapping.source == source_idx)
            .map(|mapping| (mapping.original_line, mapping.original_column))
            .collect();
        let dense_len = mappings
            .iter()
            .map(|(line, _)| *line)
            .max()
            .and_then(|line| usize::try_from(line).ok())
            .and_then(|line| line.checked_add(1));
        let use_dense = dense_len.is_some_and(|len| {
            len <= MAX_DENSE_LINE_ENDS && len <= mappings.len().saturating_mul(MAX_DENSE_SPARSITY)
        });

        if use_dense {
            let mut line_ends = vec![0; dense_len.unwrap_or(0)];
            for (line, column) in mappings {
                if let Ok(line) = usize::try_from(line) {
                    line_ends[line] = line_ends[line].max(column);
                }
            }
            Self::Dense(line_ends)
        } else {
            let mut line_ends: BTreeMap<u32, u32> = BTreeMap::new();
            for (line, column) in mappings {
                line_ends.entry(line).and_modify(|end| *end = (*end).max(column)).or_insert(column);
            }
            Self::Sparse(line_ends)
        }
    }

    fn get(&self, line: u32) -> u32 {
        match self {
            Self::Dense(line_ends) => usize::try_from(line)
                .ok()
                .and_then(|line| line_ends.get(line))
                .copied()
                .unwrap_or(0),
            Self::Sparse(line_ends) => line_ends.get(&line).copied().unwrap_or(0),
        }
    }
}

#[derive(Default)]
struct ContentLineEnds {
    // Columns are populated only through the highest requested content line.
    columns: Vec<u32>,
    next_byte: usize,
    complete: bool,
}

impl ContentLineEnds {
    fn line_end(&mut self, content: &str, line: u32) -> Option<u32> {
        let line = usize::try_from(line).ok()?;
        while self.columns.len() <= line && !self.complete {
            let remaining = &content[self.next_byte..];
            if let Some(newline) = remaining.find('\n') {
                self.columns.push(utf16_line_len(&remaining[..newline]));
                self.next_byte += newline + 1;
            } else {
                self.columns.push(utf16_line_len(remaining));
                self.next_byte = content.len();
                self.complete = true;
            }
        }
        self.columns.get(line).copied()
    }
}

fn utf16_line_len(text: &str) -> u32 {
    let text = text.strip_suffix('\r').unwrap_or(text);
    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX)
}

#[derive(Clone)]
struct MappedLocation {
    source: u32,
    location: Location,
}

struct RemapContext<'a> {
    sm: &'a srcmap_sourcemap::SourceMap,
    caches: &'a mut RemapCaches,
}

impl<'a> RemapContext<'a> {
    fn new(sm: &'a srcmap_sourcemap::SourceMap, caches: &'a mut RemapCaches) -> Self {
        Self { sm, caches }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LocationKey {
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
}

impl From<&Location> for LocationKey {
    fn from(loc: &Location) -> Self {
        Self {
            start_line: loc.start.line,
            start_column: loc.start.column,
            end_line: loc.end.line,
            end_column: loc.end.column,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct OriginalLineKey {
    source: u32,
    line: u32,
}

fn apply_source_map_single(
    coverage: &FileCoverage,
    sm: &srcmap_sourcemap::SourceMap,
    options: RemapOptions,
    path: String,
) -> FileCoverage {
    let mut output = coverage.clone();
    output.path = path;
    output.input_source_map = None;
    let mut caches = RemapCaches::default();
    let mut ctx = RemapContext::new(sm, &mut caches);

    if options.drop_unmapped {
        prune_single_source_unmapped(&mut output, &mut ctx);
    } else {
        for location in output.statement_map.values_mut() {
            remap_single_source_location(location, &mut ctx);
        }
        for function in output.fn_map.values_mut() {
            remap_single_source_location(&mut function.decl, &mut ctx);
            remap_single_source_location(&mut function.loc, &mut ctx);
            function.line = function.loc.start.line;
        }
        for branch in output.branch_map.values_mut() {
            remap_single_source_location(&mut branch.loc, &mut ctx);
            for arm in &mut branch.locations {
                remap_single_source_location(arm, &mut ctx);
            }
            branch.line = branch.loc.start.line;
        }
    }
    output.prune_orphan_counters();
    output
}

fn remap_single_source_location(location: &mut Location, ctx: &mut RemapContext<'_>) {
    if location.start.line != 0
        && location.end.line != 0
        && let Some(mapped) = get_mapping_location_cached(ctx, location)
    {
        *location = mapped;
        return;
    }
    legacy_remap_location(location, ctx.sm);
}

fn try_remap_single_source_location(location: &mut Location, ctx: &mut RemapContext<'_>) -> bool {
    if location.start.line == 0 || location.end.line == 0 {
        legacy_remap_location(location, ctx.sm);
        return true;
    }
    let Some(mapped) = get_mapping_location_cached(ctx, location) else {
        return false;
    };
    *location = mapped;
    true
}

fn prune_single_source_unmapped(coverage: &mut FileCoverage, ctx: &mut RemapContext<'_>) {
    let mut dropped = Vec::new();
    coverage.statement_map.retain(|id, location| {
        if try_remap_single_source_location(location, ctx) {
            true
        } else {
            dropped.push(id.clone());
            false
        }
    });
    for id in std::mem::take(&mut dropped) {
        coverage.s.remove(&id);
    }

    coverage.fn_map.retain(|id, function| {
        let decl_maps = try_remap_single_source_location(&mut function.decl, ctx);
        let loc_maps = try_remap_single_source_location(&mut function.loc, ctx);
        if decl_maps && loc_maps {
            function.line = function.loc.start.line;
            true
        } else {
            dropped.push(id.clone());
            false
        }
    });
    for id in std::mem::take(&mut dropped) {
        coverage.f.remove(&id);
        if let Some(overlay) = coverage.x_fallow_function_map.as_mut() {
            overlay.remove(&id);
        }
    }

    let mut surviving_arms = BTreeMap::new();
    coverage.branch_map.retain(|id, branch| {
        let umbrella_maps = try_remap_single_source_location(&mut branch.loc, ctx);
        let mut indices = Vec::new();
        let mut locations = Vec::new();
        for (index, arm) in branch.locations.iter_mut().enumerate() {
            if try_remap_single_source_location(arm, ctx) {
                indices.push(index);
                locations.push(arm.clone());
            }
        }
        if locations.is_empty() {
            dropped.push(id.clone());
            return false;
        }
        if !umbrella_maps {
            branch.loc.clone_from(&locations[0]);
        }
        branch.locations = locations;
        branch.line = branch.loc.start.line;
        surviving_arms.insert(id.clone(), indices);
        true
    });
    for id in dropped {
        coverage.b.remove(&id);
        if let Some(b_t) = coverage.b_t.as_mut() {
            b_t.remove(&id);
        }
    }
    project_branch_counters(&mut coverage.b, &surviving_arms);
    if let Some(b_t) = coverage.b_t.as_mut() {
        project_branch_counters(b_t, &surviving_arms);
    }
}

fn project_branch_counters(
    counters: &mut BTreeMap<String, Vec<u32>>,
    surviving_arms: &BTreeMap<String, Vec<usize>>,
) {
    for (id, indices) in surviving_arms {
        if let Some(hits) = counters.get_mut(id) {
            *hits = project_arm_counts(hits, indices);
        }
    }
}

fn apply_source_map_to_map(
    coverage: &FileCoverage,
    sm: &srcmap_sourcemap::SourceMap,
    options: RemapOptions,
) -> Option<BTreeMap<String, FileCoverage>> {
    apply_source_map_to_map_internal(coverage, sm, options, true)
}

fn apply_source_map_to_map_internal(
    coverage: &FileCoverage,
    sm: &srcmap_sourcemap::SourceMap,
    options: RemapOptions,
    canonicalize_ids: bool,
) -> Option<BTreeMap<String, FileCoverage>> {
    let resolved_sources: Vec<(u32, String)> = sm
        .sources
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            let index = u32::try_from(index).ok()?;
            resolve_source_path(sm, index).map(|path| (index, path))
        })
        .collect();
    if resolved_sources.is_empty() {
        return None;
    }

    let unique_paths: BTreeMap<&str, u32> =
        resolved_sources.iter().map(|(source, path)| (path.as_str(), *source)).collect();
    let fallback_source =
        (!options.drop_unmapped && unique_paths.len() == 1).then(|| resolved_sources[0].0);
    let mut outputs = BTreeMap::new();
    let mut caches = RemapCaches::default();
    let mut ctx = RemapContext::new(sm, &mut caches);

    for (key, location) in &coverage.statement_map {
        let Some(mapped) = mapped_or_legacy_location(location, &mut ctx, fallback_source) else {
            continue;
        };
        let Some(output) = output_for_source(&mut outputs, coverage, sm, mapped.source) else {
            continue;
        };
        let id =
            if canonicalize_ids { output.statement_map.len().to_string() } else { key.clone() };
        output.statement_map.insert(id.clone(), mapped.location);
        if let Some(hits) = coverage.s.get(key) {
            output.s.insert(id, *hits);
        }
    }

    for (key, function) in &coverage.fn_map {
        let mapped_decl = mapped_or_legacy_location(&function.decl, &mut ctx, fallback_source);
        let mapped_loc = mapped_or_legacy_location(&function.loc, &mut ctx, fallback_source);
        let Some((source, decl, loc)) =
            matching_function_locations(function, mapped_decl, mapped_loc, &ctx, fallback_source)
        else {
            continue;
        };
        let Some(output) = output_for_source(&mut outputs, coverage, sm, source) else {
            continue;
        };
        let id = if canonicalize_ids { output.fn_map.len().to_string() } else { key.clone() };
        output.fn_map.insert(
            id.clone(),
            FnEntry { name: function.name.clone(), line: loc.start.line, decl, loc },
        );
        if let Some(hits) = coverage.f.get(key) {
            output.f.insert(id.clone(), *hits);
        }
        if let (Some(input_overlay), Some(output_overlay)) =
            (coverage.x_fallow_function_map.as_ref(), output.x_fallow_function_map.as_mut())
            && let Some(identity) = input_overlay.get(key)
        {
            output_overlay.insert(id, identity.clone());
        }
    }

    for (key, branch) in &coverage.branch_map {
        let mapped_loc = mapped_or_legacy_location(&branch.loc, &mut ctx, fallback_source);
        let mut kept_indices = Vec::new();
        let mut locations = Vec::new();
        let mut branch_source = None;
        let mut first_mapped_arm = None;
        let mut source_mismatch = false;
        for (index, arm) in branch.locations.iter().enumerate() {
            if branch.branch_type == "if" && index > 0 && location_is_unknown(arm) {
                kept_indices.push(index);
                locations.push(arm.clone());
                continue;
            }
            let Some(mapped_arm) = mapped_or_legacy_location(arm, &mut ctx, fallback_source) else {
                continue;
            };
            if branch_source.is_some_and(|source| source != mapped_arm.source) {
                source_mismatch = true;
                break;
            }
            branch_source = Some(mapped_arm.source);
            first_mapped_arm.get_or_insert_with(|| mapped_arm.location.clone());
            kept_indices.push(index);
            locations.push(mapped_arm.location);
        }
        let Some(branch_source) = branch_source else {
            continue;
        };
        if source_mismatch || locations.is_empty() {
            continue;
        }
        let branch_loc = mapped_loc
            .map(|mapped| mapped.location)
            .or(first_mapped_arm)
            .expect("branch source comes from a mapped arm");
        let Some(output) = output_for_source(&mut outputs, coverage, sm, branch_source) else {
            continue;
        };
        let id = if canonicalize_ids { output.branch_map.len().to_string() } else { key.clone() };
        output.branch_map.insert(
            id.clone(),
            BranchEntry {
                loc: branch_loc.clone(),
                line: branch_loc.start.line,
                branch_type: branch.branch_type.clone(),
                locations,
            },
        );
        if let Some(hits) = coverage.b.get(key) {
            output.b.insert(id.clone(), project_arm_counts(hits, &kept_indices));
        }
        if let (Some(input_b_t), Some(output_b_t)) = (coverage.b_t.as_ref(), output.b_t.as_mut())
            && let Some(hits) = input_b_t.get(key)
        {
            output_b_t.insert(id, project_arm_counts(hits, &kept_indices));
        }
    }

    for output in outputs.values_mut() {
        if canonicalize_ids {
            let mut canonical = empty_file_coverage(output, output.path.clone());
            merge_file_coverage(&mut canonical, output);
            *output = canonical;
        } else {
            output.prune_orphan_counters();
        }
    }
    Some(outputs)
}

fn empty_file_coverage(template: &FileCoverage, path: String) -> FileCoverage {
    FileCoverage {
        path,
        statement_map: BTreeMap::new(),
        fn_map: BTreeMap::new(),
        branch_map: BTreeMap::new(),
        s: BTreeMap::new(),
        f: BTreeMap::new(),
        b: BTreeMap::new(),
        b_t: template.b_t.as_ref().map(|_| BTreeMap::new()),
        input_source_map: None,
        x_fallow_function_map: template.x_fallow_function_map.as_ref().map(|_| BTreeMap::new()),
    }
}

fn output_for_source<'a>(
    outputs: &'a mut BTreeMap<String, FileCoverage>,
    template: &FileCoverage,
    sm: &srcmap_sourcemap::SourceMap,
    source: u32,
) -> Option<&'a mut FileCoverage> {
    let path = resolve_source_path(sm, source)?;
    Some(outputs.entry(path.clone()).or_insert_with(|| empty_file_coverage(template, path)))
}

fn mapped_or_legacy_location(
    location: &Location,
    ctx: &mut RemapContext<'_>,
    fallback_source: Option<u32>,
) -> Option<MappedLocation> {
    if location.start.line != 0
        && location.end.line != 0
        && let Some(mapped) = get_mapped_location_cached(ctx, location)
    {
        return Some(mapped);
    }
    let source = fallback_source?;
    let mut location = location.clone();
    legacy_remap_location(&mut location, ctx.sm);
    Some(MappedLocation { source, location })
}

fn matching_function_locations(
    function: &FnEntry,
    mapped_decl: Option<MappedLocation>,
    mapped_loc: Option<MappedLocation>,
    ctx: &RemapContext<'_>,
    fallback_source: Option<u32>,
) -> Option<(u32, Location, Location)> {
    match (mapped_decl, mapped_loc) {
        (Some(decl), Some(loc)) if decl.source == loc.source => {
            Some((decl.source, decl.location, loc.location))
        }
        _ => {
            let source = fallback_source?;
            let mut decl = function.decl.clone();
            let mut loc = function.loc.clone();
            legacy_remap_location(&mut decl, ctx.sm);
            legacy_remap_location(&mut loc, ctx.sm);
            Some((source, decl, loc))
        }
    }
}

fn project_arm_counts(hits: &[u32], indices: &[usize]) -> Vec<u32> {
    indices.iter().map(|index| hits.get(*index).copied().unwrap_or(0)).collect()
}

fn location_is_unknown(location: &Location) -> bool {
    location.start.line == 0
        && location.start.column == 0
        && location.end.line == 0
        && location.end.column == 0
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FunctionMergeKey {
    decl: LocationKey,
}

impl From<&FnEntry> for FunctionMergeKey {
    fn from(function: &FnEntry) -> Self {
        Self { decl: LocationKey::from(&function.decl) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BranchMergeKey {
    locations: Vec<LocationKey>,
}

impl From<&BranchEntry> for BranchMergeKey {
    fn from(branch: &BranchEntry) -> Self {
        Self { locations: branch.locations.iter().map(LocationKey::from).collect() }
    }
}

fn insert_or_merge_coverage(
    coverage_map: &mut BTreeMap<String, FileCoverage>,
    incoming: FileCoverage,
) {
    if let Some(existing) = coverage_map.get_mut(&incoming.path) {
        let mut merged = empty_file_coverage(existing, existing.path.clone());
        merge_file_coverage(&mut merged, existing);
        merge_file_coverage(&mut merged, &incoming);
        *existing = merged;
    } else {
        coverage_map.insert(incoming.path.clone(), incoming);
    }
}

fn merge_file_coverage(existing: &mut FileCoverage, incoming: &FileCoverage) {
    merge_statements(existing, incoming);
    merge_functions(existing, incoming);
    merge_branches(existing, incoming);
    existing.input_source_map = None;
    existing.prune_orphan_counters();
}

fn merge_statements(existing: &mut FileCoverage, incoming: &FileCoverage) {
    let mut ids: BTreeMap<LocationKey, String> = existing
        .statement_map
        .iter()
        .map(|(id, location)| (LocationKey::from(location), id.clone()))
        .collect();
    for (incoming_id, location) in &incoming.statement_map {
        let key = LocationKey::from(location);
        let output_id = if let Some(id) = ids.get(&key) {
            id.clone()
        } else {
            let id = existing.statement_map.len().to_string();
            existing.statement_map.insert(id.clone(), location.clone());
            ids.insert(key, id.clone());
            id
        };
        merge_scalar_counter(&mut existing.s, &output_id, incoming.s.get(incoming_id).copied());
    }
}

fn merge_functions(existing: &mut FileCoverage, incoming: &FileCoverage) {
    if incoming.fn_map.is_empty() {
        return;
    }
    let existing_has_functions = !existing.fn_map.is_empty();
    if !existing_has_functions {
        existing.x_fallow_function_map =
            incoming.x_fallow_function_map.as_ref().map(|_| BTreeMap::new());
    }
    let mut ids: BTreeMap<FunctionMergeKey, String> = existing
        .fn_map
        .iter()
        .map(|(id, function)| (FunctionMergeKey::from(function), id.clone()))
        .collect();
    let incoming_overlay = incoming.x_fallow_function_map.as_ref();
    let mut overlay_conflict = incoming_overlay.is_none()
        || (existing_has_functions && existing.x_fallow_function_map.is_none());

    for (incoming_id, function) in &incoming.fn_map {
        let key = FunctionMergeKey::from(function);
        let (output_id, existed) = if let Some(id) = ids.get(&key) {
            (id.clone(), true)
        } else {
            let id = existing.fn_map.len().to_string();
            existing.fn_map.insert(id.clone(), function.clone());
            ids.insert(key, id.clone());
            (id, false)
        };
        merge_scalar_counter(&mut existing.f, &output_id, incoming.f.get(incoming_id).copied());

        if overlay_conflict {
            continue;
        }
        let incoming_identity = incoming_overlay.and_then(|overlay| overlay.get(incoming_id));
        let existing_overlay = existing.x_fallow_function_map.as_mut().expect("checked above");
        if existed {
            let existing_identity = existing_overlay.get(&output_id);
            if !matches!((existing_identity, incoming_identity),
                (Some(left), Some(right)) if function_identities_equal(left, right))
            {
                overlay_conflict = true;
            }
        } else if let Some(identity) = incoming_identity {
            existing_overlay.insert(output_id, identity.clone());
        } else {
            overlay_conflict = true;
        }
    }

    if overlay_conflict {
        existing.x_fallow_function_map = None;
    }
}

fn merge_branches(existing: &mut FileCoverage, incoming: &FileCoverage) {
    let mut ids: BTreeMap<BranchMergeKey, String> = existing
        .branch_map
        .iter()
        .map(|(id, branch)| (BranchMergeKey::from(branch), id.clone()))
        .collect();
    if existing.b_t.is_none() && incoming.b_t.is_some() {
        existing.b_t = Some(BTreeMap::new());
    }

    for (incoming_id, branch) in &incoming.branch_map {
        let key = BranchMergeKey::from(branch);
        let output_id = if let Some(id) = ids.get(&key) {
            id.clone()
        } else {
            let id = existing.branch_map.len().to_string();
            existing.branch_map.insert(id.clone(), branch.clone());
            ids.insert(key, id.clone());
            id
        };
        merge_vector_counter(&mut existing.b, &output_id, incoming.b.get(incoming_id));
        if let (Some(existing_b_t), Some(incoming_b_t)) =
            (existing.b_t.as_mut(), incoming.b_t.as_ref())
        {
            merge_vector_counter(existing_b_t, &output_id, incoming_b_t.get(incoming_id));
        }
    }
}

fn merge_scalar_counter(counters: &mut BTreeMap<String, u32>, id: &str, incoming: Option<u32>) {
    let Some(incoming) = incoming else {
        return;
    };
    let counter = counters.entry(id.to_string()).or_default();
    *counter = counter.saturating_add(incoming);
}

fn merge_vector_counter(
    counters: &mut BTreeMap<String, Vec<u32>>,
    id: &str,
    incoming: Option<&Vec<u32>>,
) {
    let Some(incoming) = incoming else {
        return;
    };
    let current = counters.entry(id.to_string()).or_insert_with(|| vec![0; incoming.len()]);
    if current.len() < incoming.len() {
        current.resize(incoming.len(), 0);
    }
    for (current, incoming) in current.iter_mut().zip(incoming) {
        *current = current.saturating_add(*incoming);
    }
}

fn function_identities_equal(
    left: &oxc_coverage_types::FunctionIdentity,
    right: &oxc_coverage_types::FunctionIdentity,
) -> bool {
    left.id == right.id
        && left.name == right.name
        && left.path == right.path
        && LocationKey::from(&left.decl) == LocationKey::from(&right.decl)
        && LocationKey::from(&left.loc) == LocationKey::from(&right.loc)
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
/// store.add_map("intermediate.js", &serde_json::from_str(input_sm).unwrap());
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
    pub fn add_map(&mut self, file: impl Into<String>, source_map: &serde_json::Value) {
        let parsed = serde_json::to_string(source_map)
            .ok()
            .and_then(|json| srcmap_sourcemap::SourceMap::from_json(&json).ok());
        self.maps.insert(file.into(), parsed);
    }

    /// Add a source map from its JSON string form.
    ///
    /// This is equivalent to [`SourceMapStore::add_map`] but avoids a
    /// Value parse plus serialization round-trip when callers already have
    /// source map JSON, as JavaScript bindings and disk loaders commonly do.
    pub fn add_map_json(&mut self, file: impl Into<String>, source_map_json: &str) {
        let parsed = srcmap_sourcemap::SourceMap::from_json(source_map_json).ok();
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
            if let Some(path) = sole_resolved_source_path(sm) {
                return Some(apply_source_map_single(coverage, sm, options, path));
            }
            let remapped = apply_source_map_to_map_internal(coverage, sm, options, false)?;
            return select_single_remap(remapped);
        }
        remap_coverage_with_options(coverage, options)
    }

    /// Remap one file into every source represented by its stored or embedded
    /// source map.
    #[must_use]
    pub fn transform_coverage_to_map(
        &self,
        coverage: &FileCoverage,
    ) -> Option<BTreeMap<String, FileCoverage>> {
        self.transform_coverage_to_map_with_options(coverage, RemapOptions::default())
    }

    /// Like [`SourceMapStore::transform_coverage_to_map`], with remap options.
    #[must_use]
    pub fn transform_coverage_to_map_with_options(
        &self,
        coverage: &FileCoverage,
        options: RemapOptions,
    ) -> Option<BTreeMap<String, FileCoverage>> {
        if let Some(sm) = self.maps.get(&coverage.path) {
            let sm = sm.as_ref()?;
            return apply_source_map_to_map(coverage, sm, options);
        }
        remap_coverage_to_map_with_options(coverage, options)
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
            match self.transform_coverage_to_map_with_options(fc, options) {
                Some(remapped) => {
                    for (_, coverage) in remapped {
                        insert_or_merge_coverage(&mut out, coverage);
                    }
                }
                None => {
                    // Same invariant pass as the standalone map helper: an
                    // entry with no store/embedded map still gets its orphan
                    // counters dropped (issue #107) before passing through.
                    let mut passthrough = fc.clone();
                    passthrough.prune_orphan_counters();
                    passthrough.path.clone_from(path);
                    insert_or_merge_coverage(&mut out, passthrough);
                }
            }
        }
        out
    }
}

fn resolve_primary_source(sm: &srcmap_sourcemap::SourceMap) -> Option<String> {
    resolve_source_path(sm, 0)
}

fn resolve_source_path(sm: &srcmap_sourcemap::SourceMap, source: u32) -> Option<String> {
    let source = sm.sources.get(source as usize)?;
    if source.is_empty() {
        return None;
    }
    let root = sm.source_root.as_deref().unwrap_or("");
    if root.is_empty() {
        return Some(source.clone());
    }

    // `srcmap-sourcemap::from_json` pre-joins `sourceRoot` and each source via
    // literal concatenation (spec-strict). `istanbul-lib-source-maps` inserts
    // a `/` separator when `sourceRoot` lacks one. Strip the literal prefix
    // and re-join with Istanbul's separator rule so coverage paths match what
    // existing reporters expect.
    let bare = source.strip_prefix(root).unwrap_or(source.as_str());
    if root.ends_with('/') || bare.starts_with('/') {
        Some(format!("{root}{bare}"))
    } else {
        Some(format!("{root}/{bare}"))
    }
}

fn sole_resolved_source_path(sm: &srcmap_sourcemap::SourceMap) -> Option<String> {
    let mut paths: BTreeSet<String> = sm
        .sources
        .iter()
        .enumerate()
        .filter_map(|(source, _)| {
            let source = u32::try_from(source).ok()?;
            resolve_source_path(sm, source)
        })
        .collect();
    if paths.len() != 1 {
        return None;
    }
    paths.pop_first()
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
    /// length at use via [`RemapCaches::original_line_end_column`].
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

#[derive(Clone, Copy)]
struct OriginalLookup<'a> {
    source: &'a str,
    line: u32,
    column: u32,
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
/// result faithful when several mappings share a generated column.
/// The first lookup for a `(source, line)` pair builds a sorted original-column
/// index in [`RemapContext`]; following lookups on that line are logarithmic.
fn all_generated_positions_for_lub(
    ctx: &mut RemapContext<'_>,
    lookup: OriginalLookup<'_>,
) -> Vec<srcmap_sourcemap::GeneratedLocation> {
    let OriginalLookup { source, line, column } = lookup;
    // Key the scan by name -> index (not the caller's raw mapping index): istanbul
    // and trace-mapping look up the source by name too, so for a map where the
    // same name appears at multiple indices this matches the library's first-match
    // behaviour. Well-formed maps have unique source names, so the two coincide.
    let Some(source_idx) = ctx.sm.source_index(source) else {
        return Vec::new();
    };
    let matched_column = matched_original_column(ctx, source_idx, line, column);
    let Some(matched_column) = matched_column else {
        return Vec::new();
    };
    ctx.sm.all_generated_positions_for(source, line, matched_column)
}

fn matched_original_column(
    ctx: &mut RemapContext<'_>,
    source_idx: u32,
    line: u32,
    column: u32,
) -> Option<u32> {
    let key = OriginalLineKey { source: source_idx, line };
    if !ctx.caches.original_line_columns.contains_key(&key) {
        let mut columns: Vec<u32> = ctx
            .sm
            .all_mappings()
            .iter()
            .filter(|m| m.source == source_idx && m.original_line == line)
            .map(|m| m.original_column)
            .collect();
        columns.sort_unstable();
        columns.dedup();
        ctx.caches.original_line_columns.insert(key, columns);
    }
    let columns = ctx.caches.original_line_columns.get(&key)?;
    let idx = columns.partition_point(|mapped| *mapped < column);
    columns.get(idx).copied()
}

/// `originalEndPositionFor`: resolve the exclusive end of a generated range to
/// the start of the next original segment, or the end of the original line.
/// `gen_end_line` / `gen_end_col` are 0-based (srcmap space).
fn original_end_position_for(
    ctx: &mut RemapContext<'_>,
    gen_end_line: u32,
    gen_end_col: u32,
) -> Option<EndResult> {
    // beforeEnd = originalPositionTryBoth(line, column - 1). A column-0 exclusive
    // end would make istanbul evaluate `column - 1 === -1`, which
    // `@jridgewell/trace-mapping` rejects (it throws); we conservatively report
    // "no mapping" so the caller drops the entry (drop mode) or keeps the
    // generated position (no-drop mode) instead of panicking.
    let before_col = gen_end_col.checked_sub(1)?;
    let before = original_position_try_both(ctx.sm, gen_end_line, before_col)?;
    let source = ctx.sm.source(before.source);

    // afterEndMappings = allGeneratedPositionsFor(LUB) one column to the right;
    // map each back (GLB) and take the first that lands on the same original
    // line: that segment's start is our exclusive end.
    let after = all_generated_positions_for_lub(
        ctx,
        OriginalLookup { source, line: before.line, column: before.column + 1 },
    );
    for gen_pos in &after {
        if let Some(orig) = ctx.sm.original_position_for_with_bias(
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

/// Resolve a `Location` through istanbul `getMapping` semantics. Returns the
/// remapped original-space location, or `None` when `getMapping` would yield
/// `null` (start or end fails to map, or they map to different sources). The
/// returned positions are in Istanbul space (1-based line, 0-based column).
///
/// The `line: 0` "unknown" sentinel is handled by the callers, not here:
/// `getMapping` has no notion of it and `line - 1` would underflow.
fn get_mapping_location(loc: &Location, ctx: &mut RemapContext<'_>) -> Option<MappedLocation> {
    let start = original_position_try_both(ctx.sm, loc.start.line - 1, loc.start.column)?;
    let end = original_end_position_for(ctx, loc.end.line - 1, loc.end.column)?;

    let (end_source, mut end_line, mut end_col, end_is_eol) = match end {
        EndResult::Mapped { source, line, column } => (source, line, column, false),
        EndResult::EndOfLine { source, line } => {
            (source, line, ctx.caches.original_line_end_column(ctx.sm, source, line), true)
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
        let lub = original_position_for_lub(ctx.sm, loc.end.line - 1, loc.end.column)?;
        end_line = lub.line;
        // get-mapping.js does `end.column -= 1` unconditionally; when LUB lands on
        // column 0 it yields a JS `-1`, which can never round-trip through our
        // `u32` (a negative column is an invalid Istanbul position anyway). We
        // saturate to 0 instead. This sub-case requires the degenerate guard to
        // fire AND the recomputed LUB to sit at column 0 on the same generated
        // line; istanbul itself marks the branch "edge case too hard to test for".
        end_col = lub.column.saturating_sub(1);
    }

    Some(MappedLocation {
        source: start.source,
        location: Location {
            start: Position { line: start.line + 1, column: start.column },
            end: Position { line: end_line + 1, column: end_col },
        },
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

fn legacy_remap_location(loc: &mut Location, sm: &srcmap_sourcemap::SourceMap) {
    legacy_remap_position(&mut loc.start, sm);
    legacy_remap_position(&mut loc.end, sm);
}

// --- remap entry points (route through getMapping) ----------------------------

fn get_mapped_location_cached(
    ctx: &mut RemapContext<'_>,
    loc: &Location,
) -> Option<MappedLocation> {
    let key = LocationKey::from(loc);
    if let Some(cached) = ctx.caches.mapping_cache.get(&key) {
        return cached.clone();
    }
    let remapped = get_mapping_location(loc, ctx);
    ctx.caches.mapping_cache.insert(key, remapped.clone());
    remapped
}

fn get_mapping_location_cached(ctx: &mut RemapContext<'_>, loc: &Location) -> Option<Location> {
    get_mapped_location_cached(ctx, loc).map(|mapped| mapped.location)
}
