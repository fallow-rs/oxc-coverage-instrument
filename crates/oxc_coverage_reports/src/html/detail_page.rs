//! File detail page: the highlighted source view with its per-line hit
//! gutter, branch notes, and the notice shown when the source file
//! cannot be read from disk.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use oxc_coverage_report::ReportNode;
use oxc_coverage_types::FileCoverage;

use super::RenderContext;
use super::highlight;
use super::line_coverage::{BranchSummary, compute_branched_lines, compute_fn_lines};
use super::page::{SummaryHeaderInput, render_page, render_summary_header};
use crate::escape::html_text;
use crate::projection::line_hits;

#[derive(Clone, Copy)]
pub(super) struct RenderDetailInput<'a> {
    pub(super) node: &'a ReportNode,
    pub(super) coverage: &'a FileCoverage,
    pub(super) ctx: &'a RenderContext<'a>,
    pub(super) depth: usize,
}

pub(super) fn render_detail(input: RenderDetailInput<'_>) -> String {
    let RenderDetailInput { node, coverage, ctx, depth } = input;
    let title = node.relative_path.clone();
    let source = read_source(coverage, ctx.root_dir);
    let statement_lines = line_hits(coverage);
    let branched_lines = compute_branched_lines(coverage);
    let fn_lines = compute_fn_lines(coverage);

    let mut body = String::new();
    body.push_str(&render_summary_header(SummaryHeaderInput {
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
    body.push_str(&render_detail_source(DetailSourceInput {
        source,
        coverage_path: &coverage.path,
        line_hits: &statement_lines,
        branched_lines: &branched_lines,
        fn_lines: &fn_lines,
    }));
    body.push_str("    </div>\n");
    body.push_str(
        "    <div class=\"copy-toast\" id=\"cov-copy-toast\" role=\"status\" aria-atomic=\"true\"></div>\n",
    );
    render_page(&title, depth, &body)
}

struct DetailSourceInput<'a> {
    source: Result<String, MissingSource>,
    coverage_path: &'a str,
    line_hits: &'a BTreeMap<u32, u32>,
    branched_lines: &'a BTreeMap<u32, BranchSummary>,
    fn_lines: &'a BTreeMap<u32, u32>,
}

fn render_detail_source(input: DetailSourceInput<'_>) -> String {
    let DetailSourceInput { source, coverage_path, line_hits, branched_lines, fn_lines } = input;
    let mut out = String::new();
    match source {
        Ok(text) => {
            let highlighted = highlight::highlight_lines(&text, Path::new(coverage_path));
            out.push_str("      <table class=\"source\">\n");
            out.push_str(&render_source_table(SourceTableInput {
                lines: &highlighted,
                line_hits,
                branched: branched_lines,
                fns: fn_lines,
            }));
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

#[derive(Clone, Copy)]
struct SourceTableInput<'a> {
    lines: &'a [String],
    line_hits: &'a BTreeMap<u32, u32>,
    branched: &'a BTreeMap<u32, BranchSummary>,
    fns: &'a BTreeMap<u32, u32>,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a source file with more than u32::MAX lines cannot be addressed by istanbul's u32 line numbers"
)]
fn render_source_table(input: SourceTableInput<'_>) -> String {
    let SourceTableInput { lines, line_hits, branched, fns } = input;
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
    let class = source_row_class(stmt_hits, branch);
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

#[derive(Clone, Copy)]
struct RowAriaInput<'a> {
    line_no: u32,
    class: &'a str,
    branch: Option<&'a BranchSummary>,
    fn_hits: Option<u32>,
}

/// Human-readable status used as the row's `aria-label`. Avoids the raw
/// class tokens (`miss` and friends) that would otherwise leak into
/// assistive-technology output. The result needs no escaping: every part
/// is drawn from a bounded set of phrases plus a line number and hit
/// counts.
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

fn source_row_class(stmt_hits: Option<u32>, branch: Option<&BranchSummary>) -> &'static str {
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
