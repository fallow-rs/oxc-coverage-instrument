//! Coverage reporters for the oxc-coverage suite.
//!
//! This crate is a Rust port of [`istanbul-reports`][upstream]: each module
//! defines a reporter that consumes a [`ReportNode`][rn] tree (built by
//! [`oxc_coverage_report::summarize`]) and writes output in one of the
//! Istanbul-compatible formats.
//!
//! Available reporters:
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
//! - `html`: self-contained directory of HTML pages with per-file source
//!   gutter. This format is available with the `html` Cargo feature. It ships
//!   sortable index tables, an auto/light/dark theme toggle, server-side
//!   syntax highlighting via [syntect], and a strict Content-Security-Policy
//!   so the report performs zero outbound network requests.
//!
//! [syntect]: https://docs.rs/syntect
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
//! [upstream]: https://github.com/istanbuljs/istanbuljs/tree/istanbul-reports-v3.2.0/packages/istanbul-reports
//! [rn]: oxc_coverage_report::ReportNode

pub mod cobertura;
mod escape;
#[cfg(feature = "html")]
pub mod html;

/// Configuration for the html reporter.
///
/// The reporter itself needs the `html` Cargo feature, but its options type
/// stays available without it so [`Format::write_to_dir`] keeps one
/// signature in every feature configuration.
#[cfg(not(feature = "html"))]
pub mod html {
    mod options;

    pub use options::{HtmlOptions, HtmlOptionsError};
}

pub mod json_summary;
pub mod lcov;
mod projection;
pub mod text;
pub mod text_summary;

use std::path::Path;

/// Reporter selected at runtime, typically from a CLI flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Fixed-width console table, one row per folder and file. See [`text`].
    Text,
    /// Four-line metric rollup. See [`text_summary`].
    TextSummary,
    /// `coverage-summary.json`. See [`json_summary`].
    JsonSummary,
    /// LCOV tracefile. See [`lcov`].
    Lcov,
    /// Cobertura XML. See [`cobertura`].
    Cobertura,
    /// Multi-file directory output. Use [`Format::write_to_dir`]; calling
    /// [`Format::write`] on this variant returns an error because there is
    /// no single stream to write to. Available only when the `html`
    /// Cargo feature is enabled.
    #[cfg(feature = "html")]
    Html,
}

impl Format {
    /// Parse a CLI-style format name: `text`, `text-summary`,
    /// `json-summary`, `lcov`, `cobertura`, and `html` when the `html`
    /// feature is enabled.
    ///
    /// Returns `None` for anything else, including a name that differs only
    /// in case; the caller owns the user-facing error.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "text" => Some(Self::Text),
            "text-summary" => Some(Self::TextSummary),
            "json-summary" => Some(Self::JsonSummary),
            "lcov" => Some(Self::Lcov),
            "cobertura" => Some(Self::Cobertura),
            #[cfg(feature = "html")]
            "html" => Some(Self::Html),
            _ => None,
        }
    }

    /// The CLI-style name of this format, the inverse of [`Format::parse`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::TextSummary => "text-summary",
            Self::JsonSummary => "json-summary",
            Self::Lcov => "lcov",
            Self::Cobertura => "cobertura",
            #[cfg(feature = "html")]
            Self::Html => "html",
        }
    }

    /// `true` when the format writes a directory tree instead of a single
    /// stream. The CLI dispatches such formats through [`Format::write_to_dir`]
    /// and rejects `-o <file>` for them.
    #[cfg_attr(
        not(feature = "html"),
        expect(
            clippy::unused_self,
            reason = "self is only inspected when the `html` feature adds a multi-file variant",
        )
    )]
    pub fn is_multi_file(self) -> bool {
        #[cfg(feature = "html")]
        {
            matches!(self, Self::Html)
        }
        #[cfg(not(feature = "html"))]
        {
            false
        }
    }

    /// Render `root` in this format to `out`.
    ///
    /// `root_dir` is only consulted by formats that emit source-file paths
    /// (currently [`Format::Lcov`] and [`Format::Cobertura`]); other formats
    /// ignore it. Pass `Path::new("")` if path relativization is not needed.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if:
    ///   * `out` fails to accept a write
    ///   * `self` is a multi-file format, with kind `InvalidInput`; those
    ///     cannot stream into a single sink, so use [`Format::write_to_dir`]
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
            #[cfg(feature = "html")]
            Self::Html => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "html format produces a directory tree; use Format::write_to_dir",
            )),
        }
    }

    /// Render this format into `output_dir`. The full coverage map is
    /// required (rather than a pre-summarized [`oxc_coverage_report::ReportNode`])
    /// so that multi-file formats can apply source-map remapping internally
    /// and reflect original TS/JSX source in their output.
    ///
    /// `html_options` is consumed by the `Html` variant and ignored by every
    /// other one. The parameter is present in every feature configuration so
    /// that turning the `html` feature on elsewhere in a dependency graph
    /// cannot change this function's arity.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if:
    ///   * writing the report tree under `output_dir` fails
    ///   * a coverage path escapes `output_dir` or maps onto a symlinked
    ///     component, with kind `InvalidInput`
    ///   * `self` is a single-file format, with kind `InvalidInput`; those
    ///     are dispatched through [`Format::write`] instead. Without the
    ///     `html` feature every variant is single-file, so every call
    ///     returns this error
    #[cfg_attr(
        not(feature = "html"),
        expect(
            unused_variables,
            reason = "only the `html` feature adds a multi-file variant that consumes them",
        )
    )]
    pub fn write_to_dir(
        self,
        coverage_map: &oxc_coverage_report::CoverageMap,
        root_dir: &Path,
        output_dir: &Path,
        html_options: &html::HtmlOptions,
    ) -> std::io::Result<()> {
        match self {
            #[cfg(feature = "html")]
            Self::Html => html::write(coverage_map, root_dir, output_dir, html_options),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "single-file format; use Format::write",
            )),
        }
    }
}
