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
//!   [`oxc_coverage_source_maps::remap_coverage_map`] BEFORE summarization,
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
//!   flash of unstyled source, works with JS off. Per-file rendering
//!   parallelizes across cores via [`rayon`]: on a 100-file project at
//!   200 LOC per file emit takes under two seconds on a typical laptop.
//! - **Graceful missing-source**: if a file's source cannot be read from
//!   disk (CI runs without the original tree, remapped path that does
//!   not exist locally), the detail page shows a notice with the attempted
//!   path and search root rather than silently rendering blank source.
//!
//! [syntect]: https://docs.rs/syntect
//! [two_face]: https://docs.rs/two-face
//! [rayon]: https://docs.rs/rayon

mod highlight;

use crate::escape::{html_attr, html_text};
use oxc_coverage_report::{CoverageMap, CoverageSummary, Metric, NodeKind, ReportNode, summarize};
use oxc_coverage_source_maps::remap_coverage_map;
use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Filename used for every folder-level index page.
const INDEX_FILE: &str = "index.html";

/// Suffix appended to every per-file detail page (`a.js` -> `a.js.html`).
const DETAIL_SUFFIX: &str = ".html";

/// Embedded stylesheet copied to `<output_dir>/base.css`.
const BASE_CSS: &str = include_str!("base.css");

/// Vendored slice of fallow's design tokens, copied to
/// `<output_dir>/coverage-tokens.css`. `base.css` `@import`s this file
/// so the palette layer can be re-vendored into fallow-cloud (or any
/// other consumer) without touching the structural rules. See the file
/// header comment for the sync convention.
const COVERAGE_TOKENS_CSS: &str = include_str!("coverage-tokens.css");

/// Version stamp embedded in the `<meta name="generator">` tag on every
/// page. Lets CI tooling identify which release of the reporter
/// produced a given report directory.
const GENERATOR: &str = concat!("oxc-coverage-reports ", env!("CARGO_PKG_VERSION"));

/// Embedded enhancement script copied to `<output_dir>/base.js`.
///
/// Provides the sortable index tables and the auto / light / dark theme
/// toggle. Pure DOM API, never assigns HTML strings, never makes a
/// network request, so the page stays compatible with the strict CSP
/// emitted by [`render_page`]. Syntax highlighting is done server-side
/// in Rust via [`syntect`] in the sibling [`highlight`] module.
const BASE_JS: &str = include_str!("base.js");

/// Strict Content-Security-Policy applied to every emitted page.
///
/// - `default-src 'self'`: only same-origin scripts, styles, images.
/// - `connect-src 'none'`: no `fetch` / `XMLHttpRequest` / `WebSocket`.
/// - `font-src 'none'`: no web-font fetches (we use system fonts only).
/// - `object-src 'none'`: no embeds / plugins.
/// - `base-uri 'self'`: prevents `<base>` tag tampering by injection.
/// - `form-action 'none'`: no form submissions (we have no forms).
///
/// The combined effect is that opening a report in a browser, even one
/// served from a corporate HTTP proxy, performs zero outbound requests
/// to anything other than the report's own directory.
const CSP_POLICY: &str = "default-src 'self'; connect-src 'none'; \
font-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'";

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
pub fn write(
    coverage_map: &CoverageMap,
    root_dir: &Path,
    output_dir: &Path,
    options: &HtmlOptions,
) -> io::Result<()> {
    let remapped = remap_coverage_map(coverage_map);
    let root = summarize(&remapped);

    fs::create_dir_all(output_dir)?;
    fs::write(output_dir.join("base.css"), BASE_CSS)?;
    fs::write(output_dir.join("coverage-tokens.css"), COVERAGE_TOKENS_CSS)?;
    fs::write(output_dir.join("base.js"), BASE_JS)?;

    let ctx = RenderContext { root_dir, options };
    render_node(RenderNodeInput { node: &root, ctx: &ctx, output_dir, depth: 0 })?;
    Ok(())
}

/// Tunable knobs for the HTML reporter.
///
/// Construct with [`HtmlOptions::new`] so invalid thresholds cannot bypass
/// CLI-side validation when the library is used directly.
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /// Percentage cutoff that separates "high" coverage (green) from
    /// "medium" (amber) on per-metric colouring, and powers the index
    /// page's "N of M files fall below the X% coverage threshold"
    /// sentence. The medium-to-low boundary stays fixed at 50%.
    /// Must be in `[0.0, 100.0]`.
    /// Default: `80.0` (Istanbul's traditional value).
    green_threshold: f64,
}

/// Invalid HTML reporter options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HtmlOptionsError {
    value: f64,
}

impl std::fmt::Display for HtmlOptionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "html green threshold must be a finite number in [0, 100], got {}", self.value)
    }
}

impl std::error::Error for HtmlOptionsError {}

impl HtmlOptions {
    pub fn new(green_threshold: f64) -> Result<Self, HtmlOptionsError> {
        if !green_threshold.is_finite() || !(0.0..=100.0).contains(&green_threshold) {
            return Err(HtmlOptionsError { value: green_threshold });
        }
        Ok(Self { green_threshold })
    }

    pub fn green_threshold(&self) -> f64 {
        self.green_threshold
    }
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self { green_threshold: 80.0 }
    }
}

struct RenderContext<'a> {
    root_dir: &'a Path,
    options: &'a HtmlOptions,
}

#[derive(Clone, Copy)]
struct RenderNodeInput<'a> {
    node: &'a ReportNode,
    ctx: &'a RenderContext<'a>,
    output_dir: &'a Path,
    depth: usize,
}

