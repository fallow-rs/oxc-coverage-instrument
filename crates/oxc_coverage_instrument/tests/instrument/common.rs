//! Assertions shared by the fixture-driven integration tests.

use oxc_allocator::Allocator;
use oxc_coverage_instrument::InstrumentResult;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Assert that the coverage map is internally consistent: one hit counter per
/// map entry, one hit slot per branch arm, every counter still at zero, and a
/// clean JSON round-trip carrying the Istanbul field set.
pub fn assert_coverage_map_well_formed(result: &InstrumentResult, filename: &str) {
    let coverage = &result.coverage_map;

    assert!(!result.code.is_empty(), "{filename}: instrumented code is empty");
    assert_eq!(coverage.path, filename, "{filename}: path mismatch");

    assert_eq!(coverage.s.len(), coverage.statement_map.len(), "{filename}: s/statementMap sizes");
    assert_eq!(coverage.f.len(), coverage.fn_map.len(), "{filename}: f/fnMap sizes");
    assert_eq!(coverage.b.len(), coverage.branch_map.len(), "{filename}: b/branchMap sizes");

    for (id, entry) in &coverage.branch_map {
        assert_eq!(
            coverage.b[id].len(),
            entry.locations.len(),
            "{filename}: branch {id} has one hit slot per arm"
        );
    }

    for (id, count) in &coverage.s {
        assert_eq!(*count, 0, "{filename}: s[{id}] must start at 0");
    }
    for (id, count) in &coverage.f {
        assert_eq!(*count, 0, "{filename}: f[{id}] must start at 0");
    }

    let json = serde_json::to_string(coverage).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["path"], filename, "{filename}: path mismatch in JSON");
    assert!(parsed["statementMap"].is_object(), "{filename}: missing statementMap");
    assert!(parsed["fnMap"].is_object(), "{filename}: missing fnMap");
    assert!(parsed["branchMap"].is_object(), "{filename}: missing branchMap");
}

/// Assert that `code` parses cleanly under the source type `filename` implies.
///
/// # Panics
///
/// Panics if `filename` has no extension `oxc_span` recognises.
pub fn assert_reparses(code: &str, filename: &str) {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| panic!("{filename}: unable to determine source type from path"));
    let parsed = Parser::new(&allocator, code, source_type).parse();
    let diagnostics =
        parsed.diagnostics.errors().map(|error| format!("{error}")).collect::<Vec<_>>();
    assert!(
        diagnostics.is_empty(),
        "{filename}: instrumented code has parse diagnostics: {}",
        diagnostics.join("; ")
    );
}
