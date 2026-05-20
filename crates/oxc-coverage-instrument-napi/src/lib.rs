//! Node.js bindings for oxc-coverage-instrument.
//!
//! Exposes the `instrument` function to JavaScript via napi-rs.

// napi-derive generates code that triggers needless_pass_by_value
#![expect(clippy::needless_pass_by_value, reason = "napi function signatures require owned types")]
// napi-derive lowers TS `Record<string, string>` to `HashMap<String, String>` with the default
// hasher; we can't add a generic hasher parameter because napi-derive does not propagate
// generics into the JS-facing function signature.
#![expect(
    clippy::implicit_hasher,
    reason = "napi-derive cannot lower a generic BuildHasher parameter into the napi function signature"
)]

use std::collections::HashMap;

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
    /// When true, run the TypeScript-strip pass before instrumentation.
    /// Set this when passing raw TypeScript source that has not been
    /// pre-transformed by Babel / tsc / esbuild. Defaults to false, which
    /// preserves backward compatibility with existing Vitest / nyc callers
    /// that supply already-transformed JavaScript. If false and you pass
    /// raw TypeScript, the output will contain TypeScript syntax and will
    /// not be executable as JavaScript (no error is returned).
    pub strip_typescript: Option<bool>,
    /// When true, lower TypeScript `experimentalDecorators` syntax
    /// (`@Injectable()` / `@Controller()` style used by NestJS, Angular,
    /// class-validator, TypeORM) into runtime `_decorate(...)`
    /// calls. Mirrors the `experimentalDecorators` flag in `tsconfig.json`.
    ///
    /// The instrumented output references imports from `@oxc-project/runtime` at
    /// runtime; install `@oxc-project/runtime` (or provide an equivalent shim).
    /// See the README for details and troubleshooting.
    ///
    /// Has no effect unless `stripTypescript` is also true. Defaults to false.
    pub experimental_decorators: Option<bool>,
    /// When true, emit TypeScript-style decorator metadata
    /// (`design:type`, `design:paramtypes`, `design:returntype`) alongside
    /// each decorated class / method / property. Required for NestJS
    /// dependency injection, TypeORM column type inference, and
    /// class-validator's metadata-driven validation. Mirrors the
    /// `emitDecoratorMetadata` flag in `tsconfig.json`.
    ///
    /// Setting this to true implicitly enables `experimentalDecorators`.
    /// The instrumented output requires `@oxc-project/runtime` at execution;
    /// see the README.
    ///
    /// Has no effect unless `stripTypescript` is also true. Defaults to false.
    pub emit_decorator_metadata: Option<bool>,
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
            strip_typescript: o.strip_typescript.unwrap_or(false),
            experimental_decorators: o.experimental_decorators.unwrap_or(false),
            emit_decorator_metadata: o.emit_decorator_metadata.unwrap_or(false),
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
/// the Vitest istanbul reporter path. For nyc's disk-read flow (Mode A
/// fallback) use [`remap_coverage_map_with_loader`] and supply a
/// `Record<string, string>` of preloaded maps keyed by FileCoverage path.
#[napi]
pub fn remap_coverage_map(coverage_json: String) -> napi::Result<String> {
    let parsed = oxc_coverage_instrument::parse_coverage_map(&coverage_json).map_err(|e| {
        napi::Error::new(napi::Status::InvalidArg, format!("invalid coverage JSON: {e}"))
    })?;
    let remapped = oxc_coverage_instrument::remap_coverage_map(&parsed);
    serde_json::to_string(&remapped)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}

/// Like [`remap_coverage_map`], but with a preloaded map dictionary used as
/// the Mode A disk-read fallback. Entries whose path is a key in
/// `source_maps` and which carry no embedded `inputSourceMap` use the
/// dictionary's value as the source map JSON. Each `source_maps` value must
/// be a valid source map JSON string; entries that fail to parse silently
/// pass through.
///
/// The dictionary form matches the practical Jest/nyc/istanbul JS workflow:
/// the caller has already read maps from disk (or another source) before
/// calling the converter. The richer Rust-side
/// [`oxc_coverage_instrument::SourceMapStore`] (Mode B continuous remap)
/// stays Rust-only until a Jest provider integration targets it directly.
#[napi]
pub fn remap_coverage_map_with_loader(
    coverage_json: String,
    source_maps: HashMap<String, String>,
) -> napi::Result<String> {
    let parsed = oxc_coverage_instrument::parse_coverage_map(&coverage_json).map_err(|e| {
        napi::Error::new(napi::Status::InvalidArg, format!("invalid coverage JSON: {e}"))
    })?;
    let remapped = oxc_coverage_instrument::remap_coverage_map_with_loader(&parsed, |path| {
        source_maps.get(path).cloned()
    });
    serde_json::to_string(&remapped)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}

