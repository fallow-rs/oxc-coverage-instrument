//! Cobertura XML reporter.
//!
//! Produces a `coverage.xml` matching the Cobertura 0.4 DTD with the layout
//! `coverage > packages > package > classes > class > methods + lines`. The
//! emitter is hand-rolled (no XML library dependency) and only escapes the
//! five XML entities required for attribute values.
//!
//! Compatibility constraints:
//!
//! - `<class filename=...>` paths are repo-relative (stripped of the caller's
//!   `root_dir`, the `--root` CLI argument). The GitLab MR widget and Jenkins'
//!   Cobertura plugin both require relative class filenames; absolute paths
//!   render the file as "unknown" or attach the coverage to the wrong source
//!   tree.
//! - Every `<method>` carries `complexity="0"`. Older versions of Microsoft's
//!   Azure DevOps Cobertura parser reject methods without a `complexity`
//!   attribute, even though the DTD makes it optional.
//! - `<missing-branches>` is NEVER emitted. It is a Coverage.py extension
//!   outside the published DTD; strict DTD validators reject it and several
//!   real-world consumers (Azure DevOps among them) have been observed
//!   refusing the document.
//! - `line-rate`, `branch-rate`, and `timestamp` are set on the root
//!   `<coverage>` element. Several consumers (Codecov, GitLab) error out
//!   silently when these are missing and report zero coverage.
//! - `condition-coverage` is computed per branched line as `N% (covered/total)`
//!   to match istanbul-reports.
//!
//! Timestamps default to `SystemTime::now()` seconds since epoch, matching
//! the Cobertura 0.4 DTD. The deterministic-timestamp variant
//! [`write_with_timestamp`] is `#[doc(hidden)]` and exists only for snapshot
//! testing; production callers should use [`write()`].

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use cow_utils::CowUtils as _;
use oxc_coverage_report::{NodeKind, ReportNode, Visitor, walk};
use oxc_coverage_types::FileCoverage;

use crate::escape::xml_attr;
use crate::projection::{branch_counts, branched_lines, line_hits, relative_source_path};

/// Write a Cobertura XML report to `out` with a timestamp of "now".
///
/// # Errors
/// Returns [`io::Error`] if `out` fails to accept a write.
pub fn write<W: io::Write>(root: &ReportNode, root_dir: &Path, out: &mut W) -> io::Result<()> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    write_with_timestamp(root, root_dir, ts, out)
}

/// Write a Cobertura XML report with an explicit `timestamp` (seconds since
/// epoch, matching the Cobertura 0.4 DTD). Implementation detail used by
/// the snapshot tests to produce deterministic output; production callers
/// should use [`write`] instead.
///
/// # Errors
/// Returns [`io::Error`] if `out` fails to accept a write.
#[doc(hidden)]
pub fn write_with_timestamp<W: io::Write>(
    root: &ReportNode,
    root_dir: &Path,
    timestamp_secs: u64,
    out: &mut W,
) -> io::Result<()> {
    let mut collector = FileCollector::default();
    walk(root, &mut collector)?;

    let files = collector.files;
    let (lines_total, lines_covered) = accumulate_lines(files.iter().map(|f| f.coverage.as_ref()));
    let (branches_total, branches_covered) =
        accumulate_branches(files.iter().map(|f| f.coverage.as_ref()));
    let line_rate = rate(lines_covered, lines_total);
    let branch_rate = rate(branches_covered, branches_total);

    writeln!(out, "<?xml version=\"1.0\" ?>")?;
    writeln!(
        out,
        "<!DOCTYPE coverage SYSTEM \"http://cobertura.sourceforge.net/xml/coverage-04.dtd\">"
    )?;
    writeln!(
        out,
        "<coverage lines-valid=\"{lines_total}\" lines-covered=\"{lines_covered}\" line-rate=\"{line_rate}\" branches-valid=\"{branches_total}\" branches-covered=\"{branches_covered}\" branch-rate=\"{branch_rate}\" timestamp=\"{timestamp_secs}\" complexity=\"0\" version=\"0.1\">"
    )?;
    writeln!(out, "  <sources>")?;
    writeln!(out, "    <source>.</source>")?;
    writeln!(out, "  </sources>")?;
    writeln!(out, "  <packages>")?;

    let grouped = group_by_package(&files, root_dir);
    for (package_name, package_files) in &grouped {
        write_package(out, package_name, package_files, root_dir)?;
    }

    writeln!(out, "  </packages>")?;
    writeln!(out, "</coverage>")?;
    Ok(())
}

