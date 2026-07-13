# Plan 005: Validate V8 conversion with real Node inspector output

> **Executor instructions**: Execute this plan step by step. Run every
> verification and stop on any STOP condition. Update the status row in
> `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- crates/oxc-coverage-v8/src/lib.rs crates/oxc-coverage-instrument/tests/v8_to_istanbul_test.rs crates/oxc-coverage-instrument-napi/test.mjs .github/workflows/ci.yml`
> Stop if the V8 JSON contract or N-API function names changed.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: tests
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

The converter promises to accept `Profiler.FunctionCoverage`, but every current
integration case hand-builds V8 ranges. Real inspector output includes a script
record, nested functions, wrapper or source offsets, and block ranges chosen by
the active Node engine. A Node-driven contract test is needed before changing
the V8 lookup algorithm and to catch protocol-shape drift across supported Node
releases.

## Current state

- `crates/oxc-coverage-v8/src/lib.rs:38-60` declares serde names identical to
  `Profiler.FunctionCoverage`.
- `v8_to_istanbul_test.rs:1-18` explicitly uses hand-crafted ranges.
- `crates/oxc-coverage-instrument-napi/test.mjs:829-940` also derives offsets
  from strings and constructs the range array itself.
- The CI N-API job already installs Node 22, builds the binding, and runs Node
  scripts. Put the inspector test there to avoid another build.
- Match existing Node test style: `node:assert/strict`, explicit error messages,
  no external test framework.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build binding | `npm --prefix crates/oxc-coverage-instrument-napi run build:debug` | exits 0 |
| Inspector contract | `node scripts/v8-inspector-smoke.mjs` | exits 0 |
| Existing N-API tests | `node crates/oxc-coverage-instrument-napi/test.mjs` | exits 0 |
| Rust V8 tests | `cargo test -p oxc_coverage_v8` | exits 0 |
| Full suite | `cargo test --workspace --all-targets` | exits 0 |

## Scope

**In scope**:

- `scripts/v8-inspector-smoke.mjs` (create)
- `.github/workflows/ci.yml`
- `CONTRIBUTING.md` to list the script
- `crates/oxc-coverage-instrument-napi/test.mjs` only if a small shared helper
  avoids duplication

**Out of scope**:

- Changing V8 conversion semantics.
- Full JSON snapshots of inspector output.
- Supporting unpinned experimental Node versions in one assertion set.
- Browser inspector protocols.

## Git workflow

- Branch: `codex/005-test-real-v8-inspector-output`
- Commit: `test(v8): cover node inspector output`
- Use `git commit -S`. Do not push or open a PR unless instructed.

## Steps

### Step 1: Capture precise coverage from Node

Create `scripts/v8-inspector-smoke.mjs` using `node:inspector` and a promisified
`Session.post`. The script must:

1. connect a session
2. enable `Profiler`
3. start precise coverage with `callCount: true` and `detailed: true`
4. execute a source string with a unique `//# sourceURL=` marker
5. take precise coverage
6. stop coverage, disable the profiler, and disconnect in `finally`
7. select the exact script by URL and pass its `functions` array unchanged to
   the N-API `v8ToIstanbul` function

The executed source should include a top-level statement, nested function, one
taken branch, one untaken branch, and repeated calls with different counts.

**Verify**: log only a concise PASS line. The script exits nonzero if the URL or
function records are absent.

### Step 2: Assert stable behavioral invariants

Parse the returned Istanbul object and assert:

- the requested filename is retained
- the executed statements and function have nonzero counts
- the deliberately untaken branch arm is zero
- the taken arm and repeated function count are correct
- every counter id has matching metadata

Do not assert raw range ordering, anonymous function names, or a full snapshot.
Those details may vary across Node patch releases without breaking the public
contract.

**Verify**: `node scripts/v8-inspector-smoke.mjs` exits 0 on the repository's
documented Node version.

### Step 3: Add an actual repository fixture smoke

Run the inspector flow on one checked-in, dependency-free JavaScript fixture,
preferably a conformance fixture or `examples/cloudflare-workers/src/fixture.js`.
Execute only local code in an isolated VM context. Assert conversion succeeds
and all metadata/counter invariants hold; avoid hardcoding engine-specific raw
ranges.

**Verify**: the script proves both the deterministic inline case and the
checked-in fixture case.

### Step 4: Wire the test into CI and run full gates

Add the script to the existing Node 22 N-API job after the binding build.

```bash
npm --prefix crates/oxc-coverage-instrument-napi ci
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
node scripts/v8-inspector-smoke.mjs
node crates/oxc-coverage-instrument-napi/test.mjs
actionlint .github/workflows/*.yml
zizmor --config .github/zizmor.yml --min-confidence medium --format plain .github/
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

**Verify**: every command exits 0.

## Test plan

- Real `Profiler.takePreciseCoverage` records, passed through unchanged.
- Top-level, nested function, taken branch, untaken branch, and repeated count.
- One checked-in repository fixture as real-project-like validation.
- Stable invariants only, no engine-internal snapshots.
- Guaranteed cleanup through `finally` so failures do not leave an inspector
  session running.

## Done criteria

- [ ] At least one test consumes actual Node inspector function records.
- [ ] The test asserts observable Istanbul counts and invariants.
- [ ] A checked-in fixture also passes through the inspector path.
- [ ] The test runs in the existing Node 22 CI job.
- [ ] Existing N-API and full workspace tests pass.
- [ ] Only in-scope files and `plans/README.md` are modified.

## STOP conditions

Stop and report if:

- Node 22 inspector output cannot be selected deterministically by source URL.
- The existing converter produces wrong counts from real inspector output.
  That is a separate production bug requiring its own minimal fix.
- Stable assertions require snapshotting raw range layout.
- The test leaves the inspector session active on failure.
- A verification fails twice after a reasonable correction.

## Maintenance notes

When CI changes its Node major, run this script first and adjust only assertions
that are proven engine-internal. Never normalize or rewrite the inspector
records before calling the public converter.
