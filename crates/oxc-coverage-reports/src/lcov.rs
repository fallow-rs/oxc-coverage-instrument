//! LCOV `tracefile` reporter.
//!
//! Produces the same record layout as istanbul-reports' `lcovonly` output:
//! one `SF:`/`end_of_record` block per file, with `FN:`/`FNDA:`/`FNF:`/`FNH:`
//! for function coverage, `DA:`/`LF:`/`LH:` for line coverage, and
//! `BRDA:`/`BRF:`/`BRH:` for branch coverage.
//!
//! Compatibility notes (deferred from PR E panel; non-trivial to get right):
//!
//! - `SF:` paths are relative to a caller-supplied `root_dir` (the `--root` CLI
//!   flag, defaulting to the cwd). Absolute paths break Codecov self-hosted
//!   runners and the GitLab MR widget; consumers expect repo-relative paths.
//! - `BRDA:<line>,<block>,<branch>,<taken>` sets `<block>` from the
//!   [`BranchEntry`][be] map key (the istanbul branch ID), not the line number.
//!   Putting every arm in `block=0` makes Codecov merge branches across
//!   unrelated `if` statements that share a line.
//! - Both `DA:` (line hits) AND `BRDA:` (branch hits) are emitted. Coveralls
//!   ignores `BRDA:` entirely and reports 0% branch coverage if a tracefile
//!   ships branches but no lines.
//! - `FN:` / `FNDA:` names are stripped of surrounding parens, so
//!   `(anonymous_0)` becomes `anonymous_0`. Codecov's function-coverage parser
//!   rejects names containing parens.
//! - When a branch's block was never entered (the sum of all arm counts is
//!   zero), every arm emits `-` for `<taken>` rather than `0`. `0` means
//!   "block entered, arm not taken", which is nonsensical for an `if` whose
//!   condition never evaluated.
//!
//! The emitter never panics on coverage data; missing or malformed entries are
//! treated as zero hits.
//!
//! [be]: oxc_coverage_types::BranchEntry

use oxc_coverage_report::{NodeKind, ReportNode, Visitor, walk};
use oxc_coverage_types::FileCoverage;
use std::collections::BTreeMap;
use std::io;
use std::path::Path;

/// Write an LCOV report to `out`. `root_dir` is used to relativize `SF:` paths.
pub fn write<W: io::Write>(root: &ReportNode, root_dir: &Path, out: &mut W) -> io::Result<()> {
    let mut emitter = LcovEmitter { out, root_dir };
    walk(root, &mut emitter)
}

struct LcovEmitter<'a, W: io::Write> {
    out: &'a mut W,
    root_dir: &'a Path,
}

impl<W: io::Write> Visitor for LcovEmitter<'_, W> {
    fn on_detail(&mut self, node: &ReportNode) -> io::Result<()> {
        if let NodeKind::File { coverage } = &node.kind {
            write_file_record(self.out, coverage, node, self.root_dir)?;
        }
        Ok(())
    }
}

fn write_file_record<W: io::Write>(
    out: &mut W,
    file: &FileCoverage,
    node: &ReportNode,
    root_dir: &Path,
) -> io::Result<()> {
    let sf = source_path(file, node, root_dir);
    writeln!(out, "TN:")?;
    writeln!(out, "SF:{sf}")?;

    write_functions(out, file)?;
    write_lines(out, file)?;
    write_branches(out, file)?;

    writeln!(out, "end_of_record")?;
    Ok(())
}

fn source_path(file: &FileCoverage, node: &ReportNode, root_dir: &Path) -> String {
    let raw = if file.path.is_empty() { node.relative_path.as_str() } else { file.path.as_str() };
    let p = Path::new(raw);
    let relative =
        p.strip_prefix(root_dir).map_or_else(|_| raw.to_owned(), |s| s.to_string_lossy().into());
    relative.replace('\\', "/")
}

fn write_functions<W: io::Write>(out: &mut W, file: &FileCoverage) -> io::Result<()> {
    let mut total: u32 = 0;
    let mut hit: u32 = 0;

    // Order by id (BTreeMap iteration), but match istanbul-reports by emitting
    // FN: rows first, then all FNDA: rows.
    let entries: Vec<(&String, &oxc_coverage_types::FnEntry, u32)> = file
        .fn_map
        .iter()
        .map(|(id, entry)| (id, entry, file.f.get(id).copied().unwrap_or(0)))
        .collect();

    for (_id, entry, _count) in &entries {
        writeln!(out, "FN:{},{}", entry.decl.start.line, sanitize_fn_name(&entry.name))?;
        total += 1;
    }
    for (_id, entry, count) in &entries {
        if *count > 0 {
            hit += 1;
        }
        writeln!(out, "FNDA:{},{}", count, sanitize_fn_name(&entry.name))?;
    }

    writeln!(out, "FNF:{total}")?;
    writeln!(out, "FNH:{hit}")?;
    Ok(())
}

fn sanitize_fn_name(name: &str) -> String {
    // Strip exactly one matched outer pair of parens. `trim_*_matches` would
    // eat every leading `(` and trailing `)` and could mangle names like
    // `(<anonymous>)` into `<anonymous>`, leaving angle brackets that some
    // LCOV parsers treat as delimiters.
    let stripped = name.strip_prefix('(').and_then(|s| s.strip_suffix(')')).unwrap_or(name);
    if stripped.is_empty() { "anonymous".to_owned() } else { stripped.to_owned() }
}

