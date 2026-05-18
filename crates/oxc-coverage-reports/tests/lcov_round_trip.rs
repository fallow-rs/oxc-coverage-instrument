//! Parses our emitted LCOV back with the upstream `lcov` crate to assert the
//! tracefile is structurally valid: the panel verdict on PR E required this
//! gate so a future drift in the hand-rolled emitter shape (e.g. missing
//! `end_of_record`, wrong record order, malformed `BRDA` arity) fails the
//! suite instead of silently shipping a broken tracefile that Codecov / GitLab
//! / `genhtml` reject at parse time.
//!
//! The `lcov` crate is a dev-dependency only; the runtime emitter takes no
//! production dependencies.

use oxc_coverage_report::summarize;
use oxc_coverage_reports::lcov as emitter;
use oxc_coverage_types::parse_coverage_map;
use std::path::Path;

const FIXTURE: &str = r#"{
  "src/a.js": {
    "path": "src/a.js",
    "statementMap": {
      "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
      "1": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}}
    },
    "fnMap": {
      "0": {"name": "(anonymous_0)", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 5, "column": 0}}}
    },
    "branchMap": {
      "0": {"loc": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}}, "line": 2, "type": "if", "locations": [{"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 2}}, {"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 5}}]}
    },
    "s": {"0": 1, "1": 0},
    "f": {"0": 1},
    "b": {"0": [1, 0]}
  },
  "src/b.js": {
    "path": "src/b.js",
    "statementMap": {
      "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}
    },
    "fnMap": {},
    "branchMap": {},
    "s": {"0": 0},
    "f": {},
    "b": {}
  }
}"#;

#[test]
fn emitted_lcov_parses_with_upstream_crate() {
    let map = parse_coverage_map(FIXTURE).unwrap();
    let root = summarize(&map);
    let mut buf = Vec::new();
    emitter::write(&root, Path::new(""), &mut buf).unwrap();

    let mut record_count = 0usize;
    let reader = lcov::Reader::new(&buf[..]);
    for record in reader {
        let _record = record.expect("lcov parser must accept our emitted tracefile");
        record_count += 1;
    }
    assert!(record_count > 0, "lcov parser must observe at least one record");
}
