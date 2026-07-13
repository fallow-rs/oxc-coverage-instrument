# Plan 012: Normalize V8 inspector UTF-16 offsets

> **Executor instructions**: Preserve the real Node inspector regression while
> implementing this plan. Use one internal coordinate representation for every
> containment query. Run every verification gate and stop on any STOP
> condition. Update `plans/README.md` when complete.
>
> **Drift check (run first)**: `git diff --stat ac4ecf5..HEAD -- crates/oxc-coverage-v8/src/lib.rs crates/oxc-coverage-v8/benches/apply.rs crates/oxc-coverage-instrument/src/v8_to_istanbul.rs crates/oxc-coverage-instrument/tests/v8_to_istanbul_test.rs crates/oxc-coverage-instrument-napi/src/lib.rs crates/oxc-coverage-instrument-napi/index.d.ts scripts/v8-inspector-smoke.mjs`
> Plans 005 and 011 are expected to be DONE. Stop if their real inspector or
> branch inheritance contracts are absent.

## Status

- **Status**: DONE
- **Priority**: P1
- **Effort**: M
- **Risk**: HIGH
- **Depends on**: `plans/005-test-real-v8-inspector-output.md`, `plans/011-fix-real-inspector-branch-counts.md`
- **Category**: correctness
- **Found at**: commit `ac4ecf5`, 2026-07-13

## Why this matters

Node inspector reports `startOffset` and `endOffset` as absolute UTF-16 code
units. The converter treated those values as UTF-8 byte offsets. ASCII source
masked the mismatch, but astral characters and BMP multibyte characters before
a function shifted every later comparison. Function counts could fall back to
an outer count, and if-arm counts could inherit the wrong parent count.

## Coordinate contract

| Surface | Unit | Required handling |
| --- | --- | --- |
| Inspector ranges | Absolute UTF-16 code units | Keep unchanged and half-open. |
| Istanbul locations | 1-based line, 0-based UTF-16 column | Add the exact absolute UTF-16 line start. |
| Oxc branch body spans | UTF-8 byte offsets | Convert to absolute UTF-16 once before range matching. |
| `wrapper_length` | UTF-16 code-unit base | Add only for producers that report shifted ranges. |
| Source-map generated coordinates | UTF-16 columns | Leave unchanged and attach maps after count matching. |
| CRLF | Two UTF-16 code units | Preserve both code units in absolute line starts. |

The default `wrapper_length` for source-relative Node inspector output is zero.
Nonzero values are an explicit compatibility base for other producers. The
converter does not infer or reconstruct wrapper text.

## Reproduction

The real inspector fixture places six astral characters and one BMP multibyte
character before `unicodeBranch`, calls it twice with a truthy value, and
captures `Profiler.takePreciseCoverage` unchanged. Node 22 reports:

```text
top-level:       0..232 count 1
unicodeBranch: 32..130 count 2
untaken block: 98..128 count 0
```

Before the fix, the JavaScript N-API smoke and the equivalent Rust integration
test reported function count 1 instead of 2. The JavaScript failure is recorded
in `/tmp/oxc-plan-012-node-red.log`; the Rust failure is recorded in
`/tmp/oxc-plan-012-rust-red.log`.

## Scope

**In scope**:

- `crates/oxc-coverage-v8/src/lib.rs`
- `crates/oxc-coverage-v8/benches/apply.rs`
- `crates/oxc-coverage-instrument/src/instrument.rs`
- `crates/oxc-coverage-instrument/src/v8_to_istanbul.rs`
- `crates/oxc-coverage-instrument/tests/v8_to_istanbul_test.rs`
- `crates/oxc-coverage-instrument-napi/src/lib.rs`
- `crates/oxc-coverage-instrument-napi/index.d.ts`
- `crates/oxc-coverage-instrument-napi/test.mjs`
- `scripts/v8-inspector-smoke.mjs`
- V8 conversion documentation and plans

**Out of scope**:

- Plan 006 range-index implementation
- Changing Plan 011 child-range and inheritance semantics
- Remapping Istanbul locations during V8 matching
- Inferring CommonJS wrapper source
- New dependencies

## Git workflow

- Branch: `codex/fix-all-improvements`
- Commit: `fix(v8): normalize inspector utf16 offsets`
- Use `git commit -S`. Do not push or open a PR unless instructed.

## Steps

