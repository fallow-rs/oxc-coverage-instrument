//! Istanbul-compatible coverage data types.
//!
//! First-party serde types derived from Istanbul's JSON schema
//! (`@istanbuljs/schema`). Produces `coverage-final.json` compatible
//! output that Jest, Vitest, c8, nyc, and Codecov all consume.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Coverage data for a single file. Serializes to Istanbul's `coverage-final.json` format.
///
/// The root `coverage-final.json` is a map of file paths to `FileCoverage` objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    /// Absolute file path.
    pub path: String,
    /// Statement locations, keyed by sequential string IDs ("0", "1", ...).
    #[serde(rename = "statementMap")]
    pub statement_map: BTreeMap<String, Location>,
    /// Function metadata, keyed by sequential string IDs.
    #[serde(rename = "fnMap")]
    pub fn_map: BTreeMap<String, FnEntry>,
    /// Branch metadata, keyed by sequential string IDs.
    #[serde(rename = "branchMap")]
    pub branch_map: BTreeMap<String, BranchEntry>,
    /// Statement hit counts, keyed by the same IDs as `statement_map`.
    pub s: BTreeMap<String, u32>,
    /// Function hit counts, keyed by the same IDs as `fn_map`.
    pub f: BTreeMap<String, u32>,
    /// Branch hit counts, keyed by the same IDs as `branch_map`.
    /// Each value is a Vec with one count per branch arm.
    pub b: BTreeMap<String, Vec<u32>>,
    /// Input source map from a prior transformation (e.g., TypeScript → JS).
    /// Stored so downstream tools can chain back to the original source.
    /// Only present when `InstrumentOptions::input_source_map` was provided.
    #[serde(rename = "inputSourceMap", skip_serializing_if = "Option::is_none")]
    pub input_source_map: Option<serde_json::Value>,
}

/// A source location span with start and end positions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub start: Position,
    pub end: Position,
}

/// A 1-based line, 0-based column position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// 1-based line number.
    pub line: u32,
    /// 0-based column number.
    pub column: u32,
}

/// Function entry in the coverage map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnEntry {
    /// Function name. Anonymous functions use `"(anonymous_N)"`.
    pub name: String,
    /// 1-based line of the function declaration.
    pub line: u32,
    /// Span of the function declaration (keyword to name/params).
    pub decl: Location,
    /// Span of the function body.
    pub loc: Location,
}

/// Branch entry in the coverage map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchEntry {
    /// Overall location of the branch construct.
    pub loc: Location,
    /// 1-based line where the branch starts.
    pub line: u32,
    /// Branch type: `"if"`, `"switch"`, `"cond-expr"`, `"binary-expr"`, `"default-arg"`.
    #[serde(rename = "type")]
    pub branch_type: String,
    /// One location per branch arm.
    pub locations: Vec<Location>,
}

impl FileCoverage {
    /// Deserialize a `FileCoverage` from a JSON string.
    ///
    /// Parses Istanbul-compatible `coverage-final.json` format.
    /// The input should be a single file's coverage object (not the root map).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Create a new `FileCoverage` with empty hit counts initialized from the maps.
    pub(crate) fn from_maps(
        path: String,
        statement_map: BTreeMap<String, Location>,
        fn_map: BTreeMap<String, FnEntry>,
        branch_map: BTreeMap<String, BranchEntry>,
    ) -> Self {
        let s = statement_map.keys().map(|k| (k.clone(), 0)).collect();
        let f = fn_map.keys().map(|k| (k.clone(), 0)).collect();
        let b = branch_map
            .iter()
            .map(|(k, entry)| (k.clone(), vec![0; entry.locations.len()]))
            .collect();

        Self {
            path,
            statement_map,
            fn_map,
            branch_map,
            s,
            f,
            b,
            input_source_map: None,
        }
    }
}
