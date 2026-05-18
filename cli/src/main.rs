//! CLI for oxc-coverage-instrument.
//!
//! Usage:
//!   `oxc-coverage-instrument FILE`                  # instrument, print to stdout
//!   `oxc-coverage-instrument FILE -o OUTPUT`        # instrument, write to file
//!   `oxc-coverage-instrument FILE --coverage-map`   # print coverage map JSON
//!   `oxc-coverage-instrument FILE --source-map`     # include source map

#![expect(clippy::print_stdout, clippy::print_stderr, reason = "CLI binary")]

use std::process::ExitCode;

use oxc_coverage_instrument::{InstrumentOptions, InstrumentResult, instrument};

struct CliArgs {
    filename: String,
    output_file: Option<String>,
    coverage_map_only: bool,
    source_map: bool,
    coverage_variable: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match parse_args(&args) {
        Ok(cli) => run(&cli),
        Err(code) => code,
    }
}

fn parse_args(args: &[String]) -> Result<CliArgs, ExitCode> {
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        return Err(ExitCode::SUCCESS);
    }
    if args[1] == "--version" || args[1] == "-V" {
        println!("oxc-coverage-instrument {}", env!("CARGO_PKG_VERSION"));
        return Err(ExitCode::SUCCESS);
    }

    let mut cli = CliArgs {
        filename: args[1].clone(),
        output_file: None,
        coverage_map_only: false,
        source_map: false,
        coverage_variable: "__coverage__".to_string(),
    };

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => cli.output_file = Some(take_value(args, &mut i, "--output")?),
            "--coverage-map" => cli.coverage_map_only = true,
            "--source-map" => cli.source_map = true,
            "--coverage-variable" => {
                cli.coverage_variable = take_value(args, &mut i, "--coverage-variable")?;
            }
            "--version" | "-V" => {
                println!("oxc-coverage-instrument {}", env!("CARGO_PKG_VERSION"));
                return Err(ExitCode::SUCCESS);
            }
            other => {
                eprintln!("error: unknown option: {other}");
                print_usage();
                return Err(ExitCode::FAILURE);
            }
        }
        i += 1;
    }

    Ok(cli)
}

fn take_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, ExitCode> {
    *i += 1;
    if *i < args.len() {
        Ok(args[*i].clone())
    } else {
        eprintln!("error: {flag} requires a value");
        Err(ExitCode::FAILURE)
    }
}

fn run(cli: &CliArgs) -> ExitCode {
    let source = match std::fs::read_to_string(&cli.filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", cli.filename);
            return ExitCode::FAILURE;
        }
    };

    let opts = InstrumentOptions {
        coverage_variable: cli.coverage_variable.clone(),
        source_map: cli.source_map,
        ..InstrumentOptions::default()
    };

    let result = match instrument(&source, &cli.filename, &opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if cli.coverage_map_only {
        return print_coverage_map(&result);
    }
    write_outputs(cli, &result)
}

fn print_coverage_map(result: &InstrumentResult) -> ExitCode {
    match serde_json::to_string_pretty(&result.coverage_map) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: failed to serialize coverage map: {e}");
            ExitCode::FAILURE
        }
    }
}

fn write_outputs(cli: &CliArgs, result: &InstrumentResult) -> ExitCode {
    if let Some(path) = &cli.output_file {
        if let Err(code) = write_code_and_map(path, &cli.filename, result) {
            return code;
        }
    } else {
        print!("{}", result.code);
    }

    if let Some(sm) = &result.source_map
        && let Err(code) = write_or_print_source_map(cli.output_file.as_deref(), sm)
    {
        return code;
    }

    ExitCode::SUCCESS
}

fn write_code_and_map(
    path: &str,
    filename: &str,
    result: &InstrumentResult,
) -> Result<(), ExitCode> {
    if let Err(e) = std::fs::write(path, &result.code) {
        eprintln!("error: cannot write {path}: {e}");
        return Err(ExitCode::FAILURE);
    }
    let map_path = format!("{path}.map.json");
    let map_json = serde_json::to_string_pretty(&result.coverage_map).map_err(|e| {
        eprintln!("error: failed to serialize coverage map: {e}");
        ExitCode::FAILURE
    })?;
    if let Err(e) = std::fs::write(&map_path, map_json) {
        eprintln!("error: cannot write {map_path}: {e}");
        return Err(ExitCode::FAILURE);
    }
    eprintln!("Instrumented: {filename} \u{2192} {path}");
    eprintln!("Coverage map: {map_path}");
    Ok(())
}

fn write_or_print_source_map(output_file: Option<&str>, sm: &str) -> Result<(), ExitCode> {
    match output_file {
        Some(out) => {
            let sm_path = format!("{out}.map");
            if let Err(e) = std::fs::write(&sm_path, sm) {
                eprintln!("error: cannot write {sm_path}: {e}");
                return Err(ExitCode::FAILURE);
            }
            eprintln!("Source map: {sm_path}");
        }
        None => eprintln!("{sm}"),
    }
    Ok(())
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
