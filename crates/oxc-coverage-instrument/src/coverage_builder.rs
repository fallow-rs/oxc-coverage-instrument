//! Crate-internal builder that converts the AST-traversal counter Vecs into
//! the Istanbul-shaped `FileCoverage`.
//!
//! Lives on the instrument side (not in `oxc_coverage_types`) because the
//! input shape (sequential `Vec<Location>` etc. keyed by counter id) is an
//! implementation detail of the transform pass, not part of the public data
//! model.

use std::collections::BTreeMap;

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, FunctionIdentity, Location};

use crate::transform::djb31_hex;

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

    FileCoverage {
        path,
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t,
        input_source_map: None,
        x_fallow_function_map: None,
    }
}

/// Compute the optional `x_fallow_functionMap` overlay from a populated
/// `fn_map`. Each entry's id is a stable hash of `(path, name, decl span,
/// loc span)`, formatted as `fallow:fn:<hex>`. A rename, body edit, or
/// line shift changes the hash but a re-run on byte-identical source does
/// not.
///
/// Hash input is the JSON-encoded form of the parts array. JSON encoding
/// makes the field boundaries unambiguous so a computed-key method like
/// `class C { ['x|y']() {} }` (which lands in `fn_map[].name` as `"x|y"`)
/// cannot collide with any sibling whose parts happen to align under a
/// flat string separator. Keyed by the same string ids as `fn_map`;
/// consumers join via the key.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal helper intentionally; the explicit pub(crate) documents that this is not part of the public API even though the parent module is already private"
)]
pub(crate) fn build_function_identity_map(
    path: &str,
    fn_map: &BTreeMap<String, FnEntry>,
) -> BTreeMap<String, FunctionIdentity> {
    fn_map
        .iter()
        .map(|(key, entry)| {
            (
                key.clone(),
                FunctionIdentity {
                    id: function_identity_id(path, entry),
                    name: entry.name.clone(),
                    path: path.to_string(),
                    decl: entry.decl.clone(),
                    loc: entry.loc.clone(),
                },
            )
        })
        .collect()
}

/// Compute the `fallow:fn:<hex>` id for a single `(path, FnEntry)` pair.
/// Split out from [`build_function_identity_map`] so the collision-resistance
/// invariant can be unit-tested in isolation without round-tripping through
/// the full instrumenter.
fn function_identity_id(path: &str, entry: &FnEntry) -> String {
    // Each part is its own JSON value, so escaping handles every
    // ambiguous character (delimiters, quotes, NUL, unicode). The
    // .expect is documentary: every input here is a JSON-serializable
    // primitive (string / u32), so serde_json::to_string cannot fail.
    let input = serde_json::to_string(&serde_json::json!([
        path,
        entry.name,
        entry.decl.start.line,
        entry.decl.start.column,
        entry.decl.end.line,
        entry.decl.end.column,
        entry.loc.start.line,
        entry.loc.start.column,
        entry.loc.end.line,
        entry.loc.end.column,
    ]))
    .expect("FunctionIdentity hash input serializes to JSON infallibly");
    format!("fallow:fn:{}", djb31_hex(&input))
}

#[cfg(test)]
mod tests {
    use super::function_identity_id;
    use oxc_coverage_types::{FnEntry, Location, Position};

    fn entry(name: &str) -> FnEntry {
        let pos = |line, column| Position { line, column };
        FnEntry {
            name: name.to_string(),
            line: 1,
            decl: Location { start: pos(1, 0), end: pos(1, 10) },
            loc: Location { start: pos(1, 0), end: pos(1, 10) },
        }
    }

    /// Regression test for the JSON-encoded hash input. A naive
    /// `format!("{path}|{name}|{lines...}")` shape collides on
    /// `(path="a", name="b|c")` vs `(path="a|b", name="c")` because both
    /// produce the same flat string `"a|b|c|..."`. The JSON-encoded form
    /// quotes each string so the field boundary survives any character.
    #[test]
    fn function_identity_id_is_collision_resistant_against_pipe_in_name() {
        let a = function_identity_id("a", &entry("b|c"));
        let b = function_identity_id("a|b", &entry("c"));
        assert_ne!(
            a, b,
            "JSON-encoded hash input must keep (path, name) boundaries unambiguous; \
             got collision: {a}",
        );
    }
}
