//! `PositionRemapper`, the eager instrument-time keep/drop predicate.

use oxc_coverage_source_maps::PositionRemapper;

use crate::fixtures::loc;

#[test]
fn location_maps_matches_deferred_get_mapping_keep_decision() {
    // `AAAA;EACA`: gen(1,0)->orig(1,0) ; gen(2,2)->orig(2,0). Line 2's only
    // mapping sits at column 2, AFTER a statement that starts at column 0.
    let input_sm = r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["const a = 1;\nconst b = 2;\n"],"mappings":"AAAA;EACA","names":[]}"#;
    let remapper = PositionRemapper::from_json(input_sm).expect("usable map");

    // Line 1 maps cleanly at column 0.
    assert!(remapper.location_maps(&loc(1, 0, 1, 4)), "line-1 statement maps");

    // The line-2 statement at column 0 has no mapping at-or-before it on its
    // line (the only line-2 mapping is at column 2), but `getMapping`'s
    // least-upper-bound fallback resolves it forward and keeps it. The deferred
    // `drop_unmapped` path keeps it, so the eager gate must too.
    assert!(
        remapper.location_maps(&loc(2, 0, 2, 4)),
        "col-0 statement whose line maps at col 2 is kept (LUB fallback)",
    );

    // Line 3 has no mapping at all (the map only covers generated lines 1-2):
    // both GLB and LUB miss, so the span is genuinely unmapped and dropped.
    assert!(
        !remapper.location_maps(&loc(3, 0, 3, 4)),
        "a span on a generated line with no mappings is dropped",
    );

    // The `line == 0` "unknown" sentinel is kept as a no-op, matching the
    // direct-lookup route the remap path sends it down.
    assert!(remapper.location_maps(&loc(0, 0, 0, 0)), "line-0 sentinel is a keep no-op");
}
