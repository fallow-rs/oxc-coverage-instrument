//! Page shell shared by the folder index and the file detail page: the
//! document head, the metric header with its breadcrumb, and the
//! percentage bucketing their markup keys off.

use std::fmt::Write as _;

use oxc_coverage_report::{CoverageSummary, ReportNode};

use crate::escape::{html_attr, html_text};

/// Filename used for every folder-level index page.
const INDEX_FILE: &str = "index.html";

/// Version stamp embedded in the `<meta name="generator">` tag on every
/// page. Lets CI tooling identify which release of the reporter
/// produced a given report directory.
const GENERATOR: &str = concat!("oxc-coverage-reports ", env!("CARGO_PKG_VERSION"));

/// Strict Content-Security-Policy applied to every emitted page.
///
/// The combined effect is that opening a report in a browser, even one
/// served from a corporate HTTP proxy, performs zero outbound requests
/// to anything other than the report's own directory.
const CSP_POLICY: &str = "default-src 'self'; connect-src 'none'; \
font-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'";

/// Wrap `body` in the document shell. `depth` is the number of `../`
/// segments needed to reach the report root from this page.
pub(super) fn render_page(title: &str, depth: usize, body: &str) -> String {
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
pub(super) struct SummaryHeaderInput<'a> {
    pub(super) title: &'a str,
    pub(super) summary: &'a CoverageSummary,
    pub(super) depth: usize,
    pub(super) node: &'a ReportNode,
    pub(super) threshold: f64,
}

pub(super) fn render_summary_header(input: SummaryHeaderInput<'_>) -> String {
    let SummaryHeaderInput { title, summary, depth, node, threshold } = input;
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

/// Map a percentage to `"high"`, `"medium"` or `"low"`, the modifier
/// suffix on the `kpi-cell--*`, `row-*`, `pct *` and `cov-meter--*`
/// classes.
///
/// `green_threshold` separates high from medium. The medium/low boundary
/// is fixed at 50% so the three buckets stay distinguishable whatever the
/// caller sets the green line to.
pub(super) fn pct_class(pct: f64, green_threshold: f64) -> &'static str {
    if pct >= green_threshold {
        "high"
    } else if pct >= 50.0 {
        "medium"
    } else {
        "low"
    }
}
