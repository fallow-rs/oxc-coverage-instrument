//! Stateful source map store for the remap-during-collection flow.

use std::collections::BTreeMap;

use oxc_coverage_types::FileCoverage;
use srcmap_sourcemap::SourceMap;

use crate::{
    apply::{apply_source_map_single_result, apply_source_map_to_map},
    merge::fold_remap_result,
    options::RemapOptions,
    remap::{remap_coverage_to_map_with_options, remap_coverage_with_options},
};

/// Stateful map store for the Mode B "continuous remap during collection"
/// flow.
///
/// Some runners (Jest with `transform`) instrument files incrementally and want
/// to record each file's source map as it is produced, then rewrite
/// `FileCoverage` objects on the fly rather than once at report time.
///
/// # Example
///
/// ```
/// use oxc_coverage_source_maps::SourceMapStore;
/// use oxc_coverage_types::FileCoverage;
///
/// let mut store = SourceMapStore::new();
/// let input_sm = r#"{"version":3,"sources":["src/app.ts"],"mappings":"AAAA","names":[]}"#;
/// store.add_map("intermediate.js", &serde_json::from_str(input_sm).unwrap());
///
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
    maps: BTreeMap<String, Option<SourceMap>>,
}

impl SourceMapStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source map keyed by the file path.
    ///
    /// A later `add_map` with the same key replaces the earlier entry, matching
    /// `istanbul-lib-source-maps`'s `registerMap` semantics.
    pub fn add_map(&mut self, file: impl Into<String>, source_map: &serde_json::Value) {
        let parsed = serde_json::to_string(source_map)
            .ok()
            .and_then(|json| SourceMap::from_json(&json).ok());
        self.maps.insert(file.into(), parsed);
    }

    /// Add a source map from its JSON string form.
    ///
    /// Equivalent to [`SourceMapStore::add_map`], but avoids a `Value` parse
    /// plus serialization round-trip when callers already have source map JSON,
    /// as JavaScript bindings and disk loaders commonly do.
    pub fn add_map_json(&mut self, file: impl Into<String>, source_map_json: &str) {
        let parsed = SourceMap::from_json(source_map_json).ok();
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

    /// Remap a single `FileCoverage` using the stored map for its `path`.
    ///
    /// The store takes precedence over any embedded `inputSourceMap`: when the
    /// store has an entry for `coverage.path`, that map is used; otherwise the
    /// embedded map (if any) is used; otherwise `None`.
    ///
    /// Returns `None` when neither the store nor the embedded map yields a
    /// usable map, matching [`remap_coverage`]'s fallback semantics.
    ///
    /// [`remap_coverage`]: crate::remap_coverage
    #[must_use]
    pub fn transform_coverage(&self, coverage: &FileCoverage) -> Option<FileCoverage> {
        self.transform_coverage_with_options(coverage, RemapOptions::default())
    }

    /// Like [`SourceMapStore::transform_coverage`], but with a
    /// [`RemapOptions`] argument. See [`RemapOptions::drop_unmapped`] for the
    /// pruning semantics.
    ///
    /// A `Some` result is reconciled to the Istanbul merge invariant, with
    /// orphan counters dropped. A `None` result means "no usable map": the
    /// caller keeps the original `FileCoverage` and owns reconciliation in that
    /// case. The map-level [`SourceMapStore::transform_coverage_map`]
    /// reconciles its `None` passthrough entries for you.
    #[must_use]
    pub fn transform_coverage_with_options(
        &self,
        coverage: &FileCoverage,
        options: RemapOptions,
    ) -> Option<FileCoverage> {
        if let Some(sm) = self.maps.get(&coverage.path) {
            return apply_source_map_single_result(coverage, sm.as_ref()?, options);
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

    /// Remap every entry of a coverage map using the store.
    ///
    /// Entries whose `path` is not in the store and which carry no embedded
    /// `inputSourceMap` pass through unchanged under their original key.
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
            let remapped = self.transform_coverage_to_map_with_options(fc, options);
            fold_remap_result(&mut out, path, fc, remapped);
        }
        out
    }
}
