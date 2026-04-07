//! Node.js bindings for oxc-coverage-instrument.
//!
//! Exposes the `instrument` function to JavaScript via napi-rs.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "napi crate may use print for debugging"
)]

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
}

/// Instrument a JavaScript/TypeScript source file for coverage collection.
///
/// Parses the source with Oxc, injects Istanbul-compatible coverage counters
/// via AST mutation, and returns the instrumented code with a coverage map.
///
/// 58x faster than istanbul-lib-instrument.
#[napi]
pub fn instrument(
    source: String,
    filename: String,
    options: Option<InstrumentOptions>,
) -> napi::Result<InstrumentResult> {
    let opts = options.map_or_else(
        oxc_coverage_instrument::InstrumentOptions::default,
        |o| oxc_coverage_instrument::InstrumentOptions {
            coverage_variable: o
                .coverage_variable
                .unwrap_or_else(|| "__coverage__".to_string()),
            source_map: o.source_map.unwrap_or(false),
            input_source_map: o.input_source_map,
        },
    );

    let result = oxc_coverage_instrument::instrument(&source, &filename, &opts)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

    let coverage_map = serde_json::to_string(&result.coverage_map)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

    Ok(InstrumentResult {
        code: result.code,
        coverage_map,
        source_map: result.source_map,
    })
}
