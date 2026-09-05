//! HTML reporter.
//!
//! Writes a self-contained directory of HTML pages matching the structure
//! produced by istanbul-reports' `html` reporter: a root `index.html`, a
//! per-folder `index.html`, and a per-file `<name>.html` detail page that
//! interleaves the original source with hit counts.
//!
//! Design constraints:
//!
//! - **Multi-file output**: unlike text / json-summary / lcov / cobertura,
//!   the report is a directory tree, so the entry point takes
//!   `output_dir: &Path` instead of a `Write` sink. The CLI exposes this
//!   via the `--output-dir` flag (default: `coverage/`).
//! - **Source-map remapping**: if any `FileCoverage` carries an
//!   `inputSourceMap`, the entire coverage map is remapped through
//!   [`oxc_coverage_source_maps::remap_coverage_map`] before summarization,
//!   so detail pages show original TS/JSX source instead of the
//!   instrumented JS. Entries without a map pass through unchanged.
//! - **No template engine**: hand-rolled string concatenation with
//!   careful HTML-attribute and HTML-text escaping. Adding handlebars /
//!   askama for one reporter is not worth the dependency surface.
//! - **Offline / corporate-proxy safe**: CSS and JS are embedded via
//!   `include_str!` and copied to `<output_dir>/base.css` and
//!   `<output_dir>/base.js`. Every page also carries a strict
//!   `Content-Security-Policy` `<meta>` tag (`default-src 'self';
//!   connect-src 'none'; font-src 'none'; object-src 'none'; ...`) so the
//!   report makes zero network requests even if served from an HTTP
//!   origin behind an inspecting proxy.
//! - **Progressive enhancement**: without JS the report still renders
//!   correctly; JS adds sortable index tables and an explicit
//!   auto/light/dark toggle that overrides `prefers-color-scheme`.
//! - **Server-side syntax highlighting**: detail-page source views are
//!   tokenized in Rust via [`syntect`] (extended with [`two_face`] for
//!   TypeScript / TSX / JSX coverage) and emitted as
//!   `<span class="stok-...">` markup. No client-side tokenizer, no
//!   flash of unstyled source, works with JS off. Tokenization is
//!   CPU-bound, so per-file rendering fans out across cores via [`rayon`].
//! - **Graceful missing-source**: if a file's source cannot be read from
//!   disk (CI runs without the original tree, remapped path that does
//!   not exist locally), the detail page shows a notice with the attempted
//!   path and search root rather than silently rendering blank source.
//!
//! [syntect]: https://docs.rs/syntect
//! [two_face]: https://docs.rs/two-face
//! [rayon]: https://docs.rs/rayon

mod assets;
mod detail_page;
mod highlight;
mod index_page;
mod line_coverage;
mod options;
mod output;
mod page;
mod paths;

use std::io;
use std::path::{Component, Path};

use oxc_coverage_report::{CoverageMap, NodeKind, ReportNode, summarize};
use oxc_coverage_source_maps::remap_coverage_map;
use rayon::prelude::*;

use assets::{BASE_CSS, BASE_JS, COVERAGE_TOKENS_CSS};
use detail_page::render_detail;
use index_page::render_folder_index;
use output::OutputDir;
use paths::PhysicalPaths;

pub use options::{HtmlOptions, HtmlOptionsError};

/// Render a complete HTML coverage report into `output_dir`.
///
/// `coverage_map` may carry `inputSourceMap` entries; they are remapped
/// through [`oxc_coverage_source_maps::remap_coverage_map`] before the
/// report tree is built, so detail pages show original TS/JSX source.
///
/// `root_dir` is used as the filesystem base for resolving any
/// `FileCoverage.path` that is relative. Pass [`std::env::current_dir`]
/// when invoking from a CLI.
///
/// Three companion files are written alongside the HTML tree:
/// `base.css`, `coverage-tokens.css` (which `base.css` `@import`s), and
/// `base.js`. All three must travel together; copying only `base.css`
/// to a new location without `coverage-tokens.css` leaves the report
/// visually broken (the token cascade resolves to the browser default).
///
/// The caller supplies [`HtmlOptions`] so the public API has a single
/// configured path instead of parallel default-vs-custom entry points.
///
/// # Errors
/// Returns [`io::Error`] if:
///   * a coverage path is not a plain relative path, with kind `InvalidInput`
///   * a component of `output_dir` is a symlink or is replaced by one while
///     the report is being written, with kind `InvalidInput`
///   * creating a directory or writing a page fails
pub fn write(
    coverage_map: &CoverageMap,
    root_dir: &Path,
    output_dir: &Path,
    options: &HtmlOptions,
) -> io::Result<()> {
    write_report(coverage_map, root_dir, output_dir, options, || Ok(()))
}

