//! Integration tests for the `oxc-coverage-instrument` CLI binary.
//!
//! Uses `CARGO_BIN_EXE_oxc-coverage-instrument` (set by cargo for integration
//! tests of binary packages) and `std::process::Command` to exercise the binary
//! end-to-end, so no extra dev-dependency is needed.

use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_oxc-coverage-instrument"))
}

fn write_temp(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("oxc_cov_cli_{name}"));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn help_flag_prints_usage_and_exits_success() {
    for arg in ["--help", "-h"] {
        let out = cli().arg(arg).output().unwrap();
        assert!(out.status.success(), "`{arg}` should exit 0");
        // Usage prints to stderr.
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(combined.contains("USAGE"), "`{arg}` should print USAGE, got:\n{combined}");
    }
}

#[test]
fn no_args_prints_usage_and_exits_success() {
    let out = cli().output().unwrap();
    assert!(out.status.success());
    let combined =
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(combined.contains("USAGE"));
}

#[test]
fn version_flag_prints_version() {
    for arg in ["--version", "-V"] {
        let out = cli().arg(arg).output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("oxc-coverage-instrument"),
            "`{arg}` should include the binary name, got: {stdout}"
        );
    }
}

#[test]
fn missing_file_exits_failure_with_readable_error() {
    let out = cli().arg("/tmp/this-file-does-not-exist-oxc-cov-test.js").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot read"), "should report read failure, got: {stderr}");
}

#[test]
fn unknown_option_exits_failure() {
    let src = write_temp("unknown_opt.js", "const x = 1;");
    let out = cli().arg(&src).arg("--totally-unknown").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown option"), "should reject unknown option, got: {stderr}");
}

#[test]
fn coverage_map_only_outputs_valid_json_with_expected_keys() {
    let src = write_temp("coverage_map.js", "function add(a, b) { return a + b; }");
    let out = cli().arg(&src).arg("--coverage-map").output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    for key in ["path", "statementMap", "fnMap", "branchMap", "s", "f", "b"] {
        assert!(value.get(key).is_some(), "coverage JSON missing `{key}`");
    }
    assert_eq!(
        value["fnMap"]["0"]["name"], "add",
        "function name should be resolved from declaration"
    );
}

#[test]
fn output_file_writes_code_and_map_alongside() {
    let src = write_temp("output_pair.js", "const x = 1;");
    let out_path = std::env::temp_dir().join("oxc_cov_cli_output_pair.instrumented.js");
    let map_path = std::path::PathBuf::from(format!("{}.map.json", out_path.display()));
    // Clean up any prior run
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&map_path);

    let out = cli().arg(&src).arg("-o").arg(&out_path).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let code = std::fs::read_to_string(&out_path).expect("instrumented code file");
    assert!(
        code.contains(".s[0]"),
        "instrumented code should contain a statement counter reference"
    );

    let map = std::fs::read_to_string(&map_path).expect("coverage map file");
    let value: serde_json::Value = serde_json::from_str(&map).expect("map JSON should parse");
    assert!(value["statementMap"].is_object());
}

#[test]
fn invalid_coverage_variable_exits_failure() {
    let src = write_temp("invalid_cov_var.js", "const x = 1;");
    let out =
        cli().arg(&src).arg("--coverage-variable").arg("not a valid identifier").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid coverage variable"),
        "should report invalid identifier, got: {stderr}"
    );
}

#[test]
fn coverage_variable_missing_value_exits_failure() {
    let src = write_temp("cov_var_missing.js", "const x = 1;");
    let out = cli().arg(&src).arg("--coverage-variable").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("requires a value"));
}

#[test]
fn source_map_flag_prints_source_map_to_stderr_on_stdout_run() {
    let src = write_temp("source_map.js", "const x = 1;");
    let out = cli().arg(&src).arg("--source-map").output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stderr = String::from_utf8_lossy(&out.stderr);
    // With stdout output, the map is printed to stderr (per CLI usage in main.rs).
    let value: serde_json::Value = serde_json::from_str(stderr.trim())
        .expect("source map should be emitted as JSON on stderr when no -o is provided");
    assert_eq!(value["version"], 3);
}

// ---- `report` subcommand tests -------------------------------------------

const SAMPLE_COVERAGE: &str = r#"{
  "a.js": {
    "path": "a.js",
    "statementMap": {
      "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}},
      "1": {"start": {"line": 2, "column": 0}, "end": {"line": 2, "column": 5}}
    },
    "fnMap": {},
    "branchMap": {},
    "s": {"0": 1, "1": 0},
    "f": {},
    "b": {}
  }
}"#;

