# Contributing

Thanks for your interest in contributing.

## Getting started

```bash
git clone https://github.com/fallow-rs/oxc-coverage-instrument
cd oxc-coverage-instrument
cargo build
cargo test --workspace
cargo run --example instrument
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
