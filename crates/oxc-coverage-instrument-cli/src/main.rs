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
//!   `oxc-coverage-instrument report --format lcov --root /repo COVERAGE.json -o lcov.info`
//!   `oxc-coverage-instrument report --format cobertura --root /repo COVERAGE.json -o cobertura.xml`
//!   `oxc-coverage-instrument report --format html --root /repo COVERAGE.json --output-dir coverage/`

#![expect(clippy::print_stdout, clippy::print_stderr, reason = "CLI binary")]

use std::path::PathBuf;
use std::process::ExitCode;

use oxc_coverage_instrument::{InstrumentOptions, InstrumentResult, instrument};
use oxc_coverage_report::{CoverageMap, ReportNode, summarize};
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
    /// Directory for multi-file formats (currently only `html`). Defaults to
    /// `coverage/` when `--format html` is selected and `--output-dir` is
    /// not supplied.
    output_dir: Option<PathBuf>,
    format: Format,
    /// Root directory used to relativize `SF:` (lcov) and `<class filename>`
    /// (cobertura) paths, and to resolve relative `file.path` entries to disk
    /// when rendering the html source view. Defaults to the current working
    /// directory.
    root_dir: PathBuf,
    /// `--threshold` (html only). Percentage cutoff for the high / medium
    /// coverage bucket and the "below X% threshold" sentence on the
    /// index page. Defaults to 80 if not supplied. Validated to be in
    /// `[0, 100]` at parse time.
    html_threshold: f64,
    /// Aggregate line-coverage floor. When supplied, reports still render,
    /// then the process exits with code 2 if total line coverage is below
    /// the configured percentage.
    fail_under: Option<f64>,
}

struct ReportArgsDraft {
    coverage_file: Option<String>,
    output_file: Option<String>,
    output_dir: Option<PathBuf>,
    format: Option<Format>,
    root_dir: Option<PathBuf>,
    html_threshold: f64,
    threshold_was_set: bool,
    fail_under: Option<f64>,
}

impl ReportArgsDraft {
    fn new() -> Self {
        Self {
            coverage_file: None,
            output_file: None,
            output_dir: None,
            format: None,
            root_dir: None,
            html_threshold: 80.0,
            threshold_was_set: false,
            fail_under: None,
        }
    }

    fn set_format(&mut self, value: &str) -> Result<(), ExitCode> {
        self.format = Some(Format::parse(value).ok_or_else(|| {
            eprintln!(
                "error: unknown format '{value}'. Supported: text, text-summary, json-summary, lcov, cobertura, html"
            );
            ExitCode::FAILURE
        })?);
        Ok(())
    }

    fn set_threshold(&mut self, value: &str) -> Result<(), ExitCode> {
        let parsed = value.parse::<f64>().map_err(|_| {
            eprintln!("error: --threshold must be a number between 0 and 100, got '{value}'");
            ExitCode::FAILURE
        })?;
        // `f64::from_str` accepts `nan`, `inf`, `-inf`; reject those before the
        // range check because comparisons silently return false for NaN.
        if !parsed.is_finite() {
            eprintln!(
                "error: --threshold must be a finite number between 0 and 100, got '{value}'"
            );
            return Err(ExitCode::FAILURE);
        }
        if !(0.0..=100.0).contains(&parsed) {
            eprintln!(
                "error: --threshold {parsed} is outside [0, 100]; pick a percentage in that range"
            );
            return Err(ExitCode::FAILURE);
        }
        self.html_threshold = parsed;
        self.threshold_was_set = true;
        Ok(())
    }

    fn set_fail_under(&mut self, value: &str) -> Result<(), ExitCode> {
        let parsed = parse_percentage(value, "--fail-under")?;
        self.fail_under = Some(parsed);
        Ok(())
    }

    fn finish(self) -> Result<ReportArgs, ExitCode> {
        let format = self.format.unwrap_or(Format::Text);
        self.validate_output_options(format)?;

        let coverage_file = self.coverage_file.ok_or_else(|| {
            eprintln!("error: report requires a coverage-final.json path");
            print_report_usage();
            ExitCode::FAILURE
        })?;

        let output_dir = match (format.is_multi_file(), self.output_dir) {
            (true, Some(p)) => Some(p),
            (true, None) => Some(PathBuf::from("coverage")),
            (false, _) => None,
        };

        let root_dir = self.root_dir.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|e| {
                eprintln!(
                    "warning: cannot determine current working directory ({e}); lcov/cobertura/html paths will not be relativized"
                );
                PathBuf::new()
            })
        });

        Ok(ReportArgs {
            coverage_file,
            output_file: self.output_file,
            output_dir,
            format,
            root_dir,
            html_threshold: self.html_threshold,
            fail_under: self.fail_under,
        })
    }

    fn validate_output_options(&self, format: Format) -> Result<(), ExitCode> {
        // Reject incompatible flag combinations early so the user sees a clear
        // error instead of a useless `-o ./coverage` write to a single regular
        // file for html, or a stray --output-dir on a single-file format.
        if format.is_multi_file() && self.output_file.is_some() {
            eprintln!(
                "error: --format {} produces a directory tree; use --output-dir instead of -o",
                format_name(format)
            );
            return Err(ExitCode::FAILURE);
        }
        if !format.is_multi_file() && self.output_dir.is_some() {
            eprintln!(
                "error: --output-dir is only valid for multi-file formats (html); use -o for --format {}",
                format_name(format)
            );
            return Err(ExitCode::FAILURE);
        }
        // `--threshold` only affects the html reporter (drives colour bucketing
        // + the threshold-summary sentence). Reject it on other formats so the
        // user knows it was a no-op.
        if self.threshold_was_set && !format.is_multi_file() {
            eprintln!(
                "error: --threshold only applies to --format html; remove it or switch formats"
            );
            return Err(ExitCode::FAILURE);
        }

        Ok(())
    }
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
        other if other.starts_with('-') => {
            eprintln!("error: unknown option: {other}");
            print_usage();
            Err(ExitCode::FAILURE)
        }
        _ => parse_instrument_args(&args[1..]).map(|a| run_instrument(&a)),
    }
}

