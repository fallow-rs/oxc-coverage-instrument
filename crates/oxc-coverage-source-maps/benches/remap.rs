use std::collections::BTreeMap;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_coverage_source_maps::{
    RemapOptions, SourceMapStore, remap_coverage, remap_coverage_with_options,
};
use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location, Position};
use srcmap_generator::SourceMapGenerator;

const SOURCE_PATH: &str = "src/app.ts";
const GENERATED_PATH: &str = "dist/app.js";

fn loc(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Location {
    Location {
        start: Position { line: start_line, column: start_col },
        end: Position { line: end_line, column: end_col },
    }
}

fn source_map_json(lines: u32, columns_per_line: u32) -> serde_json::Value {
    let mut generator = SourceMapGenerator::with_capacity(
        Some(GENERATED_PATH.to_string()),
        (lines * columns_per_line) as usize,
    );
    generator.set_assume_sorted(true);
    let source = generator.add_source(SOURCE_PATH);
    let content_line = "x".repeat(columns_per_line as usize + 8);
    generator.set_source_content(source, vec![content_line; lines as usize].join("\n"));

    for line in 0..lines {
        for column in 0..columns_per_line {
            generator.add_mapping(line, column * 2, source, line, column * 2);
        }
    }

    serde_json::from_str(&generator.to_json()).expect("generated source map is valid JSON")
}

fn file_coverage(entry_count: u32, map: serde_json::Value) -> FileCoverage {
    let mut statement_map = BTreeMap::new();
    let mut fn_map = BTreeMap::new();
    let mut branch_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let mut f = BTreeMap::new();
    let mut b = BTreeMap::new();

    for id in 0..entry_count {
        let key = id.to_string();
        let line = id + 1;
        let start_col = (id % 8) * 4;
        let end_col = start_col + 3;
        let entry_loc = loc(line, start_col, line, end_col);

        statement_map.insert(key.clone(), entry_loc.clone());
        s.insert(key.clone(), 0);

        fn_map.insert(
            key.clone(),
            FnEntry {
                name: format!("f{id}"),
                line,
                decl: loc(line, start_col, line, start_col + 1),
                loc: entry_loc.clone(),
            },
        );
        f.insert(key.clone(), 0);

        branch_map.insert(
            key.clone(),
            BranchEntry {
                loc: entry_loc,
                line,
                branch_type: "if".to_string(),
                locations: vec![
                    loc(line, start_col, line, start_col + 1),
                    loc(line, start_col + 2, line, end_col),
                ],
            },
        );
        b.insert(key, vec![0, 0]);
    }

    FileCoverage {
        path: GENERATED_PATH.to_string(),
        statement_map,
        fn_map,
        branch_map,
        s,
        f,
        b,
        b_t: None,
        input_source_map: Some(map),
        x_fallow_function_map: None,
    }
}

fn bench_remap(c: &mut Criterion) {
    let cases = [("medium", 200), ("large", 1_000)];
    let mut group = c.benchmark_group("remap");

    for (label, entry_count) in cases {
        let map = source_map_json(entry_count, 32);
        let coverage = file_coverage(entry_count, map.clone());

        group.bench_with_input(BenchmarkId::new("embedded", label), &coverage, |b, coverage| {
            b.iter(|| remap_coverage(coverage).expect("embedded remap succeeds"));
        });

        let mut store = SourceMapStore::new();
        store.add_map(GENERATED_PATH, &map);
        let mut coverage_without_map = coverage.clone();
        coverage_without_map.input_source_map = None;

        group.bench_with_input(
            BenchmarkId::new("store", label),
            &coverage_without_map,
            |b, coverage| {
                b.iter(|| store.transform_coverage(coverage).expect("store remap succeeds"));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("drop_unmapped", label),
            &coverage,
            |b, coverage| {
                b.iter(|| {
                    remap_coverage_with_options(coverage, RemapOptions { drop_unmapped: true })
                        .expect("drop remap succeeds")
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_remap);
criterion_main!(benches);