/// Convert V8 byte-range coverage into Istanbul `FileCoverage` JSON.
///
/// `v8FunctionsJson` is the JSON array shape that the V8 inspector emits
/// under `Profiler.takePreciseCoverage().result[].functions` (the same shape
/// Node's `--experimental-coverage` and `@vitest/coverage-v8` consume).
///
/// `wrapperLength` accounts for Node's CJS module wrapper prefix and defaults
/// to 0 (correct for ESM).
///
/// Returns a JSON object compatible with Istanbul's `FileCoverage`. Statement,
/// function, and branch counts are populated from the V8 ranges. Branch arm
/// counts resolve correctly for if-else (arm\[0\] via the collected
/// consequent-body byte span, arm\[1\] via the alternate-body span) and switch
/// cases with `{ ... }` bodies. Branch arms with no matching V8 range
/// (ternary consequent/alternate, logical-expr right-hand operands, and
/// `default-arg` expressions) report `0`; this is honest under-reporting,
/// not over-reporting, so CI coverage thresholds will not silently pass on
/// un-instrumented arms.
///
/// When the source ends with a `//# sourceMappingURL=data:application/json;base64,...`
/// (or percent-encoded) trailer, the embedded map is decoded and attached
/// to the result as `inputSourceMap`. For external `//# sourceMappingURL=foo.js.map`
/// references, use [`v8_to_istanbul_with_loader`] and pass a dictionary of
/// URL -> map JSON entries.
///
/// If the returned object has `inputSourceMap` set, chain `remapCoverageMap`
/// next to resolve coverage positions back to the original source; otherwise
/// the inline map will ride along and downstream JS reporters that also call
/// into `istanbul-lib-source-maps` may double-remap.
#[napi]
pub fn v8_to_istanbul(
    source: String,
    filename: String,
    v8_functions_json: String,
    wrapper_length: Option<u32>,
) -> napi::Result<String> {
    let functions: Vec<oxc_coverage_instrument::V8FunctionCoverage> =
        serde_json::from_str(&v8_functions_json).map_err(|e| {
            napi::Error::new(napi::Status::InvalidArg, format!("invalid V8 functions JSON: {e}"))
        })?;
    let result = oxc_coverage_instrument::v8_to_istanbul(
        &source,
        &filename,
        &functions,
        wrapper_length.unwrap_or(0),
    )
    .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}

/// Like [`v8_to_istanbul`], but accepts a preloaded `external_source_maps`
/// dictionary used to resolve external `//# sourceMappingURL=` references.
/// The dictionary key is the URL as it appears in the source's trailing
/// comment (e.g. `foo.js.map`); the value is the map's JSON content. If the
/// source has an inline data-URL map the dictionary is not consulted.
/// Entries whose value fails to parse silently leave `inputSourceMap` unset.
#[napi]
pub fn v8_to_istanbul_with_loader(
    source: String,
    filename: String,
    v8_functions_json: String,
    external_source_maps: HashMap<String, String>,
    wrapper_length: Option<u32>,
) -> napi::Result<String> {
    let functions: Vec<oxc_coverage_instrument::V8FunctionCoverage> =
        serde_json::from_str(&v8_functions_json).map_err(|e| {
            napi::Error::new(napi::Status::InvalidArg, format!("invalid V8 functions JSON: {e}"))
        })?;
    let result = oxc_coverage_instrument::v8_to_istanbul_with_loader(
        &source,
        &filename,
        &functions,
        wrapper_length.unwrap_or(0),
        |url| external_source_maps.get(url).cloned(),
    )
    .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}
