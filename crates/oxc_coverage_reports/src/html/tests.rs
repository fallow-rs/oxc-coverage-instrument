//! Tests for the html reporter entry point, grouped by concern: rejection of
//! unsafe output paths, physical-path collision mapping, the emitted CSS/JS
//! assets, and the rendered markup.

use super::*;
use oxc_coverage_types::parse_coverage_map;
use std::fs;
use std::path::PathBuf;

fn write_to_temp(json: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let map = parse_coverage_map(json).unwrap();
    write(&map, Path::new(""), dir.path(), &HtmlOptions::default()).unwrap();
    dir
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                out.extend(walkdir(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

mod output_safety {
    use super::*;

    fn assert_rejects_unsafe_report_path(json: &str, rejected_path: &str) {
        let dir = tempfile::TempDir::new().unwrap();
        let output_dir = dir.path().join("report");
        let sentinel = dir.path().join("escape").join("pwn.js.html");
        fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
        fs::write(&sentinel, "sentinel").unwrap();
        let map = parse_coverage_map(json).unwrap();

        let error = write(&map, Path::new(""), &output_dir, &HtmlOptions::default()).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains(rejected_path), "got: {error}");
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "sentinel");
        assert!(!output_dir.join("base.css").exists());
        assert!(!output_dir.join("coverage-tokens.css").exists());
        assert!(!output_dir.join("base.js").exists());
    }

    #[test]
    fn rejects_parent_traversal_before_writing_assets() {
        let json = r#"{"safe/a.js":{"path":"safe/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"../escape/pwn.js":{"path":"../escape/pwn.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        assert_rejects_unsafe_report_path(json, "../escape/pwn.js");
    }

    #[test]
    fn rejects_windows_parent_traversal_before_writing_assets() {
        let json = r#"{"safe/a.js":{"path":"safe/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"..\\escape\\pwn.js":{"path":"..\\escape\\pwn.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        assert_rejects_unsafe_report_path(json, r"..\escape\pwn.js");
    }

    #[test]
    fn rejects_windows_drive_prefixes_before_writing_assets() {
        let json = r#"{"C:/safe/a.js":{"path":"C:/safe/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"D:/other/b.js":{"path":"D:/other/b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        assert_rejects_unsafe_report_path(json, "C:/safe/a.js");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_output_component_before_writing_assets() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let output_dir = dir.path().join("report");
        let outside_dir = dir.path().join("outside");
        let sentinel = outside_dir.join("pwn.js.html");
        fs::create_dir_all(&output_dir).unwrap();
        fs::create_dir_all(&outside_dir).unwrap();
        fs::write(&sentinel, "sentinel").unwrap();
        symlink(&outside_dir, output_dir.join("link")).unwrap();
        let map = parse_coverage_map(
            r#"{"safe.js":{"path":"safe.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"link/pwn.js":{"path":"link/pwn.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#,
        )
        .unwrap();

        let error = write(&map, Path::new(""), &output_dir, &HtmlOptions::default()).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("link"), "got: {error}");
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "sentinel");
        assert!(!output_dir.join("base.css").exists());
        assert!(!output_dir.join("coverage-tokens.css").exists());
        assert!(!output_dir.join("base.js").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_output_component_replaced_by_symlink_after_root_open() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let output_dir = dir.path().join("report");
        let nested_dir = output_dir.join("link");
        let outside_dir = dir.path().join("outside");
        let sentinel = outside_dir.join("pwn.js.html");
        fs::create_dir_all(&nested_dir).unwrap();
        fs::create_dir_all(&outside_dir).unwrap();
        fs::write(&sentinel, "sentinel").unwrap();
        let map = parse_coverage_map(
            r#"{"safe.js":{"path":"safe.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"link/pwn.js":{"path":"link/pwn.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#,
        )
        .unwrap();

        let result =
            write_report(&map, Path::new(""), &output_dir, &HtmlOptions::default(), || {
                let _output_root = fs::File::open(&output_dir)?;
                fs::rename(&nested_dir, output_dir.join("original-link"))?;
                symlink(&outside_dir, &nested_dir)
            });

        assert!(result.is_err(), "a replaced output component must fail the report write");
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "sentinel");
    }

    #[test]
    fn replaces_hard_linked_report_leaf_without_truncating_outside_inode() {
        let dir = tempfile::TempDir::new().unwrap();
        let output_dir = dir.path().join("report");
        let sentinel = dir.path().join("outside-sentinel");
        fs::create_dir_all(&output_dir).unwrap();
        fs::write(&sentinel, "sentinel").unwrap();
        fs::hard_link(&sentinel, output_dir.join("a.js.html")).unwrap();
        let map = parse_coverage_map(
            r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#,
        )
        .unwrap();

        write(&map, Path::new(""), &output_dir, &HtmlOptions::default()).unwrap();

        assert_eq!(fs::read_to_string(sentinel).unwrap(), "sentinel");
        assert!(
            fs::read_to_string(output_dir.join("a.js.html")).unwrap().contains("<!doctype html>")
        );
    }
}