fn render_node(input: RenderNodeInput<'_>) -> io::Result<()> {
    let RenderNodeInput { node, ctx, output_dir, depth } = input;
    match &node.kind {
        NodeKind::Folder { children } => {
            // Folder pages live at `<output_dir>/<node.relative_path>/index.html`.
            let folder_dir = if node.relative_path.is_empty() {
                output_dir.to_path_buf()
            } else {
                output_dir.join(&node.relative_path)
            };
            fs::create_dir_all(&folder_dir)?;
            let html = render_folder_index(FolderIndexInputs { node, children, ctx, depth });
            fs::write(folder_dir.join(INDEX_FILE), html)?;

            // Render children in parallel. The file branch is CPU-bound
            // (syntect tokenization), so per-file fan-out scales well on
            // multi-core CI runners. Folder children re-enter
            // `render_node` and parallelize their own subtrees.
            children
                .par_iter()
                .map(|child| {
                    render_node(RenderNodeInput {
                        node: child,
                        ctx,
                        output_dir,
                        depth: depth + child_depth_delta(node, child),
                    })
                })
                .collect::<io::Result<Vec<_>>>()?;
        }
        NodeKind::File { coverage } => {
            // Detail page lives next to the folder index.
            let parent = node.relative_path.rsplit_once('/').map_or("", |(parent, _)| parent);
            let detail_dir =
                if parent.is_empty() { output_dir.to_path_buf() } else { output_dir.join(parent) };
            fs::create_dir_all(&detail_dir)?;

            let filename = format!("{}{DETAIL_SUFFIX}", &node.name);
            let html = render_detail(RenderDetailInputs { node, coverage, ctx, depth });
            fs::write(detail_dir.join(filename), html)?;
        }
    }
    Ok(())
}

/// Depth in the page tree (number of `..` segments needed to reach the
/// report root). Used to compute relative `<link rel="stylesheet">` and
/// breadcrumb hrefs. The root folder is depth 0; a folder at `src/` is
/// depth 1; a file `src/a.js` is depth 1 (the file is rendered in the
/// `src/` directory).
fn child_depth_delta(_parent: &ReportNode, child: &ReportNode) -> usize {
    // Depth grows when the child is a folder (its index.html sits in a
    // deeper directory). File children render in the same directory as
    // the parent's index.html, so depth does not change for them.
    usize::from(matches!(child.kind, NodeKind::Folder { .. }))
}

// -- Folder index page ------------------------------------------------------

#[derive(Clone, Copy)]
struct FolderIndexInputs<'a> {
    node: &'a ReportNode,
    children: &'a [ReportNode],
    ctx: &'a RenderContext<'a>,
    depth: usize,
}

fn render_folder_index(inputs: FolderIndexInputs<'_>) -> String {
    let FolderIndexInputs { node, children, ctx, depth } = inputs;
    let title = if node.relative_path.is_empty() {
        "All files".to_owned()
    } else {
        node.relative_path.clone()
    };
    let threshold = ctx.options.green_threshold();
    let mut body = String::new();
    body.push_str(&render_summary_header(SummaryHeaderInputs {
        title: &title,
        summary: &node.summary,
        depth,
        node,
        threshold,
    }));
    body.push_str("    <div class=\"pad1\">\n");
    body.push_str(&render_threshold_summary(children, threshold));
    body.push_str(&render_filter_group(children.len()));
    body.push_str("      <table class=\"coverage-summary\" id=\"cov-file-table\">\n");
    body.push_str(&render_summary_table_header());
    body.push_str("        <tbody>\n");
    for child in children {
        body.push_str(&render_summary_row(child, threshold));
    }
    body.push_str("        </tbody>\n");
    body.push_str("      </table>\n");
    body.push_str("    </div>\n");
    render_page(&title, depth, &body)
}

/// One-line summary above the file table: "N of M files fall below the
/// X% coverage threshold". The wording deliberately says "coverage
/// threshold" rather than "line-coverage threshold" because the same
/// `threshold` value drives the colour bucketing for every metric
/// (statements, branches, functions, lines), and the file-below count
/// is itself computed against line coverage as the canonical proxy
/// for "is this file healthy enough". The file-list inclusion still
/// uses `lines.pct` so a file with 95% lines and 30% branches is
/// considered "above"; the bucketed colour cells then surface the
/// underlying per-metric reality.
fn render_threshold_summary(children: &[ReportNode], threshold: f64) -> String {
    let total = children.len();
    if total == 0 {
        return String::new();
    }
    let below = children.iter().filter(|c| c.summary.lines.pct < threshold).count();
    let attrs = format!(
        "id=\"cov-threshold-summary\" aria-live=\"polite\" data-threshold-pct=\"{threshold:.2}\" data-total-files=\"{total}\" data-below-files=\"{below}\""
    );
    if below == 0 {
        return format!(
            "      <p class=\"threshold-summary\" {attrs}><strong>All {total} files</strong> meet the {threshold:.0}% coverage threshold.</p>\n",
        );
    }
    format!(
        "      <p class=\"threshold-summary\" {attrs}><strong>{below}</strong> of {total} files fall below the {threshold:.0}% coverage threshold.</p>\n",
    )
}

/// Filter input + live count region. JS enhances both; without JS the
/// input is still focusable and the region stays empty.
fn render_filter_group(file_count: usize) -> String {
    format!(
        concat!(
            "      <div class=\"filter-group\">\n",
            "        <label class=\"filter-group__label\" for=\"cov-filter\">Filter files</label>\n",
            "        <input class=\"filter-input\" id=\"cov-filter\" type=\"search\" autocomplete=\"off\" spellcheck=\"false\" placeholder=\"type to filter, press / to focus\" aria-controls=\"cov-file-table\" aria-describedby=\"cov-filter-count\">\n",
            "        <div class=\"filter-count\" id=\"cov-filter-count\" role=\"status\" aria-atomic=\"true\" data-total=\"{total}\"></div>\n",
            "      </div>\n",
        ),
        total = file_count,
    )
}

fn render_summary_table_header() -> String {
    let mut out = String::from("        <thead><tr>");
    for col in ["File", "Statements", "Branches", "Functions", "Lines"] {
        let _ = write!(out, "<th>{col}</th>");
    }
    out.push_str("</tr></thead>\n");
    out
}

fn render_summary_row(child: &ReportNode, threshold: f64) -> String {
    let href = match &child.kind {
        NodeKind::Folder { .. } => format!("{}/{INDEX_FILE}", html_attr(&child.name)),
        NodeKind::File { .. } => format!("{}{DETAIL_SUFFIX}", html_attr(&child.name)),
    };
    let display = html_text(&child.name);
    let row_class = pct_class(child.summary.lines.pct, threshold);
    let mut out = format!(
        "          <tr class=\"row-{row_class}\" data-file=\"{file}\" data-lines-pct=\"{lines:.2}\">\n",
        file = html_attr(&child.name),
        lines = child.summary.lines.pct,
    );
    let _ = writeln!(out, "            <td class=\"file\"><a href=\"{href}\">{display}</a></td>");
    let metrics = [
        ("Statements", child.summary.statements),
        ("Branches", child.summary.branches),
        ("Functions", child.summary.functions),
        ("Lines", child.summary.lines),
    ];
    for (idx, (label, metric)) in metrics.iter().enumerate() {
        out.push_str(&render_summary_metric_cell(
            label,
            *metric,
            threshold,
            idx == metrics.len() - 1,
        ));
    }
    out.push_str("          </tr>\n");
    out
}

