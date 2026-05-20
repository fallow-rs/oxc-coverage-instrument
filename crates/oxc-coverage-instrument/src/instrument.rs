//! Top-level instrumentation API.

use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer, TypeScriptOptions};
use oxc_traverse::traverse_mut;

use std::collections::BTreeMap;

use crate::coverage_builder::{CoverageMaps, build_file_coverage};
use crate::pragma::PragmaMap;
use crate::transform::{
    CoverageState, CoverageTransform, PreambleInputs, TransformInit, djb31_hex,
    generate_cov_fn_name, generate_preamble_source,
};
use oxc_coverage_types::{FileCoverage, UnhandledPragma};

/// Options for the `instrument` function.
#[derive(Debug, Clone)]
pub struct InstrumentOptions {
    /// Name of the global coverage variable (default: `"__coverage__"`).
    pub coverage_variable: String,
    /// Whether to generate a source map for the instrumented output.
    pub source_map: bool,
    /// Input source map JSON string from a prior transformation (e.g., TypeScript → JS).
    /// When provided, this is stored on the `FileCoverage` as `inputSourceMap` so
    /// downstream tools (nyc, istanbul-reports) can chain back to the original source.
    pub input_source_map: Option<String>,
    /// When true, adds truthy-value tracking (`bT`) for logical expression operands.
    /// This enables nyc-style logic coverage that tracks not just which branch was
    /// taken, but whether each operand evaluated to a truthy value.
    pub report_logic: bool,
    /// Class method names to exclude from coverage instrumentation.
    /// Matches Istanbul's `ignoreClassMethods` behavior for class methods and
    /// named function expressions with a matching id.
    pub ignore_class_methods: Vec<String>,
    /// When true, run `oxc_transformer`'s TypeScript-strip pass on the parsed
    /// AST before coverage instrumentation. Required when the caller passes
    /// raw TypeScript source that has not been pre-transformed by Babel /
    /// tsc / esbuild. Defaults to false so existing callers that supply
    /// pre-transformed JS are unaffected.
    pub strip_typescript: bool,
}

impl Default for InstrumentOptions {
    fn default() -> Self {
        Self {
            coverage_variable: "__coverage__".to_string(),
            source_map: false,
            input_source_map: None,
            report_logic: false,
            ignore_class_methods: Vec::new(),
            strip_typescript: false,
        }
    }
}

/// Result of instrumenting a source file.
#[derive(Debug)]
pub struct InstrumentResult {
    /// The instrumented source code with coverage counters injected.
    pub code: String,
    /// Istanbul-compatible coverage map for this file.
    pub coverage_map: FileCoverage,
    /// Pre-serialized JSON of `coverage_map`. Produced once internally for the
    /// preamble's `coverageData` literal and the hash guard, then exposed here
    /// so language bindings (napi-rs, etc.) and downstream JSON sinks can avoid
    /// a second serialization of the same `BTreeMap` tree.
    pub coverage_map_json: String,
    /// Output source map JSON string (only present if `InstrumentOptions::source_map` is true).
    pub source_map: Option<String>,
    /// Unhandled pragma comments found during instrumentation.
    /// Contains `/* istanbul ignore ... */` and `/* v8 ignore ... */` comments
    /// that were not processed. Callers should decide whether to warn or error.
    pub unhandled_pragmas: Vec<UnhandledPragma>,
}

/// Check whether a string is a valid JavaScript identifier (ASCII subset).
///
/// Returns `true` if the string is non-empty, starts with `[a-zA-Z_$]`,
/// and all remaining characters are `[a-zA-Z0-9_$]`.
fn is_valid_js_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Serialize a `FileCoverage` to JSON.
///
/// `FileCoverage` is composed of `BTreeMap`, `Vec`, `String`, and primitive
/// numbers, all with first-party serde implementations that cannot fail at
/// runtime. The .expect call documents this rather than threading a
/// never-produced error variant through the call chain.
fn serialize_coverage_map(coverage_map: &FileCoverage) -> String {
    serde_json::to_string(coverage_map).expect("FileCoverage serializes to JSON infallibly")
}