#[test]
fn report_text_format_writes_table_to_stdout() {
    let cov = write_temp("report_text_cov.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg("--format").arg("text").arg(&cov).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("All files"), "got:\n{stdout}");
    assert!(stdout.contains("% Stmts"), "got:\n{stdout}");
}

#[test]
fn report_text_summary_format_writes_four_metrics() {
    let cov = write_temp("report_text_summary_cov.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg("-f").arg("text-summary").arg(&cov).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for label in ["Statements", "Branches", "Functions", "Lines"] {
        assert!(stdout.contains(label), "missing {label} in:\n{stdout}");
    }
}

#[test]
fn report_json_summary_format_emits_valid_parseable_json() {
    let cov = write_temp("report_json_summary_cov.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg("--format").arg("json-summary").arg(&cov).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("json-summary output must parse");
    assert!(value.get("total").is_some(), "missing total key in:\n{stdout}");
    assert!(value.get("a.js").is_some(), "missing per-file key in:\n{stdout}");
}

#[test]
fn report_output_flag_writes_to_file() {
    let cov = write_temp("report_out_cov.json", SAMPLE_COVERAGE);
    let dest = std::env::temp_dir().join("oxc_cov_cli_report_out.json");
    let _ = std::fs::remove_file(&dest);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("json-summary")
        .arg("-o")
        .arg(&dest)
        .arg(&cov)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let written = std::fs::read_to_string(&dest).expect("output file should exist");
    let value: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert!(value.get("total").is_some());
}

#[test]
fn report_default_format_is_text() {
    let cov = write_temp("report_default_cov.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg(&cov).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("% Stmts"), "default format should be `text`, got:\n{stdout}");
}

#[test]
fn report_unknown_format_exits_failure() {
    let cov = write_temp("report_bad_format_cov.json", SAMPLE_COVERAGE);
    // `html` is supported as of PR G1; pick a name that no format will ever
    // claim. `xml` and `clover` are both common-sounding aliases that we
    // intentionally do not implement.
    let out = cli().arg("report").arg("--format").arg("clover").arg(&cov).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown format"), "got:\n{stderr}");
}

#[test]
fn report_missing_coverage_file_exits_failure() {
    let out = cli().arg("report").arg("--format").arg("text").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("requires a coverage-final.json"), "got:\n{stderr}");
}

#[test]
fn report_invalid_json_exits_failure() {
    let bad = write_temp("report_bad_cov.json", "not json");
    let out = cli().arg("report").arg("--format").arg("text").arg(&bad).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("failed to parse coverage map"), "got:\n{stderr}");
}

#[test]
fn report_lcov_format_writes_tracefile_with_sf_records() {
    let cov = write_temp("report_lcov_cov.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg("--format").arg("lcov").arg(&cov).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty(), "lcov output must not be empty");
    assert!(stdout.contains("SF:a.js"), "expected SF: record, got:\n{stdout}");
    assert!(stdout.contains("end_of_record"), "expected end_of_record, got:\n{stdout}");
}

#[test]
fn report_cobertura_format_writes_coverage_xml_root() {
    let cov = write_temp("report_cobertura_cov.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg("--format").arg("cobertura").arg(&cov).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty(), "cobertura output must not be empty");
    assert!(stdout.contains("<?xml"), "expected XML declaration, got:\n{stdout}");
    assert!(stdout.contains("<coverage "), "expected <coverage> root, got:\n{stdout}");
    assert!(stdout.contains("line-rate="), "expected line-rate attribute, got:\n{stdout}");
    assert!(stdout.contains("timestamp="), "expected timestamp attribute, got:\n{stdout}");
}

#[test]
fn report_root_flag_relativizes_lcov_sf_paths() {
    // Build a coverage map with absolute paths under a synthetic root and
    // verify the emitter strips that root in the SF: records.
    let absolute_cov = r#"{
        "/synthetic/proj/src/a.js": {
            "path": "/synthetic/proj/src/a.js",
            "statementMap": {
                "0": {"start": {"line": 1, "column": 0}, "end": {"line": 1, "column": 5}}
            },
            "fnMap": {},
            "branchMap": {},
            "s": {"0": 1},
            "f": {},
            "b": {}
        }
    }"#;
    let cov = write_temp("report_root_cov.json", absolute_cov);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("lcov")
        .arg("--root")
        .arg("/synthetic/proj")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("SF:src/a.js"), "expected relativized SF, got:\n{stdout}");
    assert!(!stdout.contains("SF:/synthetic/proj"));
}

#[test]
fn report_html_format_writes_directory_tree() {
    let cov = write_temp("report_html_cov.json", SAMPLE_COVERAGE);
    let out_dir = std::env::temp_dir().join("oxc_cov_cli_html_out");
    let _ = std::fs::remove_dir_all(&out_dir);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg("--output-dir")
        .arg(&out_dir)
        .arg(&cov)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(out_dir.join("index.html").exists(), "root index.html missing");
    assert!(out_dir.join("base.css").exists(), "base.css missing");
    assert!(out_dir.join("base.js").exists(), "base.js missing");
    // base.css `@import`s coverage-tokens.css: dropping the sibling file
    // would silently break the palette without any compile-time signal,
    // so guard it at the CLI integration layer too.
    assert!(out_dir.join("coverage-tokens.css").exists(), "coverage-tokens.css missing");
    assert!(out_dir.join("a.js.html").exists(), "per-file detail page missing");
    let detail = std::fs::read_to_string(out_dir.join("a.js.html")).unwrap();
    assert!(detail.contains("<title>Coverage: a.js</title>"), "got:\n{detail}");
    assert!(detail.contains("Content-Security-Policy"), "CSP meta missing in CLI output");
    assert!(detail.contains("base.js"), "script reference missing in CLI output");
    assert!(detail.contains("<meta name=\"generator\""), "generator meta missing in CLI output");
}

#[test]
fn report_html_rejects_dash_o_with_friendly_error() {
    let cov = write_temp("report_html_dash_o.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg("-o")
        .arg("/tmp/should_not_be_used.html")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("directory tree"), "got: {stderr}");
}

#[test]
fn report_html_threshold_flag_overrides_default_summary() {
    let cov = write_temp("report_html_threshold.json", SAMPLE_COVERAGE);
    let out_dir = std::env::temp_dir().join("oxc_cov_cli_html_threshold_out");
    let _ = std::fs::remove_dir_all(&out_dir);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg("--output-dir")
        .arg(&out_dir)
        .arg("--threshold")
        .arg("40")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let index = std::fs::read_to_string(out_dir.join("index.html")).unwrap();
    // SAMPLE_COVERAGE has one file with 50% statement coverage. With the
    // CLI default 80% threshold the sentence reads "below the 80%
    // coverage threshold"; with --threshold 40 the cutoff drops and
    // the file is now above, so the sentence flips to the all-met form.
    assert!(
        index.contains("40% coverage threshold"),
        "summary should reflect --threshold 40, got:\n{index}",
    );
}

#[test]
fn report_html_threshold_rejects_out_of_range_values() {
    let cov = write_temp("report_html_threshold_bad.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg("--output-dir")
        .arg(std::env::temp_dir().join("oxc_cov_cli_html_threshold_bad_out"))
        .arg("--threshold")
        .arg("150")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success(), "should reject --threshold 150");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("outside [0, 100]"), "got: {stderr}");
}