mod collisions {
    use super::*;

    const COLLISION_COVERAGE: &str = r#"{
      "index":{"path":"index","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "keep.js":{"path":"keep.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "src/index":{"path":"src/index","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "base.css/a.js":{"path":"base.css/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "base.js/a.js":{"path":"base.js/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},
      "coverage-tokens.css/a.js":{"path":"coverage-tokens.css/a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}
    }"#;

    #[test]
    fn renders_root_and_nested_index_files_without_overwriting_folder_indexes() {
        let dir = write_to_temp(COLLISION_COVERAGE);

        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(root_index.contains("<title>Coverage: All files</title>"));
        assert!(root_index.contains("href=\"index.oxc-file-1.html\""));
        assert!(dir.path().join("index.oxc-file-1.html").exists());

        let nested_index = fs::read_to_string(dir.path().join("src/index.html")).unwrap();
        assert!(nested_index.contains("href=\"index.oxc-file-1.html\""));
        assert!(dir.path().join("src/index.oxc-file-1.html").exists());
    }

    #[test]
    fn renders_asset_named_root_directories_and_keeps_assets_intact() {
        let dir = write_to_temp(COLLISION_COVERAGE);
        for asset in ["base.css", "coverage-tokens.css", "base.js"] {
            assert!(dir.path().join(asset).is_file(), "asset must remain a file: {asset}");
            let mapped_dir = format!("{asset}.oxc-dir-1");
            assert!(dir.path().join(&mapped_dir).join("index.html").is_file());
            assert!(
                fs::read_to_string(dir.path().join("index.html")).unwrap().contains(&mapped_dir)
            );
        }
    }
}

mod emitted_assets {
    use super::*;

    #[test]
    fn writes_root_index_and_base_css() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(root_index.contains("<title>Coverage: All files</title>"));
        assert!(root_index.contains("<link rel=\"stylesheet\" href=\"base.css\">"));
        let css = fs::read_to_string(dir.path().join("base.css")).unwrap();
        assert!(css.contains(".kpi-cell"), "base.css missing KPI cell rules");
        let tokens = fs::read_to_string(dir.path().join("coverage-tokens.css")).unwrap();
        assert!(tokens.contains("--hit-bg"), "coverage-tokens.css missing semantic alias");
        assert!(tokens.contains("--font-body"), "coverage-tokens.css missing font stack");
    }

    #[test]
    fn writes_base_js_alongside_base_css() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let js = fs::read_to_string(dir.path().join("base.js")).unwrap();
        assert!(js.contains("buildThemeToggle"), "base.js missing theme toggle invocation");
        assert!(js.contains("initSortable"), "base.js missing sortable init invocation");
        assert!(
            js.contains("updateThresholdSummary"),
            "base.js should update the threshold summary when filtering",
        );
        assert!(
            js.contains("table.addEventListener('click'"),
            "line anchors should use a delegated table click handler",
        );
        assert!(
            !js.contains("anchors.forEach"),
            "line anchors should not attach one listener per row",
        );
        // The source view is pre-rendered by syntect, so base.js must ship no
        // tokenizer.
        assert!(!js.contains("initPrettify"), "base.js must not tokenize client-side");
        assert!(!js.contains("KEYWORD_SET"), "base.js must not tokenize client-side");
    }

    #[test]
    fn emitted_assets_have_no_external_urls() {
        // Corporate-proxy hardening: the report must not contain any
        // absolute external URL in HTML, CSS, or JS.
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let banned = [
            "http://",
            "https://",
            "ws://",
            "wss://",
            "//cdn.",
            "fonts.googleapis",
            "fonts.gstatic",
        ];
        for path in walkdir(dir.path()) {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            for needle in banned {
                assert!(
                    !text.contains(needle),
                    "{path:?} contains external reference {needle:?}:\n{text}"
                );
            }
        }
    }

    #[test]
    fn base_css_exposes_theme_override_and_token_classes() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let css = fs::read_to_string(dir.path().join("base.css")).unwrap();
        let tokens = fs::read_to_string(dir.path().join("coverage-tokens.css")).unwrap();
        assert!(tokens.contains(":root[data-theme=\"dark\"]"), "missing dark override in tokens");
        assert!(
            tokens.contains(":root:not([data-theme=\"light\"])"),
            "missing light escape hatch in tokens",
        );
        assert!(css.contains(".sortable"), "missing .sortable selector");
        assert!(css.contains("aria-sort=\"ascending\""), "missing aria-sort hook");
        for cls in [
            ".stok-comment",
            ".stok-keyword",
            ".stok-storage",
            ".stok-string",
            ".stok-constant",
            ".stok-support",
            ".stok-entity",
            ".stok-punctuation",
        ] {
            assert!(css.contains(cls), "missing token class {cls}");
        }
        assert!(css.contains(".theme-toggle__btn"), "missing theme-toggle button class");
    }
}

