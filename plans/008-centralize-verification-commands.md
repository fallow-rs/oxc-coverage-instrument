# Plan 008: Centralize local and CI verification commands

> **Executor instructions**: Preserve every existing gate and CI job boundary.
> First map commands to profiles, then migrate callers one at a time. Run all
> verification below and update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- CONTRIBUTING.md .githooks/pre-push .github/workflows/ci.yml package.json scripts`
> Rebuild the command matrix if any of these surfaces changed. Stop if a command
> cannot be reproduced without changing its working directory or environment.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MEDIUM
- **Depends on**: none
- **Category**: DX
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

Verification commands are copied across contributor docs, the pre-push hook,
package scripts, and CI. The documented full check omits some CI-only gates,
while the hook deliberately runs only a fast subset. Contributors have no one
repository-owned entrypoint that states exactly what ran, what prerequisites
are missing, and what remains CI-only. Copying commands also creates drift.

## Current state

`CONTRIBUTING.md` currently gives a long inline command chain:

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --all-targets && npm --prefix crates/oxc-coverage-instrument-napi run build:debug && node scripts/istanbul-diff.mjs && node crates/oxc-coverage-instrument-napi/test.mjs && typos
```

`.githooks/pre-push` independently runs format, clippy, optional typos, and
optional workspace tests. `.github/workflows/ci.yml` repeats Rust commands and
also owns version synchronization, audit, deny, shear, N-API, Istanbul, Vitest,
WASM, and platform matrix jobs. Some jobs prepare artifacts before the command
and cannot be collapsed into a generic local call.

## Commands you will need

```bash
sed -n '1,220p' CONTRIBUTING.md
sed -n '1,180p' .githooks/pre-push
rg -n "run:|working-directory:|uses:" .github/workflows/ci.yml
rg -n '"scripts"|istanbul|test|build' package.json crates/oxc-coverage-instrument-napi/package.json
```

## Scope

### In scope

- New `scripts/check.sh` with primitive and aggregate profiles.
- `CONTRIBUTING.md` verification instructions.
- `.githooks/pre-push` delegation to the fast profile.
- `.github/workflows/ci.yml` delegation where job setup already satisfies a
  profile's prerequisites.
- A machine-readable profile listing or self-test that detects documentation
  drift without executing expensive checks twice.

### Out of scope

- Collapsing CI into one job.
- Removing platform matrices, WASM checks, Codecov, CodSpeed, or release gates.
- Replacing actions such as `cargo-deny-action` merely for uniformity.
- Installing tools or dependencies from `scripts/check.sh`.
- Silently skipping a profile requested directly by a user or CI.
- Adding a third-party task runner.

## Git workflow

- Branch: `codex/008-centralize-verification`
- Commit: `chore: centralize verification commands`
- Use a signed commit.

## Steps

### Step 1: Create a complete command matrix

Inventory every command from the drift-check surfaces. For each command record:

- Current caller and working directory.
- Required tools, dependencies, features, and generated artifacts.
- Whether it is host-reproducible or CI-only.
- The primitive profile that will own it, or why it stays action-owned.

At minimum define primitives for `fmt`, `clippy`, `rust-test`, `doc-test`,
`typos`, `version-sync`, `napi-test`, `istanbul-diff`, `package-surface`,
`audit`, and `shear`. Keep `cargo deny` action-owned unless command parity is
proven. Define aggregates `rust`, `pre-push`, and `all-local`.

### Step 2: Implement the profile runner

Create executable `scripts/check.sh` with `set -euo pipefail`, a usage function,
one function per primitive, aggregate functions, and a final `case` statement.
Resolve the repository root from the script location so invocation directory
does not change behavior.

Every primitive must:

- Print the exact check it is starting.
- Check required tools and artifacts before invoking the command.
- Exit nonzero with an actionable message when a direct request cannot run.
- Preserve the current command, working directory, flags, and environment.

`all-local` must run every host-reproducible primitive and print a final list of
matrix, WASM, coverage upload, and action-owned CI checks it cannot reproduce.
It must not describe those checks as passed.

### Step 3: Migrate the pre-push hook without weakening it

Keep branch-deletion handling, `SKIP_PRE_PUSH`, and `RUN_TESTS` behavior in the
hook. Delegate its check body to `scripts/check.sh pre-push`. Preserve the
current optional-tool behavior for pre-push only, but print every skip. Direct
`typos` and `all-local` profiles must still fail when the tool is missing.

Add a shell-level regression test or test mode that proves:

- Unknown profiles fail.
- Direct missing-tool requests fail.
- The pre-push aggregate reports its intentional optional skip.
- Invocation outside the repository root still resolves paths correctly.

### Step 4: Migrate documentation and safe CI callers

Replace the long contributor command chain with `./scripts/check.sh all-local`.
Document prerequisites and list CI-only gates separately.

In CI, replace only command steps whose environment already matches a primitive
profile. Good first candidates are Rust test, doc-test, clippy, format, version
sync, N-API test, and Istanbul diff. Preserve step names, job names, matrices,
permissions, setup, caching, artifact upload, and failure semantics. Leave an
action-owned gate unchanged when wrapping it adds no shared command value.

### Step 5: Prove command equivalence

Compare the command matrix with the final diff. Every old command must have a
profile or an explicit action-owned or CI-only classification. Run each
host-reproducible profile after preparing its documented artifacts.

## Test plan

```bash
bash -n scripts/check.sh .githooks/pre-push
./scripts/check.sh --list
if ./scripts/check.sh unknown-profile >/tmp/oxc-check-unknown.log 2>&1; then exit 1; fi
./scripts/check.sh version-sync
./scripts/check.sh rust
./scripts/check.sh package-surface
./scripts/check.sh audit
./scripts/check.sh shear
actionlint .github/workflows/ci.yml
zizmor --config .github/zizmor.yml --min-confidence medium --format plain .github/
git diff --check
```

After the documented N-API build, also run:

```bash
./scripts/check.sh napi-test
./scripts/check.sh istanbul-diff
./scripts/check.sh all-local
```

Redirect verbose build and test output to bounded log files. Inspect the tail
and targeted failure lines. Confirm all profiles pass and the final CI-only
list is accurate.

## Done criteria

- Every previous verification command has one profile or explicit CI-only or
  action-owned classification.
- Local docs, pre-push, and suitable CI steps share the script.
- CI job boundaries, matrices, permissions, setup, caches, and artifacts remain
  unchanged.
- Direct requested profiles fail clearly on missing prerequisites.
- Unknown profiles fail with usage output.
- All host-reproducible profiles and workflow checks pass.

## STOP conditions

- Wrapping a command changes its working directory, environment, features, or
  artifact inputs.
- A profile would download dependencies or regenerate source unexpectedly.
- CI-only setup cannot be represented without broad workflow restructuring.
- A migrated caller silently runs fewer checks than before.

## Maintenance notes

New verification gates should receive a primitive profile when they expose a
portable command. Otherwise classify them explicitly as action-owned or
CI-only. Docs, hooks, and CI should call profiles instead of copying commands.
