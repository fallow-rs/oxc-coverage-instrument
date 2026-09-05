//! Exercises the `Format` runtime dispatcher: name parsing, single- versus
//! multi-file routing, the error each entry point returns when a caller mixes
//! the two up, and that dispatching a format renders exactly what its own
//! module writer does. The reporters' own output is pinned by the snapshot
//! tests.

mod common;

use common::TWO_FILE_MAP;
use oxc_coverage_report::summarize;
use oxc_coverage_reports::html::HtmlOptions;
use oxc_coverage_reports::{Format, json_summary, lcov, text, text_summary};
use oxc_coverage_types::{parse_coverage_map, parse_coverage_map_validated};
use std::path::Path;

const EMPTY_MAP: &str =
    r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;

const NULL_INNER_PATH_MAP: &str = r#"{"canonical.js":{"path":null,"statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":33}}},"fnMap":{},"branchMap":{},"s":{"0":1},"f":{},"b":{}}}"#;

fn reject_directory_write(format: Format, map: &oxc_coverage_report::CoverageMap, dir: &Path) {
    let error = format
        .write_to_dir(map, Path::new(""), dir, &HtmlOptions::default())
        .expect_err("single-file formats must reject write_to_dir");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn parse_recognises_every_format_name() {
    assert_eq!(Format::parse("text"), Some(Format::Text));
    assert_eq!(Format::parse("text-summary"), Some(Format::TextSummary));
    assert_eq!(Format::parse("json-summary"), Some(Format::JsonSummary));
    assert_eq!(Format::parse("lcov"), Some(Format::Lcov));
    assert_eq!(Format::parse("cobertura"), Some(Format::Cobertura));
    #[cfg(feature = "html")]
    assert_eq!(Format::parse("html"), Some(Format::Html));

    // Unknown spellings must return `None`; the CLI relies on this to surface
    // a user-facing error rather than silently picking a default format.
    for name in ["yaml", "xml", "clover", "", "TEXT"] {
        assert_eq!(Format::parse(name), None, "{name:?} is not a format name");
    }
    // Without the feature the reporter is absent, so its name must not parse.
    #[cfg(not(feature = "html"))]
    assert_eq!(Format::parse("html"), None);
}

#[test]
fn dispatch_matches_module_writers() {
    let map = parse_coverage_map(TWO_FILE_MAP).unwrap();
    let root = summarize(&map);

    let mut a = Vec::new();
    text::write(&root, &mut a).unwrap();
    let mut b = Vec::new();
    Format::Text.write(&root, Path::new(""), &mut b).unwrap();
    assert_eq!(a, b);

    let mut a = Vec::new();
    text_summary::write(&root, &mut a).unwrap();
    let mut b = Vec::new();
    Format::TextSummary.write(&root, Path::new(""), &mut b).unwrap();
    assert_eq!(a, b);

    let mut a = Vec::new();
    json_summary::write(&root, &mut a).unwrap();
    let mut b = Vec::new();
    Format::JsonSummary.write(&root, Path::new(""), &mut b).unwrap();
    assert_eq!(a, b);

    let mut a = Vec::new();
    lcov::write(&root, Path::new(""), &mut a).unwrap();
    let mut b = Vec::new();
    Format::Lcov.write(&root, Path::new(""), &mut b).unwrap();
    assert_eq!(a, b);
}

#[test]
fn validated_path_identity_reaches_every_report_surface() {
    let map = parse_coverage_map_validated(NULL_INNER_PATH_MAP).unwrap();
    assert_eq!(map["canonical.js"].path, "canonical.js");

    let root = summarize(&map);
    let leaf = &root.children()[0];
    assert_eq!(leaf.relative_path, "canonical.js");

    let mut output = Vec::new();
    Format::JsonSummary.write(&root, Path::new(""), &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains(r#""canonical.js""#));

    let mut output = Vec::new();
    Format::Lcov.write(&root, Path::new(""), &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("SF:canonical.js\n"));

    let mut output = Vec::new();
    Format::Cobertura.write(&root, Path::new(""), &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("filename=\"canonical.js\""));

    #[cfg(feature = "html")]
    {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("canonical.js"), "globalThis.canonicalMarker = 1;\n")
            .unwrap();
        let output_dir = dir.path().join("html");
        Format::Html.write_to_dir(&map, dir.path(), &output_dir, &HtmlOptions::default()).unwrap();
        let detail = std::fs::read_to_string(output_dir.join("canonical.js.html")).unwrap();
        assert!(detail.contains("canonical.js"));
        assert!(detail.contains("canonicalMarker"));
    }
}

#[cfg(feature = "html")]
#[test]
fn write_rejects_html_as_single_stream() {
    let map = parse_coverage_map(EMPTY_MAP).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    let err = Format::Html
        .write(&root, Path::new(""), &mut buf)
        .expect_err("html cannot stream into a single Write");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(buf.is_empty(), "no bytes should leak before the routing check");
}

#[test]
fn write_to_dir_rejects_single_file_formats() {
    let map = parse_coverage_map(EMPTY_MAP).unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    for fmt in
        [Format::Text, Format::TextSummary, Format::JsonSummary, Format::Lcov, Format::Cobertura]
    {
        reject_directory_write(fmt, &map, dir.path());
    }
}

#[cfg(feature = "html")]
#[test]
fn write_to_dir_routes_html_through_emitter() {
    let map = parse_coverage_map(EMPTY_MAP).unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    Format::Html.write_to_dir(&map, Path::new(""), dir.path(), &HtmlOptions::default()).unwrap();
    assert!(
        dir.path().join("index.html").exists(),
        "html dispatch must materialise the root index page",
    );
}

#[cfg(feature = "html")]
#[test]
fn write_to_dir_threads_options_into_html() {
    let map = parse_coverage_map(EMPTY_MAP).unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    // A non-default threshold lands in the index sentence, so an override
    // dropped on the way through would show the default 80% instead.
    let opts = HtmlOptions::new(55.0).unwrap();
    Format::Html.write_to_dir(&map, Path::new(""), dir.path(), &opts).unwrap();
    let index = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
    assert!(index.contains("55%"), "custom HtmlOptions threshold must reach the renderer");
}

#[test]
fn html_options_rejects_invalid_thresholds() {
    for value in [f64::NAN, f64::INFINITY, -1.0, 101.0] {
        assert!(HtmlOptions::new(value).is_err(), "should reject {value}");
    }
}
