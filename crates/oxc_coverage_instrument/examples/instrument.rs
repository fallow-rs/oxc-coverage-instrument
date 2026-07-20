#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "example binary: println and eprintln are the output"
)]
//! Runnable example: instrument a JS source string and print the output.
//!
//! Run with: `cargo run --example instrument`

use std::collections::BTreeMap;

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn main() {
    let source = r"
function add(a, b) {
  if (a > 0) {
    return a + b;
  } else {
    return b;
  }
}

const multiply = (x, y) => x * y;

class Calculator {
  divide(a, b) {
    return b !== 0 ? a / b : 0;
  }

  subtract(a, b) {
    return a - b;
  }
}

const result = add(1, 2) || multiply(3, 4);
";

    let result = instrument(source, "example.js", &InstrumentOptions::default()).unwrap();

    let mut root = BTreeMap::new();
    root.insert(result.coverage_map.path.clone(), &result.coverage_map);
    eprintln!("=== Coverage Map (coverage-final.json) ===");
    eprintln!("{}", serde_json::to_string_pretty(&root).unwrap());

    eprintln!("\n=== Functions ===");
    for (id, entry) in &result.coverage_map.fn_map {
        eprintln!("  fn[{id}]: {} (line {})", entry.name, entry.line);
    }

    eprintln!("\n=== Stats ===");
    eprintln!("  Statements: {}", result.coverage_map.statement_map.len());
    eprintln!("  Functions:  {}", result.coverage_map.fn_map.len());
    eprintln!("  Branches:   {}", result.coverage_map.branch_map.len());

    eprintln!("\n=== Instrumented Code ===");
    println!("{}", result.code);
}