/// Instrument a JavaScript/TypeScript source file for coverage collection.
///
/// Parses the source with `oxc_parser`, collects statement/function/branch
/// locations via AST traversal, injects coverage counter expressions into
/// the AST, and emits the instrumented code via `oxc_codegen`.
///
/// # Errors
///
/// Returns an error if the source cannot be parsed.
///
/// # Example
///
/// ```
/// use oxc_coverage_instrument::{instrument, InstrumentOptions};
///
/// let source = "function add(a, b) { return a + b; }";
/// let result = instrument(source, "add.js", &InstrumentOptions::default()).unwrap();
///
/// // coverage_map contains fnMap, statementMap, branchMap
/// assert_eq!(result.coverage_map.fn_map.len(), 1);
/// assert_eq!(result.coverage_map.fn_map["0"].name, "add");
/// ```
pub fn instrument(
    source: &str,
    filename: &str,
    options: &InstrumentOptions,
) -> Result<InstrumentResult, InstrumentError> {
    if !is_valid_js_identifier(&options.coverage_variable) {
        return Err(InstrumentError::InvalidCoverageVariable(options.coverage_variable.clone()));
    }

    let allocator = Allocator::default();
    let mut parsed = parse_program(&allocator, source, filename)?;

    let (pragmas, unhandled_pragmas) = PragmaMap::from_program(&parsed.program, source);
    if pragmas.ignore_file {
        return Ok(empty_coverage_result(filename, source, unhandled_pragmas));
    }

    let mut scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();

    if options.strip_typescript {
        scoping = strip_typescript_pass(&allocator, filename, &mut parsed.program, scoping)?;
    }

    let cov_fn_name = generate_cov_fn_name(filename);

    let mut transform = CoverageTransform::new(TransformInit {
        allocator: &allocator,
        source,
        cov_fn_name: &cov_fn_name,
        report_logic: options.report_logic,
        ignore_class_methods: options.ignore_class_methods.clone(),
    });
    let state = CoverageState { pragmas };
    let scoping = traverse_mut(&mut transform, &allocator, &mut parsed.program, scoping, state);

    let coverage_map = build_coverage_map(filename, transform, options.input_source_map.as_deref());

    // Serialize the coverage map once and reuse it for both the hash guard and
    // the preamble's coverageData literal. Istanbul refreshes stale coverage
    // objects when the same path is reinstrumented with a different shape, and
    // the hash is computed over the same JSON we embed in the preamble.
    let coverage_json = serialize_coverage_map(&coverage_map);
    let coverage_hash = djb31_hex(&coverage_json);

    let preamble = generate_preamble_source(&PreambleInputs {
        coverage: &coverage_map,
        coverage_json: &coverage_json,
        coverage_hash: &coverage_hash,
        coverage_var: &options.coverage_variable,
        cov_fn_name: &cov_fn_name,
        report_logic: options.report_logic,
    });

    let (code, raw_source_map) = emit_code(EmitInputs {
        program: &parsed.program,
        scoping,
        source,
        filename,
        preamble: &preamble,
        options,
    });
    let source_map = raw_source_map
        .as_ref()
        .map(|sm| finalize_source_map(sm, &preamble, options.input_source_map.as_deref()));

    Ok(InstrumentResult {
        code,
        coverage_map,
        coverage_map_json: coverage_json,
        source_map,
        unhandled_pragmas,
    })
}

/// Run `oxc_transformer`'s TypeScript-strip pass on the parsed program in
/// place. Returns the updated `Scoping` produced by the transformer (the
/// semantic state may change as type-only nodes are removed). Surviving nodes
/// retain their original `Span` values, so positions still refer to the
/// original TypeScript source offsets.
fn strip_typescript_pass<'a>(
    allocator: &'a Allocator,
    filename: &str,
    program: &mut Program<'a>,
    scoping: Scoping,
) -> Result<Scoping, InstrumentError> {
    let options = TransformOptions {
        typescript: TypeScriptOptions::default(),
        ..TransformOptions::default()
    };
    let transformer = Transformer::new(allocator, Path::new(filename), &options);
    let ret = transformer.build_with_scoping(scoping, program);
    if !ret.errors.is_empty() {
        return Err(InstrumentError::TransformError(
            ret.errors.iter().map(|e| format!("{e}")).collect::<Vec<_>>().join("; "),
        ));
    }
    Ok(ret.scoping)
}

fn parse_program<'a>(
    allocator: &'a Allocator,
    source: &'a str,
    filename: &str,
) -> Result<ParserReturn<'a>, InstrumentError> {
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let parsed = Parser::new(allocator, source, source_type).parse();
    if parsed.errors.is_empty() {
        Ok(parsed)
    } else {
        Err(InstrumentError::ParseError(
            parsed.errors.iter().map(|e| format!("{e}")).collect::<Vec<_>>().join("; "),
        ))
    }
}

