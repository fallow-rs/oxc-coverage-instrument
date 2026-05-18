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
