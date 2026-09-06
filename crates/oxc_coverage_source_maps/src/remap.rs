//! Entry points that remap a `FileCoverage`, or a whole coverage map, through
//! an input source map.

use std::collections::BTreeMap;

use oxc_coverage_types::FileCoverage;
use srcmap_sourcemap::SourceMap;

use crate::{
    apply::{apply_source_map_single_result, apply_source_map_to_map},
    merge::fold_remap_result,
    options::RemapOptions,
};

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
/// dropped via [`FileCoverage::prune_orphan_counters`].
#[must_use]
pub fn remap_coverage(coverage: &FileCoverage) -> Option<FileCoverage> {
    remap_coverage_with_loader_and_options(coverage, |_| None, RemapOptions::default())
}

/// Like [`remap_coverage`], but with a [`RemapOptions`] argument. See
/// [`RemapOptions::drop_unmapped`] for the pruning semantics.
#[must_use]
pub fn remap_coverage_with_options(
    coverage: &FileCoverage,
    options: RemapOptions,
) -> Option<FileCoverage> {
    remap_coverage_with_loader_and_options(coverage, |_| None, options)
}

/// Like [`remap_coverage`], but with a loader fallback for entries that carry
/// no embedded `inputSourceMap`.
///
/// The loader is called with the `FileCoverage.path` and returns the source map
/// JSON, matching `istanbul-lib-source-maps`'s `sourceStore` callback used by
/// nyc when the map sits next to the instrumented file rather than embedded
/// inside the coverage object. Returning `None` from the loader (or supplying
/// `|_| None`) makes this behave identically to [`remap_coverage`].
#[must_use]
pub fn remap_coverage_with_loader<L>(coverage: &FileCoverage, loader: L) -> Option<FileCoverage>
where
    L: Fn(&str) -> Option<String>,
{
    remap_coverage_with_loader_and_options(coverage, loader, RemapOptions::default())
}

/// Like [`remap_coverage_with_loader`], with a [`RemapOptions`] argument. See
/// [`RemapOptions::drop_unmapped`] for the pruning semantics.
#[must_use]
pub fn remap_coverage_with_loader_and_options<L>(
    coverage: &FileCoverage,
    loader: L,
    options: RemapOptions,
) -> Option<FileCoverage>
where
    L: Fn(&str) -> Option<String>,
{
    let sm = source_map_with_loader(coverage, loader)?;
    apply_source_map_single_result(coverage, &sm, options)
}

/// Remap one `FileCoverage` into every original source represented by its
/// mappings. Returns `None` only when no usable source map is available.
///
/// Unlike [`remap_coverage`], this API can represent a true multi-source
/// result. Each returned file has contiguous metadata ids, aligned counters,
/// and no `inputSourceMap`.
#[must_use]
pub fn remap_coverage_to_map(coverage: &FileCoverage) -> Option<BTreeMap<String, FileCoverage>> {
    remap_coverage_to_map_with_loader_and_options(coverage, |_| None, RemapOptions::default())
}

/// Like [`remap_coverage_to_map`], with a [`RemapOptions`] argument.
#[must_use]
pub fn remap_coverage_to_map_with_options(
    coverage: &FileCoverage,
    options: RemapOptions,
) -> Option<BTreeMap<String, FileCoverage>> {
    remap_coverage_to_map_with_loader_and_options(coverage, |_| None, options)
}

/// Like [`remap_coverage_to_map`], with a source-map loader fallback.
#[must_use]
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
#[must_use]
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

fn source_map_with_loader<L>(coverage: &FileCoverage, loader: L) -> Option<SourceMap>
where
    L: Fn(&str) -> Option<String>,
{
    let input_sm_json = match coverage.input_source_map.as_ref() {
        Some(value) => serde_json::to_string(value).ok()?,
        None => loader(&coverage.path)?,
    };
    SourceMap::from_json(&input_sm_json).ok()
}

/// Remap every `FileCoverage` in a coverage map.
///
/// Entries without an `inputSourceMap` pass through unchanged under their
/// original key. Entries with an `inputSourceMap` are rewritten and re-keyed by
/// their resolved original source path.
///
/// Entries that fan into the same original path merge by remapped location.
/// Equivalent hit counts use saturating `u32` addition, matching the counter
/// representation without allowing debug and release overflow to diverge.
#[must_use]
pub fn remap_coverage_map(
    coverage_map: &BTreeMap<String, FileCoverage>,
) -> BTreeMap<String, FileCoverage> {
    remap_coverage_map_with_loader_and_options(coverage_map, |_| None, RemapOptions::default())
}

/// Like [`remap_coverage_map`], with a [`RemapOptions`] argument. See
/// [`RemapOptions::drop_unmapped`] for the pruning semantics.
#[must_use]
pub fn remap_coverage_map_with_options(
    coverage_map: &BTreeMap<String, FileCoverage>,
    options: RemapOptions,
) -> BTreeMap<String, FileCoverage> {
    remap_coverage_map_with_loader_and_options(coverage_map, |_| None, options)
}

/// Like [`remap_coverage_map`], but with a disk-read fallback.
///
/// The loader is called with the `FileCoverage.path` of each entry that lacks
/// an embedded `inputSourceMap`.
#[must_use]
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
#[must_use]
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
        let remapped = remap_coverage_to_map_with_loader_and_options(fc, &loader, options);
        fold_remap_result(&mut out, path, fc, remapped);
    }
    out
}
