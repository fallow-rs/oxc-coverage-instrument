//! Folder index page: the per-child summary table with its threshold
//! sentence, filter controls, and inline metric meters.

use std::fmt::Write as _;

use oxc_coverage_report::{Metric, ReportNode};

use super::RenderContext;
use super::page::{SummaryHeaderInput, pct_class, render_page, render_summary_header};
use super::paths::PhysicalPaths;
use crate::escape::{html_attr, html_text};

pub(super) fn render_folder_index(
    node: &ReportNode,
    children: &[ReportNode],
    ctx: &RenderContext<'_>,
    depth: usize,
) -> String {
    let title = if node.relative_path.is_empty() {
        "All files".to_owned()
    } else {
        node.relative_path.clone()
    };
    let threshold = ctx.options.green_threshold();
    let mut body = String::new();
    body.push_str(&render_summary_header(SummaryHeaderInput {
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
        body.push_str(&render_summary_row(child, threshold, ctx.physical_paths));
    }
    body.push_str("        </tbody>\n");
    body.push_str("      </table>\n");
    body.push_str("    </div>\n");
    render_page(&title, depth, &body)
}

/// One-line summary above the file table: "N of M files fall below the
/// X% coverage threshold".
///
/// The count is against `summary.lines.pct`; `threshold` also drives colour
/// bucketing for every metric, so a file with 95% lines and 30% branches
/// counts as above while its branch cell still renders red.
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

fn render_summary_row(child: &ReportNode, threshold: f64, paths: &PhysicalPaths) -> String {
    let href = html_attr(paths.href_from_parent(child));
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

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pct is clamped to 0.0..=100.0 before the cast"
)]
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
