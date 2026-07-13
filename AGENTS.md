@~/.Codex/rules/release-companion-repos.md

# Repository guidance

Keep changes scoped, preserve unrelated worktree edits, and use checked-in policy as the source of truth. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, detailed commands, code conventions, and submission requirements.

## Repository layout and ownership

- `Cargo.toml` and crate-local manifests under `crates/` define the Rust workspace, crate versions, publication status, and dependency pins. Rust source and tests live with their owning crates.
- `crates/oxc-coverage-instrument-napi/` owns the published Node package and N-API adapter. Its `package.json` is the public npm manifest; the root `package.json` is private development tooling.
- `crates/oxc-coverage-instrument-napi/npm/` contains the platform package manifests. Keep these synchronized with the public npm manifest whenever their shared package contract changes.
- `scripts/` contains conformance, real-world, release synchronization, and package-surface checks. `.github/workflows/release-npm.yml` is authoritative for release topology and publication behavior.

## Generated artifacts

The root `index.js`, `index.d.ts`, `browser.js`, native binaries, WebAssembly files, and WASI loaders and workers are build outputs owned by the N-API tooling and repository patch scripts. Do not treat generated files as primary sources. Update the owning input or patch script, regenerate the artifacts, and verify that no unexpected drift remains.

## Version synchronization

Follow the [version policy](CONTRIBUTING.md#version-policy). The public Rust instrumenter and published npm package move together, while companion Rust crates follow their own manifest versions. Use the checked-in synchronization and version-check scripts, and keep the public npm manifest, platform manifests, optional dependency pins, and relevant Rust dependency pins consistent.

## Verification

Select the relevant verification categories from [CONTRIBUTING.md](CONTRIBUTING.md): Rust formatting, linting, tests, and docs; N-API build and tests; conformance and real-world checks; dependency policy; and workflow security. The pre-push hook is only a fast subset, so it does not replace the full checks required for the affected surfaces.

A bug fix is complete only after a minimal reproduction test passes, the fix is validated against at least one real user project, and the full relevant suite passes. Never copy private project names, source, output, secrets, or credentials into public fixtures, reports, documentation, commit messages, or release notes. Use generic examples instead.

## Git hygiene

Use signed commits with Conventional Commit subjects. Inspect the final diff and staged scope before committing, and leave unrelated changes untouched.
