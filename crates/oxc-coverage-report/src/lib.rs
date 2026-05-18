//! Coverage summarization and tree-visitor base for the oxc-coverage suite.
//!
//! This crate is a Rust port of [`istanbul-lib-report`][upstream]: it consumes
//! Istanbul-compatible coverage data (`CoverageMap` from
//! [`oxc_coverage_types`]), summarizes hit counts into per-metric totals, and
//! organizes per-file coverage into a folder/file [`ReportNode`] tree that a
//! reporter walks via the [`Visitor`] trait.
//!
//! Reporters (text, json-summary, lcov, cobertura, html) live in the sibling
//! `oxc_coverage_reports` crate. Custom reporters can implement [`Visitor`]
//! directly against this crate.
//!
//! # Example
//!
//! ```
//! use oxc_coverage_types::parse_coverage_map;
//! use oxc_coverage_report::{summarize, walk, Visitor, ReportNode};
//!
//! struct Counter { files: usize, folders: usize }
//! impl Visitor for Counter {
//!     fn on_detail(&mut self, _node: &ReportNode) -> std::io::Result<()> {
//!         self.files += 1;
//!         Ok(())
//!     }
//!     fn on_summary(&mut self, _node: &ReportNode) -> std::io::Result<()> {
//!         self.folders += 1;
//!         Ok(())
//!     }
//! }
//!
//! let json = r#"{"a/b.js": {"path":"a/b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
//! let map = parse_coverage_map(json).unwrap();
//! let root = summarize(&map);
//! let mut visitor = Counter { files: 0, folders: 0 };
//! walk(&root, &mut visitor).unwrap();
//! assert_eq!(visitor.files, 1);
//! ```
//!
//! [upstream]: https://github.com/istanbuljs/istanbuljs/tree/main/packages/istanbul-lib-report

mod summarizer;
mod summary;
mod tree;
mod visitor;

pub use summarizer::{CoverageMap, summarize};
pub use summary::{CoverageSummary, Metric};
pub use tree::{NodeKind, ReportNode};
pub use visitor::{Visitor, walk};