fn render_summary_metric_cell(
    label: &str,
    metric: Metric,
    threshold: f64,
    include_meter: bool,
) -> String {
    let mc = pct_class(metric.pct, threshold);
    let meter = if include_meter {
        let pct = metric.pct.clamp(0.0, 100.0);
        format!(
            "<span class=\"cov-meter cov-meter--{mc}\" role=\"meter\" aria-label=\"{label} coverage\" aria-valuenow=\"{val}\" aria-valuemin=\"0\" aria-valuemax=\"100\"><span class=\"cov-meter__fill\" style=\"width:{val}%\" aria-hidden=\"true\"></span></span>",
            val = pct as u32,
        )
    } else {
        String::new()
    };
    format!(
        "            <td class=\"pct {mc}\">{:.2}%<span class=\"quiet\"> ({}/{})</span>{meter}</td>\n",
        metric.pct, metric.covered, metric.total,
    )
}

// -- File detail page -------------------------------------------------------

#[derive(Clone, Copy)]
struct RenderDetailInputs<'a> {
    node: &'a ReportNode,
    coverage: &'a FileCoverage,
    ctx: &'a RenderContext<'a>,
    depth: usize,
}

fn render_detail(inputs: RenderDetailInputs<'_>) -> String {
    let RenderDetailInputs { node, coverage, ctx, depth } = inputs;
    let title = node.relative_path.clone();
    let source = read_source(coverage, ctx.root_dir);
    let line_hits = compute_line_hits(coverage);
    let branched_lines = compute_branched_lines(coverage);
    let fn_lines = compute_fn_lines(coverage);

    let mut body = String::new();
    body.push_str(&render_summary_header(SummaryHeaderInputs {
        title: &title,
        summary: &node.summary,
        depth,
        node,
        threshold: ctx.options.green_threshold(),
    }));
    body.push_str("    <div class=\"pad1\">\n");
    body.push_str(
        "      <div class=\"detail-actions\">\n        <button type=\"button\" class=\"btn-ghost\" id=\"cov-next-uncovered\" aria-label=\"Jump to next uncovered line\" disabled>Next uncovered</button>\n      </div>\n",
    );
    body.push_str(&render_detail_source(DetailSourceInputs {
        source,
        coverage_path: &coverage.path,
        line_hits: &line_hits,
        branched_lines: &branched_lines,
        fn_lines: &fn_lines,
    }));
    body.push_str("    </div>\n");
    body.push_str(
        "    <div class=\"copy-toast\" id=\"cov-copy-toast\" role=\"status\" aria-atomic=\"true\"></div>\n",
    );
    render_page(&title, depth, &body)
}

struct DetailSourceInputs<'a> {
    source: Result<String, MissingSource>,
    coverage_path: &'a str,
    line_hits: &'a BTreeMap<u32, u32>,
    branched_lines: &'a BTreeMap<u32, BranchSummary>,
    fn_lines: &'a BTreeMap<u32, u32>,
}

fn render_detail_source(inputs: DetailSourceInputs<'_>) -> String {
    let DetailSourceInputs { source, coverage_path, line_hits, branched_lines, fn_lines } = inputs;
    let mut out = String::new();
    match source {
        Ok(text) => {
            let highlighted = highlight::highlight_lines(&text, Path::new(coverage_path));
            out.push_str("      <table class=\"source\">\n");
            out.push_str(&render_source_table(&highlighted, line_hits, branched_lines, fn_lines));
            out.push_str("      </table>\n");
        }
        Err(missing) => {
            out.push_str(&render_source_unavailable_notice(&missing));
            out.push_str("      <table class=\"source\">\n");
            out.push_str(&render_source_unavailable(line_hits));
            out.push_str("      </table>\n");
        }
    }
    out
}

fn render_source_table(
    lines: &[String],
    line_hits: &BTreeMap<u32, u32>,
    branched: &BTreeMap<u32, BranchSummary>,
    fns: &BTreeMap<u32, u32>,
) -> String {
    let mut out = String::new();
    out.push_str("        <thead><tr><th class=\"line-num\">Line</th><th class=\"hits\">Hits</th><th class=\"src\">Source</th></tr></thead>\n        <tbody>\n");
    for (idx, line_html) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let stmt_hits = line_hits.get(&line_no).copied();
        let branch = branched.get(&line_no);
        let fn_hits = fns.get(&line_no).copied();
        out.push_str(&render_source_row(SourceRow {
            line_no,
            src_html: line_html,
            stmt_hits,
            branch,
            fn_hits,
        }));
    }
    out.push_str("        </tbody>\n");
    out
}

#[derive(Clone, Copy)]
struct SourceRow<'a> {
    line_no: u32,
    src_html: &'a str,
    stmt_hits: Option<u32>,
    branch: Option<&'a BranchSummary>,
    fn_hits: Option<u32>,
}

/// Render one source row. `src_html` is already escaped and may carry
/// syntect `<span class="stok-...">` markup; do not re-escape.
fn render_source_row(row: SourceRow<'_>) -> String {
    let SourceRow { line_no, src_html, stmt_hits, branch, fn_hits } = row;
    let class = source_row_class(stmt_hits, branch, fn_hits);
    let hits_text = match (stmt_hits, fn_hits) {
        (Some(h), _) | (None, Some(h)) => format!("{h}x"),
        _ => String::new(),
    };
    let fn_note = if matches!(fn_hits, Some(0)) && stmt_hits.is_some_and(|h| h > 0) {
        "<span class=\"fn-note\" aria-hidden=\"true\">fn 0x</span>"
    } else {
        ""
    };
    let glyph = severity_glyph(class);
    let aria = row_aria_label(RowAriaInput { line_no, class, branch, fn_hits });
    let branch_note = render_branch_note(branch);
    format!(
        "          <tr class=\"line {class}\" id=\"L{line_no}\" aria-label=\"{aria}\">\
<td class=\"line-num\">\
<button type=\"button\" class=\"line-anchor\" data-line=\"{line_no}\" aria-label=\"Copy link to line {line_no}\">{line_no}</button>\
</td>\
<td class=\"hits\">{glyph}{hits_text}{fn_note}</td>\
<td class=\"src\"><pre>{src_html}</pre>{branch_note}</td>\
</tr>\n",
    )
}

