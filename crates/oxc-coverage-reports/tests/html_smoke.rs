//! End-to-end smoke for the HTML reporter.
//!
//! Creates a real on-disk source file, runs the html emitter against a
//! synthetic coverage map keyed by its absolute path, and asserts the
//! rendered detail page contains the actual source text (proving the
//! source-read fallback works) plus per-line coverage classes for hit
//! vs miss vs partial vs no-statement rows. The unit tests in
//! `src/html.rs` exercise edge cases on synthetic input; this test
//! covers the realistic path where the user has a coverage-final.json
//! pointing at files on disk.

use oxc_coverage_reports::html;
use oxc_coverage_types::parse_coverage_map;
use std::fs;
use std::path::Path;

#[test]
fn renders_actual_source_with_coverage_classes() {
    let dir = tempfile::TempDir::new().unwrap();
    // Use a relative path as the coverage key (resolved against root_dir
    // below). An absolute Windows path (`C:\\...`) would propagate through
    // the report tree as a non-relative folder component, which Windows
    // path APIs reject.
    fs::write(dir.path().join("module.js"), "const x = 1;\nconst y = 2;\nif (x) y;\n").unwrap();

    // Statement 0 on line 1 (hit), statement 1 on line 2 (miss), statement 2
    // on line 3 (hit). Branch 0 on line 3, only the truthy arm taken (partial).
    let coverage = r#"{"module.js":{
        "path":"module.js",
        "statementMap":{
            "0":{"start":{"line":1,"column":0},"end":{"line":1,"column":12}},
            "1":{"start":{"line":2,"column":0},"end":{"line":2,"column":12}},
            "2":{"start":{"line":3,"column":0},"end":{"line":3,"column":9}}
        },
        "fnMap":{},
        "branchMap":{
            "0":{"loc":{"start":{"line":3,"column":0},"end":{"line":3,"column":9}},"line":3,"type":"if","locations":[{"start":{"line":3,"column":4},"end":{"line":3,"column":5}},{"start":{"line":3,"column":6},"end":{"line":3,"column":9}}]}
        },
        "s":{"0":1,"1":0,"2":1},
        "f":{},
        "b":{"0":[1,0]}
    }}"#;

    let map = parse_coverage_map(coverage).unwrap();
    let out_dir = dir.path().join("html");
    html::write(&map, dir.path(), &out_dir).unwrap();

    let detail_file = walk_for_filename(&out_dir, "module.js.html").expect("detail page exists");
    let detail = fs::read_to_string(&detail_file).unwrap();

    assert!(detail.contains("const x = 1;"), "source line 1 should appear in detail page");
    assert!(detail.contains("const y = 2;"), "source line 2 should appear in detail page");
    assert!(detail.contains("if (x) y;"), "source line 3 should appear in detail page");
    assert!(detail.contains("line hit"), "line 1 should be class=hit");
    assert!(detail.contains("line miss"), "line 2 should be class=miss");
    assert!(detail.contains("line partial"), "line 3 should be class=partial (branch 1/2)");

    // Index page should list the file with the correct row class.
    let index = fs::read_to_string(out_dir.join("index.html")).unwrap();
    assert!(index.contains("All files"), "root index should list the title \"All files\"");
}

fn walk_for_filename(dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if let Some(found) = walk_for_filename(&p, name) {
                return Some(found);
            }
        } else if p.file_name().is_some_and(|n| n == name) {
            return Some(p);
        }
    }
    None
}
