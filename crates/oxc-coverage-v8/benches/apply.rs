use std::collections::BTreeMap;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location, Position};
use oxc_coverage_v8::{V8CoverageRange, V8FunctionCoverage, apply_v8_coverage};

fn loc(line: u32, start_col: u32, end_col: u32) -> Location {
    Location {
        start: Position { line, column: start_col },
        end: Position { line, column: end_col },
    }
}

fn fixture(lines: u32, unicode: bool) -> (String, FileCoverage, Vec<V8FunctionCoverage>) {
    let mut source = String::new();
    let mut statement_map = BTreeMap::new();
    let mut fn_map = BTreeMap::new();
    let mut branch_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let mut f = BTreeMap::new();
    let mut b = BTreeMap::new();
    let mut ranges = Vec::new();

    for line in 1..=lines {
        let line_start = source.len() as u32;
        if unicode && line % 8 == 0 {
            source.push_str(&format!("const value{line} = \"emoji 😀\";\n"));
        } else {
            source.push_str(&format!("const value{line} = input + {line};\n"));
        }
        let line_end = source.len() as u32;
        ranges.push(V8CoverageRange {
            start_offset: line_start,
            end_offset: line_end.saturating_sub(1),
            count: u32::from(line % 7 != 0),
        });

        let key = (line - 1).to_string();
        statement_map.insert(key.clone(), loc(line, 0, 18));
        s.insert(key, 0);

        if line % 20 == 0 {
            let fn_key = (line / 20).to_string();
            fn_map.insert(
                fn_key.clone(),
                FnEntry {
                    name: format!("fn{line}"),
                    line,
                    decl: loc(line, 0, 8),
                    loc: loc(line, 0, 18),
                },
            );
            f.insert(fn_key, 0);
        }

        if line % 10 == 0 {
            let branch_key = (line / 10).to_string();
            branch_map.insert(
                branch_key.clone(),
                BranchEntry {
                    loc: loc(line, 0, 18),
                    line,
                    branch_type: "if".to_string(),
                    locations: vec![loc(line, 0, 8), loc(line, 9, 18)],
                },
            );
            b.insert(branch_key, vec![0, 0]);
        }
    }

    ranges
        .insert(0, V8CoverageRange { start_offset: 0, end_offset: source.len() as u32, count: 1 });

    let coverage = FileCoverage {
        path: "src/app.ts".to_string(),
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t: None,
        input_source_map: None,
        x_fallow_function_map: None,
    };
    let functions =
        vec![V8FunctionCoverage { function_name: String::new(), ranges, is_block_coverage: true }];

    (source, coverage, functions)
}

fn branch_heavy_fixture(
    branches: u32,
) -> (String, FileCoverage, Vec<V8FunctionCoverage>, BTreeMap<String, Vec<(u32, u32)>>) {
    let mut source = String::new();
    let mut statement_map = BTreeMap::new();
    let mut branch_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let f = BTreeMap::new();
    let mut b = BTreeMap::new();
    let mut spans = BTreeMap::new();
    let mut ranges = Vec::new();

    for id in 0..branches {
        let line = id + 1;
        let line_start = source.len() as u32;
        source.push_str(&format!("if (flag{id}) {{ hit{id}(); }} else {{ miss{id}(); }}\n"));
        let true_start = line_start + 13 + digits(id);
        let true_end = true_start + 6 + digits(id);
        let false_start = true_end + 12;
        let false_end = false_start + 7 + digits(id);
        let line_end = source.len() as u32;

        ranges.push(V8CoverageRange {
            start_offset: line_start,
            end_offset: line_end.saturating_sub(1),
            count: 1,
        });
        ranges.push(V8CoverageRange { start_offset: true_start, end_offset: true_end, count: 1 });
        ranges.push(V8CoverageRange {
            start_offset: false_start,
            end_offset: false_end,
            count: u32::from(id % 3 != 0),
        });

        let key = id.to_string();
        statement_map.insert(key.clone(), loc(line, 0, 24));
        s.insert(key.clone(), 0);
        branch_map.insert(
            key.clone(),
            BranchEntry {
                loc: loc(line, 0, 24),
                line,
                branch_type: "if".to_string(),
                locations: vec![loc(line, 12, 20), loc(line, 30, 40)],
            },
        );
        b.insert(key.clone(), vec![0, 0]);
        spans.insert(key, vec![(true_start, true_end), (false_start, false_end)]);
    }

    ranges
        .insert(0, V8CoverageRange { start_offset: 0, end_offset: source.len() as u32, count: 1 });

    let coverage = FileCoverage {
        path: "src/branches.ts".to_string(),
        statement_map,
        fn_map: BTreeMap::new(),
        branch_map,
        s,
        f,
        b,
        b_t: None,
        input_source_map: None,
        x_fallow_function_map: None,
    };
    let functions =
        vec![V8FunctionCoverage { function_name: String::new(), ranges, is_block_coverage: true }];

    (source, coverage, functions, spans)
}

fn digits(n: u32) -> u32 {
    if n == 0 {
        return 1;
    }
    n.ilog10() + 1
}

fn bench_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("v8_apply");
    let cases = [("ascii", false), ("unicode", true)];

    for (label, unicode) in cases {
        let (source, coverage, functions) = fixture(1_000, unicode);
        group.bench_with_input(BenchmarkId::new("ranges", label), &coverage, |b, coverage| {
            b.iter(|| {
                let mut coverage = coverage.clone();
                apply_v8_coverage(&mut coverage, &source, &functions, 0, &BTreeMap::new());
                coverage
            });
        });
    }

    let (source, coverage, functions, spans) = branch_heavy_fixture(1_000);
    group.bench_with_input(BenchmarkId::new("branches", "dense"), &coverage, |b, coverage| {
        b.iter(|| {
            let mut coverage = coverage.clone();
            apply_v8_coverage(&mut coverage, &source, &functions, 0, &spans);
            coverage
        });
    });

    group.finish();
}

criterion_group!(benches, bench_apply);
criterion_main!(benches);
