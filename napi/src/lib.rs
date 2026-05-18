//! Node.js bindings for oxc-coverage-instrument.
//!
//! Exposes the `instrument` function to JavaScript via napi-rs.

// napi-derive generates code that triggers needless_pass_by_value
#![expect(clippy::needless_pass_by_value, reason = "napi function signatures require owned types")]

use napi_derive::napi;

/// Options for the instrument function.
#[napi(object)]
pub struct InstrumentOptions {
    /// Name of the global coverage variable (default: "__coverage__").
    pub coverage_variable: Option<String>,
    /// Whether to generate a source map for the instrumented output.
    pub source_map: Option<bool>,
    /// Input source map JSON string from a prior transformation.
    pub input_source_map: Option<String>,
    /// When true, adds truthy-value tracking (bT) for logical expression operands.
    pub report_logic: Option<bool>,
    /// Class method names to exclude from coverage instrumentation.
    pub ignore_class_methods: Option<Vec<String>>,
}

/// A coverage pragma comment that was found but not handled.
#[napi(object)]
pub struct UnhandledPragma {
    /// The full comment text.
    pub comment: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based column.
    pub column: u32,
}

/// Result of instrumenting a source file.
#[napi(object)]
pub struct InstrumentResult {
    /// The instrumented source code with coverage counters injected.
    pub code: String,
    /// Istanbul-compatible coverage map as a JSON string.
    /// Parse with `JSON.parse()` to get the coverage object.
    pub coverage_map: String,
    /// Output source map JSON string (only present if source_map option is true).
    pub source_map: Option<String>,
    /// Unhandled pragma comments found during instrumentation.
    pub unhandled_pragmas: Vec<UnhandledPragma>,
}

/// Instrument a JavaScript/TypeScript source file for coverage collection.
///
/// Parses the source with Oxc, injects Istanbul-compatible coverage counters
/// via AST mutation, and returns the instrumented code with a coverage map.
#[napi]
pub fn instrument(
    source: String,
    filename: String,
    options: Option<InstrumentOptions>,
) -> napi::Result<InstrumentResult> {
    let opts = options.map_or_else(oxc_coverage_instrument::InstrumentOptions::default, |o| {
        oxc_coverage_instrument::InstrumentOptions {
            coverage_variable: o.coverage_variable.unwrap_or_else(|| "__coverage__".to_string()),
            source_map: o.source_map.unwrap_or(false),
            input_source_map: o.input_source_map,
            report_logic: o.report_logic.unwrap_or(false),
            ignore_class_methods: o.ignore_class_methods.unwrap_or_default(),
        }
    });

    let result = oxc_coverage_instrument::instrument(&source, &filename, &opts)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

    let unhandled_pragmas = result
        .unhandled_pragmas
        .into_iter()
        .map(|p| UnhandledPragma { comment: p.comment, line: p.line, column: p.column })
        .collect();

    Ok(InstrumentResult {
        code: result.code,
        coverage_map: result.coverage_map_json,
        source_map: result.source_map,
        unhandled_pragmas,
    })
}

/// Remap a coverage-final.json-shaped JSON string through each entry's
/// embedded `inputSourceMap` (typically attached during instrumentation).
///
/// Entries without an `inputSourceMap` are returned unchanged under their
/// original key. Entries with one are walked through the map and re-keyed by
/// the original source path (with `sourceRoot` joined per
/// `istanbul-lib-source-maps` semantics). Returns the remapped JSON.
///
/// Equivalent to `createSourceMapStore().transformCoverage(coverageMap)` in
/// the Vitest istanbul reporter path, minus the disk-read fallback used by
/// nyc's CLI mode.
#[napi]
pub fn remap_coverage_map(coverage_json: String) -> napi::Result<String> {
    let parsed = oxc_coverage_instrument::parse_coverage_map(&coverage_json)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;
    let remapped = oxc_coverage_instrument::remap_coverage_map(&parsed);
    serde_json::to_string(&remapped)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}
