//! Integration tests for the `report` subcommand: format dispatch, output
//! destinations, the coverage gates, and the html output tree.

mod common;

use std::path::{Path, PathBuf};

use common::{cli, temp_path, write_temp};

const FILESYSTEM_COMPONENT_LIMIT: usize = 255;

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

const DAMAGED_COVERAGE: &str = r#"{
  "a.js": {
    "path": "a.js",
    "statementMap": {"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}}},
    "fnMap": {"0":{"name":"f","decl":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"line":1}},
    "branchMap": {"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},
    "s": {"0":0,"99":7}, "f": {"0":0,"99":7}, "b": {"0":[3]}
  }
}"#;

fn collect_html_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(collect_html_files(&path));
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("html") {
            files.push(path);
        }
    }
    files
}

fn local_hrefs(page: &str) -> Vec<&str> {
    page.split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(href, _)| href))
        .filter(|href| {
            !href.contains("://")
                && !href.starts_with("//")
                && !href.starts_with("data:")
                && !href.starts_with("mailto:")
        })
        .collect()
}

fn url_href_path(href: &str) -> PathBuf {
    let path = href.split(['#', '?']).next().unwrap();
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1).expect("percent escape has two digits"));
            let low = hex_value(*bytes.get(index + 2).expect("percent escape has two digits"));
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    PathBuf::from(String::from_utf8(decoded).expect("href path is UTF-8"))
}

fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("invalid percent escape"),
    }
}

fn resolved_local_href(page: &Path, href: &str) -> PathBuf {
    let path = url_href_path(href);
    if path.as_os_str().is_empty() { page.to_owned() } else { page.parent().unwrap().join(path) }
}

fn assert_portable_output_tree(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let component = path.file_name().unwrap().to_str().unwrap();
        assert!(component.is_ascii(), "non-ASCII output component: {component:?}");
        assert!(
            component.len() <= FILESYSTEM_COMPONENT_LIMIT,
            "overlong output component: {component:?}",
        );
        assert!(
            !component
                .bytes()
                .any(|byte| byte < b' ' || byte == 0x7f || b"<>:\"/\\|?*".contains(&byte)),
            "nonportable output component: {component:?}",
        );
        assert!(!component.ends_with(['.', ' ']), "trailing output alias: {component:?}");
        if path.is_dir() {
            assert_portable_output_tree(&path);
        }
    }
}

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
fn report_json_summary_uses_metadata_cardinality() {
    let cov = write_temp("report_damaged_metadata.json", DAMAGED_COVERAGE);
    let out = cli().arg("report").arg("-f").arg("json-summary").arg(cov).output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["total"]["statements"]["total"], 1);
    assert_eq!(value["total"]["statements"]["covered"], 0);
    assert_eq!(value["total"]["functions"]["total"], 1);
    assert_eq!(value["total"]["functions"]["covered"], 0);
    assert_eq!(value["total"]["branches"]["total"], 2);
    assert_eq!(value["total"]["branches"]["covered"], 1);
    assert_eq!(value["a.js"]["statements"]["total"], 1);
    assert_eq!(value["a.js"]["statements"]["covered"], 0);
    assert_eq!(value["a.js"]["functions"]["total"], 1);
    assert_eq!(value["a.js"]["functions"]["covered"], 0);
    assert_eq!(value["a.js"]["branches"]["total"], 2);
    assert_eq!(value["a.js"]["branches"]["covered"], 1);
}

#[test]
fn report_output_flag_writes_to_file() {
    let cov = write_temp("report_out_cov.json", SAMPLE_COVERAGE);
    let dest = temp_path("report_out.json");
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
    // `clover` is a common-sounding reporter name this CLI does not implement.
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
    assert!(stderr.contains("is not a valid coverage-final.json"), "got:\n{stderr}");
    assert!(stderr.contains(&bad.display().to_string()), "path missing from error:\n{stderr}");
}

#[test]
fn report_nonexistent_coverage_file_reports_read_error() {
    let missing = temp_path("report_absent_file.json");
    let _ = std::fs::remove_file(&missing);
    let out = cli().arg("report").arg("--format").arg("text").arg(&missing).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot read"), "should report read failure, got: {stderr}");
}

#[test]
fn report_empty_coverage_map_has_distinct_exit_code_and_writes_nothing() {
    let cov = write_temp("report_empty_cov.json", "{}");
    let out = cli().arg("report").arg("--format").arg("text").arg(&cov).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("contains no files"), "got:\n{stderr}");
    assert!(
        String::from_utf8_lossy(&out.stdout).is_empty(),
        "empty maps should not render misleading 100% totals"
    );
}