/// `output_opened` runs after the output root is opened and before any page
/// is written. Only the symlink-race test passes a non-trivial hook, to
/// replace a directory component with a symlink at exactly that instant.
fn write_report(
    coverage_map: &CoverageMap,
    root_dir: &Path,
    output_dir: &Path,
    options: &HtmlOptions,
    output_opened: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    // Only pay for the remap (and its clone plus orphan-counter prune) when at
    // least one entry carries an `inputSourceMap`; a map with none renders
    // identically from the borrowed original. The skipped branch therefore does
    // not run `FileCoverage::prune_orphan_counters`, which is safe because
    // every counter consumer below iterates `statementMap` / `fnMap` /
    // `branchMap` and reads the `s` / `f` / `b` slot via
    // `.get(id).unwrap_or(0)`, so an orphan slot is never observed. Revisit if
    // this function ever re-serializes a raw `FileCoverage`.
    let remapped;
    let report_map = if coverage_map.values().any(|coverage| coverage.input_source_map.is_some()) {
        remapped = remap_coverage_map(coverage_map);
        &remapped
    } else {
        coverage_map
    };
    let root = summarize(report_map);
    validate_report_paths(&root)?;
    let physical_paths = PhysicalPaths::build(&root)?;
    let output = OutputDir::open(output_dir)?;
    output_opened()?;

    let ctx = RenderContext { root_dir, options, physical_paths: &physical_paths };
    render_node(&root, &ctx, &output, 0)?;

    output.write(Path::new("base.css"), BASE_CSS.as_bytes())?;
    output.write(Path::new("coverage-tokens.css"), COVERAGE_TOKENS_CSS.as_bytes())?;
    output.write(Path::new("base.js"), BASE_JS.as_bytes())?;
    Ok(())
}

fn validate_report_paths(node: &ReportNode) -> io::Result<()> {
    if let NodeKind::Folder { children } = &node.kind {
        for child in children {
            validate_report_paths(child)?;
        }
    }

    if !node.relative_path.is_empty()
        && !node.relative_path.split('/').all(is_safe_report_path_component)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsafe HTML report path: {}", node.relative_path),
        ));
    }

    Ok(())
}

fn is_safe_report_path_component(component: &str) -> bool {
    let bytes = component.as_bytes();
    if component.contains('\\')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
    {
        return false;
    }

    let mut components = Path::new(component).components();
    matches!((components.next(), components.next()), (Some(Component::Normal(_)), None))
}

struct RenderContext<'a> {
    root_dir: &'a Path,
    options: &'a HtmlOptions,
    physical_paths: &'a PhysicalPaths,
}

/// `depth` drives the `../` prefix on stylesheet, script and breadcrumb
/// hrefs: a folder's `index.html` sits one directory deeper than its parent's,
/// while a file's page renders beside it.
fn render_node(
    node: &ReportNode,
    ctx: &RenderContext<'_>,
    output: &OutputDir,
    depth: usize,
) -> io::Result<()> {
    match &node.kind {
        NodeKind::Folder { children } => {
            let html = render_folder_index(node, children, ctx, depth);
            output.write(ctx.physical_paths.output_path(node), html.as_bytes())?;

            // Detail pages are CPU-bound on syntect tokenization, so fanning
            // the children out scales with the cores a CI runner has. Folder
            // children re-enter here and parallelize their own subtrees.
            children
                .par_iter()
                .map(|child| {
                    render_node(child, ctx, output, depth + usize::from(child.is_folder()))
                })
                .collect::<io::Result<Vec<_>>>()?;
        }
        NodeKind::File { coverage } => {
            let html = render_detail(node, coverage, ctx, depth);
            output.write(ctx.physical_paths.output_path(node), html.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
