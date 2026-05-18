//! Istanbul-compatible coverage data types.
//!
//! First-party serde types derived from Istanbul's JSON schema
//! (`@istanbuljs/schema`). Produces `coverage-final.json` compatible
//! output that Jest, Vitest, c8, nyc, and Codecov all consume.
//!
//! This crate is the data-model layer of the `oxc-coverage` suite. The
//! instrumenter, source-map remapper, V8-to-Istanbul converter, and any future
//! reporter all share these types.
//!
//! # Example
//!
//! ```
//! use oxc_coverage_types::{FileCoverage, parse_coverage_map};
//!
//! let json = r#"{"file.js": {"path": "file.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}}}"#;
//! let map = parse_coverage_map(json).unwrap();
//! assert!(map.contains_key("file.js"));
//! ```

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A coverage pragma comment that was found but not handled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnhandledPragma {
    /// The full comment text.
    pub comment: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based column.
    pub column: u32,
}

/// Coverage data for a single file. Serializes to Istanbul's `coverage-final.json` format.
///
/// The root `coverage-final.json` is a map of file paths to `FileCoverage` objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    /// Absolute file path.
    #[serde(default, deserialize_with = "deserialize_null_as_empty")]
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
    /// Istanbul allows `null` values (e.g., uninstrumented statements); these are coerced to `0`.
    #[serde(deserialize_with = "deserialize_null_as_zero_map")]
    pub s: BTreeMap<String, u32>,
    /// Function hit counts, keyed by the same IDs as `fn_map`.
    /// Istanbul allows `null` values; these are coerced to `0`.
    #[serde(deserialize_with = "deserialize_null_as_zero_map")]
    pub f: BTreeMap<String, u32>,
    /// Branch hit counts, keyed by the same IDs as `branch_map`.
    /// Each value is a Vec with one count per branch arm.
    /// Istanbul allows `null` values in both the Vec and individual elements; these are coerced to `0`.
    #[serde(deserialize_with = "deserialize_null_as_zero_vec_map")]
    pub b: BTreeMap<String, Vec<u32>>,
    /// Branch truthy tracking hit counts. Only present when `report_logic` is enabled.
    /// Tracks whether each operand in a logical expression evaluated to a truthy value.
    /// Keyed by the same IDs as `binary-expr` entries in `branch_map`.
    #[serde(
        rename = "bT",
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "deserialize_optional_null_as_zero_vec_map"
    )]
    pub b_t: Option<BTreeMap<String, Vec<u32>>>,
    /// Input source map from a prior transformation (e.g., TypeScript to JS).
    /// Stored so downstream tools can chain back to the original source.
    /// Only present when the instrumenter received an `inputSourceMap`.
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
///
/// Istanbul allows `null` or missing fields for line/column when the position
/// is unknown (e.g., empty `{}` objects for branch locations); these are
/// coerced to `0` on deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct Position {
    /// 1-based line number.
    #[serde(default, deserialize_with = "deserialize_null_as_zero")]
    pub line: u32,
    /// 0-based column number.
    #[serde(default, deserialize_with = "deserialize_null_as_zero")]
    pub column: u32,
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.line == 0 && self.column == 0 {
            return serializer.serialize_struct("Position", 0)?.end();
        }

        let mut state = serializer.serialize_struct("Position", 2)?;
        state.serialize_field("line", &self.line)?;
        state.serialize_field("column", &self.column)?;
        state.end()
    }
}

/// Function entry in the coverage map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnEntry {
    /// Function name. Anonymous functions use `"(anonymous_N)"`.
    #[serde(default, deserialize_with = "deserialize_null_as_empty")]
    pub name: String,
    /// 1-based line of the function declaration.
    #[serde(default, deserialize_with = "deserialize_null_as_zero")]
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
    #[serde(default, deserialize_with = "deserialize_null_as_zero")]
    pub line: u32,
    /// Branch type: `"if"`, `"switch"`, `"cond-expr"`, `"binary-expr"`, `"default-arg"`.
    #[serde(rename = "type", default, deserialize_with = "deserialize_null_as_empty")]
    pub branch_type: String,
    /// One location per branch arm.
    pub locations: Vec<Location>,
}

/// Deserialize a `String` where `null` is coerced to an empty string.
fn deserialize_null_as_empty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// Deserialize a `u32` where `null` is coerced to `0`.
///
/// Istanbul allows `null` for position fields (line, column) when the exact
/// location is unknown, and for individual hit counts in some edge cases.
fn deserialize_null_as_zero<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<u32>::deserialize(deserializer)?.unwrap_or(0))
}

/// Deserialize a `BTreeMap<String, u32>` where `null` values are coerced to `0`.
///
/// Istanbul coverage tools may emit `null` for hit counts when a statement or
/// function was never instrumented or the count is indeterminate.
fn deserialize_null_as_zero_map<'de, D>(deserializer: D) -> Result<BTreeMap<String, u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: BTreeMap<String, Option<u32>> = Deserialize::deserialize(deserializer)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.unwrap_or(0))).collect())
}

/// Deserialize a `BTreeMap<String, Vec<u32>>` where `null` values are coerced to `0`.
///
/// Handles `null` at both the Vec level (coerced to empty Vec) and individual
/// element level (coerced to `0`).
fn deserialize_null_as_zero_vec_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, Vec<u32>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: BTreeMap<String, Option<Vec<Option<u32>>>> = Deserialize::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .map(|(k, v)| {
            let vec = v.unwrap_or_default().into_iter().map(|x| x.unwrap_or(0)).collect();
            (k, vec)
        })
        .collect())
}

/// Deserialize an `Option<BTreeMap<String, Vec<u32>>>` with null-tolerance.
fn deserialize_optional_null_as_zero_vec_map<'de, D>(
    deserializer: D,
) -> Result<Option<BTreeMap<String, Vec<u32>>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<BTreeMap<String, Option<Vec<Option<u32>>>>> =
        Deserialize::deserialize(deserializer)?;
    Ok(opt.map(|raw| {
        raw.into_iter()
            .map(|(k, v)| {
                let vec = v.unwrap_or_default().into_iter().map(|x| x.unwrap_or(0)).collect();
                (k, vec)
            })
            .collect()
    }))
}

/// Parse a `coverage-final.json` string into a map of file paths to coverage data.
///
/// This is the inverse of instrumentation: it reads existing coverage data
/// produced by any Istanbul-compatible tool (Jest, Vitest, c8, nyc, etc.).
///
/// # Example
///
/// ```
/// use oxc_coverage_types::parse_coverage_map;
///
/// let json = r#"{"file.js": {"path": "file.js", "statementMap": {}, "fnMap": {}, "branchMap": {}, "s": {}, "f": {}, "b": {}}}"#;
/// let map = parse_coverage_map(json).unwrap();
/// assert!(map.contains_key("file.js"));
/// ```
pub fn parse_coverage_map(json: &str) -> Result<BTreeMap<String, FileCoverage>, serde_json::Error> {
    serde_json::from_str(json)
}

impl FileCoverage {
    /// Deserialize a `FileCoverage` from a JSON string.
    ///
    /// Parses Istanbul-compatible `coverage-final.json` format.
    /// The input should be a single file's coverage object (not the root map).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
