# Contributing

Thanks for your interest in contributing.

## Getting started

```bash
git clone https://github.com/fallow-rs/oxc-coverage-instrument
cd oxc-coverage-instrument
cargo build
cargo test
cargo run --example instrument
```

## Development workflow

```bash
# Check it compiles
cargo check

# Run tests (including doc tests)
cargo test

# Run clippy
cargo clippy -- -D warnings

# Format
cargo fmt --check
```

## What to work on

Check the [ROADMAP.md](ROADMAP.md) for planned features. The highest-impact items for v0.2.0:

1. **AST-level counter injection via `Traverse`**: the current source-level injection has edge cases. This is the biggest improvement.
2. **Istanbul ignore pragma handling**: needed before the crate is usable in production.
3. **Conformance test suite**: run the same fixtures through both Babel's Istanbul instrumenter and this crate, compare counter structures.

## Code conventions

- Rust 2024 edition
- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- Doc comments on all public types and functions
- Tests for new visitor handlers (statement types, branch types, function types)

## Submitting changes

1. Fork the repo
2. Create a branch from `main`
3. Make your changes
4. Run `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
5. Open a PR with a clear description of what changed and why

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
