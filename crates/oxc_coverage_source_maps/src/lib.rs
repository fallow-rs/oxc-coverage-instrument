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
//! Surviving positions resolve through istanbul's `getMapping` range
//! semantics: starts resolve with greatest-lower-bound, ends resolve to the
//! next original segment or the end of the original line, matching
//! `istanbul-lib-source-maps`'s `createSourceMapStore().transformCoverage`
//! byte-for-byte.
//!
//! ## Files
//!
//! * `remap.rs`:             Public entry points and the source map loader fallback.
//! * `store.rs`:             [`SourceMapStore`], the Mode B container.
//! * `options.rs`:           [`RemapOptions`].
//! * `position_remapper.rs`: [`PositionRemapper`], prepared-map reuse and the instrument-time gate.
//! * `apply.rs`:             Rewriting one `FileCoverage` through a parsed source map.
//! * `merge.rs`:             Merging entries that land on the same original path.
//! * `get_mapping.rs`:       Istanbul `getMapping` position resolution.
//! * `sources.rs`:           `sources` / `sourceRoot` to coverage path resolution.
//! * `context.rs`:           Lookup context and caches.

mod apply;
mod context;
mod get_mapping;
mod merge;
mod options;
mod position_remapper;
mod remap;
mod sources;
mod store;

pub use crate::{
    options::RemapOptions,
    position_remapper::{GeneratedLineShift, PositionRemapper, finalize_generated_source_map},
    remap::{
        remap_coverage, remap_coverage_map, remap_coverage_map_with_loader,
        remap_coverage_map_with_loader_and_options, remap_coverage_map_with_options,
        remap_coverage_to_map, remap_coverage_to_map_with_loader,
        remap_coverage_to_map_with_loader_and_options, remap_coverage_to_map_with_options,
        remap_coverage_with_loader, remap_coverage_with_loader_and_options,
        remap_coverage_with_options,
    },
    store::SourceMapStore,
};