mod markup {
    use super::*;
    use crate::escape::html_text;

    const DAMAGED_BRANCH: &str = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}}},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{"0":1},"f":{},"b":{"0":[4,0,9]}}}"#;

    #[test]
    fn branch_metadata_controls_detail_arms() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "if (x) y;\n").unwrap();
        let map = parse_coverage_map(DAMAGED_BRANCH).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("true=4"));
        assert!(detail.contains("false=0"));
        assert!(!detail.contains("arm 3"));
    }

    #[test]
    fn writes_per_file_detail_page() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let detail = fs::read_to_string(dir.path().join("a.js.html")).unwrap();
        assert!(detail.contains("<title>Coverage: a.js</title>"));
        assert!(detail.contains("class=\"breadcrumb\""));
    }

    #[test]
    fn nested_folders_have_correct_css_href_depth() {
        // The summarizer strips common prefixes so a SINGLE file under "src/"
        // would collapse to root. Use one file at root and one nested so the
        // tree actually keeps the "src/" folder.
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let detail = fs::read_to_string(dir.path().join("src").join("foo.js.html")).unwrap();
        assert!(
            detail.contains("href=\"../base.css\""),
            "expected ../base.css href in nested file, got:\n{detail}"
        );
        assert!(detail.contains("src=\"../base.js\""), "nested script href wrong: {detail}");
        let root_detail = fs::read_to_string(dir.path().join("a.js.html")).unwrap();
        assert!(
            root_detail.contains("href=\"base.css\""),
            "expected base.css href at root, got:\n{root_detail}"
        );
        assert!(root_detail.contains("src=\"base.js\""), "root script href wrong: {root_detail}");
    }

    #[test]
    fn html_special_chars_are_escaped_in_titles_and_breadcrumbs() {
        // Use '&' as the HTML-special character: it exercises the escaping
        // path (`&` -> `&amp;`) AND is a valid filename on every platform,
        // unlike `<` or `>` which Windows path APIs reject.
        let json = r#"{"root.js":{"path":"root.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/a&b.js":{"path":"src/a&b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let detail = fs::read_to_string(dir.path().join("src").join("a&b.js.html")).unwrap();
        assert!(detail.contains("a&amp;b.js"), "got:\n{detail}");
    }

    #[test]
    fn detail_row_class_marks_misses() {
        // The coverage key is a relative path resolved against `root_dir` so
        // the same JSON works on every platform; an absolute Windows path
        // (`C:\\...`) would round-trip through the report tree as a
        // non-relative folder component, which Windows path APIs reject.
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "const x = 1;\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":12}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("line miss"), "expected miss class; got:\n{detail}");
    }

    #[test]
    fn every_page_carries_csp_and_script_tag() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        for html_path in walkdir(dir.path())
            .into_iter()
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("html"))
        {
            let html = fs::read_to_string(&html_path).unwrap();
            assert!(html.contains("Content-Security-Policy"), "missing CSP meta in {html_path:?}");
            assert!(
                html.contains("connect-src &#39;none&#39;"),
                "CSP lacks connect-src 'none' in {html_path:?}"
            );
            assert!(
                html.contains("font-src &#39;none&#39;"),
                "CSP lacks font-src 'none' in {html_path:?}"
            );
            assert!(html.contains("<script src="), "missing <script> in {html_path:?}");
            assert!(html.contains("base.js"), "page does not reference base.js in {html_path:?}");
            assert!(
                html.contains(" defer></script>"),
                "<script> not marked defer in {html_path:?}"
            );
            assert!(html.contains("viewport"), "missing viewport meta in {html_path:?}");
        }
    }

    #[test]
    fn index_emits_kpi_pills_with_bracket_labels() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(root_index.contains("class=\"kpi-row\""), "kpi-row class missing");
        assert!(root_index.contains("kpi-cell--"), "kpi-cell severity modifier missing");
        for label in ["[ Statements ]", "[ Branches ]", "[ Functions ]", "[ Lines ]"] {
            assert!(root_index.contains(label), "missing bracket label {label:?}");
        }
    }

    #[test]
    fn index_emits_inline_cov_meter_on_lines_column() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(
            root_index.contains("class=\"cov-meter"),
            "summary row missing inline mini coverage meter",
        );
        assert!(root_index.contains("role=\"meter\""), "cov-meter must carry role=\"meter\"");
        assert!(
            root_index.contains("aria-valuemax=\"100\""),
            "cov-meter must declare aria-valuemax",
        );
    }

    #[test]
    fn index_emits_threshold_summary_and_filter_group() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let root_index = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(
            root_index.contains("threshold-summary"),
            "index missing threshold summary sentence",
        );
        assert!(
            root_index.contains("id=\"cov-threshold-summary\""),
            "threshold summary should expose a stable id",
        );
        assert!(
            root_index.contains("data-threshold-pct=\"80.00\""),
            "threshold summary should expose its threshold for JS updates",
        );
        assert!(
            root_index.contains("data-lines-pct=\"100.00\""),
            "summary rows should expose line pct for filter-reactive threshold counts",
        );
        assert!(root_index.contains("id=\"cov-filter\""), "filter input missing");
        assert!(
            root_index.contains("<label class=\"filter-group__label\" for=\"cov-filter\""),
            "filter input must have an explicit <label>",
        );
        // `role="status"` implies `aria-live="polite"` per WAI-ARIA, so the
        // markup carries the role alone.
        assert!(
            root_index.contains("id=\"cov-filter-count\" role=\"status\""),
            "filter result counter must be a polite live region",
        );
        assert!(
            root_index.contains("aria-controls=\"cov-file-table\""),
            "filter input must reference the controlled table",
        );
    }

    #[test]
    fn detail_emits_next_uncovered_button_and_copy_toast() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        let detail = fs::read_to_string(dir.path().join("a.js.html")).unwrap();
        assert!(
            detail.contains("id=\"cov-next-uncovered\""),
            "detail page missing Next uncovered button",
        );
        assert!(
            detail.contains("class=\"btn-ghost\""),
            "Next uncovered must use the ghost button style",
        );
        assert!(
            detail.contains("id=\"cov-copy-toast\""),
            "detail page missing copy toast live region",
        );
    }

    #[test]
    fn source_rows_carry_line_anchor_button_and_severity_glyph() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "const x = 1;\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":12}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("id=\"L1\""), "missed line should have id=\"L1\" anchor target");
        assert!(
            detail.contains("class=\"line-anchor\" data-line=\"1\""),
            "missed line should expose a line-anchor button",
        );
        assert!(
            detail.contains("aria-label=\"Copy link to line 1\""),
            "line-anchor button must be self-labeling for screen readers",
        );
        assert!(
            detail.contains("sev-glyph sev-glyph--miss"),
            "missed line should carry the [M] severity glyph",
        );
        assert!(
            detail.contains("aria-label=\"Line 1, not covered\""),
            "missed row should carry a human-readable aria-label",
        );
    }

    #[test]
    fn partial_branch_rows_show_per_arm_counts() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "if (x) y;\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":9}}},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":9}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":4},"end":{"line":1,"column":5}},{"start":{"line":1,"column":6},"end":{"line":1,"column":9}}]}},"s":{"0":1},"f":{},"b":{"0":[12,0]}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("branch-arm--hit"), "hit arm marker missing:\n{detail}");
        assert!(detail.contains("true=12"), "true-arm count missing:\n{detail}");
        assert!(detail.contains("branch-arm--miss"), "miss arm marker missing:\n{detail}");
        assert!(detail.contains("false=0"), "false-arm count missing:\n{detail}");
        assert!(
            detail.contains("true arm hit 12 times, false arm missed 0 times"),
            "aria label should expose branch-arm detail:\n{detail}",
        );
    }

    #[test]
    fn covered_function_declaration_line_gets_fn_zero_cue_not_miss_class() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.js"), "function f() {}\n").unwrap();
        let json = r#"{"a.js":{"path":"a.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":15}}},"fnMap":{"0":{"name":"f","line":1,"decl":{"start":{"line":1,"column":0},"end":{"line":1,"column":10}},"loc":{"start":{"line":1,"column":13},"end":{"line":1,"column":15}}}},"branchMap":{},"s":{"0":1},"f":{"0":0},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("a.js.html")).unwrap();
        assert!(detail.contains("class=\"line hit\""), "decl line should stay hit:\n{detail}");
        assert!(detail.contains("fn 0x"), "function-uncalled cue missing:\n{detail}");
        assert!(
            detail.contains("Line 1, covered: function not called"),
            "aria label should expose uncalled function:\n{detail}",
        );
    }

    #[test]
    fn missing_source_detail_shows_path_and_search_root_notice() {
        let dir = tempfile::TempDir::new().unwrap();
        let json = r#"{"missing.js":{"path":"missing.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":12}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let out_dir = dir.path().join("html");
        write(&map, dir.path(), &out_dir, &HtmlOptions::default()).unwrap();
        let detail = fs::read_to_string(out_dir.join("missing.js.html")).unwrap();
        assert!(
            detail.contains("Source file unavailable at"),
            "missing source notice absent:\n{detail}",
        );
        assert!(detail.contains("missing.js"), "attempted path missing:\n{detail}");
        assert!(
            detail.contains(&html_text(&dir.path().display().to_string())),
            "search root missing from notice:\n{detail}",
        );
    }

    #[test]
    fn every_page_carries_generator_meta_tag() {
        let json = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/foo.js":{"path":"src/foo.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let dir = write_to_temp(json);
        for html_path in walkdir(dir.path())
            .into_iter()
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("html"))
        {
            let html = fs::read_to_string(&html_path).unwrap();
            assert!(
                html.contains("<meta name=\"generator\" content=\"oxc-coverage-reports "),
                "missing generator meta in {html_path:?}",
            );
        }
    }

    #[test]
    fn custom_threshold_drives_summary_sentence_and_pct_class() {
        // Two files: one at 70% lines, one at 90% lines. With the default
        // 80% threshold the 70% file is "below"; with --threshold 60 both
        // are "above" and the row colour bucketing flips.
        let json = r#"{
            "lo.js":{"path":"lo.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":3,"column":0},"end":{"line":3,"column":1}},"3":{"start":{"line":4,"column":0},"end":{"line":4,"column":1}},"4":{"start":{"line":5,"column":0},"end":{"line":5,"column":1}},"5":{"start":{"line":6,"column":0},"end":{"line":6,"column":1}},"6":{"start":{"line":7,"column":0},"end":{"line":7,"column":1}},"7":{"start":{"line":8,"column":0},"end":{"line":8,"column":1}},"8":{"start":{"line":9,"column":0},"end":{"line":9,"column":1}},"9":{"start":{"line":10,"column":0},"end":{"line":10,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":1,"2":1,"3":1,"4":1,"5":1,"6":1,"7":0,"8":0,"9":0},"f":{},"b":{}},
            "hi.js":{"path":"hi.js","statementMap":{"0":{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},"1":{"start":{"line":2,"column":0},"end":{"line":2,"column":1}},"2":{"start":{"line":3,"column":0},"end":{"line":3,"column":1}},"3":{"start":{"line":4,"column":0},"end":{"line":4,"column":1}},"4":{"start":{"line":5,"column":0},"end":{"line":5,"column":1}},"5":{"start":{"line":6,"column":0},"end":{"line":6,"column":1}},"6":{"start":{"line":7,"column":0},"end":{"line":7,"column":1}},"7":{"start":{"line":8,"column":0},"end":{"line":8,"column":1}},"8":{"start":{"line":9,"column":0},"end":{"line":9,"column":1}},"9":{"start":{"line":10,"column":0},"end":{"line":10,"column":1}}},"fnMap":{},"branchMap":{},"s":{"0":1,"1":1,"2":1,"3":1,"4":1,"5":1,"6":1,"7":1,"8":1,"9":0},"f":{},"b":{}}
        }"#;
        let map = parse_coverage_map(json).unwrap();

        // Under the default 80% threshold the 70% file buckets "medium"
        // (>= 50%) and counts as the one file below threshold.
        let dir_default = tempfile::TempDir::new().unwrap();
        write(&map, Path::new(""), dir_default.path(), &HtmlOptions::default()).unwrap();
        let default_root = fs::read_to_string(dir_default.path().join("index.html")).unwrap();
        assert!(
            default_root.contains("1</strong> of 2 files fall below the 80%"),
            "default threshold should report 1 below 80%: {default_root}",
        );
        assert!(
            default_root.contains("row-medium"),
            "70% file should bucket medium under default threshold",
        );

        // Custom 60%: both files are above; sentence should report all met.
        let dir_loose = tempfile::TempDir::new().unwrap();
        let opts = HtmlOptions::new(60.0).unwrap();
        write(&map, Path::new(""), dir_loose.path(), &opts).unwrap();
        let loose_root = fs::read_to_string(dir_loose.path().join("index.html")).unwrap();
        assert!(
            loose_root.contains("All 2 files</strong> meet the 60% coverage threshold"),
            "60% threshold should clear all files: {loose_root}",
        );
        assert!(
            loose_root.contains("row-high"),
            "70% file should bucket high under a 60% threshold",
        );
    }
}
