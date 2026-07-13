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

## Git hooks

Versioned hooks under `.githooks/` mirror the CI jobs that block PR merges, so catching a failure locally saves a round-trip:

- **`pre-push`** delegates the fast checks to `./scripts/check.sh pre-push` before every push.
- **`commit-msg`** lints the commit message against the Conventional Commits rules (the CI "Commit messages" job). It skips itself when commitlint is not installed, so a pure-Rust contributor is never blocked.

**Enable once per clone:**

```bash
git config core.hooksPath .githooks
```

**Install `typos` (optional but recommended):**

```bash
cargo install typos-cli
```

**Enable `commit-msg` linting (optional):** install the commitlint toolchain at the repo root.

```bash
npm ci   # installs @commitlint/cli from package.json
```

**Opt-in extras:**

```bash
RUN_TESTS=1 git push   # also run `cargo test --workspace` (~5-10s)
```

**Bypass (use sparingly, prefer fixing the root cause):**

```bash
SKIP_PRE_PUSH=1 git push
```

## Development workflow

List the canonical repository checks and run focused profiles as needed:

```bash
# Check it compiles
./scripts/check.sh rust-check

# Run tests (including doc tests)
./scripts/check.sh rust-test
./scripts/check.sh doc-test

# Run clippy (strict: all + pedantic + nursery)
./scripts/check.sh clippy

# Format
./scripts/check.sh fmt

# Typos
./scripts/check.sh typos

# Docs
./scripts/check.sh rust-doc

# Show every primitive and aggregate profile
./scripts/check.sh --list
```

The full local profile requires Rust, Node.js 22, npm dependencies at the
repository root, N-API package, and Vitest example, plus `typos`, `cargo-audit`,
`cargo-shear`, `actionlint`, and `zizmor`. Build the native N-API artifact
before running it. The package-surface primitive also requires the generated
threaded and single-threaded WASI package artifacts prepared by the release
tooling.

```bash
npm install
npm --prefix crates/oxc-coverage-instrument-napi install
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
npm --prefix examples/vitest-typescript install
./scripts/check.sh all-local
```

Direct profiles fail with an installation or artifact hint when a prerequisite
is missing. Only the pre-push aggregate may skip `typos`, and it reports the
skip.

### Dependency and supply-chain gates

The CI runs `cargo audit`, `cargo deny`, and `cargo shear` on every push. Reproduce locally before touching `Cargo.toml`:

```bash
# Security advisories (RustSec)
cargo install cargo-audit
./scripts/check.sh audit

# License / dependency policy (see deny.toml)
cargo install cargo-deny
cargo deny check

# Unused / misplaced dependencies
cargo install cargo-shear
./scripts/check.sh shear
```

### Workflow files (`.github/`)

When editing anything under `.github/workflows/` or `.github/actions/`, the CI runs `actionlint` and `zizmor` against the result. Reproduce locally:

```bash
brew install actionlint zizmor                 # or `cargo install` / `pip install`
./scripts/check.sh actionlint
./scripts/check.sh zizmor
```

Every external action must be SHA-pinned with a version comment (`uses: owner/action@<40-hex> # vX.Y.Z`). Floating `@v6` / `@main` tags are rejected by zizmor's policy.

### Commit messages

CI enforces conventional commits via commitlint. To reproduce locally before pushing:

```bash
npm ci --ignore-scripts                                  # installs @commitlint/cli
./scripts/check.sh commitlint                             # lint your branch's commits
```

Config lives in `commitlint.config.mjs`. Allowed types: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, `test`.

## Napi bindings (Node.js)

```bash
cd crates/oxc-coverage-instrument-napi
npm install
npx napi build --platform
cd ../..
./scripts/check.sh napi-test
./scripts/check.sh wasi-shim-test
```

### Checks that stay in CI

CI continues to own the OS and target matrices, MSRV toolchain selection,
clean-worktree postcondition, WASM builds and tests, native versus WASM parity,
platform examples, coverage generation and upload, artifact handling, and
release publication. The Typos and Cargo Deny jobs stay action-owned. Their
setup, environment, matrices, and artifact inputs are not reproduced by
`all-local`; the profile prints this remainder instead of describing it as
passed.

## Conformance test suite

The conformance tests compare our output against `istanbul-lib-instrument`. To regenerate reference data:

```bash
npm install  # in repo root (installs istanbul-lib-instrument for scripts/)
node crates/oxc-coverage-instrument/tests/conformance/generate-reference.mjs
```

## Helper scripts

- `scripts/istanbul-diff.mjs`: byte-for-byte conformance diff against `istanbul-lib-instrument` on the shared fixture corpus.
- `scripts/istanbul-upstream-specs.mjs`: runtime compatibility checks copied from upstream Istanbul specs.
- `scripts/real-world-output.mjs`: runtime behavior and counter-placement checks for production-like JavaScript and stripped TypeScript.
- `scripts/v8-inspector-smoke.mjs`: real Node inspector coverage conversion and stable Istanbul behavior checks.
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
   ./scripts/check.sh all-local
   ```
5. Open a PR with a clear description of what changed and why. The PR body must contain a `Closes #N` (or `Fixes #N` / `Resolves #N`) keyword for any issue it closes, or the literal string `N/A` if it does not close an issue. A workflow rejects PRs that omit both; the check is skipped for `[bot]` authors.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
