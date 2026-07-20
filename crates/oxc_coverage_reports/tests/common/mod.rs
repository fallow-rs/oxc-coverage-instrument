//! Fixtures shared by the reporter integration tests.

/// Two files exercising every counter kind: a hit statement, a missed
/// statement, a called function, and a partially taken `if` branch in
/// `src/a.js`, plus a fully uncovered `src/b.js`.
pub const TWO_FILE_MAP: &str = r#"{
  "src/a.js": {
    "path": "src/a.js",
    "statementMap": {
      "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
      "1": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}},
      "2": {"start": {"line": 4, "column": 0}, "end": {"line": 4, "column": 5}}
    },
    "fnMap": {
      "0": {"name": "foo", "line": 1, "decl": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 3}}, "loc": {"start": {"line": 1, "column": 0}, "end": {"line": 5, "column": 0}}}
    },
    "branchMap": {
      "0": {"loc": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}}, "line": 2, "type": "if", "locations": [{"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 2}}, {"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 5}}]}
    },
    "s": {"0": 1, "1": 1, "2": 0},
    "f": {"0": 1},
    "b": {"0": [1, 0]}
  },
  "src/b.js": {
    "path": "src/b.js",
    "statementMap": {
      "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
      "1": {"start": {"line": 3, "column": 0}, "end": {"line": 3, "column": 5}}
    },
    "fnMap": {},
    "branchMap": {},
    "s": {"0": 0, "1": 0},
    "f": {},
    "b": {}
  }
}"#;
