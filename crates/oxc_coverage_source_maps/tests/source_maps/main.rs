//! Behaviour tests for the `oxc_coverage_source_maps` public API: multi-source
//! fan-out, merge by remapped location, `drop_unmapped` pruning, `getMapping`
//! range remap, orphan-counter reconciliation, [`SourceMapStore`] and
//! [`PositionRemapper`].
//!
//! [`SourceMapStore`]: oxc_coverage_source_maps::SourceMapStore
//! [`PositionRemapper`]: oxc_coverage_source_maps::PositionRemapper

mod drop_unmapped;
mod fan_out;
mod fixtures;
mod get_mapping;
mod id_order;
mod merge;
mod orphan_counters;
mod position_remapper;
mod remap;
mod store;
