//! Cobertura XML reporter.
//!
//! Produces a `coverage.xml` matching the Cobertura 0.4 DTD with the layout
//! `coverage > packages > package > classes > class > methods + lines`. The
//! emitter is hand-rolled (no XML library dependency) and only escapes the
//! five XML entities required for attribute values.
//!
//! Compatibility notes (deferred from PR E panel; non-trivial to get right):
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

use oxc_coverage_report::{NodeKind, ReportNode, Visitor, walk};
use oxc_coverage_types::{BranchEntry, FileCoverage};
use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Write a Cobertura XML report to `out` with a timestamp of "now".
pub fn write<W: io::Write>(root: &ReportNode, root_dir: &Path, out: &mut W) -> io::Result<()> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    write_with_timestamp(root, root_dir, ts, out)
}

/// Write a Cobertura XML report with an explicit `timestamp` (seconds since
/// epoch, matching the Cobertura 0.4 DTD). Implementation detail used by
/// the snapshot tests in the sibling integration-test crate to produce
/// deterministic output; production callers should use [`write`] instead.
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
    let (lines_total, lines_covered) = totals_lines(&files);
    let (branches_total, branches_covered) = totals_branches(&files);
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
    let (lines_total, lines_covered) = sum_lines(files);
    let (branches_total, branches_covered) = sum_branches(files);
    let line_rate = rate(lines_covered, lines_total);
    let branch_rate = rate(branches_covered, branches_total);

    // `complexity` is #REQUIRED on <package> per the Cobertura 0.4 DTD;
    // strict validators (xmllint --valid) reject the document without it.
    writeln!(
        out,
        "    <package name=\"{}\" line-rate=\"{line_rate}\" branch-rate=\"{branch_rate}\" complexity=\"0\">",
        crate::escape::xml_attr(name)
    )?;
    writeln!(out, "      <classes>")?;
    for entry in files {
        write_class(out, entry, root_dir)?;
    }
    writeln!(out, "      </classes>")?;
    writeln!(out, "    </package>")?;
    Ok(())
}

