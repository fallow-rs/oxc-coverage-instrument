//! The Istanbul merge invariant: an `s` / `f` / `b` / `bT` key with no
//! location-map entry is dropped on every remap path.
//!
//! Coverage can reach the remap pipeline already carrying such an orphan
//! counter. An upstream instrumenter that emitted `++cov.s[id]` for a slot
//! later pruned from the map computes `undefined + 1 = NaN` at runtime, which
//! serializes as `null` and (via `deserialize_null_as_zero_map`) reappears as
//! an orphan `s` key. Passing it through unchanged crashes
//! `istanbul-lib-coverage`'s `CoverageMap.merge`.

use std::collections::BTreeMap;

use oxc_coverage_source_maps::{remap_coverage, remap_coverage_map_with_loader};

use crate::fixtures::{full_shape_file_coverage, identity_three_line_map};

#[test]
fn remap_drops_preexisting_orphan_statement_counter() {
    // Mapped statement "0" survives; orphan "1" has an `s` slot but no
    // statementMap entry. The embedded identity map drives a no-drop remap, so
    // nothing here is pruned by mapping failure: only the invariant pass can
    // remove the orphan.
    let mut fc = full_shape_file_coverage(Some(identity_three_line_map(None)));
    fc.s.insert("1".to_string(), 0);
    assert!(!fc.statement_map.contains_key("1"), "precondition: orphan has no statementMap entry");

    let remapped = remap_coverage(&fc).expect("identity remap succeeds");
    assert!(
        !remapped.s.contains_key("1"),
        "orphan `s` counter must be dropped so istanbul-lib-coverage can merge the result",
    );
    for key in remapped.s.keys() {
        assert!(
            remapped.statement_map.contains_key(key),
            "every surviving `s` key must have a statementMap entry (key {key})",
        );
    }
}

#[test]
fn passthrough_entry_without_map_still_drops_orphan_counter() {
    // An already-composed entry has no embedded `inputSourceMap`, so the
    // map-level remap leaves it under its original key (the `None` branch). It
    // must still be reconciled: a runtime orphan here would otherwise survive a
    // `remapCoverageMap` round-trip untouched.
    let mut fc = full_shape_file_coverage(None);
    fc.f.insert("9".to_string(), 0); // orphan function counter, no fnMap["9"]
    let mut coverage_map = BTreeMap::new();
    coverage_map.insert(fc.path.clone(), fc);

    let out = remap_coverage_map_with_loader(&coverage_map, |_| None);
    let entry = out.get("intermediate.js").expect("entry passes through under original key");
    assert!(!entry.f.contains_key("9"), "orphan `f` counter dropped on passthrough");
    for key in entry.f.keys() {
        assert!(entry.fn_map.contains_key(key), "every surviving `f` key has an fnMap entry");
    }
}

#[test]
fn prune_orphan_counters_removes_every_orphan_section() {
    // `statementMap` is missing "1" while `s["1"]` is present, serialized as
    // `null` and coerced to 0 on ingest. The same rule covers f/fnMap,
    // b/branchMap and bT/branchMap orphans; the `x_fallow_functionMap` overlay
    // is keyed by fn id and tracks fnMap, so an orphan overlay entry drops too.
    let json = r#"{
        "path": "/x/mod.ts",
        "statementMap": {"0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 12}}},
        "fnMap": {},
        "branchMap": {},
        "s": {"0": 1, "1": null},
        "f": {"2": 3},
        "b": {"4": [1, 0]},
        "bT": {"5": [0]},
        "x_fallow_functionMap": {"2": {"id": "fallow:fn:00000000", "name": "g", "path": "/x/mod.ts", "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 9}}}}
    }"#;
    let mut fc =
        oxc_coverage_types::FileCoverage::from_json(json).expect("parses with null-as-zero");
    assert_eq!(fc.s.get("1"), Some(&0), "null `s` value ingests as 0 before reconciliation");

    let removed = fc.prune_orphan_counters();
    assert_eq!(
        removed, 5,
        "all five orphan entries removed: counters s.1, f.2, b.4, bT.5 plus the overlay entry 2",
    );
    assert_eq!(fc.s.keys().collect::<Vec<_>>(), vec!["0"], "only the mapped statement survives");
    assert!(fc.f.is_empty() && fc.b.is_empty(), "orphan function/branch counters removed");
    assert!(fc.b_t.as_ref().is_none_or(BTreeMap::is_empty), "orphan bT counter removed");
    assert!(
        fc.x_fallow_function_map.as_ref().is_none_or(BTreeMap::is_empty),
        "orphan overlay entry (fn id absent from fnMap) is pruned with its counter",
    );
}