fn empty_coverage_result(
    filename: &str,
    source: &str,
    unhandled_pragmas: Vec<UnhandledPragma>,
) -> InstrumentResult {
    let coverage_map = build_file_coverage(CoverageMaps {
        path: filename.to_string(),
        statement_locs: Vec::new(),
        fn_entries: Vec::new(),
        branch_entries: Vec::new(),
        logical_branch_ids: Vec::new(),
    });
    let coverage_map_json = serialize_coverage_map(&coverage_map);
    InstrumentResult {
        code: source.to_string(),
        coverage_map,
        coverage_map_json,
        source_map: None,
        unhandled_pragmas,
    }
}

fn build_coverage_map(
    filename: &str,
    transform: CoverageTransform<'_, '_>,
    input_source_map: Option<&str>,
) -> FileCoverage {
    let mut coverage_map = build_file_coverage(CoverageMaps {
        path: filename.to_string(),
        statement_locs: transform.statement_map,
        fn_entries: transform.fn_map,
        branch_entries: transform.branch_map,
        logical_branch_ids: transform.logical_branch_ids,
    });
    if let Some(input_sm) = input_source_map {
        coverage_map.input_source_map = serde_json::from_str(input_sm).ok();
    }
    coverage_map
}

/// Output of [`collect_for_v8_to_istanbul`]: the Istanbul `FileCoverage`
/// (statement / function / branch maps) plus a side-table of body byte spans
/// keyed by surviving branch id. Both are needed by `v8_to_istanbul` to
/// resolve V8 hit counts; the body byte spans solve the if-arm 0 case where
/// the istanbul-reported location (the whole `IfStatement`) does not match
/// any V8 block range.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal type intentionally; the explicit pub(crate) documents that this is not part of the public API even though the parent module is already private"
)]
pub(crate) struct V8CollectResult {
    pub(crate) coverage_map: FileCoverage,
    /// `arm_body_byte_spans["<branch_id>"][<arm_idx>]` is the `(start, end)`
    /// byte range of the arm body when known, or `(0, 0)` when the body span
    /// is synthetic / unknown (e.g. a synthesized else-arm).
    pub(crate) arm_body_byte_spans: BTreeMap<String, Vec<(u32, u32)>>,
}

/// Parse, scan pragmas, traverse the AST, and build the `FileCoverage` map
/// without performing codegen, preamble emission, or coverage-map JSON
/// serialization. This is the visit-only path `v8_to_istanbul` uses; the
/// codegen + preamble + hash work that `instrument()` performs is dead work
/// when the caller only needs the location maps to intersect against V8
/// byte ranges.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal function intentionally; the explicit pub(crate) documents that this is not part of the public API even though the parent module is already private"
)]
pub(crate) fn collect_for_v8_to_istanbul(
    source: &str,
    filename: &str,
) -> Result<V8CollectResult, InstrumentError> {
    let allocator = Allocator::default();
    let mut parsed = parse_program(&allocator, source, filename)?;

    let (pragmas, _unhandled_pragmas) = PragmaMap::from_program(&parsed.program, source);
    if pragmas.ignore_file {
        let coverage_map = build_file_coverage(CoverageMaps {
            path: filename.to_string(),
            statement_locs: Vec::new(),
            fn_entries: Vec::new(),
            branch_entries: Vec::new(),
            logical_branch_ids: Vec::new(),
        });
        return Ok(V8CollectResult { coverage_map, arm_body_byte_spans: BTreeMap::new() });
    }

    let scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();
    let cov_fn_name = generate_cov_fn_name(filename);

    let mut transform = CoverageTransform::new(TransformInit {
        allocator: &allocator,
        source,
        cov_fn_name: &cov_fn_name,
        report_logic: false,
        ignore_class_methods: Vec::new(),
    });
    let state = CoverageState { pragmas };
    let _scoping = traverse_mut(&mut transform, &allocator, &mut parsed.program, scoping, state);

    // Build the body-byte-span side-table BEFORE moving `branch_map` into
    // `from_maps`. The id assignment in `from_maps` filters out branches with
    // empty `locations` and preserves the original sequential id via
    // `enumerate` (filter runs AFTER), so the keys here use the same
    // pre-filter index and only retain surviving entries.
    let mut arm_body_byte_spans: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
    for (idx, body_spans) in transform.branch_arm_body_byte_spans.iter().enumerate() {
        let surviving =
            transform.branch_map.get(idx).is_some_and(|entry| !entry.locations.is_empty());
        if surviving {
            arm_body_byte_spans.insert(idx.to_string(), body_spans.clone());
        }
    }

    let coverage_map = build_coverage_map(filename, transform, None);
    Ok(V8CollectResult { coverage_map, arm_body_byte_spans })
}

