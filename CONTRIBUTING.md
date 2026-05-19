# Contributing

Thanks for your interest in contributing.

## Getting started

```bash
git clone https://github.com/fallow-rs/oxc-coverage-instrument
cd oxc-coverage-instrument
git config core.hooksPath .githooks   # enable pre-push checks (see below)
cargo build
cargo test --workspace
cargo run --example instrument
```

## Pre-push hook

A versioned pre-push hook at `.githooks/pre-push` runs the fast CI checks (`cargo fmt --check`, `cargo clippy -D warnings`, `typos .`) before every push. It mirrors the CI jobs that block PR merges, so catching the failure locally saves a round-trip.

**Enable once per clone:**

```bash
git config core.hooksPath .githooks
```

**Install `typos` (optional but recommended):**

```bash
cargo install typos-cli
```

**Opt-in extras:**

```bash
RUN_TESTS=1 git push   # also run `cargo test --workspace` (~5-10s)
```

**Bypass (use sparingly — prefer fixing the root cause):**

```bash
SKIP_PRE_PUSH=1 git push
```

## Development workflow

```bash
# Check it compiles
cargo check --workspace

# Run tests (including doc tests)
cargo test --workspace --all-targets
cargo test --workspace --doc

# Run clippy (strict: all + pedantic + nursery)
cargo clippy --workspace --all-targets -- -D warnings

# Format
cargo fmt --all --check

# Typos
typos

# Docs
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
```

## Napi bindings (Node.js)

```bash
cd crates/oxc-coverage-instrument-napi
npm install
npx napi build --platform
node test.mjs
```

## Conformance test suite

The conformance tests compare our output against `istanbul-lib-instrument`. To regenerate reference data:

```bash
npm install  # in repo root (installs istanbul-lib-instrument for scripts/)
node crates/oxc-coverage-instrument/tests/conformance/generate-reference.mjs
```

## Helper scripts

- `scripts/istanbul-diff.mjs`: byte-for-byte conformance diff against `istanbul-lib-instrument` on the shared fixture corpus.
- `scripts/istanbul-upstream-specs.mjs`: runtime compatibility checks copied from upstream Istanbul specs.
- `scripts/benchmark-comparison.sh`: performance comparison against Istanbul, Babel, and SWC coverage instrumenters.
- `scripts/real-world-parity.mjs`: count-level parity check over the benchmark corpus populated by `benchmark-comparison.sh`.
- `scripts/compare-istanbul.mjs`: ad hoc reference-output dumper used when investigating Istanbul shape differences.
- `scripts/sync-npm-versions.sh`: syncs the N-API package and platform package versions during release prep.

## Code conventions

- Rust 2024 edition, MSRV 1.92
- Strict clippy (all + pedantic + nursery + Oxc-level restriction lints)
- `cargo fmt` with `style_edition = "2024"`, `use_small_heuristics = "Max"`
- Doc comments on all public types and functions
- Tests for new coverage constructs (statement types, branch types, function types)
- `#[expect(..., reason = "...")]` instead of `#[allow]`

## Version policy

- `oxc_coverage_instrument` and the published npm package move together and currently share the public package version.
- Companion Rust crates (`oxc_coverage_types`, `oxc_coverage_source_maps`, `oxc_coverage_v8`, `oxc_coverage_report`, `oxc_coverage_reports`) may stay on lower 0.x versions until their APIs mature.
- Internal adapter crates with `publish = false` (`oxc-coverage-instrument-cli`, `oxc_coverage_instrument_napi`) are implementation packages; their Cargo versions do not define the public release version.
- The root `package.json` is a private helper manifest for repository scripts. The published npm manifest is `crates/oxc-coverage-instrument-napi/package.json`.

## Submitting changes

1. Fork the repo
2. Create a branch from `main`
3. Make your changes
4. Run the full quality check:
   ```bash
   cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --all-targets && npm --prefix crates/oxc-coverage-instrument-napi run build:debug && node scripts/istanbul-diff.mjs && node crates/oxc-coverage-instrument-napi/test.mjs && typos
   ```
5. Open a PR with a clear description of what changed and why

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