fn parse_instrument_args(args: &[String]) -> Result<InstrumentArgs, ExitCode> {
    if args.is_empty() {
        eprintln!("error: instrument requires a file argument");
        print_instrument_usage();
        return Err(ExitCode::FAILURE);
    }

    if args[0] == "--help" || args[0] == "-h" {
        print_instrument_usage();
        return Err(ExitCode::SUCCESS);
    }
    if args[0] == "--version" || args[0] == "-V" {
        print_version();
        return Err(ExitCode::SUCCESS);
    }
    if args[0].starts_with('-') {
        eprintln!("error: unknown option: {}", args[0]);
        print_instrument_usage();
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
            "--help" | "-h" => {
                print_instrument_usage();
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
    let mut report = ReportArgsDraft::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--format" | "-f" => {
                let value = take_value(args, &mut i, "--format")?;
                report.set_format(&value)?;
            }
            "-o" | "--output" => report.output_file = Some(take_value(args, &mut i, "--output")?),
            "--output-dir" => {
                report.output_dir = Some(PathBuf::from(take_value(args, &mut i, "--output-dir")?));
            }
            "--root" => report.root_dir = Some(PathBuf::from(take_value(args, &mut i, "--root")?)),
            "--threshold" => {
                let value = take_value(args, &mut i, "--threshold")?;
                report.set_threshold(&value)?;
            }
            "--fail-under" => {
                let value = take_value(args, &mut i, "--fail-under")?;
                report.set_fail_under(&value)?;
            }
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
                if report.coverage_file.is_some() {
                    eprintln!("error: only one coverage file may be supplied (got '{other}')");
                    return Err(ExitCode::FAILURE);
                }
                report.coverage_file = Some(other.to_owned());
            }
        }
        i += 1;
    }

    report.finish()
}

fn format_name(format: Format) -> &'static str {
    match format {
        Format::Text => "text",
        Format::TextSummary => "text-summary",
        Format::JsonSummary => "json-summary",
        Format::Lcov => "lcov",
        Format::Cobertura => "cobertura",
        Format::Html => "html",
    }
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

// Heuristic: does this token look like a filesystem path the user typed on
// purpose, or like a bare word that is more likely a misspelled subcommand?
// A path has a separator, a leading `~`, or a file extension. A token without
// any of those (`totally-unknown`, `inst`, `repot`) is usually a typo.
fn looks_like_path(s: &str) -> bool {
    s.contains('/')
        || s.contains('\\')
        || s.starts_with('~')
        || std::path::Path::new(s).extension().is_some()
}

fn parse_percentage(value: &str, flag: &str) -> Result<f64, ExitCode> {
    let parsed = value.parse::<f64>().map_err(|_| {
        eprintln!("error: {flag} must be a number between 0 and 100, got '{value}'");
        ExitCode::FAILURE
    })?;
    if !parsed.is_finite() {
        eprintln!("error: {flag} must be a finite number between 0 and 100, got '{value}'");
        return Err(ExitCode::FAILURE);
    }
    if !(0.0..=100.0).contains(&parsed) {
        eprintln!("error: {flag} {parsed} is outside [0, 100]; pick a percentage in that range");
        return Err(ExitCode::FAILURE);
    }
    Ok(parsed)
}

