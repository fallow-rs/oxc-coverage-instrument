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
cd napi
npm install
npx napi build --platform
node test.mjs
```

## Conformance test suite

The conformance tests compare our output against `istanbul-lib-instrument`. To regenerate reference data:

```bash
npm install  # in repo root (installs istanbul-lib-instrument)
node tests/conformance/generate-reference.mjs
```

## Code conventions

- Rust 2024 edition, MSRV 1.92
- Strict clippy (all + pedantic + nursery + Oxc-level restriction lints)
- `cargo fmt` with `style_edition = "2024"`, `use_small_heuristics = "Max"`
- Doc comments on all public types and functions
- Tests for new coverage constructs (statement types, branch types, function types)
- `#[expect(..., reason = "...")]` instead of `#[allow]`

## Submitting changes

1. Fork the repo
2. Create a branch from `main`
3. Make your changes
4. Run the full quality check:
   ```bash
   cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && typos
   ```
5. Open a PR with a clear description of what changed and why

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