#[test]
fn report_html_threshold_rejects_non_finite_values() {
    let cov = write_temp("report_html_threshold_nan.json", SAMPLE_COVERAGE);
    // `f64::from_str` parses NaN successfully; without an explicit
    // `is_finite()` guard the value would slip past the range check
    // and silently degrade every percent to the "medium" bucket.
    for bad in ["nan", "inf", "-inf"] {
        let out = cli()
            .arg("report")
            .arg("--format")
            .arg("html")
            .arg("--output-dir")
            .arg(std::env::temp_dir().join("oxc_cov_cli_html_threshold_nan_out"))
            .arg("--threshold")
            .arg(bad)
            .arg(&cov)
            .output()
            .unwrap();
        assert!(!out.status.success(), "should reject --threshold {bad}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("finite number") || stderr.contains("outside [0, 100]"),
            "rejection for {bad:?} should mention finite-number or range; got: {stderr}"
        );
    }
}

#[test]
fn report_threshold_rejected_on_non_html_formats() {
    let cov = write_temp("report_threshold_lcov.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("lcov")
        .arg("--threshold")
        .arg("70")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success(), "should reject --threshold on lcov");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--threshold only applies to --format html"), "got: {stderr}");
}

#[test]
fn report_text_rejects_output_dir_with_friendly_error() {
    let cov = write_temp("report_text_dash_dir.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("text")
        .arg("--output-dir")
        .arg("/tmp/should_not_be_used")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("only valid for multi-file formats"), "got: {stderr}");
}

#[test]
fn explicit_instrument_subcommand_works() {
    let src = write_temp("explicit_instrument.js", "const x = 1;");
    let out = cli().arg("instrument").arg(&src).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("var "), "explicit instrument should still emit instrumented code");
}

