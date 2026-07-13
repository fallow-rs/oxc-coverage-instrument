# Secure HTML Output Writes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make HTML report output resistant to symlink replacement and hard-link truncation races on every supported platform.

**Architecture:** Add a focused capability-backed output helper using `cap-std` and `cap-fs-ext`. Retain directory handles, traverse with no-follow opens, and replace leaves through temporary siblings and handle-relative renames.

**Tech Stack:** Rust 2024, cap-std 4.0.2, cap-fs-ext 4.0.2, tempfile, existing HTML report tests.

## Global Constraints

- Preserve the public `html::write` and `Format::write_to_dir` signatures.
- Preserve recursive output layout and overwriting of prior generated files.
- Add no unsafe code.
- Use a failing regression before production edits.
- Validate against the real Vitest TypeScript example and the full relevant suite.

---

### Task 1: Prove the race and hard-link failures

**Files:**
- Modify: `crates/oxc-coverage-reports/src/html/mod.rs`

**Interfaces:**
- Consumes: existing `html::write` API.
- Produces: black-box regressions that protect containment behavior.

- [ ] Add a test that opens the output root, replaces a valid nested directory
  entry with a symlink to an outside directory before the nested leaf write,
  and asserts the outside sentinel is unchanged.
- [ ] Add a test that hard-links the future report leaf to an outside sentinel,
  writes the report, and asserts the outside inode content is unchanged.
- [ ] Run the focused tests and confirm RED because path-based writes either
  follow the swapped symlink or truncate the hard-linked inode.

Run:

```bash
cargo test -p oxc_coverage_reports html::tests:: --features html
```

Expected: the new containment tests fail for the reviewed behavior.

### Task 2: Add the capability-backed output helper

**Files:**
- Create: `crates/oxc-coverage-reports/src/html/output.rs`
- Modify: `crates/oxc-coverage-reports/src/html/mod.rs`
- Modify: `crates/oxc-coverage-reports/Cargo.toml`
- Modify: `Cargo.lock`

**Interfaces:**
- Produces: `OutputDir::open(&Path) -> io::Result<OutputDir>`.
- Produces: `OutputDir::write(&Path, &[u8]) -> io::Result<()>`.

- [ ] Add exact dependencies `cap-std = "4.0.2"` and
  `cap-fs-ext = "4.0.2"` behind the existing `html` feature.
- [ ] Implement component validation and handle-relative
  `open_dir_nofollow` traversal in `output.rs`.
- [ ] Implement temporary `create_new` leaf writes and same-directory rename.
  If replacement is unsupported, remove only the destination directory entry
  through the held parent handle, then retry rename.
- [ ] Ensure every failure removes its temporary sibling and never truncates
  an existing destination inode.
- [ ] Replace every HTML asset, folder-index, and detail-page `fs::write` and
  `create_dir_all` call with `OutputDir` operations.
- [ ] Remove the obsolete check-then-use symlink preflight.

### Task 3: Verify secure and ordinary rendering

**Files:**
- Modify: `crates/oxc-coverage-reports/src/html/mod.rs` only if a regression
  exposes a missing case.

- [ ] Run the focused RED tests and confirm GREEN.
- [ ] Run report crate tests, strict clippy, and formatting.
- [ ] Build the N-API binding, run the real Vitest example, and render its
  generated coverage through the HTML CLI.
- [ ] Run the full workspace suite.
- [ ] Commit with a signed Conventional Commit subject.

Run:

```bash
cargo fmt --all --check
cargo clippy -p oxc_coverage_reports --all-targets -- -D warnings
cargo test -p oxc_coverage_reports --all-targets
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
npm --prefix examples/vitest-typescript run coverage
npm --prefix examples/vitest-typescript run verify
cargo test --workspace --all-targets
```

Expected: every command exits successfully and both outside sentinels remain
unchanged.