fn render_branch_note(branch: Option<&BranchSummary>) -> String {
    branch.map_or(String::new(), |b| {
        if b.total == 0 {
            String::new()
        } else if let Some(detail) = b.detail_html() {
            format!(" <span class=\"branch-note\" aria-hidden=\"true\">{detail}</span>")
        } else {
            format!(
                " <span class=\"branch-note\" aria-hidden=\"true\">branches {}/{}</span>",
                b.covered, b.total
            )
        }
    })
}

/// Non-color cue placed in the hits column. Coverage status is also
/// already on the `<tr>` class; the glyph is `aria-hidden` so screen
/// readers don't double-announce alongside the `aria-label` on the row.
fn severity_glyph(class: &str) -> &'static str {
    match class {
        "hit" => "<span class=\"sev-glyph sev-glyph--hit\" aria-hidden=\"true\">[H]</span>",
        "miss" => "<span class=\"sev-glyph sev-glyph--miss\" aria-hidden=\"true\">[M]</span>",
        "partial" => "<span class=\"sev-glyph sev-glyph--partial\" aria-hidden=\"true\">[P]</span>",
        _ => "",
    }
}

/// Human-readable status used as the row's `aria-label`. Avoids the
/// raw class tokens (`miss` etc.) that would otherwise leak into AT
/// output. Returned values are safe to drop into an HTML attribute
/// because they are bounded to known phrases.
#[derive(Clone, Copy)]
struct RowAriaInput<'a> {
    line_no: u32,
    class: &'a str,
    branch: Option<&'a BranchSummary>,
    fn_hits: Option<u32>,
}

fn row_aria_label(input: RowAriaInput<'_>) -> String {
    let RowAriaInput { line_no, class, branch, fn_hits } = input;
    let phrase = match class {
        "hit" => "covered",
        "miss" => "not covered",
        "partial" => "partially covered",
        _ => "no statement",
    };
    let mut out = format!("Line {line_no}, {phrase}");
    if let Some(details) = branch.and_then(BranchSummary::detail_aria) {
        out.push_str(": ");
        out.push_str(&details);
    }
    if matches!(fn_hits, Some(0)) {
        if branch.and_then(BranchSummary::detail_aria).is_some() {
            out.push_str("; ");
        } else {
            out.push_str(": ");
        }
        out.push_str("function not called");
    }
    out
}

fn source_row_class(
    stmt_hits: Option<u32>,
    branch: Option<&BranchSummary>,
    _fn_hits: Option<u32>,
) -> &'static str {
    if let Some(hits) = stmt_hits {
        if hits == 0 {
            return "miss";
        }
        if let Some(b) = branch
            && b.total > 0
            && b.covered < b.total
        {
            return "partial";
        }
        "hit"
    } else {
        "no-stmt"
    }
}

fn render_source_unavailable_notice(missing: &MissingSource) -> String {
    format!(
        "      <p class=\"source-unavailable\">Source file unavailable at <code>{}</code> (search root: <code>{}</code>).</p>\n",
        html_text(&missing.display_path),
        html_text(&missing.search_root),
    )
}

fn render_source_unavailable(line_hits: &BTreeMap<u32, u32>) -> String {
    let mut out = String::from(
        "        <thead><tr><th class=\"line-num\">Line</th><th class=\"hits\">Hits</th><th class=\"src\">Source</th></tr></thead>\n        <tbody>\n",
    );
    for (line, hits) in line_hits {
        let class = if *hits == 0 { "miss" } else { "hit" };
        let glyph = severity_glyph(class);
        let aria =
            row_aria_label(RowAriaInput { line_no: *line, class, branch: None, fn_hits: None });
        let _ = writeln!(
            out,
            "          <tr class=\"line {class}\" id=\"L{line}\" aria-label=\"{aria}\">\
<td class=\"line-num\">\
<button type=\"button\" class=\"line-anchor\" data-line=\"{line}\" aria-label=\"Copy link to line {line}\">{line}</button>\
</td>\
<td class=\"hits\">{glyph}{hits}x</td>\
<td class=\"src\"><pre>(source unavailable)</pre></td>\
</tr>",
        );
    }
    out.push_str("        </tbody>\n");
    out
}

// -- Shared page chrome -----------------------------------------------------

fn render_page(title: &str, depth: usize, body: &str) -> String {
    let css_href = relative_to_root(depth, "base.css");
    let js_href = relative_to_root(depth, "base.js");
    let mut out = String::with_capacity(body.len() + 1024);
    out.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("  <meta charset=\"utf-8\">\n");
    out.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    let _ = writeln!(out, "  <meta name=\"generator\" content=\"{}\">", html_attr(GENERATOR));
    let _ = writeln!(
        out,
        "  <meta http-equiv=\"Content-Security-Policy\" content=\"{}\">",
        html_attr(CSP_POLICY),
    );
    let _ = writeln!(out, "  <title>Coverage: {}</title>", html_text(title));
    let _ = writeln!(out, "  <link rel=\"stylesheet\" href=\"{}\">", html_attr(&css_href));
    let _ = writeln!(out, "  <script src=\"{}\" defer></script>", html_attr(&js_href));
    out.push_str("</head>\n<body>\n");
    out.push_str(body);
    out.push_str("</body>\n</html>\n");
    out
}

#[derive(Clone, Copy)]
struct SummaryHeaderInputs<'a> {
    title: &'a str,
    summary: &'a CoverageSummary,
    depth: usize,
    node: &'a ReportNode,
    threshold: f64,
}

