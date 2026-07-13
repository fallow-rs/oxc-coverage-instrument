# Plan 001: Keep HTML report writes inside the selected output directory

> **Executor instructions**: Follow this plan step by step. Run every verification
> command and confirm the expected result before moving on. Stop and report if a
> STOP condition occurs. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- crates/oxc-coverage-reports/src/html/mod.rs crates/oxc-coverage-instrument-cli/tests/cli_test.rs`
> If either file changed, compare the excerpts below with the live code. Stop if
> the output-path flow no longer matches this plan.

## Status

- **Status**: DONE
- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: security
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

The HTML reporter turns coverage-map keys into directory names and joins them
directly onto the caller's output directory. A coverage key containing retained
`..` segments or Windows separators can make the reporter create or overwrite
HTML outside that directory. The fix must reject unsafe report paths before the
first filesystem write while preserving ordinary relative and absolute Istanbul
paths.

## Current state

- `crates/oxc-coverage-report/src/summarizer.rs:39-53` splits only on `/` and
  preserves `.` and `..` as tree segments:

```rust
let paths: Vec<Vec<&str>> = map.keys().map(|k| split_path(k)).collect();
let prefix_len = common_prefix_len(&paths);
for (path_str, file) in map {
    let segments = split_path(path_str);
    let rel: &[&str] = &segments[prefix_len..];
    insert(&mut root, rel, file.clone());
}
```

- `crates/oxc-coverage-reports/src/html/mod.rs:218-257` joins the resulting
  `relative_path` directly onto `output_dir` for folders and files:

```rust
let folder_dir = if node.relative_path.is_empty() {
    output_dir.to_path_buf()
} else {
    output_dir.join(&node.relative_path)
};
```

- `html::write` writes shared assets before walking the tree. Validation must
  occur after `summarize` but before `create_dir_all(output_dir)` so a rejected
  input leaves no partial report.
- Existing HTML unit tests live inline in `html/mod.rs`; CLI integration follows
  `report_html_format_writes_directory_tree` in `cli_test.rs:407`.
- Match repository error handling: return `io::Error` with
  `io::ErrorKind::InvalidInput`; the CLI already renders this as
  `error: failed to render report: ...`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Focused library tests | `cargo test -p oxc_coverage_reports --features html` | exits 0 |
| Focused CLI tests | `cargo test -p oxc-coverage-instrument-cli cli_test` | exits 0 |
| Full suite | `cargo test --workspace --all-targets` | exits 0 |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | exits 0 |
| Format | `cargo fmt --all --check` | exits 0 |

## Scope

**In scope**:

- `crates/oxc-coverage-reports/src/html/mod.rs`
- `crates/oxc-coverage-instrument-cli/tests/cli_test.rs`

**Out of scope**:

- `crates/oxc-coverage-report/src/summarizer.rs`: text, LCOV, and Cobertura may
  legitimately display paths that are unsafe as output paths.
- Source-file reads under `root_dir`: this plan controls report writes only.
- Changing visible breadcrumbs or coverage-map paths.

## Git workflow

- Branch: `codex/001-contain-html-report-paths`
- Commit: `fix(reports): contain html output paths`
- Use `git commit -S`. Do not push or open a PR unless instructed.

## Steps

### Step 1: Add failing traversal regression tests

In the inline HTML tests, construct a coverage map with two divergent keys so
common-prefix stripping does not remove the malicious segment. Include:

- `safe/a.js` plus `../escape/pwn.js`
- `safe/a.js` plus `..\\escape\\pwn.js`
- a mixed-drive shape such as `C:/safe/a.js` plus `D:/other/b.js`

For traversal inputs, pre-create a sentinel outside `output_dir`, call
`html::write`, assert `InvalidInput`, assert the error names the rejected
coverage path, assert the sentinel is unchanged, and assert no shared assets
were created. Add one CLI test with the same sentinel invariant.

**Verify**: run the focused tests before the fix and confirm at least the
forward-slash traversal repro fails for the expected reason.

### Step 2: Validate every output path before writing

Add a private validation pass in `html/mod.rs`. Call it after `summarize` and
before any `fs::create_dir_all` or `fs::write`. Validate every folder and file
node recursively.

Treat a node path as safe only when every forward-slash segment is a normal,
single filesystem component on the current platform. Reject:

- `.` and `..`
- any backslash, because it is a separator on Windows even when the report tree
  was built on another platform
- a Windows drive-prefix component such as `C:`
- any component that `Path::components()` does not classify as one normal
  component

Return `InvalidInput` with the original `node.relative_path`. Do not silently
rewrite or flatten paths because that can create collisions between distinct
coverage entries.

**Verify**: `cargo test -p oxc_coverage_reports --features html` exits 0.

### Step 3: Prove valid reports still work

Retain the existing nested-folder and absolute-path tests. Add a control case
with safe punctuation such as `src/a&b.js` to prove validation is not an
overbroad alphanumeric-only filter.

Build the native binding and run the existing Vitest TypeScript example to
produce a real `coverage-final.json`, then render it through the CLI:

```bash
npm --prefix crates/oxc-coverage-instrument-napi ci
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
npm --prefix examples/vitest-typescript ci
npm --prefix examples/vitest-typescript run coverage
cargo run -p oxc-coverage-instrument-cli -- report --format html --output-dir /tmp/oxc-html-real examples/vitest-typescript/coverage/coverage-final.json
```

**Verify**: every command exits 0 and `/tmp/oxc-html-real/index.html` exists.

### Step 4: Run repository gates

**Verify**:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

All commands must exit 0.

## Test plan

- Minimal repro: retained `../` folder escapes the output root before the fix.
- Adjacent forms: Windows backslashes and drive prefixes.
- Atomic failure: no report assets or escaped files are written on rejection.
- Valid controls: nested folders, absolute Istanbul keys after common-prefix
  stripping, and safe punctuation.
- Real project: generate and render coverage from `examples/vitest-typescript`.

## Done criteria

- [ ] Unsafe coverage keys return `InvalidInput` before any filesystem write.
- [ ] The regression test proves an outside sentinel cannot be overwritten.
- [ ] Existing valid HTML report tests pass.
- [ ] The Vitest TypeScript example renders successfully.
- [ ] Full format, clippy, and workspace tests pass.
- [ ] Only in-scope files and `plans/README.md` are modified.

## STOP conditions

Stop and report if:

- Safe absolute Istanbul paths are rejected after common-prefix stripping.
- Containment requires canonicalizing paths that do not yet exist.
- The fix needs changes to the generic report tree or non-HTML reporters.
- A verification command fails twice after a reasonable correction.

## Maintenance notes

Any future multi-file reporter must apply the same rule before its first write.
Reviewers should check Windows behavior explicitly, because a backslash is an
ordinary character on Unix but a path separator on Windows.
