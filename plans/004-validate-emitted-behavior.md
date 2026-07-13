# Plan 004: Make production-like instrumentation tests prove emitted behavior

> **Executor instructions**: Follow the steps in order and run every gate. Stop
> and report rather than weakening assertions when a STOP condition occurs.
> Update this plan's row in `plans/README.md` when complete.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- crates/oxc-coverage-instrument/tests/real_world_test.rs scripts/istanbul-upstream-specs.mjs scripts/real-world-parity.mjs .github/workflows/ci.yml`
> Stop if the shared real-world assertion or CI runtime-test placement changed.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: tests
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

The production-like Rust fixtures currently prove only that instrumentation
returns non-empty text, internally aligned maps, and serializable JSON. Invalid
emitted syntax, changed runtime behavior, or counters inserted on the wrong path
can still pass. The suite should reparse every emitted fixture and execute
representative JavaScript and stripped TypeScript while asserting observable
results and runtime coverage counters.

## Current state

- `real_world_test.rs:12-33` is the only shared validation helper:

```rust
fn assert_valid_instrumentation(source: &str, filename: &str) {
    let result = instrument(source, filename, &default_opts()).unwrap();
    assert!(!result.code.is_empty());
    assert_eq!(result.coverage_map.s.len(), result.coverage_map.statement_map.len());
    let json = serde_json::to_string(&result.coverage_map).unwrap();
    let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
}
```

- The React, Express, TypeScript service, and control-flow fixtures all call
  this helper without parsing or executing `result.code`.
- `scripts/istanbul-upstream-specs.mjs` is the repository exemplar for runtime
  execution: it builds the native binding, runs instrumented code in `node:vm`,
  and checks output plus statement, function, and branch counts.
- `scripts/real-world-parity.mjs` checks map cardinality across a cached corpus,
  but does not execute emitted code.
- The instrument crate already depends on Oxc parser and source-type crates;
  use them in Rust tests instead of adding a parser dependency.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Rust real-world tests | `cargo test -p oxc_coverage_instrument --test real_world_test` | exits 0 |
| N-API build | `npm --prefix crates/oxc-coverage-instrument-napi run build:debug` | exits 0 |
| Runtime behavior script | `node scripts/real-world-output.mjs` | exits 0 |
| Existing upstream runtime cases | `node scripts/istanbul-upstream-specs.mjs` | exits 0 |
| Full suite | `cargo test --workspace --all-targets` | exits 0 |

## Scope

**In scope**:

- `crates/oxc-coverage-instrument/tests/real_world_test.rs`
- `scripts/real-world-output.mjs` (create)
- `.github/workflows/ci.yml` to execute the new script in the existing N-API
  job after the binding is built
- `CONTRIBUTING.md` to list the new helper script

**Out of scope**:

- Changing instrumentation behavior to make a new test pass without a separate
  diagnosed bug and minimal regression.
- Executing JSX directly in Node. JSX should be reparsed with JSX source type;
  runtime execution belongs to plain JS or stripped TS fixtures.
- Adding snapshot-only coverage checks.
- Downloading a new fixture corpus.

## Git workflow

- Branch: `codex/004-validate-emitted-behavior`
- Commit: `test(instrument): validate emitted runtime behavior`
- Use `git commit -S`. Do not push or open a PR unless instructed.

## Steps

### Step 1: Reparse every production-like result

Refactor `assert_valid_instrumentation` to return `InstrumentResult` and parse
`result.code` with Oxc using `SourceType::from_path(filename)`. Assert parser
diagnostics are empty and include the filename plus rendered diagnostics in a
failure message. Preserve all current map-invariant assertions.

For TypeScript and JSX fixtures, use the matching source type. Do not force
plain JavaScript parsing on syntax the fixture intentionally preserves.

**Verify**: `cargo test -p oxc_coverage_instrument --test real_world_test`
exits 0.

### Step 2: Add deterministic runtime fixtures

Create `scripts/real-world-output.mjs`, modeled on
`scripts/istanbul-upstream-specs.mjs`. Use the built N-API binding and `node:vm`.
Include:

- a plain-JavaScript control-flow case with success and failure paths
- optional chaining with tracking on and off
- a stripped TypeScript service using `stripTypescript: true`
- one async function case

For every case, assert the original return value or side effect, then assert
selected runtime statement, function, and branch counters by location or stable
id discovered from the returned coverage map. Do not assert only total map
sizes.

**Verify**: before finalizing assertions, deliberately perturb one expected
counter and confirm the script exits nonzero, then restore it and confirm exit 0.

### Step 3: Wire the runtime check into CI

In the existing N-API job, run `node scripts/real-world-output.mjs` after the
native binding build and before or next to the upstream Istanbul script. Do not
create another job that rebuilds the same binding.

**Verify**:

```bash
actionlint .github/workflows/*.yml
zizmor --config .github/zizmor.yml --min-confidence medium --format plain .github/
```

Both commands exit 0.

### Step 4: Run real-project and full-suite validation

Use the checked-in Vitest TypeScript example as the real-project smoke:

```bash
npm --prefix crates/oxc-coverage-instrument-napi ci
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
node scripts/real-world-output.mjs
node scripts/istanbul-upstream-specs.mjs
npm --prefix examples/vitest-typescript ci
npm --prefix examples/vitest-typescript run coverage
npm --prefix examples/vitest-typescript run verify
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

**Verify**: every command exits 0.

## Test plan

- Reparse all existing production-like outputs with the correct source type.
- Execute controlled JS, optional-chain, async, and stripped-TS cases.
- Assert behavior plus specific runtime counters.
- Run the existing upstream runtime compatibility script.
- Run the Vitest TypeScript example as real-project validation.

## Done criteria

- [ ] Every fixture in `real_world_test.rs` reparses without diagnostics.
- [ ] Runtime tests prove original behavior and selected counters.
- [ ] The runtime script is part of the existing N-API CI job.
- [ ] The Vitest TypeScript example passes.
- [ ] Full format, clippy, and workspace tests pass.
- [ ] Only in-scope files and `plans/README.md` are modified.

## STOP conditions

Stop and report if:

- Oxc cannot parse its own generated output for an existing fixture.
- A runtime failure reveals a production bug. Isolate that bug in a new test
  and report it before changing implementation.
- Node execution would require network access or third-party application code.
- CI would need a second native binding build.
- A verification fails twice after a reasonable correction.

## Maintenance notes

New syntax features should add both structural parsing coverage and a runtime
case when Node can execute the emitted form. Keep runtime assertions behavioral
and location-based so refactors can renumber internal ids safely.