fn render_summary_header(inputs: SummaryHeaderInputs<'_>) -> String {
    let SummaryHeaderInputs { title, summary, depth, node, threshold } = inputs;
    let mut out = String::from("    <header class=\"summary\">\n");
    let _ = writeln!(out, "      <h1>{}</h1>", html_text(title));
    out.push_str(&render_breadcrumb(node, depth));
    out.push_str("      <ul class=\"kpi-row\" aria-label=\"Coverage metrics\">\n");
    for (label, m) in [
        ("Statements", summary.statements),
        ("Branches", summary.branches),
        ("Functions", summary.functions),
        ("Lines", summary.lines),
    ] {
        let class = pct_class(m.pct, threshold);
        let _ = writeln!(
            out,
            "        <li class=\"kpi-cell kpi-cell--{class}\"><span class=\"kpi-cell__label\">[ {label} ]</span><span class=\"kpi-cell__value\">{:.2}%</span><span class=\"kpi-cell__sub\">{}/{}</span></li>",
            m.pct, m.covered, m.total
        );
    }
    out.push_str("      </ul>\n");
    out.push_str("    </header>\n");
    out
}

fn render_breadcrumb(node: &ReportNode, depth: usize) -> String {
    if node.relative_path.is_empty() {
        return String::new();
    }
    let mut out = String::from("      <nav class=\"breadcrumb\" aria-label=\"Breadcrumb\">\n");
    out.push_str("        <ol>\n");
    let root_href = relative_to_root(depth, INDEX_FILE);
    let _ = writeln!(
        out,
        "          <li><a href=\"{}\">All files</a><span class=\"breadcrumb-sep\" aria-hidden=\"true\">/</span></li>",
        html_attr(&root_href),
    );

    out.push_str(&render_breadcrumb_items(&node.relative_path, depth));
    out.push_str("        </ol>\n");
    out.push_str("      </nav>\n");
    out
}

fn render_breadcrumb_items(relative_path: &str, depth: usize) -> String {
    let parts: Vec<&str> = relative_path.split('/').collect();
    let mut depth_remaining = depth;
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        let is_last = i + 1 == parts.len();
        if is_last {
            let _ = writeln!(
                out,
                "          <li><span class=\"current\" aria-current=\"page\">{}</span></li>",
                html_text(part),
            );
        } else {
            depth_remaining = depth_remaining.saturating_sub(1);
            let href = relative_to_root(depth_remaining, INDEX_FILE);
            let _ = writeln!(
                out,
                "          <li><a href=\"{}\">{}</a><span class=\"breadcrumb-sep\" aria-hidden=\"true\">/</span></li>",
                html_attr(&href),
                html_text(part),
            );
        }
    }
    out
}

fn relative_to_root(depth: usize, file: &str) -> String {
    if depth == 0 {
        return file.to_owned();
    }
    let mut out = String::with_capacity(depth * 3 + file.len());
    for _ in 0..depth {
        out.push_str("../");
    }
    out.push_str(file);
    out
}

// -- Per-file analysis helpers ---------------------------------------------

struct BranchSummary {
    total: u32,
    covered: u32,
    arms: Vec<BranchArm>,
}

struct BranchArm {
    label: String,
    hits: u32,
}

impl BranchSummary {
    fn detail_html(&self) -> Option<String> {
        if self.arms.is_empty() || self.arms.len() > 4 {
            return None;
        }
        let mut out = String::from("branches:");
        for arm in &self.arms {
            let mark = if arm.hits > 0 { "hit" } else { "miss" };
            let symbol = if arm.hits > 0 { "&#10003;" } else { "&#10007;" };
            let _ = write!(
                out,
                " <span class=\"branch-arm branch-arm--{mark}\">{symbol} {}={}</span>",
                html_text(&arm.label),
                arm.hits
            );
        }
        Some(out)
    }

    fn detail_aria(&self) -> Option<String> {
        if self.arms.is_empty() || self.arms.len() > 4 {
            return None;
        }
        let mut parts = Vec::with_capacity(self.arms.len());
        for arm in &self.arms {
            let state = if arm.hits > 0 { "hit" } else { "missed" };
            parts.push(format!("{} arm {state} {} times", arm.label, arm.hits));
        }
        Some(parts.join(", "))
    }
}

fn compute_line_hits(file: &FileCoverage) -> BTreeMap<u32, u32> {
    let mut by_line: BTreeMap<u32, u32> = BTreeMap::new();
    for (id, loc) in &file.statement_map {
        let line = loc.start.line;
        if line == 0 {
            continue;
        }
        let hits = file.s.get(id).copied().unwrap_or(0);
        by_line.entry(line).and_modify(|v| *v = (*v).max(hits)).or_insert(hits);
    }
    by_line
}

fn compute_branched_lines(file: &FileCoverage) -> BTreeMap<u32, BranchSummary> {
    let mut out: BTreeMap<u32, BranchSummary> = BTreeMap::new();
    for (id, entry) in &file.branch_map {
        let line = branch_line(entry);
        if line == 0 {
            continue;
        }
        let arms = file.b.get(id).cloned().unwrap_or_default();
        let total = arms.len() as u32;
        let covered = arms.iter().filter(|&&v| v > 0).count() as u32;
        let labeled_arms = branch_arms(entry, &arms);
        out.entry(line)
            .and_modify(|s| {
                s.total += total;
                s.covered += covered;
                s.arms.extend(branch_arms(entry, &arms));
            })
            .or_insert(BranchSummary { total, covered, arms: labeled_arms });
    }
    out
}

fn branch_arms(entry: &BranchEntry, hits: &[u32]) -> Vec<BranchArm> {
    hits.iter()
        .enumerate()
        .map(|(idx, &hits)| BranchArm { label: branch_arm_label(entry, idx), hits })
        .collect()
}

fn branch_arm_label(entry: &BranchEntry, idx: usize) -> String {
    match entry.branch_type.as_str() {
        "if" => match idx {
            0 => "true".to_owned(),
            1 => "false".to_owned(),
            _ => format!("arm {}", idx + 1),
        },
        "cond-expr" => match idx {
            0 => "consequent".to_owned(),
            1 => "alternate".to_owned(),
            _ => format!("arm {}", idx + 1),
        },
        "switch" => format!("case {}", idx + 1),
        "binary-expr" => format!("operand {}", idx + 1),
        "default-arg" => "default".to_owned(),
        _ => format!("arm {}", idx + 1),
    }
}

