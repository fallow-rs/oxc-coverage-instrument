//! CLI for the oxc-coverage suite.
//!
//! Two invocation shapes:
//!
//! Instrument (default, also via the explicit `instrument` subcommand):
//!   `oxc-coverage-instrument FILE`                  # instrument, print to stdout
//!   `oxc-coverage-instrument FILE -o OUTPUT`        # instrument, write to file
//!   `oxc-coverage-instrument FILE --coverage-map`   # print coverage map JSON
//!   `oxc-coverage-instrument FILE --source-map`     # include source map
//!
//! Report (consumes a `coverage-final.json`):
//!   `oxc-coverage-instrument report --format text COVERAGE.json`
//!   `oxc-coverage-instrument report --format text-summary COVERAGE.json`
//!   `oxc-coverage-instrument report --format json-summary COVERAGE.json -o summary.json`

#![expect(clippy::print_stdout, clippy::print_stderr, reason = "CLI binary")]

use std::process::ExitCode;

use oxc_coverage_instrument::{InstrumentOptions, InstrumentResult, instrument};
use oxc_coverage_report::summarize;
use oxc_coverage_reports::Format;
use oxc_coverage_types::parse_coverage_map;

struct InstrumentArgs {
    filename: String,
    output_file: Option<String>,
    coverage_map_only: bool,
    source_map: bool,
    coverage_variable: String,
}

struct ReportArgs {
    coverage_file: String,
    output_file: Option<String>,
    format: Format,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match dispatch(&args) {
        Ok(code) | Err(code) => code,
    }
}

fn dispatch(args: &[String]) -> Result<ExitCode, ExitCode> {
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        return Ok(ExitCode::SUCCESS);
    }
    if args[1] == "--version" || args[1] == "-V" {
        print_version();
        return Ok(ExitCode::SUCCESS);
    }

    match args[1].as_str() {
        "report" => parse_report_args(&args[2..]).map(|a| run_report(&a)),
        "instrument" => parse_instrument_args(&args[2..]).map(|a| run_instrument(&a)),
        _ => parse_instrument_args(&args[1..]).map(|a| run_instrument(&a)),
    }
}

fn parse_instrument_args(args: &[String]) -> Result<InstrumentArgs, ExitCode> {
    if args.is_empty() {
        eprintln!("error: instrument requires a file argument");
        print_usage();
        return Err(ExitCode::FAILURE);
    }

    let mut cli = InstrumentArgs {
        filename: args[0].clone(),
        output_file: None,
        coverage_map_only: false,
        source_map: false,
        coverage_variable: "__coverage__".to_string(),
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => cli.output_file = Some(take_value(args, &mut i, "--output")?),
            "--coverage-map" => cli.coverage_map_only = true,
            "--source-map" => cli.source_map = true,
            "--coverage-variable" => {
                cli.coverage_variable = take_value(args, &mut i, "--coverage-variable")?;
            }
            "--version" | "-V" => {
                print_version();
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

fn parse_report_args(args: &[String]) -> Result<ReportArgs, ExitCode> {
    let mut format: Option<Format> = None;
    let mut output_file: Option<String> = None;
    let mut coverage_file: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--format" | "-f" => {
                let value = take_value(args, &mut i, "--format")?;
                format = Some(Format::parse(&value).ok_or_else(|| {
                    eprintln!(
                        "error: unknown format '{value}'. Supported: text, text-summary, json-summary"
                    );
                    ExitCode::FAILURE
                })?);
            }
            "-o" | "--output" => output_file = Some(take_value(args, &mut i, "--output")?),
            "--help" | "-h" => {
                print_report_usage();
                return Err(ExitCode::SUCCESS);
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown option: {other}");
                print_report_usage();
                return Err(ExitCode::FAILURE);
            }
            other => {
                if coverage_file.is_some() {
                    eprintln!("error: only one coverage file may be supplied (got '{other}')");
                    return Err(ExitCode::FAILURE);
                }
                coverage_file = Some(other.to_owned());
            }
        }
        i += 1;
    }

    let coverage_file = coverage_file.ok_or_else(|| {
        eprintln!("error: report requires a coverage-final.json path");
        print_report_usage();
        ExitCode::FAILURE
    })?;
    let format = format.unwrap_or(Format::Text);

    Ok(ReportArgs { coverage_file, output_file, format })
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

fn run_instrument(cli: &InstrumentArgs) -> ExitCode {
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

fn run_report(args: &ReportArgs) -> ExitCode {
    let json = match std::fs::read_to_string(&args.coverage_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", args.coverage_file);
            return ExitCode::FAILURE;
        }
    };
    let map = match parse_coverage_map(&json) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: failed to parse coverage map: {e}");
            return ExitCode::FAILURE;
        }
    };
    let root = summarize(&map);

    match &args.output_file {
        Some(path) => {
            let mut file = match std::fs::File::create(path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("error: cannot write {path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = args.format.write(&root, &mut file) {
                eprintln!("error: failed to render report: {e}");
                return ExitCode::FAILURE;
            }
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            if let Err(e) = args.format.write(&root, &mut handle) {
                eprintln!("error: failed to render report: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
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

fn write_outputs(cli: &InstrumentArgs, result: &InstrumentResult) -> ExitCode {
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

fn print_version() {
    println!("oxc-coverage-instrument {}", env!("CARGO_PKG_VERSION"));
}

fn print_usage() {
    eprintln!(
        "oxc-coverage-instrument {}
Istanbul-compatible JS/TS coverage instrumentation and reporting using Oxc

USAGE:
    oxc-coverage-instrument <file> [options]
    oxc-coverage-instrument instrument <file> [options]
    oxc-coverage-instrument report --format <fmt> <coverage.json> [options]

INSTRUMENT OPTIONS:
    -o, --output <file>          Write instrumented code to file (default: stdout)
    --coverage-map               Print only the coverage map JSON
    --source-map                 Generate source map
    --coverage-variable <name>   Coverage variable name (default: __coverage__)

REPORT OPTIONS:
    -f, --format <fmt>           Output format: text (default), text-summary, json-summary
    -o, --output <file>          Write report to file (default: stdout)

GLOBAL OPTIONS:
    -V, --version                Print version
    -h, --help                   Print help",
        env!("CARGO_PKG_VERSION")
    );
}

fn print_report_usage() {
    eprintln!(
        "oxc-coverage-instrument report

USAGE:
    oxc-coverage-instrument report --format <fmt> <coverage.json> [options]

OPTIONS:
    -f, --format <fmt>           Output format: text (default), text-summary, json-summary
    -o, --output <file>          Write report to file (default: stdout)
    -h, --help                   Print this help"
    );
}
