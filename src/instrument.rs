//! Top-level instrumentation API.

use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_traverse::traverse_mut;

use crate::pragma::PragmaMap;
use crate::transform::{
    CoverageState, CoverageTransform, generate_cov_fn_name, generate_preamble_source,
};
use crate::types::FileCoverage;

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
}

impl Default for InstrumentOptions {
    fn default() -> Self {
        Self {
            coverage_variable: "__coverage__".to_string(),
            source_map: false,
            input_source_map: None,
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
    /// Output source map JSON string (only present if `InstrumentOptions::source_map` is true).
    pub source_map: Option<String>,
    /// Unhandled pragma comments found during instrumentation.
    /// Contains `/* istanbul ignore ... */` and `/* v8 ignore ... */` comments
    /// that were not processed. Callers should decide whether to warn or error.
    pub unhandled_pragmas: Vec<UnhandledPragma>,
}

/// A coverage pragma comment that was found but not handled.
#[derive(Debug, Clone)]
pub struct UnhandledPragma {
    /// The full comment text.
    pub comment: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based column.
    pub column: u32,
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
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let parser = Parser::new(&allocator, source, source_type);
    let mut parsed = parser.parse();

    if !parsed.errors.is_empty() {
        return Err(InstrumentError::ParseError(
            parsed.errors.iter().map(|e| format!("{e}")).collect::<Vec<_>>().join("; "),
        ));
    }

    // Build pragma map from comments before semantic analysis modifies anything
    let (pragmas, unhandled_pragmas) = PragmaMap::from_program(&parsed.program, source);

    // If the entire file is ignored, return empty coverage
    if pragmas.ignore_file {
        let coverage_map = FileCoverage::from_maps(
            filename.to_string(),
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
        );
        return Ok(InstrumentResult {
            code: source.to_string(),
            coverage_map,
            source_map: None,
            unhandled_pragmas,
        });
    }

    // Build semantic analysis for Scoping (required by traverse_mut)
    let semantic_ret = SemanticBuilder::new().build(&parsed.program);
    let scoping = semantic_ret.semantic.into_scoping();

    // Generate deterministic coverage function name
    let cov_fn_name = generate_cov_fn_name(filename);

    // Phase 1: Traverse AST, collect coverage spans, and inject counter expressions
    let mut transform = CoverageTransform::new(source, cov_fn_name.clone());
    let state = CoverageState { pragmas };

    let scoping = traverse_mut(&mut transform, &allocator, &mut parsed.program, scoping, state);

    // Build coverage map from collected metadata
    let mut coverage_map = FileCoverage::from_maps(
        filename.to_string(),
        transform.statement_map,
        transform.fn_map,
        transform.branch_map,
    );

    // Store input source map on coverage data for downstream tools (matches Istanbul behavior)
    if let Some(ref input_sm) = options.input_source_map {
        coverage_map.input_source_map = serde_json::from_str(input_sm).ok();
    }

    // Phase 2: Generate preamble source and prepend to program
    let preamble =
        generate_preamble_source(&coverage_map, &options.coverage_variable, &cov_fn_name)
            .map_err(|e| InstrumentError::SerializationError(e.to_string()))?;

    // Phase 3: Emit instrumented code via codegen
    let codegen_options = CodegenOptions {
        source_map_path: if options.source_map { Some(PathBuf::from(filename)) } else { None },
        ..CodegenOptions::default()
    };

    let codegen_ret = Codegen::new()
        .with_options(codegen_options)
        .with_source_text(source)
        .with_scoping(Some(scoping))
        .build(&parsed.program);

    let code = format!("{preamble}{}", codegen_ret.code);

    // If source map was generated, offset all mappings by the preamble line count
    // so positions in the combined output map correctly back to the original source.
    let source_map_json = codegen_ret.map.map(|sm| {
        let preamble_lines = preamble.chars().filter(|&c| c == '\n').count() as u32;
        let offset_sm = if preamble_lines > 0 {
            let builder =
                oxc_sourcemap::ConcatSourceMapBuilder::from_sourcemaps(&[(&sm, preamble_lines)]);
            builder.into_sourcemap()
        } else {
            sm
        };

        // If an input source map was provided, compose it with the output source map
        // so the final map chains back to the original source (e.g., TypeScript).
        if let Some(ref input_sm_json) = options.input_source_map {
            if let Ok(input_sm) = oxc_sourcemap::SourceMap::from_json_string(input_sm_json) {
                return compose_source_maps(&offset_sm, &input_sm).to_json_string();
            }
        }

        offset_sm.to_json_string()
    });

    Ok(InstrumentResult { code, coverage_map, source_map: source_map_json, unhandled_pragmas })
}

/// Compose two source maps: for each mapping in `output_sm` (instrumented → intermediate),
/// look up the corresponding position in `input_sm` (intermediate → original) to produce
/// a composed map (instrumented → original).
fn compose_source_maps(
    output_sm: &oxc_sourcemap::SourceMap,
    input_sm: &oxc_sourcemap::SourceMap,
) -> oxc_sourcemap::SourceMap {
    let input_lookup = input_sm.generate_lookup_table();
    let mut builder = oxc_sourcemap::SourceMapBuilder::default();

    // Copy source files and contents from input (the originals)
    for (source, content) in input_sm.get_sources().zip(input_sm.get_source_contents()) {
        let content_str = content.map_or("", |c| c.as_ref());
        builder.add_source_and_content(source, content_str);
    }

    // Copy names from input map
    for name in input_sm.get_names() {
        builder.add_name(name);
    }

    // For each token in the output map, look up in the input map
    for token in output_sm.get_tokens() {
        let src_line = token.get_src_line();
        let src_col = token.get_src_col();

        if let Some(original) = input_sm.lookup_token(&input_lookup, src_line, src_col) {
            builder.add_token(
                token.get_dst_line(),
                token.get_dst_col(),
                original.get_src_line(),
                original.get_src_col(),
                original.get_source_id(),
                original.get_name_id(),
            );
        } else {
            // No mapping in input — keep the output mapping as-is
            builder.add_token(
                token.get_dst_line(),
                token.get_dst_col(),
                src_line,
                src_col,
                token.get_source_id(),
                token.get_name_id(),
            );
        }
    }

    builder.into_sourcemap()
}

/// Error type for instrumentation failures.
#[derive(Debug, Clone)]
pub enum InstrumentError {
    /// The source could not be parsed.
    ParseError(String),
    /// The coverage variable name is not a valid JavaScript identifier.
    InvalidCoverageVariable(String),
    /// Coverage data serialization failed.
    SerializationError(String),
}

impl std::fmt::Display for InstrumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::SerializationError(msg) => write!(f, "serialization error: {msg}"),
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