struct EmitInputs<'a, 'arena> {
    program: &'a Program<'arena>,
    scoping: Scoping,
    source: &'a str,
    filename: &'a str,
    preamble: &'a str,
    options: &'a InstrumentOptions,
}

fn emit_code(inputs: EmitInputs<'_, '_>) -> (String, Option<oxc_sourcemap::SourceMap>) {
    let EmitInputs { program, scoping, source, filename, preamble, options } = inputs;
    let codegen_options = CodegenOptions {
        source_map_path: if options.source_map { Some(PathBuf::from(filename)) } else { None },
        ..CodegenOptions::default()
    };
    let codegen_ret = Codegen::new()
        .with_options(codegen_options)
        .with_source_text(source)
        .with_scoping(Some(scoping))
        .build(program);
    let code = format!("{preamble}{}", codegen_ret.code);
    (code, codegen_ret.map)
}

/// Offset the codegen source map by the preamble line count and, if an input
/// source map was provided, compose the result with it so the final map chains
/// all the way back to the original source (e.g., TypeScript).
///
/// Composition is delegated to `srcmap-remapping`, which mirrors the semantics
/// of `@ampproject/remapping` (the prior art `istanbul-lib-source-maps` and
/// most JS bundlers also follow). Line offsetting uses `srcmap-remapping`'s
/// `ConcatBuilder`.
fn finalize_source_map(
    sm: &oxc_sourcemap::SourceMap,
    preamble: &str,
    input_source_map: Option<&str>,
) -> String {
    let preamble_lines =
        u32::try_from(preamble.chars().filter(|&c| c == '\n').count()).unwrap_or(u32::MAX);

    // Bridge oxc_sourcemap → srcmap_sourcemap via JSON. Both crates emit the
    // standard source map v3 format, so the round-trip is lossless. Bail out
    // to the raw oxc serialization if the parse ever fails.
    let output_json = sm.to_json_string();
    let Ok(output_sm) = srcmap_sourcemap::SourceMap::from_json(&output_json) else {
        return output_json;
    };

    let offset_sm = if preamble_lines > 0 {
        let mut builder = srcmap_remapping::ConcatBuilder::new(None);
        builder.add_map(&output_sm, preamble_lines);
        builder.build()
    } else {
        output_sm
    };

    if let Some(input_sm_json) = input_source_map
        && let Ok(input_sm) = srcmap_sourcemap::SourceMap::from_json(input_sm_json)
    {
        // The output map has exactly one source (the instrumented file).
        // Return the input map for any source name; remap drops sources it
        // can't load. Clone per call since `remap` may invoke the loader
        // more than once per unique source name.
        let composed = srcmap_remapping::remap(&offset_sm, |_name: &str| Some(input_sm.clone()));
        return composed.to_json();
    }

    offset_sm.to_json()
}

/// Error type for instrumentation failures.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InstrumentError {
    /// The source could not be parsed.
    ParseError(String),
    /// The coverage variable name is not a valid JavaScript identifier.
    InvalidCoverageVariable(String),
    /// Coverage data serialization failed. Reserved for future use: the current
    /// `FileCoverage` shape only contains types whose serde implementations are
    /// infallible, so `instrument()` does not currently construct this variant.
    SerializationError(String),
    /// The TypeScript strip pass produced diagnostics. Only emitted when
    /// `InstrumentOptions::strip_typescript` is enabled.
    TransformError(String),
}

impl std::fmt::Display for InstrumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::SerializationError(msg) => write!(f, "serialization error: {msg}"),
            Self::TransformError(msg) => write!(f, "transform error: {msg}"),
            Self::InvalidCoverageVariable(name) => {
                write!(
                    f,
                    "invalid coverage variable: {name:?} is not a valid JavaScript identifier"
                )
            }
        }
    }
}

impl std::error::Error for InstrumentError {}