fn write_lines<W: io::Write>(out: &mut W, file: &FileCoverage) -> io::Result<()> {
    // Group statements by start line, take the max hit count per line.
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

    let total = by_line.len() as u32;
    let hit = by_line.values().filter(|&&v| v > 0).count() as u32;
    for (line, count) in &by_line {
        writeln!(out, "DA:{line},{count}")?;
    }
    writeln!(out, "LF:{total}")?;
    writeln!(out, "LH:{hit}")?;
    Ok(())
}

fn write_branches<W: io::Write>(out: &mut W, file: &FileCoverage) -> io::Result<()> {
    let mut total: u32 = 0;
    let mut hit: u32 = 0;
    for (id, entry) in &file.branch_map {
        let block: u32 = id.parse().unwrap_or(0);
        let arms = file.b.get(id).cloned().unwrap_or_default();
        let block_entered = arms.iter().any(|&c| c > 0);
        for (idx, count) in arms.iter().enumerate() {
            total += 1;
            if *count > 0 {
                hit += 1;
            }
            let taken: String = if block_entered { count.to_string() } else { "-".to_owned() };
            // Fall back to the branch entry's reported line when individual arm
            // locations have unknown positions (line == 0). Some emitters omit
            // arm-level locations.
            let line = if entry.line > 0 {
                entry.line
            } else {
                entry.locations.get(idx).map_or(0, |loc| loc.start.line)
            };
            writeln!(out, "BRDA:{line},{block},{idx},{taken}")?;
        }
    }
    writeln!(out, "BRF:{total}")?;
    writeln!(out, "BRH:{hit}")?;
    Ok(())
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
        write(&root, Path::new(""), &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn emits_tn_sf_and_end_of_record() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let out = render(json);
        assert!(out.starts_with("TN:\nSF:a.js"));
        assert!(out.contains("end_of_record"));
    }

    #[test]
    fn brda_block_uses_branch_entry_id_not_line() {
        // Two if-statements both starting on line 5 (id "0" at block 0, id "1"
        // at block 1). BRDA block field MUST differentiate the two so Codecov
        // does not merge them into a single 4-arm branch.
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {},
                "fnMap": {},
                "branchMap": {
                    "0": {"loc": {"start": {"line": 5, "column": 0}, "end": {"line": 5, "column": 10}}, "line": 5, "type": "if", "locations": [{"start": {"line": 5, "column": 0}, "end": {"line": 5, "column": 5}}, {"start": {"line": 5, "column": 6}, "end": {"line": 5, "column": 10}}]},
                    "1": {"loc": {"start": {"line": 5, "column": 11}, "end": {"line": 5, "column": 20}}, "line": 5, "type": "if", "locations": [{"start": {"line": 5, "column": 11}, "end": {"line": 5, "column": 15}}, {"start": {"line": 5, "column": 16}, "end": {"line": 5, "column": 20}}]}
                },
                "s": {},
                "f": {},
                "b": {"0": [1, 0], "1": [0, 1]}
            }
        }"#;
        let out = render(json);
        assert!(out.contains("BRDA:5,0,0,1"), "got:\n{out}");
        assert!(out.contains("BRDA:5,0,1,0"), "got:\n{out}");
        assert!(out.contains("BRDA:5,1,0,0"), "got:\n{out}");
        assert!(out.contains("BRDA:5,1,1,1"), "got:\n{out}");
    }

    #[test]
    fn brda_taken_is_dash_when_block_never_entered() {
        // All arms zero -> the if condition itself never ran -> emit `-`.
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {},
                "fnMap": {},
                "branchMap": {
                    "0": {"loc": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, "line": 1, "type": "if", "locations": [{"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}, {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 1}}]}
                },
                "s": {},
                "f": {},
                "b": {"0": [0, 0]}
            }
        }"#;
        let out = render(json);
        assert!(out.contains("BRDA:1,0,0,-"), "got:\n{out}");
        assert!(out.contains("BRDA:1,0,1,-"), "got:\n{out}");
    }

    #[test]
    fn anonymous_function_name_has_parens_stripped() {
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {},
                "fnMap": {
                    "0": {"name": "(anonymous_0)", "line": 3, "decl": {"start": {"line": 3, "column": 0}, "end": {"line": 3, "column": 3}}, "loc": {"start": {"line": 3, "column": 0}, "end": {"line": 5, "column": 0}}}
                },
                "branchMap": {},
                "s": {},
                "f": {"0": 2},
                "b": {}
            }
        }"#;
        let out = render(json);
        assert!(out.contains("FN:3,anonymous_0"), "got:\n{out}");
        assert!(out.contains("FNDA:2,anonymous_0"), "got:\n{out}");
        assert!(!out.contains("(anonymous_0)"));
    }

    #[test]
    fn both_da_and_brda_emitted() {
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
        assert!(out.contains("DA:1,1"));
        assert!(out.contains("BRDA:1,0,0,1"));
    }

    #[test]
    fn sf_is_relativized_against_root_dir() {
        let json = r#"{"/proj/src/a.js":{"path":"/proj/src/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let root = summarize(&map);
        let mut buf = Vec::new();
        write(&root, Path::new("/proj"), &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("SF:src/a.js"), "got:\n{out}");
        assert!(!out.contains("SF:/proj/src/a.js"));
    }

    #[test]
    fn lf_lh_counts_unique_lines() {
        let json = r#"{
            "a.js": {
                "path": "a.js",
                "statementMap": {
                    "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
                    "1": {"start": {"line": 1, "column": 6}, "end": {"line": 1, "column": 10}},
                    "2": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}}
                },
                "fnMap": {},
                "branchMap": {},
                "s": {"0": 1, "1": 0, "2": 0},
                "f": {},
                "b": {}
            }
        }"#;
        let out = render(json);
        assert!(out.contains("LF:2"), "got:\n{out}");
        assert!(out.contains("LH:1"), "got:\n{out}");
    }
}
