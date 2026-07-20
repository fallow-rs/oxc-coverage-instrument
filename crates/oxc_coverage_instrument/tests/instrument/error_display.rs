//! `Display` output of the crate's error types.
//!
//! One case per enum arm: these strings are the user-facing surface in CLI and
//! test-runner output, where a format change is not otherwise visible.

use oxc_coverage_instrument::{InstrumentError, V8ToIstanbulError};

#[test]
fn transform_error_display_format() {
    let err = InstrumentError::TransformError(vec![
        "unsupported syntax at line 5".to_string(),
        "missing semicolon at line 8".to_string(),
    ]);
    let rendered = format!("{err}");
    assert!(rendered.starts_with("transform error: "), "got: {rendered}");
    assert!(rendered.contains("unsupported syntax at line 5"), "got: {rendered}");
    assert!(rendered.contains("missing semicolon at line 8"), "got: {rendered}");
    assert!(rendered.contains("; "), "diagnostics must be joined with '; ', got: {rendered}");
}

#[test]
fn transform_error_single_diagnostic_displays_without_separator() {
    let err = InstrumentError::TransformError(vec!["only one diagnostic".to_string()]);
    let rendered = format!("{err}");
    assert_eq!(rendered, "transform error: only one diagnostic");
}

#[test]
fn v8_to_istanbul_error_display_includes_diagnostic() {
    let err = V8ToIstanbulError::Parse("unexpected token".to_string());
    let rendered = err.to_string();
    assert!(rendered.starts_with("parse error: "), "got: {rendered}");
    assert!(rendered.contains("unexpected token"), "got: {rendered}");

    // `std::error::Error` is implemented so callers can carry this through
    // their own error enums with `?`.
    let as_err: &dyn std::error::Error = &err;
    assert!(as_err.source().is_none());
}
