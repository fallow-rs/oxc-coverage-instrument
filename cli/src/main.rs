//! CLI for oxc-coverage-instrument.
//!
//! Usage:
//!   oxc-coverage-instrument <file>                  # instrument, print to stdout
//!   oxc-coverage-instrument <file> -o <output>      # instrument, write to file
//!   oxc-coverage-instrument <file> --coverage-map   # print coverage map JSON
//!   oxc-coverage-instrument <file> --source-map     # include source map

#![expect(clippy::print_stdout, clippy::print_stderr, reason = "CLI binary")]

use std::process::ExitCode;

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let filename = &args[1];
    let mut output_file: Option<&str> = None;
    let mut coverage_map_only = false;
    let mut source_map = false;
    let mut coverage_variable = "__coverage__".to_string();

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(&args[i]);
                } else {
                    eprintln!("error: --output requires a file path");
                    return ExitCode::FAILURE;
                }
            }
            "--coverage-map" => coverage_map_only = true,
            "--source-map" => source_map = true,
            "--coverage-variable" => {
                i += 1;
                if i < args.len() {
                    coverage_variable.clone_from(&args[i]);
                } else {
                    eprintln!("error: --coverage-variable requires a value");
                    return ExitCode::FAILURE;
                }
            }
            "--version" | "-V" => {
                println!("oxc-coverage-instrument {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("error: unknown option: {other}");
                print_usage();
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let source = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {filename}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let opts = InstrumentOptions {
        coverage_variable,
        source_map,
        input_source_map: None,
    };

    let result = match instrument(&source, filename, &opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if coverage_map_only {
        let json = serde_json::to_string_pretty(&result.coverage_map).unwrap_or_default();
        println!("{json}");
        return ExitCode::SUCCESS;
    }

    let output = &result.code;

    if let Some(path) = output_file {
        if let Err(e) = std::fs::write(path, output) {
            eprintln!("error: cannot write {path}: {e}");
            return ExitCode::FAILURE;
        }
        // Write coverage map alongside
        let map_path = format!("{path}.map.json");
        let map_json = serde_json::to_string_pretty(&result.coverage_map).unwrap_or_default();
        if let Err(e) = std::fs::write(&map_path, map_json) {
            eprintln!("error: cannot write {map_path}: {e}");
            return ExitCode::FAILURE;
        }
        eprintln!("Instrumented: {filename} → {path}");
        eprintln!("Coverage map: {map_path}");
    } else {
        print!("{output}");
    }

    if let Some(sm) = &result.source_map {
        if let Some(out) = output_file {
            let sm_path = format!("{out}.map");
            if let Err(e) = std::fs::write(&sm_path, sm) {
                eprintln!("error: cannot write {sm_path}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("Source map: {sm_path}");
        } else {
            eprintln!("{sm}");
        }
    }

    ExitCode::SUCCESS
}

fn print_usage() {
    eprintln!(
        "oxc-coverage-instrument {}
Istanbul-compatible JS/TS coverage instrumentation using Oxc

USAGE:
    oxc-coverage-instrument <file> [options]

OPTIONS:
    -o, --output <file>          Write instrumented code to file (default: stdout)
    --coverage-map               Print only the coverage map JSON
    --source-map                 Generate source map
    --coverage-variable <name>   Coverage variable name (default: __coverage__)
    -V, --version                Print version
    -h, --help                   Print help",
        env!("CARGO_PKG_VERSION")
    );
}
