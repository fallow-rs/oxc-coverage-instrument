//! Crate-internal builder that converts the AST-traversal counter Vecs into
//! the Istanbul-shaped `FileCoverage`.
//!
//! Lives on the instrument side (not in `oxc_coverage_types`) because the
//! input shape (sequential `Vec<Location>` etc. keyed by counter id) is an
//! implementation detail of the transform pass, not part of the public data
//! model.

use std::collections::BTreeMap;

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, FunctionIdentity, Location};

use sha2::{Digest, Sha256};

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
    let b_t = build_truthy_hit_map(&branch_map, &logical_branch_ids);

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

/// Build the optional truthy (`bT`) hit map: one zeroed hit Vec per logical
/// branch id, sized to that branch's surviving arm count. Returns `None` when
/// no logical branches were tracked (the common case, `report_logic` off) so
/// the field is omitted from the serialized map.
fn build_truthy_hit_map(
    branch_map: &BTreeMap<String, BranchEntry>,
    logical_branch_ids: &[usize],
) -> Option<BTreeMap<String, Vec<u32>>> {
    if logical_branch_ids.is_empty() {
        return None;
    }
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
}

/// Compute the optional `x_fallow_functionMap` overlay from a populated
/// `fn_map`. Each entry's id is `fallow:fn:<8 hex>` derived from
/// `SHA-256(path + name + start_line + "function")` truncated to the first
/// 4 bytes. The byte-equal formula lives in `fallow-cov-protocol` as
/// `function_identity_id`; matching it lets the overlay serve as a
/// cross-surface join key against V8 / Istanbul / source-mapped findings
/// in the fallow ecosystem (protocol invariant: "two producers observing
/// the same function with different positional fidelity MUST produce the
/// same `stable_id`").
///
/// `decl.start.line` is the canonical line input. A rename or moving the
/// function to a different line changes the id; a body edit on the same
/// line does not. Columns survive on the overlay's `decl` / `loc` fields
/// for display and same-line disambiguation, but are deliberately NOT
/// part of the hash so positional fidelity differences between producers
/// don't fork the join key.
///
/// The path enters the hash verbatim from the `filename` argument passed
/// to `instrument()`; consumers that need stable ids across tools must
/// normalise paths first (`./app.js`, `app.js`, and `/abs/repo/app.js`
/// all hash differently).
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

/// Compute the `fallow:fn:<8 hex>` id for a single `(path, FnEntry)` pair.
/// Bit-equal to `fallow_cov_protocol::function_identity_id(path, name,
/// start_line)`; see [`build_function_identity_map`] for the rationale.
fn function_identity_id(path: &str, entry: &FnEntry) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(entry.name.as_bytes());
    hasher.update(entry.decl.start.line.to_string().as_bytes());
    hasher.update(b"function");
    let digest = hasher.finalize();
    let mut hex = String::with_capacity("fallow:fn:".len() + 8);
    hex.push_str("fallow:fn:");
    for byte in &digest[..4] {
        // `{:02x}` keeps the leading zero on bytes < 0x10 so the suffix is
        // always exactly 8 hex chars; this is part of the protocol's wire
        // contract (`fallow-cov-protocol::function_identity_id` invariant).
        use std::fmt::Write;
        write!(&mut hex, "{byte:02x}").expect("writing to String is infallible");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::function_identity_id;
    use oxc_coverage_types::{FnEntry, Location, Position};
    use sha2::{Digest, Sha256};

    fn entry(name: &str, start_line: u32) -> FnEntry {
        let pos = |line, column| Position { line, column };
        FnEntry {
            name: name.to_string(),
            line: start_line,
            decl: Location { start: pos(start_line, 0), end: pos(start_line, 10) },
            loc: Location { start: pos(start_line, 0), end: pos(start_line, 10) },
        }
    }

    /// Reimplements `fallow_cov_protocol::function_identity_id` inline so
    /// we can assert byte-equality without taking a dep on the protocol
    /// crate (which would create an awkward cycle if the protocol ever
    /// grew a dep on oxc-coverage-types). If this helper drifts from
    /// fallow-cov-protocol's canonical formula, the cross-surface join
    /// silently breaks; update both sides together.
    fn protocol_function_identity_id(file: &str, name: &str, start_line: u32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file.as_bytes());
        hasher.update(name.as_bytes());
        hasher.update(start_line.to_string().as_bytes());
        hasher.update(b"function");
        let digest = hasher.finalize();
        let mut out = String::from("fallow:fn:");
        for byte in &digest[..4] {
            use std::fmt::Write;
            write!(&mut out, "{byte:02x}").unwrap();
        }
        out
    }

    /// Cross-protocol parity: the overlay's id MUST match what a consumer
    /// would compute via `fallow_cov_protocol::function_identity_id` from
    /// the public `(path, name, decl.start.line)` triple, otherwise the
    /// "cross-surface join key" promise in the rustdoc is a lie.
    #[test]
    fn function_identity_id_matches_fallow_cov_protocol() {
        for (path, name, line) in [
            ("src/app.ts", "handler", 1u32),
            ("src/app.ts", "handler", 42),
            ("a/b/c.ts", "(anonymous_0)", 7),
            ("computed.ts", "x|y", 3),
        ] {
            let ours = function_identity_id(path, &entry(name, line));
            let theirs = protocol_function_identity_id(path, name, line);
            assert_eq!(
                ours, theirs,
                "x_fallow_functionMap.id must match fallow-cov-protocol \
                 function_identity_id({path:?}, {name:?}, {line})",
            );
        }
    }

    /// Suffix shape: always `fallow:fn:` + 8 lowercase hex chars. The 8-char
    /// width is part of the wire contract: pinned by SHA-256-truncated-to-4-bytes
    /// in the protocol; renderers and length-sensitive tests downstream
    /// depend on it. Catches `format!("{}", u8)` (drops leading zero) or
    /// a hex-encoder swap that changes byte ordering.
    #[test]
    fn function_identity_id_has_fixed_8_hex_suffix() {
        let id = function_identity_id("src/app.ts", &entry("handler", 1));
        assert_eq!(id.len(), "fallow:fn:".len() + 8, "got {id:?}");
        assert!(id.starts_with("fallow:fn:"));
        let hex = &id["fallow:fn:".len()..];
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit() && (c.is_ascii_digit() || c.is_lowercase())),
            "suffix must be lowercase hex: {hex:?}",
        );
    }
}
