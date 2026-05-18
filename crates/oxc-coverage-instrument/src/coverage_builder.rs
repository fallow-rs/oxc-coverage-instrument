//! Crate-internal builder that converts the AST-traversal counter Vecs into
//! the Istanbul-shaped `FileCoverage`.
//!
//! Lives on the instrument side (not in `oxc_coverage_types`) because the
//! input shape (sequential `Vec<Location>` etc. keyed by counter id) is an
//! implementation detail of the transform pass, not part of the public data
//! model.

use std::collections::BTreeMap;

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location};

/// Inputs to [`build_file_coverage`], grouped so callers thread one value
/// instead of five.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal type intentionally; the explicit pub(crate) documents that this is not part of the public API even though the parent module is already private"
)]
pub(crate) struct CoverageMaps {
    /// File path stored on the resulting `FileCoverage`.
    pub(crate) path: String,
    /// Statement spans collected during traversal, indexed by counter id.
    pub(crate) statement_locs: Vec<Location>,
    /// Function metadata (name, decl span, body span) indexed by counter id.
    pub(crate) fn_entries: Vec<FnEntry>,
    /// Branch metadata indexed by counter id; entries with empty `locations`
    /// are dropped during map construction.
    pub(crate) branch_entries: Vec<BranchEntry>,
    /// Counter ids of branches that should also be tracked in the truthy
    /// (`bT`) map; only populated when `report_logic` is on.
    pub(crate) logical_branch_ids: Vec<usize>,
}

/// Convert sequential id-indexed Vecs collected during AST traversal into the
/// Istanbul-shaped `FileCoverage`. The Vecs are converted once into the
/// `BTreeMap<String, _>` here so the hot traversal path avoids per-add String
/// allocations and tree rebalancing.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal function intentionally; the explicit pub(crate) documents that this is not part of the public API even though the parent module is already private"
)]
pub(crate) fn build_file_coverage(maps: CoverageMaps) -> FileCoverage {
    let CoverageMaps { path, statement_locs, fn_entries, branch_entries, logical_branch_ids } =
        maps;
    let statement_map: BTreeMap<String, Location> =
        statement_locs.into_iter().enumerate().map(|(i, loc)| (i.to_string(), loc)).collect();
    let fn_map: BTreeMap<String, FnEntry> =
        fn_entries.into_iter().enumerate().map(|(i, e)| (i.to_string(), e)).collect();
    // Drop branches that never got any path locations (e.g. both `if` arms
    // suppressed by pragmas). Original ids are preserved so generated
    // counter ids still line up with the public maps.
    let branch_map: BTreeMap<String, BranchEntry> = branch_entries
        .into_iter()
        .enumerate()
        .filter(|(_, entry)| !entry.locations.is_empty())
        .map(|(i, entry)| (i.to_string(), entry))
        .collect();

    let s = statement_map.keys().map(|k| (k.clone(), 0)).collect();
    let f = fn_map.keys().map(|k| (k.clone(), 0)).collect();
    let b =
        branch_map.iter().map(|(k, entry)| (k.clone(), vec![0; entry.locations.len()])).collect();
    let b_t = if logical_branch_ids.is_empty() {
        None
    } else {
        Some(
            logical_branch_ids
                .iter()
                .filter_map(|&id| {
                    let key = id.to_string();
                    let len = branch_map.get(&key)?.locations.len();
                    Some((key, vec![0; len]))
                })
                .collect(),
        )
    };

    FileCoverage { path, statement_map, fn_map, branch_map, s, f, b, b_t, input_source_map: None }
}
