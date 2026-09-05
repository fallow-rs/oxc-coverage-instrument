//! Integration tests for the instrument path of the CLI, plus the top-level
//! argument dispatch that routes into it.

mod common;

use common::{cli, temp_path, write_temp};

#[test]
fn help_flag_prints_usage_and_exits_success() {
    for arg in ["--help", "-h"] {
        let out = cli().arg(arg).output().unwrap();
        assert!(out.status.success(), "`{arg}` should exit 0");
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
    assert!(
        !stderr.contains("not a known subcommand"),
        "path-shaped misses must not show the typo hint, got: {stderr}"
    );
}

#[test]
fn bare_word_typo_hints_at_subcommands() {
    let out = cli().arg("totally-unknown").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot read"));
    assert!(
        stderr.contains("not a known subcommand"),
        "bare-word typo should show the subcommand hint, got: {stderr}"
    );
}

#[test]
fn unknown_option_exits_failure() {
    let src = write_temp("unknown_opt.js", "const x = 1;");
    for explicit in [false, true] {
        let mut command = cli();
        if explicit {
            command.arg("instrument");
        }
        let out = command.arg(&src).arg("--totally-unknown").output().unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("unknown option"), "should reject unknown option, got: {stderr}");
        assert!(stderr.contains("oxc-coverage-instrument instrument"), "got: {stderr}");
        assert!(
            !stderr.contains("--fail-under"),
            "instrument usage must omit report flags: {stderr}"
        );
        assert!(
            !stderr.contains("--threshold"),
            "instrument usage must omit report flags: {stderr}"
        );
    }
}

#[test]
fn leading_flag_shaped_token_is_rejected_as_unknown_option() {
    for args in [vec!["--bogus"], vec!["instrument", "--bogus"]] {
        let out = cli().args(args).output().unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("unknown option: --bogus"), "got: {stderr}");
        assert!(
            !stderr.contains("cannot read"),
            "flag typo should not become a filename: {stderr}"
        );
    }
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
fn coverage_map_only_rejects_discarded_output_options() {
    let src = write_temp("coverage_map_invalid_options.js", "const x = 1;");

    for explicit in [false, true] {
        let out_path = temp_path(if explicit {
            "coverage_map_explicit_out.json"
        } else {
            "coverage_map_implicit_out.json"
        });
        let _ = std::fs::remove_file(&out_path);

        let mut output_command = cli();
        if explicit {
            output_command.arg("instrument");
        }
        let output = output_command
            .arg(&src)
            .arg("--coverage-map")
            .arg("-o")
            .arg(&out_path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--coverage-map cannot be combined with -o or --output"),
            "got: {stderr}"
        );
        assert!(!out_path.exists(), "invalid combination must not create an output file");

        let mut source_map_command = cli();
        if explicit {
            source_map_command.arg("instrument");
        }
        let source_map = source_map_command
            .arg(&src)
            .arg("--coverage-map")
            .arg("--source-map")
            .output()
            .unwrap();
        assert_eq!(source_map.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&source_map.stderr);
        assert!(
            stderr.contains("--coverage-map cannot be combined with --source-map"),
            "got: {stderr}"
        );
    }
}

#[test]
fn output_file_writes_code_and_map_alongside() {
    let src = write_temp("output_pair.js", "const x = 1;");
    let out_path = temp_path("output_pair.instrumented.js");
    let map_path = std::path::PathBuf::from(format!("{}.map.json", out_path.display()));
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
fn help_documents_output_sidecar_coverage_map() {
    let out = cli().arg("--help").output().unwrap();
    assert!(out.status.success());
    let combined =
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(
        combined.contains("Also writes <file>.map.json"),
        "help should document the sidecar coverage map, got:\n{combined}"
    );
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
    // Without `-o` the instrumented code owns stdout, so the map goes to stderr.
    let value: serde_json::Value = serde_json::from_str(stderr.trim())
        .expect("source map should be emitted as JSON on stderr when no -o is provided");
    assert_eq!(value["version"], 3);
}

#[test]
fn source_map_with_output_file_writes_sidecar_map() {
    let src = write_temp("source_map_sidecar.js", "const x = 1;");
    let out_path = temp_path("source_map_sidecar.instrumented.js");
    let sm_path = std::path::PathBuf::from(format!("{}.map", out_path.display()));
    let map_path = std::path::PathBuf::from(format!("{}.map.json", out_path.display()));
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&sm_path);
    let _ = std::fs::remove_file(&map_path);

    let out = cli().arg(&src).arg("--source-map").arg("-o").arg(&out_path).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let sm = std::fs::read_to_string(&sm_path).expect("source map sidecar should be written");
    let value: serde_json::Value =
        serde_json::from_str(&sm).expect("sidecar .map should be valid JSON");
    assert_eq!(value["version"], 3, "expected a v3 source map in the sidecar");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Source map:"), "should report the sidecar path, got: {stderr}");
}

#[test]
fn output_to_unwritable_path_reports_write_error() {
    let src = write_temp("output_unwritable.js", "const x = 1;");
    // `-o` under a directory that does not exist: the code-file write fails.
    let bad_dir = temp_path("nonexistent_dir_xyz");
    let _ = std::fs::remove_dir_all(&bad_dir);
    let bad_out = bad_dir.join("nested").join("out.js");
    let out = cli().arg(&src).arg("-o").arg(&bad_out).output().unwrap();
    assert!(!out.status.success(), "writing under a missing dir should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot write"), "got: {stderr}");
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
fn explicit_instrument_subcommand_without_file_reports_error() {
    let out = cli().arg("instrument").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("instrument requires a file argument"), "got: {stderr}");
}

#[test]
fn instrument_help_flag_prints_subcommand_specific_usage() {
    for arg in ["--help", "-h"] {
        let out = cli().arg("instrument").arg(arg).output().unwrap();
        assert!(out.status.success(), "`instrument {arg}` should exit 0");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("oxc-coverage-instrument instrument"), "got: {stderr}");
        assert!(stderr.contains("--coverage-variable"), "got: {stderr}");
        assert!(!stderr.contains("--threshold"), "instrument help should not include report flags");
        assert!(
            !stderr.contains("--fail-under"),
            "instrument help should not include report flags"
        );
    }
}

#[test]
fn instrument_help_flag_mid_args_short_circuits() {
    let src = write_temp("help_mid_args.js", "const x = 1;");
    let out = cli().arg(&src).arg("--help").output().unwrap();
    assert!(out.status.success(), "mid-args --help should exit 0");
    let combined =
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(combined.contains("USAGE"), "mid-args --help should print USAGE, got:\n{combined}");
}

#[test]
fn instrument_version_flag_mid_args_short_circuits() {
    // `--version` must short-circuit even when a filename argument precedes it.
    let src = write_temp("version_mid_args.js", "const x = 1;");
    let out = cli().arg(&src).arg("--version").output().unwrap();
    assert!(out.status.success(), "mid-args --version should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("oxc-coverage-instrument"), "got: {stdout}");
}
