//! Merging `FileCoverage` entries by remapped location: used both to
//! canonicalize one file's metadata ids and to fold entries that land on the
//! same original path.

use std::collections::BTreeMap;

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, FunctionIdentity};

use crate::context::LocationKey;

/// Visit `map` in ascending numeric order of its ids, then string order for ids
/// that do not parse as integers. This reproduces the object-key enumeration
/// order `istanbul-lib-source-maps` and `istanbul-lib-coverage` assign output
/// ids in: integer-like keys first, ascending. JavaScript enumerates the
/// remaining keys in insertion order, which a `BTreeMap` no longer carries, so
/// those fall back to string order here. Our own ids are always decimal, but
/// ingested JSON may carry non-numeric keys.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn numeric_id_order<T>(map: &BTreeMap<String, T>) -> Vec<(&String, &T)> {
    let mut entries: Vec<(&String, &T)> = map.iter().collect();
    entries.sort_by(|(left, _), (right, _)| id_order_key(left).cmp(&id_order_key(right)));
    entries
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum IdOrderKey<'a> {
    Numeric(u64),
    Text(&'a str),
}

fn id_order_key(id: &str) -> IdOrderKey<'_> {
    id.parse::<u64>().map_or(IdOrderKey::Text(id), IdOrderKey::Numeric)
}

/// An empty `FileCoverage` at `path` carrying the optional `bT` and
/// `x_fallow_functionMap` sections exactly when `template` carries them.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn empty_file_coverage(template: &FileCoverage, path: String) -> FileCoverage {
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

/// Insert `incoming` under its own path, folding it into an entry already at
/// that path.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn insert_or_merge_coverage(
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

/// Fold `incoming` into `existing`, matching entries by remapped location and
/// summing their counters.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn merge_file_coverage(existing: &mut FileCoverage, incoming: &FileCoverage) {
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
    for (incoming_id, location) in numeric_id_order(&incoming.statement_map) {
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

    for (incoming_id, function) in numeric_id_order(&incoming.fn_map) {
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

    for (incoming_id, branch) in numeric_id_order(&incoming.branch_map) {
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

// The `id.to_string()` in both counter helpers is deferred to the insert arm:
// merging runs once per statement, function and branch of every file, and the
// key is already present for every entry after the first.
fn merge_scalar_counter(counters: &mut BTreeMap<String, u32>, id: &str, incoming: Option<u32>) {
    let Some(incoming) = incoming else {
        return;
    };
    if let Some(counter) = counters.get_mut(id) {
        *counter = counter.saturating_add(incoming);
    } else {
        counters.insert(id.to_string(), incoming);
    }
}

fn merge_vector_counter(
    counters: &mut BTreeMap<String, Vec<u32>>,
    id: &str,
    incoming: Option<&Vec<u32>>,
) {
    let Some(incoming) = incoming else {
        return;
    };
    let Some(current) = counters.get_mut(id) else {
        counters.insert(id.to_string(), incoming.clone());
        return;
    };
    if current.len() < incoming.len() {
        current.resize(incoming.len(), 0);
    }
    for (current, incoming) in current.iter_mut().zip(incoming) {
        *current = current.saturating_add(*incoming);
    }
}

fn function_identities_equal(left: &FunctionIdentity, right: &FunctionIdentity) -> bool {
    left.id == right.id
        && left.name == right.name
        && left.path == right.path
        && LocationKey::from(&left.decl) == LocationKey::from(&right.decl)
        && LocationKey::from(&left.loc) == LocationKey::from(&right.loc)
}
