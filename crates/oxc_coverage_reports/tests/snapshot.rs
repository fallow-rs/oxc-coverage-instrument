//! End-to-end snapshot fixtures for every reporter on a shared two-file map.
//!
//! Each snapshot locks the byte-for-byte rendering. Updating an output format
//! intentionally requires accepting the snapshot via `cargo insta review`.

mod common;

use common::TWO_FILE_MAP;
use oxc_coverage_report::summarize;
use oxc_coverage_reports::{Format, cobertura, json_summary, lcov};
use oxc_coverage_types::parse_coverage_map;
use std::path::Path;

fn render(format: Format) -> String {
    let map = parse_coverage_map(TWO_FILE_MAP).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    format.write(&root, Path::new(""), &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn text_snapshot() {
    let out = render(Format::Text);
    insta::assert_snapshot!(out);
}

#[test]
fn text_summary_snapshot() {
    let out = render(Format::TextSummary);
    insta::assert_snapshot!(out);
}

#[test]
fn json_summary_snapshot() {
    let map = parse_coverage_map(TWO_FILE_MAP).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    json_summary::write_pretty(&root, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    insta::assert_snapshot!(out);
}

#[test]
fn lcov_snapshot() {
    let map = parse_coverage_map(TWO_FILE_MAP).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    lcov::write(&root, Path::new(""), &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    insta::assert_snapshot!(out);
}

#[test]
fn cobertura_snapshot() {
    let map = parse_coverage_map(TWO_FILE_MAP).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    // `cobertura::write` stamps `SystemTime::now()`; pinning the timestamp
    // keeps the snapshot deterministic.
    cobertura::write_with_timestamp(&root, Path::new(""), 0, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    insta::assert_snapshot!(out);
}