fn compute_fn_lines(file: &FileCoverage) -> BTreeMap<u32, u32> {
    let mut out: BTreeMap<u32, u32> = BTreeMap::new();
    for (id, entry) in &file.fn_map {
        let line = fn_line(entry);
        if line == 0 {
            continue;
        }
        let hits = file.f.get(id).copied().unwrap_or(0);
        out.entry(line).and_modify(|v| *v = (*v).max(hits)).or_insert(hits);
    }
    out
}

fn branch_line(entry: &BranchEntry) -> u32 {
    if entry.line > 0 {
        entry.line
    } else {
        entry.locations.first().map_or(0, |loc| loc.start.line)
    }
}

fn fn_line(entry: &FnEntry) -> u32 {
    if entry.line > 0 { entry.line } else { entry.decl.start.line }
}

struct MissingSource {
    display_path: String,
    search_root: String,
}

fn read_source(file: &FileCoverage, root_dir: &Path) -> Result<String, MissingSource> {
    if file.path.is_empty() {
        return Err(MissingSource {
            display_path: "(empty coverage path)".to_owned(),
            search_root: root_dir.display().to_string(),
        });
    }
    let path = Path::new(&file.path);
    let absolute: PathBuf =
        if path.is_absolute() { path.to_path_buf() } else { root_dir.join(path) };
    fs::read_to_string(&absolute).map_err(|_| MissingSource {
        display_path: absolute.display().to_string(),
        search_root: root_dir.display().to_string(),
    })
}

// -- Class assignment ------------------------------------------------------

