//! Istanbul `getMapping` range semantics: starts resolve with
//! greatest-lower-bound, ends resolve to the next original segment or the end
//! of the original line.

use oxc_coverage_source_maps::remap_coverage;
use oxc_coverage_types::FileCoverage;
use srcmap_generator::SourceMapGenerator;

use crate::fixtures::{SRC_PATH, loc, single_statement_coverage, two_segment_map};

/// Original line the [`line_start_map`] fixture carries in `sourcesContent`,
/// 25 UTF-16 units long.
const CONTENT_LINE: &str = "const value = 1234567890;";

/// A map whose generated line `n` maps to column 0 of original line `n`, so
/// every statement end resolves through the end-of-line clamp rather than a
/// following segment.
fn line_start_map(lines: u32, include_sources_content: bool) -> serde_json::Value {
    let mut generator = SourceMapGenerator::new(Some("intermediate.js".to_string()));
    let source = generator.add_source(SRC_PATH);
    if include_sources_content {
        generator.set_source_content(source, vec![CONTENT_LINE; lines as usize].join("\n"));
    }
    for line in 0..lines {
        generator.add_mapping(line, 0, source, line, 0);
    }
    serde_json::from_str(&generator.to_json()).expect("generated source map is valid")
}

/// One statement per generated line, keyed by its zero-based line index.
fn line_per_statement_coverage(lines: u32, map: serde_json::Value) -> FileCoverage {
    let mut coverage = single_statement_coverage(map, 1, 0, 1, 3);
    for line in 1..lines {
        coverage.statement_map.insert(line.to_string(), loc(line + 1, 0, line + 1, 3));
        coverage.s.insert(line.to_string(), 1);
    }
    coverage
}

#[test]
fn get_mapping_widens_end_that_falls_between_segments() {
    // Generated end col 4 sits between segment starts 0 and 6. A direct lookup
    // snaps it backward to the previous segment (orig col 0); getMapping widens
    // it to the next original segment (orig col 10).
    let fc = single_statement_coverage(two_segment_map(true), 1, 0, 1, 4);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 0), "start resolves to orig (1,0)");
    assert_eq!(
        (loc.end.line, loc.end.column),
        (1, 10),
        "end widens to the next original segment, not the truncated previous one",
    );
}

#[test]
fn get_mapping_balloons_one_char_span_to_enclosing_segment() {
    // A 1-char generated span (col 6..7) on the second segment. getMapping
    // resolves the start with greatest-lower-bound (orig col 10) and the end to
    // end-of-line (no segment follows), ballooning the marker to its enclosing
    // span rather than leaving a 1-char span.
    let fc = single_statement_coverage(two_segment_map(true), 1, 6, 1, 7);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 10), "start snaps to enclosing segment");
    assert_eq!(loc.end.column, 32, "end balloons to the original line's UTF-16 length");
}

#[test]
fn get_mapping_clamps_end_of_line_from_sources_content() {
    // End col 20 is past the last segment with nothing after it: istanbul returns
    // column Infinity, which clamps to the original line's UTF-16 length (32).
    let fc = single_statement_coverage(two_segment_map(true), 1, 6, 1, 20);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 10));
    assert_eq!(loc.end.column, 32, "Infinity end clamps to the line's UTF-16 length");
}

#[test]
fn get_mapping_end_of_line_without_content_falls_back_to_last_segment() {
    // Same map without sourcesContent: the Infinity end clamps to the rightmost
    // original column mapped on the line (10) instead of the true line length.
    let fc = single_statement_coverage(two_segment_map(false), 1, 0, 1, 20);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 0));
    assert_eq!(
        loc.end.column, 10,
        "without sourcesContent the end clamps to the last mapped original column",
    );
}

#[test]
fn get_mapping_mappings_only_handles_sparse_huge_original_line() {
    let mut generator = SourceMapGenerator::new(Some("intermediate.js".to_string()));
    let source = generator.add_source(SRC_PATH);
    generator.add_mapping(0, 0, source, u32::MAX - 1, 17);
    let map = serde_json::from_str(&generator.to_json()).expect("generated source map is valid");
    let fc = single_statement_coverage(map, 1, 0, 1, 2);

    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!(loc.end.column, 17, "sparse mappings retain their greatest original column");
}

#[test]
fn get_mapping_short_sources_content_falls_back_to_last_segment() {
    let mut generator = SourceMapGenerator::new(Some("intermediate.js".to_string()));
    let source = generator.add_source(SRC_PATH);
    generator.set_source_content(source, "only the first line".to_string());
    generator.add_mapping(0, 0, source, 1, 23);
    let map = serde_json::from_str(&generator.to_json()).expect("generated source map is valid");
    let fc = single_statement_coverage(map, 1, 0, 1, 2);

    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!(
        loc.end.column, 23,
        "a missing content line falls back to its greatest mapped original column"
    );
}

#[test]
fn get_mapping_degenerate_span_recomputes_end() {
    // A map whose generated line maps gen(0,0)->orig(0,0) and gen(0,2)->orig(0,5).
    // A backwards generated span (start col 2, end col 1) resolves to start
    // (1,5) and a Mapped end that also lands on (1,5): the degenerate-span guard
    // recomputes the end via LUB(genEnd) then steps one column left (col 4).
    let map: serde_json::Value = serde_json::from_str(&format!(
        r#"{{"version":3,"sources":["{SRC_PATH}"],"mappings":"AAAA,EAAK","names":[],"sourcesContent":["abcdefghij\n"]}}"#,
    ))
    .unwrap();
    let fc = single_statement_coverage(map, 1, 2, 1, 1);
    let remapped = remap_coverage(&fc).expect("remap succeeds");
    let loc = &remapped.statement_map["0"];
    assert_eq!((loc.start.line, loc.start.column), (1, 5), "start resolves to orig (1,5)");
    assert_eq!(
        (loc.end.line, loc.end.column),
        (1, 4),
        "degenerate guard recomputes end as LUB(genEnd) column minus one",
    );
}

#[test]
fn end_of_line_clamp_resolves_every_line_from_sources_content() {
    let lines = 4;
    let coverage = line_per_statement_coverage(lines, line_start_map(lines, true));
    let remapped = remap_coverage(&coverage).expect("remap succeeds");

    for line in 0..lines {
        let statement = &remapped.statement_map[&line.to_string()];
        assert_eq!(
            (statement.end.line, statement.end.column),
            (line + 1, 25),
            "every line clamps to the UTF-16 length of its `sourcesContent` line",
        );
    }
}

#[test]
fn end_of_line_clamp_without_sources_content_uses_the_last_mapped_column() {
    let lines = 4;
    let coverage = line_per_statement_coverage(lines, line_start_map(lines, false));
    let remapped = remap_coverage(&coverage).expect("remap succeeds");

    for line in 0..lines {
        let statement = &remapped.statement_map[&line.to_string()];
        assert_eq!(
            (statement.end.line, statement.end.column),
            (line + 1, 0),
            "the only mapped original column on each line is 0",
        );
    }
}
