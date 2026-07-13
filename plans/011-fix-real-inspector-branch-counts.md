# Plan 011: Preserve branch counts when V8 omits inherited arm ranges

> **Executor instructions**: Execute this plan test first. Keep the real Node
> inspector smoke from plan 005 as the behavior-facing contract. Update this
> plan and its row in `plans/README.md` only after every verification passes.
>
> **Drift check (run first)**: `git diff --stat 28bbd69..HEAD -- crates/oxc-coverage-v8/src/lib.rs crates/oxc-coverage-instrument/src/transform.rs crates/oxc-coverage-instrument/tests/v8_to_istanbul_test.rs scripts/v8-inspector-smoke.mjs`
> Stop if another change already alters branch-arm range inheritance.

## Status

- **Status**: DONE
- **Priority**: P1
- **Effort**: S
- **Risk**: MED
- **Depends on**: plan 005
- **Category**: correctness
- **Planned at**: commit `28bbd69`, 2026-07-13

## Why this matters

Plan 005 exposed a production defect with actual
`Profiler.takePreciseCoverage` output. When every call takes the same `if` arm,
V8 omits a child range for that arm because its count is inherited from the
enclosing function range. The converter currently requires a tight arm range,
so an executed arm is reported as zero and coverage thresholds can fail
incorrectly.

## RED evidence

Node v22.22.1 reports the inline function as:

```json
{
  "functionName": "repeated",
  "ranges": [
    { "startOffset": 25, "endOffset": 163, "count": 2 },
    { "startOffset": 141, "endOffset": 161, "count": 0 }
  ],
  "isBlockCoverage": true
}
```

The first range is the enclosing function. The second is the untaken else arm.
There is no separate then-arm range because that region inherits count `2`.
Passing this record unchanged to `v8ToIstanbul` currently returns `[0, 0]`
instead of `[2, 0]`. The failure is captured in
`/tmp/oxc-plan-005-green-initial.log`.

Additional Node captures show that mixed calls emit the then block range at its
block span, but emit the alternate range from the consequent end through the
alternate end. The latter includes the `else` transition rather than matching
the alternate `BlockStatement` span. Evidence for all-then, mixed, and all-else
calls is captured in `/tmp/oxc-plan-005-range-cases.log`.

## Root cause

`CoverageContext::best_arm_range_count` accepts only a range whose boundaries
match the stored arm body within tolerance. Two assumptions are incomplete for
V8's nested range encoding. A source region without a child range inherits the
count of its smallest enclosing range, and an explicit alternate range starts
at the consequent end rather than the alternate block start. The current
fallback to zero loses inherited counts, while the alternate side-table span
misses real explicit ranges.

The fallback must be limited to concrete `if` bodies. Expression branches such
as ternaries intentionally stay at zero when no tight range exists, because the
available enclosing count cannot distinguish their arms. Synthetic else arms
must also stay at zero. The winning enclosing range must belong to a
`FunctionCoverage` record with `isBlockCoverage: true`; function-only coverage
cannot prove either arm.

A zero-width synthetic else anchor must return zero before tolerance matching.
Otherwise a short adjacent then range can be close enough to lend its nonzero
count to the synthetic arm.

Function outer ranges are not branch ranges. Nested function declarations can
sit inside or exactly equal an if arm, so their call counts must not participate
in tight arm matching. Inheritance must also require strict containment to skip
an outer range that is exactly the arm itself.

## Scope

**In scope**:

- `crates/oxc-coverage-v8/src/lib.rs`
- `crates/oxc-coverage-instrument/src/transform.rs`
- `crates/oxc-coverage-instrument/tests/v8_to_istanbul_test.rs`
- `scripts/v8-inspector-smoke.mjs` from plan 005
- plan 005 CI and contributor documentation wiring
- `plans/005-test-real-v8-inspector-output.md`
- `plans/011-fix-real-inspector-branch-counts.md`
- `plans/README.md`

**Out of scope**:

- Normalizing or rewriting inspector records before conversion.
- Falling back to enclosing counts for ternary or logical-expression arms.
- Inferring arm counts from `isBlockCoverage: false` records.
- Changing raw range ordering or adding range snapshots.
- Redesigning the V8 lookup architecture.

## Implementation steps

### Step 1: Pin V8's inherited-count shape in Rust

Add focused converter regressions using real inspector-shaped ranges:

- enclosing function count `2` plus only an untaken else range `0` must
  produce `[2, 0]`
- enclosing function count `2` plus only an untaken then range `0` must
  produce `[0, 2]`
- enclosing function count `3` plus real then and alternate ranges must
  produce `[2, 1]`
- a function-only enclosing range must leave both arms at zero
- nested function outer ranges must not become braced or Annex B arm counts

Run `cargo test -p oxc_coverage_instrument --test v8_to_istanbul_test` and
confirm both new assertions fail before production code changes.

### Step 2: Implement the narrow inheritance fallback

Store a real alternate's V8 span from the consequent end through the alternate
end. Keep exact tight-range matching first. When no tight range matches, allow
a concrete `if` arm body to inherit the smallest enclosing range count only
when that winning range's record has `isBlockCoverage: true`. Do not apply this
fallback to expression branches or zero-width synthetic arms.

Run the focused Rust test and `node scripts/v8-inspector-smoke.mjs`. Both must
pass without weakening the expected branch counts.

### Step 3: Complete plan 005 integration

Wire the real inspector smoke into the existing Node 22 N-API job after the
binding build. Add the helper to `CONTRIBUTING.md`. Preserve the existing
typecheck and production-like runtime steps.

### Step 4: Verify repository behavior

Run:

```bash
npm --prefix crates/oxc-coverage-instrument-napi ci
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
node scripts/v8-inspector-smoke.mjs
node crates/oxc-coverage-instrument-napi/test.mjs
cargo test -p oxc_coverage_v8
cargo test -p oxc_coverage_instrument --test v8_to_istanbul_test
npm --prefix examples/vitest-typescript ci
npm --prefix examples/vitest-typescript run coverage
npm --prefix examples/vitest-typescript run verify
actionlint .github/workflows/*.yml
zizmor --config .github/zizmor.yml --min-confidence medium --format plain .github/
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Every command must exit zero. Mark plans 005 and 011 `DONE` only afterward.

## Done criteria

- [x] Real inspector records remain unchanged at the N-API boundary.
- [x] Always-taken and always-untaken concrete if arms inherit correct counts.
- [x] Ternary and synthetic else under-reporting behavior is unchanged.
- [x] The real inspector and checked-in repository fixture smokes pass.
- [x] Existing N-API, real-project, workflow, security, and Rust gates pass.

## STOP conditions

Stop and report before a fourth hypothesis if three distinct fixes fail. Stop
earlier if the correction requires a V8 range architecture redesign or weakens
expression-branch under-reporting guarantees.