fn write_package<W: io::Write>(
    out: &mut W,
    name: &str,
    files: &[FileEntry<'_>],
    root_dir: &Path,
) -> io::Result<()> {
    let (lines_total, lines_covered) = accumulate_lines(files.iter().map(|f| f.coverage));
    let (branches_total, branches_covered) = accumulate_branches(files.iter().map(|f| f.coverage));
    let line_rate = rate(lines_covered, lines_total);
    let branch_rate = rate(branches_covered, branches_total);

    // `complexity` is #REQUIRED on <package> per the Cobertura 0.4 DTD;
    // strict validators (xmllint --valid) reject the document without it.
    writeln!(
        out,
        "    <package name=\"{}\" line-rate=\"{line_rate}\" branch-rate=\"{branch_rate}\" complexity=\"0\">",
        xml_attr(name)
    )?;
    writeln!(out, "      <classes>")?;
    for entry in files {
        write_class(out, entry, root_dir)?;
    }
    writeln!(out, "      </classes>")?;
    writeln!(out, "    </package>")?;
    Ok(())
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "istanbul counters are u32 on the wire, so a line count that does not fit is already outside the format"
)]
fn write_class<W: io::Write>(
    out: &mut W,
    entry: &FileEntry<'_>,
    root_dir: &Path,
) -> io::Result<()> {
    let file = entry.coverage;
    let relative = relative_source_path(entry.display_path, root_dir);
    let class_name = Path::new(&relative)
        .file_name()
        .map_or_else(|| relative.clone(), |s| s.to_string_lossy().into_owned());

    let lines = line_hits(file);
    let branches_by_line = branched_lines(file);
    let (lines_total, lines_covered) =
        (lines.len() as u32, lines.values().filter(|&&v| v > 0).count() as u32);
    let (branches_total, branches_covered) = branch_counts(file);
    let line_rate = rate(lines_covered, lines_total);
    let branch_rate = rate(branches_covered, branches_total);

    // `complexity` is #REQUIRED on <class> per the Cobertura 0.4 DTD;
    // strict validators (xmllint --valid) reject the document without it.
    writeln!(
        out,
        "        <class name=\"{}\" filename=\"{}\" line-rate=\"{line_rate}\" branch-rate=\"{branch_rate}\" complexity=\"0\">",
        xml_attr(&class_name),
        xml_attr(&relative)
    )?;

    write_methods(out, file)?;
    write_lines(out, &lines, &branches_by_line)?;

    writeln!(out, "        </class>")?;
    Ok(())
}

fn write_methods<W: io::Write>(out: &mut W, file: &FileCoverage) -> io::Result<()> {
    writeln!(out, "          <methods>")?;
    for (id, entry) in &file.fn_map {
        let hits = file.f.get(id).copied().unwrap_or(0);
        let line = entry.decl.start.line;
        // `line-rate` and `branch-rate` are #REQUIRED on <method> per the
        // Cobertura 0.4 DTD; xmllint --valid rejects the document without
        // them. The rate is binary (1.0000 / 0.0000) because the istanbul
        // model tracks only the method's invocation count, not per-line
        // statement counts within its body. Codecov, GitLab and Azure DevOps
        // read class-level rates, so the approximation is not observable.
        //
        // `hits` and `signature` are not in the DTD ATTLIST for <method> but
        // are emitted by istanbul-reports and consumed by Codecov / GitLab.
        // `complexity` is also not in the DTD for <method>, but Azure DevOps'
        // Cobertura parser has been observed rejecting methods without it.
        let method_rate = if hits > 0 { "1.0000" } else { "0.0000" };
        writeln!(
            out,
            "            <method name=\"{}\" signature=\"()V\" line-rate=\"{method_rate}\" branch-rate=\"0.0000\" hits=\"{hits}\" complexity=\"0\">",
            xml_attr(&entry.name)
        )?;
        writeln!(out, "              <lines>")?;
        writeln!(out, "                <line number=\"{line}\" hits=\"{hits}\"/>")?;
        writeln!(out, "              </lines>")?;
        writeln!(out, "            </method>")?;
    }
    writeln!(out, "          </methods>")?;
    Ok(())
}

