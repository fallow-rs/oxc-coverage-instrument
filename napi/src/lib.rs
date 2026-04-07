//! Node.js bindings for oxc-coverage-instrument.
//!
//! Exposes the `instrument` function to JavaScript via napi-rs.

// napi-derive generates code that triggers needless_pass_by_value
#![expect(
    clippy::needless_pass_by_value,
    reason = "napi function signatures require owned types"
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
///
/// 58x faster than istanbul-lib-instrument.
#[napi]
pub fn instrument(
    source: String,
    filename: String,
    options: Option<InstrumentOptions>,
) -> napi::Result<InstrumentResult> {
    let opts = options.map_or_else(oxc_coverage_instrument::InstrumentOptions::default, |o| {
        oxc_coverage_instrument::InstrumentOptions {
            coverage_variable: o
                .coverage_variable
                .unwrap_or_else(|| "__coverage__".to_string()),
            source_map: o.source_map.unwrap_or(false),
            input_source_map: o.input_source_map,
        }
    });

    let result = oxc_coverage_instrument::instrument(&source, &filename, &opts)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

    let coverage_map = serde_json::to_string(&result.coverage_map)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

    let unhandled_pragmas = result
        .unhandled_pragmas
        .into_iter()
        .map(|p| UnhandledPragma {
            comment: p.comment,
            line: p.line,
            column: p.column,
        })
        .collect();

    Ok(InstrumentResult {
        code: result.code,
        coverage_map,
        source_map: result.source_map,
        unhandled_pragmas,
    })
}
