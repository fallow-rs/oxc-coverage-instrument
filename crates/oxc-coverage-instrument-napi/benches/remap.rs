use std::collections::{BTreeMap, HashMap};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_coverage_instrument::{BranchEntry, FileCoverage, FnEntry, Location, Position};
use oxc_coverage_instrument_napi::remap_coverage_map_with_loader;
use srcmap_generator::SourceMapGenerator;

const SOURCE_PATH: &str = "src/app.ts";

fn loc(line: u32, start_col: u32, end_col: u32) -> Location {
    Location {
        start: Position { line, column: start_col },
        end: Position { line, column: end_col },
    }
}

fn source_map_json(generated_path: &str, lines: u32) -> String {
    let mut generator =
        SourceMapGenerator::with_capacity(Some(generated_path.to_string()), lines as usize * 16);
    generator.set_assume_sorted(true);
    let source = generator.add_source(SOURCE_PATH);
    generator.set_source_content(source, vec!["x".repeat(80); lines as usize].join("\n"));
    for line in 0..lines {
        for column in 0..16 {
            generator.add_mapping(line, column * 2, source, line, column * 2);
        }
    }
    generator.to_json()
}

fn file_coverage(path: String, entries: u32) -> FileCoverage {
    let mut statement_map = BTreeMap::new();
    let mut fn_map = BTreeMap::new();
    let mut branch_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let mut f = BTreeMap::new();
    let mut b = BTreeMap::new();

    for id in 0..entries {
        let key = id.to_string();
        let line = id + 1;
        statement_map.insert(key.clone(), loc(line, 0, 18));
        s.insert(key.clone(), 0);
        fn_map.insert(
            key.clone(),
            FnEntry { name: format!("f{id}"), line, decl: loc(line, 0, 6), loc: loc(line, 0, 18) },
        );
        f.insert(key.clone(), 0);
        branch_map.insert(
            key.clone(),
            BranchEntry {
                loc: loc(line, 0, 18),
                line,
                branch_type: "if".to_string(),
                locations: vec![loc(line, 0, 8), loc(line, 9, 18)],
            },
        );
        b.insert(key, vec![0, 0]);
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

fn fixture(file_count: usize, entries: u32) -> (String, HashMap<String, String>) {
    let mut coverage_map = BTreeMap::new();
    let mut source_maps = HashMap::new();
    for file_id in 0..file_count {
        let path = format!("dist/file{file_id}.js");
        coverage_map.insert(path.clone(), file_coverage(path.clone(), entries));
        source_maps.insert(path.clone(), source_map_json(&path, entries));
    }
    let coverage_json =
        serde_json::to_string(&coverage_map).expect("coverage map serializes to JSON");
    (coverage_json, source_maps)
}

fn bench_napi_remap(c: &mut Criterion) {
    let mut group = c.benchmark_group("napi_remap");
    let cases = [("small", 4, 80), ("large", 20, 200)];

    for (label, files, entries) in cases {
        let (coverage_json, source_maps) = fixture(files, entries);
        group.bench_with_input(
            BenchmarkId::new("with_loader", label),
            &coverage_json,
            |b, coverage_json| {
                b.iter(|| {
                    remap_coverage_map_with_loader(coverage_json.clone(), source_maps.clone(), None)
                        .expect("napi remap succeeds")
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_napi_remap);
criterion_main!(benches);
