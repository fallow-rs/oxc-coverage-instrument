//! Coverage reporters for the oxc-coverage suite.
//!
//! This crate is a Rust port of [`istanbul-reports`][upstream]: each module
//! defines a reporter that consumes a [`ReportNode`][rn] tree (built by
//! [`oxc_coverage_report::summarize`]) and writes output in one of the
//! Istanbul-compatible formats.
//!
//! Available reporters in this release:
//!
//! - [`text`]: ANSI-friendly console table with per-folder / per-file rows.
//! - [`text_summary`]: four-line metric rollup, useful for piping to a PR
//!   comment or CI summary.
//! - [`json_summary`]: `coverage-summary.json` shape consumed by Codecov,
//!   Vitest, and dashboard tools.
//! - [`lcov`]: LCOV `tracefile` consumed by Codecov, Coveralls, GitLab MR
//!   widget, and the `lcov`/`genhtml` toolchain.
//! - [`cobertura`]: Cobertura XML consumed by GitLab MR widget, Jenkins,
//!   Azure DevOps, and Codecov.
//!
//! The `html` reporter will land in a follow-on PR.
//!
//! # Example
//!
//! ```
//! use oxc_coverage_types::parse_coverage_map;
//! use oxc_coverage_report::summarize;
//! use oxc_coverage_reports::json_summary;
//!
//! let map = parse_coverage_map(r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#).unwrap();
//! let root = summarize(&map);
//! let mut buf = Vec::new();
//! json_summary::write(&root, &mut buf).unwrap();
//! assert!(String::from_utf8(buf).unwrap().contains("\"total\""));
//! ```
//!
//! [upstream]: https://github.com/istanbuljs/istanbuljs/tree/main/packages/istanbul-reports
//! [rn]: oxc_coverage_report::ReportNode

pub mod cobertura;
pub mod json_summary;
pub mod lcov;
pub mod text;
pub mod text_summary;

use std::path::Path;

/// Convenience enum for selecting a reporter at runtime (e.g., from a CLI flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    TextSummary,
    JsonSummary,
    Lcov,
    Cobertura,
}

impl Format {
    /// Parse a CLI-style format name (`text`, `text-summary`, `json-summary`,
    /// `lcov`, `cobertura`). Returns `None` for unknown values; the CLI is
    /// responsible for the user-facing error.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "text" => Some(Self::Text),
            "text-summary" => Some(Self::TextSummary),
            "json-summary" => Some(Self::JsonSummary),
            "lcov" => Some(Self::Lcov),
            "cobertura" => Some(Self::Cobertura),
            _ => None,
        }
    }

    /// Render `root` in this format to `out`.
    ///
    /// `root_dir` is only consulted by formats that emit source-file paths
    /// (currently [`Format::Lcov`] and [`Format::Cobertura`]); other formats
    /// ignore it. Pass `Path::new("")` if path relativization is not needed.
    pub fn write<W: std::io::Write>(
        self,
        root: &oxc_coverage_report::ReportNode,
        root_dir: &Path,
        out: &mut W,
    ) -> std::io::Result<()> {
        match self {
            Self::Text => text::write(root, out),
            Self::TextSummary => text_summary::write(root, out),
            Self::JsonSummary => json_summary::write(root, out),
            Self::Lcov => lcov::write(root, root_dir, out),
            Self::Cobertura => cobertura::write(root, root_dir, out),
        }
    }
}
