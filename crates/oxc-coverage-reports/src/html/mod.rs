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
//!   not exist locally), the detail page shows the coverage statistics
//!   alongside a `(source unavailable)` placeholder rather than failing.
//!
//! [syntect]: https://docs.rs/syntect
//! [two_face]: https://docs.rs/two-face
//! [rayon]: https://docs.rs/rayon

mod highlight;

use oxc_coverage_report::{CoverageMap, CoverageSummary, NodeKind, ReportNode, summarize};
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
pub fn write(coverage_map: &CoverageMap, root_dir: &Path, output_dir: &Path) -> io::Result<()> {
    let remapped = remap_coverage_map(coverage_map);
    let root = summarize(&remapped);

    fs::create_dir_all(output_dir)?;
    fs::write(output_dir.join("base.css"), BASE_CSS)?;
    fs::write(output_dir.join("base.js"), BASE_JS)?;

    let ctx = RenderContext { root_dir };
    render_node(&root, &ctx, output_dir, 0)?;
    Ok(())
}

struct RenderContext<'a> {
    root_dir: &'a Path,
}

fn render_node(
    node: &ReportNode,
    ctx: &RenderContext,
    output_dir: &Path,
    depth: usize,
) -> io::Result<()> {
    match &node.kind {
        NodeKind::Folder { children } => {
            // Folder pages live at `<output_dir>/<node.relative_path>/index.html`.
            let folder_dir = if node.relative_path.is_empty() {
                output_dir.to_path_buf()
            } else {
                output_dir.join(&node.relative_path)
            };
            fs::create_dir_all(&folder_dir)?;
            let html = render_folder_index(node, children, depth);
            fs::write(folder_dir.join(INDEX_FILE), html)?;

            // Render children in parallel. The file branch is CPU-bound
            // (syntect tokenization), so per-file fan-out scales well on
            // multi-core CI runners. Folder children re-enter
            // `render_node` and parallelize their own subtrees.
            children
                .par_iter()
                .map(|child| {
                    render_node(child, ctx, output_dir, depth + child_depth_delta(node, child))
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
            let html = render_detail(node, coverage, ctx, depth);
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

fn render_folder_index(node: &ReportNode, children: &[ReportNode], depth: usize) -> String {
    let title = if node.relative_path.is_empty() {
        "All files".to_owned()
    } else {
        node.relative_path.clone()
    };
    let mut body = String::new();
    body.push_str(&render_summary_header(&title, &node.summary, depth, node));
    body.push_str("    <div class=\"pad1\">\n");
    body.push_str("      <table class=\"coverage-summary\">\n");
    body.push_str(&render_summary_table_header());
    body.push_str("        <tbody>\n");
    for child in children {
        body.push_str(&render_summary_row(child));
    }
    body.push_str("        </tbody>\n");
    body.push_str("      </table>\n");
    body.push_str("    </div>\n");
    render_page(&title, depth, &body)
}

fn render_summary_table_header() -> String {
    let mut out = String::from("        <thead><tr>");
    for col in ["File", "Statements", "Branches", "Functions", "Lines"] {
        let _ = write!(out, "<th>{col}</th>");
    }
    out.push_str("</tr></thead>\n");
    out
}

fn render_summary_row(child: &ReportNode) -> String {
    let href = match &child.kind {
        NodeKind::Folder { .. } => format!("{}/{INDEX_FILE}", html_attr_escape(&child.name)),
        NodeKind::File { .. } => format!("{}{DETAIL_SUFFIX}", html_attr_escape(&child.name)),
    };
    let display = html_text_escape(&child.name);
    let row_class = pct_class(child.summary.lines.pct);
    let mut out = format!("          <tr class=\"row-{row_class}\">\n");
    let _ = writeln!(out, "            <td class=\"file\"><a href=\"{href}\">{display}</a></td>");
    for metric in [
        child.summary.statements,
        child.summary.branches,
        child.summary.functions,
        child.summary.lines,
    ] {
        let mc = pct_class(metric.pct);
        let _ = writeln!(
            out,
            "            <td class=\"pct {mc}\">{:.2}%<span class=\"quiet\"> ({}/{})</span></td>",
            metric.pct, metric.covered, metric.total
        );
    }
    out.push_str("          </tr>\n");
    out
}

// -- File detail page -------------------------------------------------------

fn render_detail(
    node: &ReportNode,
    coverage: &FileCoverage,
    ctx: &RenderContext,
    depth: usize,
) -> String {
    let title = node.relative_path.clone();
    let source = read_source(coverage, ctx.root_dir);
    let line_hits = compute_line_hits(coverage);
    let branched_lines = compute_branched_lines(coverage);
    let fn_lines = compute_fn_lines(coverage);

    let mut body = String::new();
    body.push_str(&render_summary_header(&title, &node.summary, depth, node));
    body.push_str("    <div class=\"pad1\">\n");
    body.push_str("      <table class=\"source\">\n");
    match source {
        Some(text) => {
            // Highlight up front so the per-line table render gets
            // pre-escaped, span-wrapped HTML; this also keeps scope
            // state consistent across multi-line strings/comments.
            let highlighted = highlight::highlight_lines(&text, Path::new(&coverage.path));
            body.push_str(&render_source_table(
                &highlighted,
                &line_hits,
                &branched_lines,
                &fn_lines,
            ));
        }
        None => body.push_str(&render_source_unavailable(&line_hits)),
    }
    body.push_str("      </table>\n");
    body.push_str("    </div>\n");
    render_page(&title, depth, &body)
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
        out.push_str(&render_source_row(line_no, line_html, stmt_hits, branch, fn_hits));
    }
    out.push_str("        </tbody>\n");
    out
}

/// Render one source row. `src_html` is already escaped and may carry
/// syntect `<span class="stok-...">` markup; do not re-escape.
fn render_source_row(
    line_no: u32,
    src_html: &str,
    stmt_hits: Option<u32>,
    branch: Option<&BranchSummary>,
    fn_hits: Option<u32>,
) -> String {
    let class = source_row_class(stmt_hits, branch, fn_hits);
    let hits_cell = match (stmt_hits, fn_hits) {
        (Some(h), _) | (None, Some(h)) => format!("{h}x"),
        _ => String::new(),
    };
    let branch_note = branch.map_or(String::new(), |b| {
        if b.total == 0 {
            String::new()
        } else {
            format!(" <span class=\"branch-note\">branches {}/{}</span>", b.covered, b.total)
        }
    });
    format!(
        "          <tr class=\"line {class}\"><td class=\"line-num\">{line_no}</td><td class=\"hits\">{hits_cell}</td><td class=\"src\"><pre>{src_html}</pre>{branch_note}</td></tr>\n",
    )
}

fn source_row_class(
    stmt_hits: Option<u32>,
    branch: Option<&BranchSummary>,
    fn_hits: Option<u32>,
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
        if let Some(fh) = fn_hits
            && fh == 0
        {
            return "miss";
        }
        "hit"
    } else {
        "no-stmt"
    }
}

fn render_source_unavailable(line_hits: &BTreeMap<u32, u32>) -> String {
    let mut out = String::from(
        "        <thead><tr><th class=\"line-num\">Line</th><th class=\"hits\">Hits</th><th class=\"src\">Source</th></tr></thead>\n        <tbody>\n",
    );
    for (line, hits) in line_hits {
        let class = if *hits == 0 { "miss" } else { "hit" };
        let _ = writeln!(
            out,
            "          <tr class=\"line {class}\"><td class=\"line-num\">{line}</td><td class=\"hits\">{hits}x</td><td class=\"src\"><pre>(source unavailable)</pre></td></tr>"
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
    let _ = writeln!(
        out,
        "  <meta http-equiv=\"Content-Security-Policy\" content=\"{}\">",
        html_attr_escape(CSP_POLICY),
    );
    let _ = writeln!(out, "  <title>Coverage: {}</title>", html_text_escape(title));
    let _ = writeln!(out, "  <link rel=\"stylesheet\" href=\"{}\">", html_attr_escape(&css_href));
    let _ = writeln!(out, "  <script src=\"{}\" defer></script>", html_attr_escape(&js_href));
    out.push_str("</head>\n<body>\n");
    out.push_str(body);
    out.push_str("</body>\n</html>\n");
    out
}

fn render_summary_header(
    title: &str,
    summary: &CoverageSummary,
    depth: usize,
    node: &ReportNode,
) -> String {
    let mut out = String::from("    <header class=\"summary\">\n");
    let _ = writeln!(out, "      <h1>{}</h1>", html_text_escape(title));
    out.push_str(&render_breadcrumb(node, depth));
    out.push_str("      <ul class=\"metrics\">\n");
    for (label, m) in [
        ("Statements", summary.statements),
        ("Branches", summary.branches),
        ("Functions", summary.functions),
        ("Lines", summary.lines),
    ] {
        let class = pct_class(m.pct);
        let _ = writeln!(
            out,
            "        <li class=\"metric {class}\"><span class=\"label\">{label}</span><span class=\"pct\">{:.2}%</span><span class=\"quiet\">{}/{}</span></li>",
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
    let mut out = String::from("      <nav class=\"breadcrumb\">");
    let root_href = relative_to_root(depth, INDEX_FILE);
    let _ = write!(out, "<a href=\"{}\">All files</a>", html_attr_escape(&root_href));

    let parts: Vec<&str> = node.relative_path.split('/').collect();
    let mut depth_remaining = depth;
    for (i, part) in parts.iter().enumerate() {
        let is_last = i + 1 == parts.len();
        if is_last {
            let _ = write!(out, " / <span class=\"current\">{}</span>", html_text_escape(part));
        } else {
            // Build a relative href that climbs up to that ancestor's index.
            depth_remaining = depth_remaining.saturating_sub(1);
            let href = relative_to_root(depth_remaining, INDEX_FILE);
            let _ = write!(
                out,
                " / <a href=\"{}\">{}</a>",
                html_attr_escape(&href),
                html_text_escape(part)
            );
        }
    }
    out.push_str("</nav>\n");
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
        out.entry(line)
            .and_modify(|s| {
                s.total += total;
                s.covered += covered;
            })
            .or_insert(BranchSummary { total, covered });
    }
    out
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

fn read_source(file: &FileCoverage, root_dir: &Path) -> Option<String> {
    if file.path.is_empty() {
        return None;
    }
    let path = Path::new(&file.path);
    let absolute: PathBuf =
        if path.is_absolute() { path.to_path_buf() } else { root_dir.join(path) };
    fs::read_to_string(&absolute).ok()
}

// -- Class assignment ------------------------------------------------------

fn pct_class(pct: f64) -> &'static str {
    if pct >= 80.0 {
        "high"
    } else if pct >= 50.0 {
        "medium"
    } else {
        "low"
    }
}

// -- HTML escaping ---------------------------------------------------------

fn html_text_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

fn html_attr_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_types::parse_coverage_map;

    fn write_to_temp(json: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let map = parse_coverage_map(json).unwrap();
        write(&map, Path::new(""), dir.path()).unwrap();
        dir
    }

    #[test]
    fn writes_root_index_and_base_css() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(root_index.contains("<title>Coverage: All files</title>"));
        assert!(root_index.contains("<link rel=\"stylesheet\" href=\"base.css\">"));
        assert!(fs::read_to_string(dir.path().join("base.css")).unwrap().contains(".high"));
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
        write(&map, dir.path(), &out_dir).unwrap();
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
        assert!(css.contains(":root[data-theme=\"dark\"]"), "missing dark override");
        assert!(css.contains(":root:not([data-theme=\"light\"])"), "missing light escape hatch");
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
