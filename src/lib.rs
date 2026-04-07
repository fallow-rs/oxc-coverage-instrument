//! Istanbul-compatible JavaScript/TypeScript coverage instrumentation using the Oxc AST.
//!
//! This crate parses JS/TS source with [`oxc_parser`], identifies statements,
//! functions, and branches, injects coverage counter expressions, and emits
//! instrumented code. The coverage map output is compatible with Istanbul's
//! `coverage-final.json` format (consumed by Jest, Vitest, c8, nyc, Codecov).
//!
//! # Example
//!
//! ```
//! use oxc_coverage_instrument::{instrument, InstrumentOptions};
//!
//! let source = "function add(a, b) { return a + b; }";
//! let result = instrument(source, "add.js", &InstrumentOptions::default()).unwrap();
//!
//! println!("Instrumented code:\n{}", result.code);
//! println!("Functions found: {}", result.coverage_map.fn_map.len());
//! ```
//!
//! # Coverage model
//!
//! The coverage map tracks three dimensions:
//!
//! - **Statements**: every executable statement gets a counter
//! - **Functions**: every function declaration, expression, arrow, and method
//! - **Branches**: if/else, ternary, switch cases, logical &&/||
//!
//! Function names are derived from the same Oxc parser used by other Oxc-based
//! tools, so they match consistently across the ecosystem.

mod instrument;
mod pragma;
mod transform;
mod types;

pub use instrument::{
    InstrumentError, InstrumentOptions, InstrumentResult, UnhandledPragma, instrument,
};
pub use types::{BranchEntry, FileCoverage, FnEntry, Location, Position};

/// Parse a `coverage-final.json` string into a map of file paths to coverage data.
///
/// This is the inverse of instrumentation — it reads existing coverage data
/// produced by any Istanbul-compatible tool (Jest, Vitest, c8, nyc, etc.).
///
/// # Example
///
/// ```
/// use oxc_coverage_instrument::parse_coverage_map;
///
/// let json = r#"{"file.js": {"path": "file.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}}}"#;
/// let map = parse_coverage_map(json).unwrap();
/// assert!(map.contains_key("file.js"));
/// ```
pub fn parse_coverage_map(
    json: &str,
) -> Result<std::collections::BTreeMap<String, FileCoverage>, serde_json::Error> {
    serde_json::from_str(json)
}
