use std::collections::BTreeMap;
use std::fmt::Write as _;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location, Position};
use oxc_coverage_v8::{
    V8CoverageRange, V8FunctionCoverage, apply_v8_coverage, extract_external_source_mapping_url,
    extract_inline_source_map,
};

const RANGE_CASES: &[(&str, bool)] = &[("ascii", false), ("unicode", true)];
const SCALING_SIZES: &[u32] = &[64, 256, 1_024];
const INLINE_SOURCE_MAP_CASES: &[(&str, InlineSourceMapEncoding)] =
    &[("base64", InlineSourceMapEncoding::Base64), ("percent", InlineSourceMapEncoding::Percent)];

#[derive(Clone, Copy)]
enum InlineSourceMapEncoding {
    Base64,
    Percent,
}

#[derive(Clone, Copy)]
enum RangeShape {
    Nested,
    Disjoint,
    NonLaminar,
}

impl RangeShape {
    fn label(self) -> &'static str {
        match self {
            Self::Nested => "nested",
            Self::Disjoint => "disjoint",
            Self::NonLaminar => "non_laminar",
        }
    }
}

/// Return shape of `branch_heavy_fixture`: instrumented source, the coverage
/// map, the V8 function ranges, and the per-branch span table.
type BranchFixture =
    (String, FileCoverage, Vec<V8FunctionCoverage>, BTreeMap<String, Vec<(u32, u32)>>);

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
    let mut source_utf16_len = 0u32;

    for line in 1..=lines {
        let line_start_byte = source.len();
        let line_start_utf16 = source_utf16_len;
        if unicode && line % 8 == 0 {
            let _ = writeln!(source, "const value{line} = \"emoji 😀\";");
        } else {
            let _ = writeln!(source, "const value{line} = input + {line};");
        }
        source_utf16_len = source_utf16_len.saturating_add(
            source[line_start_byte..].encode_utf16().count().try_into().unwrap_or(u32::MAX),
        );
        ranges.push(V8CoverageRange {
            start_offset: line_start_utf16,
            end_offset: source_utf16_len.saturating_sub(1),
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

    ranges.insert(0, V8CoverageRange { start_offset: 0, end_offset: source_utf16_len, count: 1 });

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

fn branch_heavy_fixture(branches: u32) -> BranchFixture {
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
        let _ = writeln!(source, "if (flag{id}) {{ hit{id}(); }} else {{ miss{id}(); }}");
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

fn scaling_fixture(
    size: u32,
    shape: RangeShape,
) -> (String, FileCoverage, Vec<V8FunctionCoverage>) {
    const NESTING_DEPTH: u32 = 4;
    const GROUP_WIDTH: u32 = NESTING_DEPTH * 2 + 2;

    let source = "x".repeat((size * GROUP_WIDTH) as usize);
    let mut statement_map = BTreeMap::new();
    let mut s = BTreeMap::new();
    let mut functions = Vec::new();

    let mut add_statement = |id: u32, start: u32, end: u32| {
        let key = id.to_string();
        statement_map.insert(key.clone(), loc(1, start, end));
        s.insert(key, 0);
    };

    match shape {
        RangeShape::Nested => {
            let mut id = 0;
            while id < size {
                let base = (id / NESTING_DEPTH) * GROUP_WIDTH;
                let group_end = base + GROUP_WIDTH - 1;
                let mut ranges = Vec::new();
                for depth in 0..NESTING_DEPTH.min(size - id) {
                    let start = base + depth;
                    let end = group_end - depth;
                    ranges.push(V8CoverageRange {
                        start_offset: start,
                        end_offset: end,
                        count: id + 1,
                    });
                    add_statement(id, start, end);
                    id += 1;
                }
                functions.push(V8FunctionCoverage {
                    function_name: String::new(),
                    ranges,
                    is_block_coverage: true,
                });
            }
        }
        RangeShape::Disjoint => {
            for id in 0..size {
                let start = id * GROUP_WIDTH;
                let end = start + GROUP_WIDTH - 1;
                add_statement(id, start, end);
                functions.push(V8FunctionCoverage {
                    function_name: String::new(),
                    ranges: vec![V8CoverageRange {
                        start_offset: start,
                        end_offset: end,
                        count: id + 1,
                    }],
                    is_block_coverage: false,
                });
            }
        }
        RangeShape::NonLaminar => {
            for pair in 0..size.div_ceil(2) {
                let base = pair * GROUP_WIDTH;
                let first_id = pair * 2;
                let second_id = first_id + 1;
                let first = V8CoverageRange {
                    start_offset: base,
                    end_offset: base + 6,
                    count: first_id + 1,
                };
                let second = V8CoverageRange {
                    start_offset: base + 3,
                    end_offset: base + 9,
                    count: second_id + 1,
                };
                add_statement(first_id, first.start_offset, first.end_offset);
                let mut ranges = vec![first];
                if second_id < size {
                    add_statement(second_id, second.start_offset, second.end_offset);
                    ranges.push(second);
                }
                functions.push(V8FunctionCoverage {
                    function_name: String::new(),
                    ranges,
                    is_block_coverage: true,
                });
            }
        }
    }

    let coverage = FileCoverage {
        path: format!("src/{}_scaling.ts", shape.label()),
        statement_map,
        fn_map: BTreeMap::new(),
        branch_map: BTreeMap::new(),
        s,
        f: BTreeMap::new(),
        b: BTreeMap::new(),
        b_t: None,
        input_source_map: None,
        x_fallow_function_map: None,
    };

    (source, coverage, functions)
}

fn digits(n: u32) -> u32 {
    if n == 0 {
        return 1;
    }
    n.ilog10() + 1
}

fn source_map_json(sources: u32) -> String {
    let source_list =
        (0..sources).map(|id| format!(r#""src/file{id}.ts""#)).collect::<Vec<_>>().join(",");
    format!(r#"{{"version":3,"sources":[{source_list}],"mappings":"","names":[]}}"#)
}

fn base64_urlsafe(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(((b1 & 0b1111) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(b2 & 0b11_1111) as usize] as char);
        }
    }
    out
}

fn percent_encode_json(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

fn inline_source_map_source(encoding: InlineSourceMapEncoding) -> String {
    let payload = source_map_json(200);
    let encoded = match encoding {
        InlineSourceMapEncoding::Base64 => base64_urlsafe(payload.as_bytes()),
        InlineSourceMapEncoding::Percent => percent_encode_json(&payload),
    };
    let marker = match encoding {
        InlineSourceMapEncoding::Base64 => "data:application/json;base64,",
        InlineSourceMapEncoding::Percent => "data:application/json,",
    };
    format!("{}\n//# sourceMappingURL={marker}{encoded}", "const x = 1;\n".repeat(500))
}

fn external_source_map_source() -> String {
    format!("{}\n//# sourceMappingURL=dist/app.js.map", "const x = 1;\n".repeat(2_000))
}

fn bench_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("v8_apply");

    for (label, unicode) in RANGE_CASES {
        let (source, coverage, functions) = fixture(1_000, *unicode);
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

fn bench_apply_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("v8_apply_scaling");

    for shape in [RangeShape::Nested, RangeShape::Disjoint, RangeShape::NonLaminar] {
        for size in SCALING_SIZES {
            let (source, coverage, functions) = scaling_fixture(*size, shape);
            group.bench_with_input(
                BenchmarkId::new(shape.label(), size),
                &coverage,
                |b, coverage| {
                    b.iter(|| {
                        let mut coverage = coverage.clone();
                        apply_v8_coverage(&mut coverage, &source, &functions, 0, &BTreeMap::new());
                        coverage
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_source_map_trailers(c: &mut Criterion) {
    let mut group = c.benchmark_group("source_map_trailer");

    for (label, encoding) in INLINE_SOURCE_MAP_CASES {
        let source = inline_source_map_source(*encoding);
        group.bench_with_input(BenchmarkId::new("inline", label), &source, |b, source| {
            b.iter(|| extract_inline_source_map(std::hint::black_box(source)));
        });
    }

    let source = external_source_map_source();
    group.bench_function("external", |b| {
        b.iter(|| extract_external_source_mapping_url(std::hint::black_box(&source)));
    });

    group.finish();
}

criterion_group!(benches, bench_apply, bench_apply_scaling, bench_source_map_trailers);
criterion_main!(benches);
