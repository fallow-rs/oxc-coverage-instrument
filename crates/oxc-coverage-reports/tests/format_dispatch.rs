//! Exercises the `Format` runtime dispatcher: name parsing, single- versus
//! multi-file routing, and the error paths each entry point returns when a
//! caller mixes them up. The snapshot tests pin each individual reporter's
//! output, but they bypass `Format` and call the per-module `write` directly;
//! these tests own the dispatcher contract instead.

#[cfg(feature = "html")]
use oxc_coverage_report::summarize;
use oxc_coverage_reports::Format;
#[cfg(feature = "html")]
use oxc_coverage_reports::html::HtmlOptions;
use oxc_coverage_types::parse_coverage_map;
use std::path::Path;

const EMPTY_MAP: &str =
    r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;

fn reject_directory_write(
    format: Format,
    map: &oxc_coverage_report::CoverageMap,
    dir: &Path,
) {
    #[cfg(feature = "html")]
    let error = format
        .write_to_dir(map, Path::new(""), dir, &HtmlOptions::default())
        .expect_err("single-file formats must reject write_to_dir");

    #[cfg(not(feature = "html"))]
    let error = format
        .write_to_dir(map, Path::new(""), dir)
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
    assert_eq!(Format::parse("yaml"), None);
    assert_eq!(Format::parse(""), None);
    assert_eq!(Format::parse("TEXT"), None);
}

#[test]
fn is_multi_file_is_true_only_for_html() {
    assert!(!Format::Text.is_multi_file());
    assert!(!Format::TextSummary.is_multi_file());
    assert!(!Format::JsonSummary.is_multi_file());
    assert!(!Format::Lcov.is_multi_file());
    assert!(!Format::Cobertura.is_multi_file());
    #[cfg(feature = "html")]
    assert!(Format::Html.is_multi_file());
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
    // A non-default threshold lands in the index sentence; if `Format::Html`
    // dropped the override and used the default 80% we would not see "55%".
    let opts = HtmlOptions::new(55.0).unwrap();
    Format::Html.write_to_dir(&map, Path::new(""), dir.path(), &opts).unwrap();
    let index = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
    assert!(index.contains("55%"), "custom HtmlOptions threshold must reach the renderer");
}

#[cfg(feature = "html")]
#[test]
fn html_options_rejects_invalid_thresholds() {
    for value in [f64::NAN, f64::INFINITY, -1.0, 101.0] {
        assert!(HtmlOptions::new(value).is_err(), "should reject {value}");
    }
}