### Step 1: Preserve real inspector RED

Extend `scripts/v8-inspector-smoke.mjs` with the Unicode source and assert the
real function count, `[taken, untaken]` if-arm counts, and branch vector to
location length equality. Capture the unmodified inspector function records.

**Verify**: the existing N-API binary fails the function-count assertion.

### Step 2: Add the equivalent Rust RED

Use the exact source and raw offsets captured from Node. Do not derive the core
regression offsets from Rust byte positions. Assert the function count, branch
counts, and branch vector length.

**Verify**: the focused integration test fails with function count 1.

### Step 3: Establish one source-coordinate index

Create a private index once per conversion. It must:

- compute absolute UTF-16 line starts without newline normalization
- convert valid Oxc UTF-8 byte boundaries to UTF-16 offsets
- preserve half-open end offsets
- clamp invalid interior byte positions to the containing character start
- use saturating arithmetic for untrusted `u32` offsets
- preserve `(0, 0)` unknown spans and zero-width synthetic spans

Keep inspector ranges in native UTF-16 units. Every statement, function, tight
arm, and inheritance query must compare values in that same unit.

### Step 4: Protect adjacent coordinate cases

Add behavioral cases for:

- BMP multibyte characters
- astral surrogate pairs
- Unicode on a prior line
- CRLF and mixed line endings
- nested function ranges inside an if arm
- inherited arm counts and tight zero-count ranges
- explicit shifted-producer wrapper bases
- source-map attachment without location mutation

Update stale Rust fixtures and Unicode benchmark input so V8 ranges are built
from UTF-16 lengths. Keep Oxc branch side spans in bytes.

### Step 5: Update the public contract

Replace byte-range claims in Rust docs, N-API declarations, README guidance,
and generated TypeScript declarations. State that Node inspector output is
normally source-relative and a nonzero wrapper value is an explicit UTF-16
base for shifted producers.

### Step 6: Run focused GREEN

Build the N-API package, then rerun the exact Rust regression and real Node
inspector smoke.

**Verify**: both paths report function count 2 and branch counts `[2, 0]`.

### Step 7: Run full gates

Run focused V8 and instrumenter tests, N-API runtime tests, Istanbul parity,
real Vitest coverage, workflow validation, format, clippy, docs, and the full
workspace suite. Redirect verbose output to bounded logs.

## Verification commands

```bash
node --version
cargo test -p oxc_coverage_v8
cargo test -p oxc_coverage_instrument --test v8_to_istanbul_test
npm run build --prefix crates/oxc-coverage-instrument-napi
node scripts/v8-inspector-smoke.mjs
node crates/oxc-coverage-instrument-napi/test.mjs
node scripts/istanbul-diff.mjs
node scripts/istanbul-upstream-specs.mjs
npm --prefix examples/vitest-typescript ci
npm --prefix examples/vitest-typescript run typecheck
npm --prefix examples/vitest-typescript run coverage
npm --prefix examples/vitest-typescript run verify
actionlint .github/workflows/*.yml
zizmor --config .github/zizmor.yml --min-confidence medium --format plain .github/
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
cargo test --workspace --all-targets
```

## Done criteria

- [x] Real Node inspector Unicode regression passes unchanged.
- [x] Exact raw inspector Rust regression passes.
- [x] Every containment consumer uses absolute UTF-16 offsets.
- [x] Branch body byte spans are converted exactly once.
- [x] Wrapper bases, CRLF, mixed newlines, Unicode forms, nested ranges, and source maps are covered.
- [x] Public docs and generated TypeScript declarations describe UTF-16 units.
- [x] Plan 006 preserves this coordinate boundary and Plan 011 semantics.
- [x] Every required verification gate passes.
- [x] The signed commit contains only intended files.

## STOP conditions

Stop and report if:

- The real inspector ranges differ from the captured source on the supported Node version.
- Correctness requires mutating Istanbul or source-map locations.
- A branch side span cannot be traced to an Oxc byte boundary.
- Nonzero wrapper behavior cannot be expressed as an explicit UTF-16 base.
- Plans 005 or 011 regress.
- Three distinct implementation hypotheses fail.

## Maintenance notes

Plan 006 must index normalized UTF-16 comparisons, not reintroduce byte-based
queries. Any future producer adapter must document whether its offsets are
source-relative or shifted and pass `wrapper_length` in UTF-16 code units.