#[test]
fn report_rejects_pathless_coverage_before_fail_under() {
    let cov = write_temp(
        "report_pathless_cov.json",
        r#"{"///":{"path":"///","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}}"#,
    );
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("text")
        .arg("--fail-under")
        .arg("1")
        .arg(&cov)
        .output()
        .unwrap();

    assert!(!out.status.success(), "pathless coverage must not pass the gate");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid coverage path"), "got:\n{stderr}");
    assert!(stderr.contains("\"///\""), "got:\n{stderr}");
    assert!(!stderr.contains("below --fail-under"), "validation must run before gating: {stderr}");
    assert!(out.stdout.is_empty(), "invalid coverage must not render a report");
}

#[test]
fn report_normalizes_a_null_inner_path() {
    let cov = write_temp(
        "report_null_inner_path.json",
        r#"{"src/a.js":{"path":null,"statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#,
    );
    let out = cli().arg("report").arg("--format").arg("lcov").arg(&cov).output().unwrap();

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains("SF:src/a.js"));
}

#[test]
fn report_rejects_mismatched_paths_before_creating_html_output() {
    let cov = write_temp(
        "report_mismatched_paths.json",
        r#"{"src/a.js":{"path":"other/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#,
    );
    let out_dir = temp_path("html_mismatched_paths");
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

    assert!(!out.status.success(), "mismatched paths must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("coverage path mismatch"), "got:\n{stderr}");
    assert!(stderr.contains("src/a.js") && stderr.contains("other/a.js"), "got:\n{stderr}");
    assert!(!out_dir.exists(), "validation must run before creating the HTML directory");
}

#[test]
fn report_fail_under_renders_then_exits_two_when_lines_are_low() {
    let cov = write_temp("report_fail_under_cov.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("text-summary")
        .arg("--fail-under")
        .arg("90")
        .arg(&cov)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("Lines"), "report should still render before gating:\n{stdout}");
    assert!(stderr.contains("coverage 50.00% is below --fail-under 90.00%"), "got: {stderr}");
}

#[test]
fn report_fail_under_passes_when_lines_meet_floor() {
    let cov = write_temp("report_fail_under_pass_cov.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("text-summary")
        .arg("--fail-under")
        .arg("50")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn report_fail_under_rejects_non_numeric_value() {
    let cov = write_temp("report_fail_under_nonnumeric.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("text-summary")
        .arg("--fail-under")
        .arg("ninety")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success(), "non-numeric --fail-under must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--fail-under must be a number"), "got: {stderr}");
}

#[test]
fn report_fail_under_rejects_out_of_range_value() {
    let cov = write_temp("report_fail_under_oob.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("text-summary")
        .arg("--fail-under")
        .arg("250")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success(), "out-of-range --fail-under must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("outside [0, 100]"), "got: {stderr}");
}

#[test]
fn report_fail_under_rejects_non_finite_value() {
    // NaN and the infinities parse as `f64` but must be rejected before the
    // range check, which silently returns false for NaN.
    let cov = write_temp("report_fail_under_nan.json", SAMPLE_COVERAGE);
    for bad in ["nan", "inf", "-inf"] {
        let out = cli()
            .arg("report")
            .arg("--format")
            .arg("text-summary")
            .arg("--fail-under")
            .arg(bad)
            .arg(&cov)
            .output()
            .unwrap();
        assert!(!out.status.success(), "should reject --fail-under {bad}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("finite number") || stderr.contains("outside [0, 100]"),
            "rejection for {bad:?} should mention finite-number or range; got: {stderr}"
        );
    }
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
    let out_dir = temp_path("html_out");
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
    // base.css `@import`s coverage-tokens.css, and a missing sibling breaks the
    // palette with no compile-time signal, so guard it here as well.
    assert!(out_dir.join("coverage-tokens.css").exists(), "coverage-tokens.css missing");
    assert!(out_dir.join("a.js.html").exists(), "per-file detail page missing");
    let detail = std::fs::read_to_string(out_dir.join("a.js.html")).unwrap();
    assert!(detail.contains("<title>Coverage: a.js</title>"), "got:\n{detail}");
    assert!(detail.contains("Content-Security-Policy"), "CSP meta missing in CLI output");
    assert!(detail.contains("base.js"), "script reference missing in CLI output");
    assert!(detail.contains("<meta name=\"generator\""), "generator meta missing in CLI output");
}

#[test]
fn report_html_disambiguates_colliding_paths_and_all_links_resolve() {
    let coverage = r#"{
      "index":{"path":"index","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "src/index":{"path":"src/index","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "base.css/a.js":{"path":"base.css/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "base.js/a.js":{"path":"base.js/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "coverage-tokens.css/a.js":{"path":"coverage-tokens.css/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "reserved #?% ü.js":{"path":"reserved #?% ü.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "folder #?% ü/nested #?% ü.js":{"path":"folder #?% ü/nested #?% ü.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
    }"#;
    let cov = write_temp("report_html_collisions.json", coverage);
    let out_dir = temp_path("html_collisions");
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
    for asset in ["base.css", "coverage-tokens.css", "base.js"] {
        assert!(out_dir.join(asset).is_file(), "missing asset {asset}");
        assert!(out_dir.join(format!("{asset}.oxc-dir-1/index.html")).is_file());
    }
    assert!(out_dir.join("index.oxc-file-1.html").is_file());
    assert!(out_dir.join("src/index.oxc-file-1.html").is_file());
    assert!(out_dir.join("reserved #_x3F_% _xC3__xBC_.js.html").is_file());
    assert!(out_dir.join("folder #_x3F_% _xC3__xBC_/nested #_x3F_% _xC3__xBC_.js.html").is_file(),);
    assert_portable_output_tree(&out_dir);

    let root_index = std::fs::read_to_string(out_dir.join("index.html")).unwrap();
    assert!(root_index.contains("reserved #?% ü.js"), "logical display name changed");
    assert!(root_index.contains("href=\"reserved%20%23_x3F_%25%20_xC3__xBC_.js.html\""));
    assert!(root_index.contains("href=\"folder%20%23_x3F_%25%20_xC3__xBC_/index.html\""));

    for html in collect_html_files(&out_dir) {
        let page = std::fs::read_to_string(&html).unwrap();
        for href in local_hrefs(&page) {
            let target = resolved_local_href(&html, href);
            assert!(target.is_file(), "broken link {href:?} from {html:?} resolved to {target:?}");
        }
    }
}

#[test]
fn report_html_rejects_parent_traversal_before_writing_assets() {
    let workdir = temp_path("html_traversal");
    let out_dir = workdir.join("report");
    let sentinel = workdir.join("escape").join("pwn.js.html");
    let _ = std::fs::remove_dir_all(&workdir);
    std::fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
    std::fs::write(&sentinel, "sentinel").unwrap();
    let cov = write_temp(
        "report_html_traversal.json",
        r#"{"safe/a.js":{"path":"safe/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"../escape/pwn.js":{"path":"../escape/pwn.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#,
    );

    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg("--output-dir")
        .arg(&out_dir)
        .arg(&cov)
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error: failed to render report"), "got: {stderr}");
    assert!(stderr.contains("../escape/pwn.js"), "got: {stderr}");
    assert_eq!(std::fs::read_to_string(sentinel).unwrap(), "sentinel");
    assert!(!out_dir.join("base.css").exists());
    assert!(!out_dir.join("coverage-tokens.css").exists());
    assert!(!out_dir.join("base.js").exists());
}

#[test]
fn report_html_defaults_output_dir_to_coverage_subdir() {
    // The default only applies on the success path, so run from a temp cwd
    // rather than littering the repo with a `coverage/` directory.
    let cov = write_temp("report_html_default_dir.json", SAMPLE_COVERAGE);
    let workdir = temp_path("html_default_workdir");
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
    let out_dir = temp_path("html_threshold_out");
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
    // The rendered threshold sentence must quote 40, not the 80 default.
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
        .arg(temp_path("html_threshold_bad_out"))
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
    // NaN parses as `f64`; without the `is_finite` guard it would slip past the
    // range check and drop every percent into the "medium" bucket.
    for bad in ["nan", "inf", "-inf"] {
        let out = cli()
            .arg("report")
            .arg("--format")
            .arg("html")
            .arg("--output-dir")
            .arg(temp_path("html_threshold_nan_out"))
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
fn report_threshold_rejects_non_numeric_value() {
    let cov = write_temp("report_threshold_nonnumeric.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg("--output-dir")
        .arg(temp_path("threshold_nonnumeric_out"))
        .arg("--threshold")
        .arg("not-a-number")
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success(), "non-numeric --threshold must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--threshold must be a number"),
        "non-numeric --threshold should surface the parse error, got: {stderr}"
    );
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
fn report_lcov_rejects_output_dir_with_friendly_error() {
    let cov = write_temp("report_lcov_dir.json", SAMPLE_COVERAGE);
    let out = cli()
        .arg("report")
        .arg("--format")
        .arg("lcov")
        .arg("--output-dir")
        .arg(temp_path("lcov_dir_out"))
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
        .arg(temp_path("cobertura_dir_out"))
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
        .arg(temp_path("text_summary_dir_out"))
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
        .arg(temp_path("json_summary_dir_out"))
        .arg(&cov)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--format json-summary"), "got: {stderr}");
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
fn report_unknown_long_option_exits_failure() {
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
    // `--format` as the last argument runs the value lookup off the end of argv.
    let out = cli().arg("report").arg("--format").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--format requires a value"),
        "trailing --format must surface a missing-value error, got: {stderr}",
    );
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
        assert!(
            stderr.contains("--fail-under"),
            "report-specific help must mention --fail-under, got: {stderr}",
        );
    }
}
