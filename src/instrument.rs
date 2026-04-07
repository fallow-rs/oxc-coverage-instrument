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
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let parser = Parser::new(&allocator, source, source_type);
    let mut parsed = parser.parse();

    if !parsed.errors.is_empty() {
        return Err(InstrumentError::ParseError(
            parsed
                .errors
                .iter()
                .map(|e| format!("{e}"))
                .collect::<Vec<_>>()
                .join("; "),
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

    let scoping = traverse_mut(
        &mut transform,
        &allocator,
        &mut parsed.program,
        scoping,
        state,
    );

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
        generate_preamble_source(&coverage_map, &options.coverage_variable, &cov_fn_name);

    // Phase 3: Emit instrumented code via codegen
    let codegen_options = CodegenOptions {
        source_map_path: if options.source_map {
            Some(PathBuf::from(filename))
        } else {
            None
        },
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
        if preamble_lines > 0 {
            let builder =
                oxc_sourcemap::ConcatSourceMapBuilder::from_sourcemaps(&[(&sm, preamble_lines)]);
            builder.into_sourcemap().to_json_string()
        } else {
            sm.to_json_string()
        }
    });

    Ok(InstrumentResult {
        code,
        coverage_map,
        source_map: source_map_json,
        unhandled_pragmas,
    })
}

/// Error type for instrumentation failures.
#[derive(Debug, Clone)]
pub enum InstrumentError {
    /// The source could not be parsed.
    ParseError(String),
}

impl std::fmt::Display for InstrumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for InstrumentError {}
