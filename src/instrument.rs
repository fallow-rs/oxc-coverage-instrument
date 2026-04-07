//! Top-level instrumentation API.

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::types::FileCoverage;
use crate::visitor::CoverageVisitor;

/// Options for the `instrument` function.
#[derive(Debug, Clone)]
pub struct InstrumentOptions {
    /// Name of the global coverage variable (default: `"__coverage__"`).
    pub coverage_variable: String,
    /// Input source map (not yet supported, reserved for future use).
    pub input_source_map: Option<String>,
}

impl Default for InstrumentOptions {
    fn default() -> Self {
        Self {
            coverage_variable: "__coverage__".to_string(),
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
    /// Output source map (not yet supported, always `None`).
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
/// locations, and produces an Istanbul-compatible coverage map. The returned
/// `code` contains the original source with coverage counter expressions
/// injected.
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
    let parsed = parser.parse();

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

    // Phase 1: Collect coverage spans via visitor
    let mut visitor = CoverageVisitor::new(source);
    visitor.visit_program(&parsed.program);

    let coverage_map = FileCoverage::from_maps(
        filename.to_string(),
        visitor.statement_map,
        visitor.fn_map,
        visitor.branch_map,
    );

    // Phase 2: Inject counters into source
    // For v0.1.0, we use source-level injection based on collected spans.
    // Future versions will use Traverse-based AST mutation + oxc_codegen.
    let code = inject_counters(source, &coverage_map, &options.coverage_variable);

    Ok(InstrumentResult {
        code,
        coverage_map,
        source_map: None,
        unhandled_pragmas: Vec::new(), // TODO: scan for istanbul/v8 ignore comments
    })
}

/// Inject coverage counter expressions into the source text.
///
/// Inserts `__coverage__.f[N]++` at function entry and `__coverage__.s[N]++`
/// before each statement. Uses the coverage map's position data to find
/// insertion points.
fn inject_counters(source: &str, coverage: &FileCoverage, coverage_var: &str) -> String {
    // Collect all insertion points: (byte_offset, counter_expression)
    let mut insertions: Vec<(u32, String)> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Function entry counters: insert at start of function body
    for (id, entry) in &coverage.fn_map {
        let offset = line_col_to_offset(&lines, entry.loc.start.line, entry.loc.start.column + 1);
        insertions.push((offset, format!("{coverage_var}.f[{id}]++;")));
    }

    // Statement counters: insert before statement
    for (id, loc) in &coverage.statement_map {
        let offset = line_col_to_offset(&lines, loc.start.line, loc.start.column);
        insertions.push((offset, format!("{coverage_var}.s[{id}]++;")));
    }

    // Sort by offset descending so insertions don't shift later positions
    insertions.sort_by(|a, b| b.0.cmp(&a.0));

    // Deduplicate: if multiple counters target the same offset, combine them
    insertions.dedup_by(|a, b| {
        if a.0 == b.0 {
            b.1 = format!("{}{}", b.1, a.1);
            true
        } else {
            false
        }
    });

    let mut result = source.to_string();
    for (offset, counter) in &insertions {
        let offset = *offset as usize;
        if offset <= result.len() {
            result.insert_str(offset, counter);
        }
    }

    // Prepend the coverage variable initialization
    let preamble = generate_preamble(coverage, coverage_var);
    format!("{preamble}{result}")
}

/// Generate the runtime preamble that initializes the coverage variable.
fn generate_preamble(coverage: &FileCoverage, coverage_var: &str) -> String {
    let coverage_json = serde_json::to_string(coverage).unwrap_or_default();
    format!(
        "var {cov} = (function() {{ var g = typeof globalThis !== 'undefined' ? globalThis : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : this; if (!g['{cov}']) g['{cov}'] = {{}}; if (!g['{cov}']['{path}']) g['{cov}']['{path}'] = {json}; return g['{cov}']['{path}']; }})();\n",
        cov = coverage_var,
        path = coverage.path,
        json = coverage_json,
    )
}

/// Convert 1-based line + 0-based column to a byte offset.
fn line_col_to_offset(lines: &[&str], line: u32, column: u32) -> u32 {
    let line_idx = (line as usize).saturating_sub(1);
    let mut offset: u32 = 0;
    for l in lines.iter().take(line_idx) {
        offset += l.len() as u32 + 1; // +1 for newline
    }
    offset + column
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