fn write_class<W: io::Write>(
    out: &mut W,
    entry: &FileEntry<'_>,
    root_dir: &Path,
) -> io::Result<()> {
    let file = entry.coverage;
    let relative = relativize(entry.display_path, root_dir);
    let class_name = Path::new(&relative)
        .file_name()
        .map_or_else(|| relative.clone(), |s| s.to_string_lossy().into_owned());

    let lines = compute_line_hits(file);
    let branched_lines = compute_branched_lines(file);
    let (lines_total, lines_covered) =
        (lines.len() as u32, lines.values().filter(|&&v| v > 0).count() as u32);
    let (branches_total, branches_covered) = file.b.values().fold((0u32, 0u32), |(t, c), arms| {
        (t + arms.len() as u32, c + arms.iter().filter(|&&v| v > 0).count() as u32)
    });
    let line_rate = rate(lines_covered, lines_total);
    let branch_rate = rate(branches_covered, branches_total);

    // `complexity` is #REQUIRED on <class> per the Cobertura 0.4 DTD;
    // strict validators (xmllint --valid) reject the document without it.
    writeln!(
        out,
        "        <class name=\"{}\" filename=\"{}\" line-rate=\"{line_rate}\" branch-rate=\"{branch_rate}\" complexity=\"0\">",
        crate::escape::xml_attr(&class_name),
        crate::escape::xml_attr(&relative)
    )?;

    write_methods(out, file)?;
    write_lines(out, &lines, &branched_lines)?;

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
        // them. We approximate as a binary did-the-method-run rate
        // (1.0000 / 0.0000) because the istanbul model tracks only the
        // method's invocation count, not per-line statement counts within
        // its body. Real consumers (Codecov, GitLab, Azure DevOps) read
        // class-level rates, so the approximation has no observable cost.
        //
        // `hits` and `signature` are not in the DTD ATTLIST for <method> but
        // are emitted by istanbul-reports and consumed by Codecov / GitLab.
        // `complexity` is also not in the DTD for <method>, but Azure DevOps'
        // Cobertura parser has been observed rejecting methods without it.
        let method_rate = if hits > 0 { "1.0000" } else { "0.0000" };
        writeln!(
            out,
            "            <method name=\"{}\" signature=\"()V\" line-rate=\"{method_rate}\" branch-rate=\"0.0000\" hits=\"{hits}\" complexity=\"0\">",
            crate::escape::xml_attr(&entry.name)
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
    branched: &BTreeMap<u32, BranchSummary>,
) -> io::Result<()> {
    writeln!(out, "          <lines>")?;
    for (line, hits) in lines {
        if let Some(b) = branched.get(line) {
            let pct =
                if b.total == 0 { 0 } else { u64::from(b.covered) * 100 / u64::from(b.total) };
            writeln!(
                out,
                "            <line number=\"{line}\" hits=\"{hits}\" branch=\"true\" condition-coverage=\"{pct}% ({covered}/{total})\"/>",
                covered = b.covered,
                total = b.total,
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
        let relative = relativize(&owned.display_path, root_dir);
        let parent = Path::new(&relative)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let key = if parent.is_empty() { ".".to_owned() } else { parent };
        out.entry(key)
            .or_default()
            .push(FileEntry { coverage: &owned.coverage, display_path: &owned.display_path });
    }
    out
}

fn relativize(path: &str, root_dir: &Path) -> String {
    let p = Path::new(path);
    let stripped =
        p.strip_prefix(root_dir).map_or_else(|_| path.to_owned(), |s| s.to_string_lossy().into());
    stripped.replace('\\', "/")
}

fn compute_line_hits(file: &FileCoverage) -> BTreeMap<u32, u32> {
    let mut by_line: BTreeMap<u32, u32> = BTreeMap::new();
    for (id, loc) in &file.statement_map {
        let line = loc.start.line;
        if line == 0 {
            continue;
        }
        let hits = file.s.get(id).copied().unwrap_or(0);
        by_line
            .entry(line)
            .and_modify(|existing| *existing = (*existing).max(hits))
            .or_insert(hits);
    }
    by_line
}

struct BranchSummary {
    total: u32,
    covered: u32,
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

fn branch_line(entry: &BranchEntry) -> u32 {
    if entry.line > 0 {
        entry.line
    } else {
        entry.locations.first().map_or(0, |loc| loc.start.line)
    }
}

fn totals_lines(files: &[OwnedFileEntry]) -> (u32, u32) {
    let mut total: u32 = 0;
    let mut covered: u32 = 0;
    for f in files {
        let lines = compute_line_hits(&f.coverage);
        total += lines.len() as u32;
        covered += lines.values().filter(|&&v| v > 0).count() as u32;
    }
    (total, covered)
}

fn totals_branches(files: &[OwnedFileEntry]) -> (u32, u32) {
    let mut total: u32 = 0;
    let mut covered: u32 = 0;
    for f in files {
        for arms in f.coverage.b.values() {
            total += arms.len() as u32;
            covered += arms.iter().filter(|&&v| v > 0).count() as u32;
        }
    }
    (total, covered)
}

fn sum_lines(files: &[FileEntry<'_>]) -> (u32, u32) {
    let mut total: u32 = 0;
    let mut covered: u32 = 0;
    for f in files {
        let lines = compute_line_hits(f.coverage);
        total += lines.len() as u32;
        covered += lines.values().filter(|&&v| v > 0).count() as u32;
    }
    (total, covered)
}

fn sum_branches(files: &[FileEntry<'_>]) -> (u32, u32) {
    let mut total: u32 = 0;
    let mut covered: u32 = 0;
    for f in files {
        for arms in f.coverage.b.values() {
            total += arms.len() as u32;
            covered += arms.iter().filter(|&&v| v > 0).count() as u32;
        }
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
    // Truncate to 4 decimals; trailing zeros are kept (e.g., "0.5000") to match
    // the istanbul-reports cobertura output shape. The clamp guards against
    // corrupted coverage data (covered > total) producing >1.0 which would
    // fail DTD-strict validators (Azure DevOps).
    format!("{:.4}", (raw * 10_000.0).round() / 10_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_report::summarize;
    use oxc_coverage_types::parse_coverage_map;

    fn render(json: &str) -> String {
        let map = parse_coverage_map(json).unwrap();
        let root = summarize(&map);
        let mut buf = Vec::new();
        write_with_timestamp(&root, Path::new(""), 0, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn root_carries_line_rate_branch_rate_timestamp() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let out = render(json);
        assert!(out.contains("line-rate=\""));
        assert!(out.contains("branch-rate=\""));
        assert!(out.contains("timestamp=\"0\""));
    }

    #[test]
    fn class_filename_is_relative() {
        let json = r#"{"/proj/src/a.js":{"path":"/proj/src/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let root = summarize(&map);
        let mut buf = Vec::new();
        write_with_timestamp(&root, Path::new("/proj"), 0, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("filename=\"src/a.js\""), "got:\n{out}");
        assert!(!out.contains("filename=\"/proj/src/a.js\""));
    }

    #[test]
    fn methods_carry_complexity_zero() {
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {},
                "fnMap": {
                    "0": {"name": "foo", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 3, "column": 0}}}
                },
                "branchMap": {},
                "s": {},
                "f": {"0": 1},
                "b": {}
            }
        }"#;
        let out = render(json);
        assert!(
            out.contains("complexity=\"0\""),
            "method must carry complexity=\"0\"; got:\n{out}"
        );
    }

    #[test]
    fn no_missing_branches_element() {
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {
                    "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}
                },
                "fnMap": {},
                "branchMap": {
                    "0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}]}
                },
                "s": {"0": 1},
                "f": {},
                "b": {"0": [1, 0]}
            }
        }"#;
        let out = render(json);
        assert!(
            !out.contains("missing-branches"),
            "Azure DevOps rejects <missing-branches>; got:\n{out}"
        );
    }

    #[test]
    fn xml_declaration_and_dtd_present() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let out = render(json);
        assert!(out.starts_with("<?xml version=\"1.0\" ?>"));
        assert!(out.contains("<!DOCTYPE coverage"));
    }

    #[test]
    fn condition_coverage_attribute_on_branched_line() {
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {
                    "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}
                },
                "fnMap": {},
                "branchMap": {
                    "0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}]}
                },
                "s": {"0": 1},
                "f": {},
                "b": {"0": [1, 0]}
            }
        }"#;
        let out = render(json);
        assert!(out.contains("branch=\"true\" condition-coverage=\"50% (1/2)\""), "got:\n{out}");
    }

    #[test]
    fn xml_illegal_control_chars_are_replaced() {
        // XML 1.0 forbids most control characters in document content;
        // strict parsers (Azure DevOps, libxml2 strict, Saxon) reject the
        // document when they appear. Verify NUL and `\x1F` are sanitized
        // out of attribute values rather than passed through unmodified.
        let json = "{
            \"a.js\": {
                \"path\": \"a.js\",
                \"statementMap\": {},
                \"fnMap\": {
                    \"0\": {\"name\": \"foo\\u0000bar\\u001fbaz\", \"line\": 1, \"decl\": {\"start\": {\"line\": 1, \"column\": 0}, \"end\": {\"line\": 1, \"column\": 3}}, \"loc\": {\"start\": {\"line\": 1, \"column\": 0}, \"end\": {\"line\": 3, \"column\": 0}}}
                },
                \"branchMap\": {},
                \"s\": {},
                \"f\": {\"0\": 1},
                \"b\": {}
            }
        }";
        let out = render(json);
        assert!(!out.contains('\u{0000}'), "NUL must not survive into output");
        assert!(!out.contains('\u{001F}'), "control char must not survive");
        // Both should be replaced by U+FFFD (the Unicode replacement char).
        assert!(out.contains('\u{FFFD}'), "expected replacement char in output:\n{out}");
    }

    #[test]
    fn complexity_attribute_present_on_class_and_package() {
        // Cobertura 0.4 DTD declares `complexity` as #REQUIRED on <coverage>,
        // <package>, <class>, and <method>. xmllint --valid rejects the
        // document if any are missing.
        let json = r#"{
            "src/a.js": {
                "path": "src/a.js",
                "statementMap": {"0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}},
                "fnMap": {},
                "branchMap": {},
                "s": {"0": 1},
                "f": {},
                "b": {}
            }
        }"#;
        let out = render(json);
        assert!(out.contains("<package "), "expected <package> tag in:\n{out}");
        assert!(out.contains("<class "), "expected <class> tag in:\n{out}");
        // Count `complexity="0"` occurrences; we expect one each on
        // <coverage>, <package>, <class>, <method> -> 3 here (no methods).
        let count = out.matches("complexity=\"0\"").count();
        assert!(
            count >= 3,
            "expected complexity=\"0\" on coverage, package, class; got {count}:\n{out}"
        );
    }

    #[test]
    fn attribute_values_are_xml_escaped() {
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {},
                "fnMap": {
                    "0": {"name": "foo<bar>&baz", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 3, "column": 0}}}
                },
                "branchMap": {},
                "s": {},
                "f": {"0": 1},
                "b": {}
            }
        }"#;
        let out = render(json);
        assert!(out.contains("name=\"foo&lt;bar&gt;&amp;baz\""), "got:\n{out}");
    }
}
