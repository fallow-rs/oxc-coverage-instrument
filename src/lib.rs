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
mod types;
mod visitor;

pub use instrument::{instrument, InstrumentError, InstrumentOptions, InstrumentResult, UnhandledPragma};
pub use types::{BranchEntry, FileCoverage, FnEntry, Location, Position};