fn write_lines<W: io::Write>(
    out: &mut W,
    lines: &BTreeMap<u32, u32>,
    branched: &BTreeMap<u32, (u32, u32)>,
) -> io::Result<()> {
    writeln!(out, "          <lines>")?;
    for (line, hits) in lines {
        if let Some(&(total, covered)) = branched.get(line) {
            let pct = if total == 0 { 0 } else { u64::from(covered) * 100 / u64::from(total) };
            writeln!(
                out,
                "            <line number=\"{line}\" hits=\"{hits}\" branch=\"true\" condition-coverage=\"{pct}% ({covered}/{total})\"/>",
            )?;
        } else {
            writeln!(
                out,
                "            <line number=\"{line}\" hits=\"{hits}\" branch=\"false\"/>"
            )?;
        }
    }
    writeln!(out, "          </lines>")?;
    Ok(())
}

struct FileEntry<'a> {
    coverage: &'a FileCoverage,
    display_path: &'a str,
}

#[derive(Default)]
struct FileCollector {
    files: Vec<OwnedFileEntry>,
}

struct OwnedFileEntry {
    coverage: Box<FileCoverage>,
    display_path: String,
}

impl Visitor for FileCollector {
    fn on_detail(&mut self, node: &ReportNode) -> io::Result<()> {
        if let NodeKind::File { coverage } = &node.kind {
            let display_path = if coverage.path.is_empty() {
                node.relative_path.clone()
            } else {
                coverage.path.clone()
            };
            self.files.push(OwnedFileEntry { coverage: coverage.clone(), display_path });
        }
        Ok(())
    }
}

fn group_by_package<'a>(
    files: &'a [OwnedFileEntry],
    root_dir: &Path,
) -> BTreeMap<String, Vec<FileEntry<'a>>> {
    let mut out: BTreeMap<String, Vec<FileEntry<'a>>> = BTreeMap::new();
    for owned in files {
        let relative = relative_source_path(&owned.display_path, root_dir);
        let parent = Path::new(&relative)
            .parent()
            .map(|p| p.to_string_lossy().cow_replace('\\', "/").into_owned())
            .unwrap_or_default();
        let key = if parent.is_empty() { ".".to_owned() } else { parent };
        out.entry(key)
            .or_default()
            .push(FileEntry { coverage: &owned.coverage, display_path: &owned.display_path });
    }
    out
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "istanbul counters are u32 on the wire, so a line count that does not fit is already outside the format"
)]
fn accumulate_lines<'a>(files: impl Iterator<Item = &'a FileCoverage>) -> (u32, u32) {
    let mut total: u32 = 0;
    let mut covered: u32 = 0;
    for file in files {
        let lines = line_hits(file);
        total += lines.len() as u32;
        covered += lines.values().filter(|&&v| v > 0).count() as u32;
    }
    (total, covered)
}

fn accumulate_branches<'a>(files: impl Iterator<Item = &'a FileCoverage>) -> (u32, u32) {
    let mut total: u32 = 0;
    let mut covered: u32 = 0;
    for file in files {
        let (file_total, file_covered) = branch_counts(file);
        total += file_total;
        covered += file_covered;
    }
    (total, covered)
}

fn rate(covered: u32, total: u32) -> String {
    if total == 0 {
        // Use the 4-decimal form (not bare "0") so every line-rate /
        // branch-rate attribute uses the same fixed shape; Azure DevOps'
        // validator has been observed rejecting `"0"` where it expects a
        // float-shaped value.
        return "0.0000".to_owned();
    }
    let raw = (f64::from(covered) / f64::from(total)).clamp(0.0, 1.0);
    // Round to 4 decimals; trailing zeros are kept (e.g. "0.5000") to match
    // the istanbul-reports cobertura output shape. The clamp guards against
    // corrupted coverage data (covered > total) producing >1.0, which
    // DTD-strict validators (Azure DevOps) reject.
    format!("{:.4}", (raw * 10_000.0).round() / 10_000.0)
}

#[cfg(test)]
mod tests;