/// Map a percentage to one of `"high"`/`"medium"`/`"low"`, used as the
/// `kpi-cell--*`, `row-*`, `pct *`, and `cov-meter--*` modifier
/// suffixes. `green_threshold` separates high from medium; the medium /
/// low boundary is fixed at 50% (a hardcoded medium threshold below
/// `green_threshold` keeps the three-bucket UX legible while still
/// honouring caller preferences for the green/amber line).
fn pct_class(pct: f64, green_threshold: f64) -> &'static str {
    if pct >= green_threshold {
        "high"
    } else if pct >= 50.0 {
        "medium"
    } else {
        "low"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_types::parse_coverage_map;

    fn write_to_temp(json: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let map = parse_coverage_map(json).unwrap();
        write(&map, Path::new(""), dir.path(), &HtmlOptions::default()).unwrap();
        dir
    }

    #[test]
    fn writes_root_index_and_base_css() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(root_index.contains("<title>Coverage: All files</title>"));
        assert!(root_index.contains("<link rel=\"stylesheet\" href=\"base.css\">"));
        let css = fs::read_to_string(dir.path().join("base.css")).unwrap();
        assert!(css.contains(".kpi-cell"), "base.css missing fallow KPI cell rules");
        let tokens = fs::read_to_string(dir.path().join("coverage-tokens.css")).unwrap();
        assert!(tokens.contains("--hit-bg"), "coverage-tokens.css missing semantic alias");
        assert!(tokens.contains("--font-body"), "coverage-tokens.css missing fallow font stack");
    }

    #[test]
    fn writes_per_file_detail_page() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let detail = fs::read_to_string(dir.path().join("a.js.html")).unwrap();
        assert!(detail.contains("<title>Coverage: a.js</title>"));
        assert!(detail.contains("class=\"breadcrumb\""));
    }

    #[test]
    fn nested_folders_have_correct_css_href_depth() {
        // The summarizer strips common prefixes so a SINGLE file under "src/"
        // would collapse to root. Use one file at root and one nested so the
        // tree actually keeps the "src/" folder.
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        // src/foo.js detail page is at depth 1 -> href should be `../base.css`.
        let detail = fs::read_to_string(dir.path().join("src").join("foo.js.html")).unwrap();
        assert!(
            detail.contains("href=\"../base.css\""),
            "expected ../base.css href in nested file, got:\n{detail}"
        );
        // The root file remains at depth 0 -> href should be `base.css`.
        let root_detail = fs::read_to_string(dir.path().join("a.js.html")).unwrap();
        assert!(
            root_detail.contains("href=\"base.css\""),
            "expected base.css href at root, got:\n{root_detail}"
        );
    }

    #[test]
    fn html_special_chars_are_escaped_in_titles_and_breadcrumbs() {
        // Use '&' as the HTML-special character: it exercises the escaping
        // path (`&` -> `&amp;`) AND is a valid filename on every platform,
        // unlike `<` or `>` which Windows path APIs reject.
        let json = r#"{"a&b.js":{"path":"a&b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let detail = fs::read_to_string(dir.path().join("a&b.js.html")).unwrap();
        assert!(detail.contains("a&amp;b.js"), "got:\n{detail}");
    }

    #[test]
    fn detail_row_class_marks_misses() {
        // statementMap with one statement on line 1, hit count 0 -> "miss" class
        // on the source row. The coverage key uses a relative path resolved
        // against `root_dir` so the same JSON works on every platform; an
        // absolute Windows path (`C:\\...`) would round-trip through the
        // report tree as a non-relative folder component, which Windows path
        // APIs reject.
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "const x = 1;\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":12}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("line miss"), "expected miss class; got:\n{detail}");
    }

    #[test]
    fn writes_base_js_alongside_base_css() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let js = fs::read_to_string(dir.path().join("base.js")).unwrap();
        // Sanity-check the remaining feature blocks are present in the
        // emitted JS. Syntax highlighting moved server-side in G3 so the
        // prettify section is intentionally gone.
        assert!(js.contains("Theme toggle"), "base.js missing theme toggle section");
        assert!(js.contains("Sortable tables"), "base.js missing sortable tables section");
        assert!(js.contains("buildThemeToggle"), "base.js missing theme toggle invocation");
        assert!(js.contains("initSortable"), "base.js missing sortable init invocation");
        assert!(
            js.contains("updateThresholdSummary"),
            "base.js should update the threshold summary when filtering",
        );
        assert!(
            js.contains("table.addEventListener('click'"),
            "line anchors should use a delegated table click handler",
        );
        assert!(
            !js.contains("anchors.forEach"),
            "line anchors should not attach one listener per row",
        );
        // No client-side tokenizer: the source view is pre-rendered by
        // syntect on the Rust side.
        assert!(!js.contains("initPrettify"), "client prettify must not be re-added");
        assert!(!js.contains("KEYWORD_SET"), "client tokenizer must not be re-added");
    }

    #[test]
    fn every_page_carries_csp_and_script_tag() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        for html_path in walkdir(dir.path())
            .into_iter()
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("html"))
        {
            let html = fs::read_to_string(&html_path).unwrap();
            assert!(html.contains("Content-Security-Policy"), "missing CSP meta in {html_path:?}");
            assert!(
                html.contains("connect-src &#39;none&#39;"),
                "CSP lacks connect-src 'none' in {html_path:?}"
            );
            assert!(
                html.contains("font-src &#39;none&#39;"),
                "CSP lacks font-src 'none' in {html_path:?}"
            );
            assert!(html.contains("<script src="), "missing <script> in {html_path:?}");
            assert!(html.contains("base.js"), "page does not reference base.js in {html_path:?}");
            assert!(
                html.contains(" defer></script>"),
                "<script> not marked defer in {html_path:?}"
            );
            assert!(html.contains("viewport"), "missing viewport meta in {html_path:?}");
        }
    }

    #[test]
    fn nested_pages_use_relative_script_href() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let nested = fs::read_to_string(dir.path().join("src").join("foo.js.html")).unwrap();
        assert!(nested.contains("src=\"../base.js\""), "nested script href wrong: {nested}");
        let root = fs::read_to_string(dir.path().join("a.js.html")).unwrap();
        assert!(root.contains("src=\"base.js\""), "root script href wrong: {root}");
    }

    #[test]
    fn emitted_assets_have_no_external_urls() {
        // Corporate-proxy hardening: the report must not contain any
        // absolute external URLs in HTML, CSS, or JS. Any future change
        // that pulls in a CDN dependency must be a deliberate one and
        // will fail this test.
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let banned = [
            "http://",
            "https://",
            "ws://",
            "wss://",
            "//cdn.",
            "fonts.googleapis",
            "fonts.gstatic",
        ];
        for path in walkdir(dir.path()) {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            for needle in banned {
                assert!(
                    !text.contains(needle),
                    "{path:?} contains external reference {needle:?}:\n{text}"
                );
            }
        }
    }

    #[test]
    fn base_css_exposes_theme_override_and_token_classes() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let css = fs::read_to_string(dir.path().join("base.css")).unwrap();
        let tokens = fs::read_to_string(dir.path().join("coverage-tokens.css")).unwrap();
        assert!(tokens.contains(":root[data-theme=\"dark\"]"), "missing dark override in tokens");
        assert!(
            tokens.contains(":root:not([data-theme=\"light\"])"),
            "missing light escape hatch in tokens",
        );
        assert!(css.contains(".sortable"), "missing .sortable selector");
        assert!(css.contains("aria-sort=\"ascending\""), "missing aria-sort hook");
        for cls in [
            ".stok-comment",
            ".stok-keyword",
            ".stok-storage",
            ".stok-string",
            ".stok-constant",
            ".stok-support",
            ".stok-entity",
            ".stok-punctuation",
        ] {
            assert!(css.contains(cls), "missing token class {cls}");
        }
        assert!(css.contains(".theme-toggle__btn"), "missing theme-toggle button class");
    }

    #[test]
    fn index_emits_kpi_pills_with_fallow_bracket_labels() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(root_index.contains("class=\"kpi-row\""), "kpi-row class missing");
        assert!(root_index.contains("kpi-cell--"), "kpi-cell severity modifier missing");
        for label in ["[ Statements ]", "[ Branches ]", "[ Functions ]", "[ Lines ]"] {
            assert!(root_index.contains(label), "missing bracket label {label:?}");
        }
    }

    #[test]
    fn index_emits_inline_cov_meter_on_lines_column() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(
            root_index.contains("class=\"cov-meter"),
            "summary row missing inline mini coverage meter",
        );
        assert!(root_index.contains("role=\"meter\""), "cov-meter must carry role=\"meter\"");
        assert!(
            root_index.contains("aria-valuemax=\"100\""),
            "cov-meter must declare aria-valuemax",
        );
    }

    #[test]
    fn index_emits_threshold_summary_and_filter_group() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(
            root_index.contains("threshold-summary"),
            "index missing threshold summary sentence",
        );
        assert!(
            root_index.contains("id=\"cov-threshold-summary\""),
            "threshold summary should expose a stable id",
        );
        assert!(
            root_index.contains("data-threshold-pct=\"80.00\""),
            "threshold summary should expose its threshold for JS updates",
        );
        assert!(
            root_index.contains("data-lines-pct=\"100.00\""),
            "summary rows should expose line pct for filter-reactive threshold counts",
        );
        assert!(root_index.contains("id=\"cov-filter\""), "filter input missing");
        assert!(
            root_index.contains("<label class=\"filter-group__label\" for=\"cov-filter\""),
            "filter input must have an explicit <label>",
        );
        // `role="status"` already implies `aria-live="polite"`; setting
        // both is redundant per the WAI-ARIA spec. We assert role only.
        assert!(
            root_index.contains("id=\"cov-filter-count\" role=\"status\""),
            "filter result counter must be a polite live region",
        );
        assert!(
            root_index.contains("aria-controls=\"cov-file-table\""),
            "filter input must reference the controlled table",
        );
    }

    #[test]
    fn detail_emits_next_uncovered_button_and_copy_toast() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let detail = fs::read_to_string(dir.path().join("a.js.html")).unwrap();
        assert!(
            detail.contains("id=\"cov-next-uncovered\""),
            "detail page missing Next uncovered button",
        );
        assert!(
            detail.contains("class=\"btn-ghost\""),
            "Next uncovered must use the ghost button style",
        );
        assert!(
            detail.contains("id=\"cov-copy-toast\""),
            "detail page missing copy toast live region",
        );
    }

    #[test]
    fn source_rows_carry_line_anchor_button_and_severity_glyph() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "const x = 1;\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":12}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("id=\"L1\""), "missed line should have id=\"L1\" anchor target");
        assert!(
            detail.contains("class=\"line-anchor\" data-line=\"1\""),
            "missed line should expose a line-anchor button",
        );
        assert!(
            detail.contains("aria-label=\"Copy link to line 1\""),
            "line-anchor button must be self-labeling for screen readers",
        );
        assert!(
            detail.contains("sev-glyph sev-glyph--miss"),
            "missed line should carry the [M] severity glyph",
        );
        assert!(
            detail.contains("aria-label=\"Line 1, not covered\""),
            "missed row should carry a human-readable aria-label",
        );
    }

    #[test]
    fn partial_branch_rows_show_per_arm_counts() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "if (x) y;\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":9}}},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":9}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":4},"end":{"line":1,"column":5}},{"start":{"line":1,"column":6},"end":{"line":1,"column":9}}]}},"s":{"0":1},"f":{},"b":{"0":[12,0]}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("branch-arm--hit"), "hit arm marker missing:\n{detail}");
        assert!(detail.contains("true=12"), "true-arm count missing:\n{detail}");
        assert!(detail.contains("branch-arm--miss"), "miss arm marker missing:\n{detail}");
        assert!(detail.contains("false=0"), "false-arm count missing:\n{detail}");
        assert!(
            detail.contains("true arm hit 12 times, false arm missed 0 times"),
            "aria label should expose branch-arm detail:\n{detail}",
        );
    }

    #[test]
    fn covered_function_declaration_line_gets_fn_zero_cue_not_miss_class() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "function f() {}\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":15}}},"fnMap":{"0":{"name":"f","line":1,"decl":{"start":{"line":1,"column":0},"end":{"line":1,"column":10}},"loc":{"start":{"line":1,"column":13},"end":{"line":1,"column":15}}}},"branchMap":{},"s":{"0":1},"f":{"0":0},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("class=\"line hit\""), "decl line should stay hit:\n{detail}");
        assert!(detail.contains("fn 0x"), "function-uncalled cue missing:\n{detail}");
        assert!(
            detail.contains("Line 1, covered: function not called"),
            "aria label should expose uncalled function:\n{detail}",
        );
    }

    #[test]
    fn missing_source_detail_shows_path_and_search_root_notice() {
        let dir = tempfile::TempDir::new().unwrap();
        let json = r#"{"missing.js":{"path":"missing.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":12}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("missing.js.html")).unwrap();
        assert!(
            detail.contains("Source file unavailable at"),
            "missing source notice absent:\n{detail}",
        );
        assert!(detail.contains("missing.js"), "attempted path missing:\n{detail}");
        assert!(
            detail.contains(&html_text(&dir.path().display().to_string())),
            "search root missing from notice:\n{detail}",
        );
    }

    #[test]
    fn every_page_carries_generator_meta_tag() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        for html_path in walkdir(dir.path())
            .into_iter()
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("html"))
        {
            let html = fs::read_to_string(&html_path).unwrap();
            assert!(
                html.contains("<meta name=\"generator\" content=\"oxc-coverage-reports "),
                "missing generator meta in {html_path:?}",
            );
        }
    }

    #[test]
    fn custom_threshold_drives_summary_sentence_and_pct_class() {
        // Two files: one at 70% lines, one at 90% lines. With the default
        // 80% threshold the 70% file is "below"; with --threshold 60 both
        // are "above" and the row colour bucketing flips.
        let json = r#"{
            "lo.js":{"path":"lo.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":3,"column":0},"end":{"line":3,"column":1}},"3":{"start":{"line":4,"column":0},"end":{"line":4,"column":1}},"4":{"start":{"line":5,"column":0},"end":{"line":5,"column":1}},"5":{"start":{"line":6,"column":0},"end":{"line":6,"column":1}},"6":{"start":{"line":7,"column":0},"end":{"line":7,"column":1}},"7":{"start":{"line":8,"column":0},"end":{"line":8,"column":1}},"8":{"start":{"line":9,"column":0},"end":{"line":9,"column":1}},"9":{"start":{"line":10,"column":0},"end":{"line":10,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":1,"2":1,"3":1,"4":1,"5":1,"6":1,"7":0,"8":0,"9":0},"f":{},"b":{}},
            "hi.js":{"path":"hi.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":3,"column":0},"end":{"line":3,"column":1}},"3":{"start":{"line":4,"column":0},"end":{"line":4,"column":1}},"4":{"start":{"line":5,"column":0},"end":{"line":5,"column":1}},"5":{"start":{"line":6,"column":0},"end":{"line":6,"column":1}},"6":{"start":{"line":7,"column":0},"end":{"line":7,"column":1}},"7":{"start":{"line":8,"column":0},"end":{"line":8,"column":1}},"8":{"start":{"line":9,"column":0},"end":{"line":9,"column":1}},"9":{"start":{"line":10,"column":0},"end":{"line":10,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":1,"2":1,"3":1,"4":1,"5":1,"6":1,"7":1,"8":1,"9":0},"f":{},"b":{}}
        }"#;
        let map = parse_coverage_map(json).unwrap();

        // Default (80%): 70% file is "low" via lines pct < 50.0? Actually
        // 70% > 50%, so it's "medium"; threshold-summary should say
        // "1 of 2 files fall below the 80% coverage threshold".
        let dir_default = tempfile::TempDir::new().unwrap();
        write(&map, Path::new(""), dir_default.path(), &HtmlOptions::default()).unwrap();
        let default_root = fs::read_to_string(dir_default.path().join("index.html")).unwrap();
        assert!(
            default_root.contains("1</strong> of 2 files fall below the 80%"),
            "default threshold should report 1 below 80%: {default_root}",
        );
        assert!(
            default_root.contains("row-medium"),
            "70% file should bucket medium under default threshold",
        );

        // Custom 60%: both files are above; sentence should report all met.
        let dir_loose = tempfile::TempDir::new().unwrap();
        let opts = HtmlOptions::new(60.0).unwrap();
        write(&map, Path::new(""), dir_loose.path(), &opts).unwrap();
        let loose_root = fs::read_to_string(dir_loose.path().join("index.html")).unwrap();
        assert!(
            loose_root.contains("All 2 files</strong> meet the 60% coverage threshold"),
            "60% threshold should clear all files: {loose_root}",
        );
        assert!(
            loose_root.contains("row-high"),
            "70% file should bucket high under a 60% threshold",
        );
    }

    fn walkdir(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    out.extend(walkdir(&p));
                } else {
                    out.push(p);
                }
            }
        }
        out
    }
}