fn run_instrument(cli: &InstrumentArgs) -> ExitCode {
    let source = match std::fs::read_to_string(&cli.filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", cli.filename);
            // A bare token like `totally-unknown` lands here when the user
            // mistyped a subcommand. Hint at the real subcommands so they
            // do not chase a phantom file path.
            if e.kind() == std::io::ErrorKind::NotFound && !looks_like_path(&cli.filename) {
                eprintln!(
                    "       (not a known subcommand either; expected `instrument`, `report`, or a file path)"
                );
            }
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
    let map = match read_coverage_map(args) {
        Ok(map) => map,
        Err(code) => return code,
    };
    let root = summarize(&map);

    if args.format.is_multi_file() {
        render_report_dir(args, &map)
    } else {
        render_report_stream(args, &root)
    }
}

fn read_coverage_map(args: &ReportArgs) -> Result<CoverageMap, ExitCode> {
    let json = match std::fs::read_to_string(&args.coverage_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", args.coverage_file);
            return Err(ExitCode::FAILURE);
        }
    };
    let map = match parse_coverage_map(&json) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {} is not a valid coverage-final.json ({e})", args.coverage_file);
            return Err(ExitCode::FAILURE);
        }
    };
    if map.is_empty() {
        eprintln!("error: {} contains no files", args.coverage_file);
        return Err(ExitCode::from(2));
    }

    Ok(map)
}

fn render_report_dir(args: &ReportArgs, map: &CoverageMap) -> ExitCode {
    let Some(output_dir) = &args.output_dir else {
        eprintln!("error: --format html requires --output-dir <dir>");
        return ExitCode::FAILURE;
    };
    let html_opts = match oxc_coverage_reports::html::HtmlOptions::new(args.html_threshold) {
        Ok(opts) => opts,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = args.format.write_to_dir(map, &args.root_dir, output_dir, &html_opts) {
        eprintln!("error: failed to render report: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("HTML report written to {}", output_dir.display());
    let root = summarize(map);
    apply_fail_under(&root, args.fail_under)
}

fn render_report_stream(args: &ReportArgs, root: &ReportNode) -> ExitCode {
    match &args.output_file {
        Some(path) => {
            let mut file = match std::fs::File::create(path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("error: cannot write {path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = args.format.write(root, &args.root_dir, &mut file) {
                eprintln!("error: failed to render report: {e}");
                return ExitCode::FAILURE;
            }
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            if let Err(e) = args.format.write(root, &args.root_dir, &mut handle) {
                eprintln!("error: failed to render report: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    apply_fail_under(root, args.fail_under)
}

fn apply_fail_under(root: &oxc_coverage_report::ReportNode, fail_under: Option<f64>) -> ExitCode {
    if let Some(floor) = fail_under {
        let actual = root.summary.lines.pct;
        if actual < floor {
            eprintln!("coverage {actual:.2}% is below --fail-under {floor:.2}%");
            return ExitCode::from(2);
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
                                 Also writes <file>.map.json with the coverage map.
    --coverage-map               Print only the coverage map JSON
    --source-map                 Generate source map
    --coverage-variable <name>   Coverage variable name (default: __coverage__)

REPORT OPTIONS:
    -f, --format <fmt>           Output format: text (default), text-summary, json-summary, lcov, cobertura, html
    -o, --output <file>          Write report to file (default: stdout). Not valid for --format html.
    --output-dir <dir>           Output directory for multi-file formats (html). Default: ./coverage
    --root <dir>                 Root directory used to relativize source paths and resolve html source view (default: cwd)
    --threshold <pct>            Percentage at or above which html-report cells render green
                                 (0-100, default: 80). Cells below this go amber down to 50%
                                 and red below 50%. Affects display only; does not gate the
                                 process exit code. Rejected on non-html formats.
    --fail-under <pct>           Render the report, then exit 2 if aggregate line coverage
                                 falls below this percentage. Works with every format.

GLOBAL OPTIONS:
    -V, --version                Print version
    -h, --help                   Print help",
        env!("CARGO_PKG_VERSION")
    );
}

fn print_instrument_usage() {
    eprintln!(
        "oxc-coverage-instrument instrument

USAGE:
    oxc-coverage-instrument <file> [options]
    oxc-coverage-instrument instrument <file> [options]

OPTIONS:
    -o, --output <file>          Write instrumented code to file (default: stdout)
                                 Also writes <file>.map.json with the coverage map.
    --coverage-map               Print only the coverage map JSON
    --source-map                 Generate source map
    --coverage-variable <name>   Coverage variable name (default: __coverage__)
    -V, --version                Print version
    -h, --help                   Print this help"
    );
}

fn print_report_usage() {
    eprintln!(
        "oxc-coverage-instrument report

USAGE:
    oxc-coverage-instrument report --format <fmt> <coverage.json> [options]

OPTIONS:
    -f, --format <fmt>           Output format: text (default), text-summary, json-summary, lcov, cobertura, html
    -o, --output <file>          Write report to file (default: stdout). Not valid for --format html.
    --output-dir <dir>           Output directory for multi-file formats (html). Default: ./coverage
    --root <dir>                 Root directory used to relativize source paths and resolve html source view (default: cwd)
    --threshold <pct>            Percentage at or above which html-report cells render green
                                 (0-100, default: 80). Cells below this go amber down to 50%
                                 and red below 50%. Affects display only; does not gate the
                                 process exit code. Rejected on non-html formats.
    --fail-under <pct>           Render the report, then exit 2 if aggregate line coverage
                                 falls below this percentage. Works with every format.
    -h, --help                   Print this help"
    );
}
