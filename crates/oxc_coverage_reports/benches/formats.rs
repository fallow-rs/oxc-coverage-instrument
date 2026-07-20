//! Tree summarization and the single-stream reporters, over a synthetic
//! coverage map at two project sizes.

use std::collections::BTreeMap;
use std::path::Path;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_coverage_report::{CoverageMap, ReportNode, summarize};
use oxc_coverage_reports::Format;
use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location, Position};

const CASES: &[(&str, usize, u32)] = &[("small", 16, 80), ("large", 160, 160)];
const FORMATS: &[Format] =
    &[Format::Text, Format::TextSummary, Format::JsonSummary, Format::Lcov, Format::Cobertura];

fn loc(line: u32, start_col: u32, end_col: u32) -> Location {
    Location {
        start: Position { line, column: start_col },
        end: Position { line, column: end_col },
    }
}

fn coverage_for(path: String, lines: u32) -> FileCoverage {
    let mut statement_map = BTreeMap::new();
    let mut fn_map = BTreeMap::new();
    let mut branch_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let mut f = BTreeMap::new();
    let mut b = BTreeMap::new();

    for line in 1..=lines {
        let key = (line - 1).to_string();
        statement_map.insert(key.clone(), loc(line, 2, 24));
        s.insert(key, u32::from(line % 7 != 0));

        if line % 12 == 0 {
            let fn_key = line.to_string();
            fn_map.insert(
                fn_key.clone(),
                FnEntry {
                    name: format!("compute_{line}"),
                    line,
                    decl: loc(line, 9, 18),
                    loc: loc(line, 2, 32),
                },
            );
            f.insert(fn_key, u32::from(line % 5 != 0));
        }

        if line % 9 == 0 {
            let branch_key = line.to_string();
            branch_map.insert(
                branch_key.clone(),
                BranchEntry {
                    loc: loc(line, 10, 42),
                    line,
                    branch_type: "cond-expr".to_string(),
                    locations: vec![loc(line, 20, 25), loc(line, 30, 36)],
                },
            );
            b.insert(branch_key, vec![1, u32::from(line % 2 == 0)]);
        }
    }

    FileCoverage {
        path,
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t: None,
        input_source_map: None,
        x_fallow_function_map: None,
    }
}

/// A coverage map of `file_count` files spread over 16 packages, each file
/// carrying one statement per line plus periodic function and branch
/// entries.
fn fixture(file_count: usize, lines: u32) -> CoverageMap {
    let mut map = BTreeMap::new();

    for file_id in 0..file_count {
        let path = format!("packages/pkg{}/src/file{file_id}.ts", file_id % 16);
        map.insert(path.clone(), coverage_for(path, lines));
    }

    map
}

fn bench_summarize(c: &mut Criterion) {
    let mut group = c.benchmark_group("report_summarize");

    for (label, file_count, lines) in CASES {
        let map = fixture(*file_count, *lines);
        group.bench_with_input(BenchmarkId::new("coverage_map", label), &map, |b, map| {
            b.iter(|| summarize(map));
        });
    }

    group.finish();
}

fn bench_stream_formats(c: &mut Criterion) {
    let mut group = c.benchmark_group("report_format");

    for (label, file_count, lines) in CASES {
        let map = fixture(*file_count, *lines);
        let root = summarize(&map);

        for format in FORMATS {
            group.bench_with_input(
                BenchmarkId::new(format.as_str(), label),
                &root,
                |b, root: &ReportNode| {
                    b.iter(|| {
                        let mut out = Vec::new();
                        format.write(root, Path::new(""), &mut out).expect("report writes");
                        out
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_summarize, bench_stream_formats);
criterion_main!(benches);