#[test]
fn report_help_flag_prints_usage_and_exits_success() {
    for arg in ["--help", "-h"] {
        let out = cli().arg("report").arg(arg).output().unwrap();
        assert!(out.status.success(), "`report {arg}` should exit 0");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("USAGE"), "`report {arg}` should print USAGE, got: {stderr}");
        assert!(
            stderr.contains("--threshold"),
            "report-specific help must mention --threshold, got: {stderr}",
        );
    }
}

#[test]
fn report_rejects_a_second_positional_coverage_file() {
    let cov_a = write_temp("report_two_a.json", SAMPLE_COVERAGE);
    let cov_b = write_temp("report_two_b.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg(&cov_a).arg(&cov_b).output().unwrap();
    assert!(!out.status.success(), "two coverage files must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("only one coverage file may be supplied"),
        "should explain the single-file rule, got: {stderr}",
    );
}

#[test]
fn report_lcov_rejects_output_dir_with_friendly_error() {
    let cov = write_temp("report_lcov_dir.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("lcov")
        .arg("--output-dir")
        .arg(std::env::temp_dir().join("oxc_cov_cli_lcov_dir_out"))
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--format lcov"), "format name must reach the error, got: {stderr}");
    assert!(stderr.contains("only valid for multi-file formats"), "got: {stderr}");
}

#[test]
fn report_cobertura_rejects_output_dir_with_friendly_error() {
    let cov = write_temp("report_cobertura_dir.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("cobertura")
        .arg("--output-dir")
        .arg(std::env::temp_dir().join("oxc_cov_cli_cobertura_dir_out"))
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--format cobertura"), "got: {stderr}");
}

#[test]
fn report_text_summary_rejects_output_dir_with_friendly_error() {
    let cov = write_temp("report_text_summary_dir.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("text-summary")
        .arg("--output-dir")
        .arg(std::env::temp_dir().join("oxc_cov_cli_text_summary_dir_out"))
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--format text-summary"), "got: {stderr}");
}

#[test]
fn report_json_summary_rejects_output_dir_with_friendly_error() {
    let cov = write_temp("report_json_summary_dir.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("json-summary")
        .arg("--output-dir")
        .arg(std::env::temp_dir().join("oxc_cov_cli_json_summary_dir_out"))
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--format json-summary"), "got: {stderr}");
}

#[test]
fn report_html_defaults_output_dir_to_coverage_subdir() {
    // Without --output-dir, the CLI defaults to `./coverage` for html so users
    // get a usable report from the simplest invocation. The default kicks in
    // only on the success path, so we have to run from a temp cwd to avoid
    // littering the repo with a `coverage/` directory.
    let cov = write_temp("report_html_default_dir.json", SAMPLE_COVERAGE);
    let workdir = std::env::temp_dir().join("oxc_cov_cli_html_default_workdir");
    let _ = std::fs::remove_dir_all(&workdir);
    std::fs::create_dir_all(&workdir).unwrap();

    let out = cli()
        .current_dir(&workdir)
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        workdir.join("coverage").join("index.html").is_file(),
        "default --output-dir should be ./coverage/ relative to cwd",
    );
}

#[test]
fn instrument_version_flag_mid_args_short_circuits() {
    // `--version` inside the instrument argument loop must short-circuit even
    // when a filename argument precedes it. The top-level dispatch tests cover
    // the bare flag; this test covers the mid-args path inside parse_instrument_args.
    let src = write_temp("version_mid_args.js", "const x = 1;");
    let out = cli().arg(&src).arg("--version").output().unwrap();
    assert!(out.status.success(), "mid-args --version should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("oxc-coverage-instrument"), "got: {stdout}");
}

#[test]
fn report_unknown_long_option_exits_failure() {
    // Hits the report parser's catch-all for unknown `--flag` arguments,
    // distinct from the report_text path tested via the format dispatcher.
    let cov = write_temp("report_unknown_long.json", SAMPLE_COVERAGE);
    let out = cli().arg("report").arg("--definitely-not-a-flag").arg(&cov).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown option"),
        "unknown report flag must yield a friendly error, got: {stderr}",
    );
}

#[test]
fn report_flag_without_value_exits_failure() {
    // `--format` is the last argument: take_value runs off the end of argv and
    // must report a clear error rather than panic on out-of-bounds indexing.
    let out = cli().arg("report").arg("--format").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--format requires a value"),
        "trailing --format must surface a missing-value error, got: {stderr}",
    );
}
