# Plan 006: Make V8 containment lookup scale sublinearly on valid range trees

> **Executor instructions**: Execute only after Plans 005, 011, and 012 are DONE. Measure before
> changing code and retain only a measured win. Run every verification gate and
> stop on any STOP condition. Update `plans/README.md` when complete.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- crates/oxc-coverage-v8/src/lib.rs crates/oxc-coverage-v8/benches/apply.rs crates/oxc-coverage-v8/tests/api_edges.rs scripts/v8-inspector-smoke.mjs`
> Plans 005, 011, and 012 are expected to change inspector validation, branch
> semantics, and coordinate normalization. Stop if any plan is not DONE or its
> contract is absent. Do not stop merely because `lib.rs` changed as planned.

## Status

- **Status**: CLOSED: NO SAFE MEASURED WIN
- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: `plans/005-test-real-v8-inspector-output.md`, `plans/011-fix-real-inspector-branch-counts.md`, `plans/012-normalize-v8-utf16-offsets.md`
- **Category**: perf
- **Planned at**: commit `321630c`, 2026-07-13

### Execution outcome

The production index was rejected after three measured rounds against the
original linear baseline:

1. An unconditional laminar index improved the largest valid cases, but
   regressed small valid cases, non-laminar fallback, and branch-heavy input.
2. A guarded index added a measured cardinality policy and adjacent-overlap
   preflight, but the preflight still regressed ineligible inputs.
3. A lazy two-stage gate removed ineligible preflight work, but linear-path
   dispatch and fallback cases still exceeded the 5 percent regression limit.

All production and oracle-test changes were reverted. Stable nested,
disjoint, and non-laminar scaling benchmarks remain so a future structurally
different approach can be measured against the same workload. The indexed
lookup done criteria below remain intentionally unmet.

### Local measurement record

The complete original baseline and all three rejected candidate rounds are
recorded as machine-readable evidence in
[`evidence/006-v8-range-index-results.json`](evidence/006-v8-range-index-results.json).
It preserves the exact local Criterion means and recorded deltas, candidate
algorithms and policies, revert decisions, and the limits of the available
evidence.

The retained scaling ids are stable across the original baseline and all three
candidate rounds:

```text
v8_apply_scaling/nested/64
v8_apply_scaling/nested/256
v8_apply_scaling/nested/1024
v8_apply_scaling/disjoint/64
v8_apply_scaling/disjoint/256
v8_apply_scaling/disjoint/1024
v8_apply_scaling/non_laminar/64
v8_apply_scaling/non_laminar/256
v8_apply_scaling/non_laminar/1024
```

Source, coverage, function records, and the empty branch-span map are built
outside the measured routine. Criterion `iter_batched` prepares each mutable
coverage clone outside timing, with `LargeInput` used for the largest fixtures
to bound setup memory. The measured routine contains only
`apply_v8_coverage`. Nested fixtures use bounded-depth containment groups,
disjoint fixtures use separate spans, and non-laminar fixtures use crossing
pairs.

The original width-sorted linear implementation produced these local Criterion
means:

| Benchmark | Baseline mean |
|---|---:|
| `v8_apply/ranges/ascii` | 280.40 us |
| `v8_apply/ranges/unicode` | 326.48 us |
| `v8_apply/branches/dense` | 1267.30 us |
| `v8_apply_scaling/nested/64` | 7.19 us |
| `v8_apply_scaling/nested/256` | 45.59 us |
| `v8_apply_scaling/nested/1024` | 341.20 us |
| `v8_apply_scaling/disjoint/64` | 5.86 us |
| `v8_apply_scaling/disjoint/256` | 32.26 us |
| `v8_apply_scaling/disjoint/1024` | 247.62 us |
| `v8_apply_scaling/non_laminar/64` | 6.02 us |
| `v8_apply_scaling/non_laminar/256` | 33.81 us |
| `v8_apply_scaling/non_laminar/1024` | 267.38 us |

Each candidate improved at least one large valid fixture, but each also crossed
the rejection threshold on a required workload. The decisive deltas from the
original baseline were:

| Round | Benchmark | Candidate mean | Delta |
|---|---|---:|---:|
| 1 | `v8_apply/branches/dense` | 5120.80 us | +304.1% |
| 1 | `v8_apply_scaling/nested/64` | 9.19 us | +27.8% |
| 1 | `v8_apply_scaling/disjoint/256` | 46.07 us | +42.8% |
| 1 | `v8_apply_scaling/non_laminar/64` | 8.93 us | +48.3% |
| 1 | `v8_apply_scaling/non_laminar/256` | 56.51 us | +67.1% |
| 1 | `v8_apply_scaling/non_laminar/1024` | 307.91 us | +15.2% |
| 2 | `v8_apply/branches/dense` | 1846.50 us | +45.7% |
| 2 | `v8_apply/ranges/unicode` | 449.75 us | +37.8% |
| 2 | `v8_apply_scaling/nested/64` | 9.03 us | +25.6% |
| 2 | `v8_apply_scaling/non_laminar/256` | 35.98 us | +6.4% |
| 3 | `v8_apply_scaling/disjoint/256` | 35.06 us | +8.7% |
| 3 | `v8_apply_scaling/non_laminar/256` | 37.98 us | +12.3% |
| 3 | `v8_apply_scaling/non_laminar/1024` | 293.13 us | +9.6% |

For context, the large nested and disjoint cases improved by 12.0% and 41.8%
in round 1, 50.6% and 48.3% in round 2, and 54.5% and 50.1% in round 3. Those
wins did not offset the required regression failures above.

The result parser produced every existing and new benchmark key for the
baseline and each round. Parsed means matched the middle value from the raw
Criterion timing interval. Local `cargo codspeed run -m simulation apply`
discovered every retained benchmark but reported that the environment cannot
measure performance. Local timing therefore used the same Criterion2 source
with the CodSpeed adapter temporarily disabled, and the manifest was restored
after each run.

These deltas compare local mean values with the original baseline. The
available record does not contain direct original-baseline significance tests,
candidate commit SHAs, a remote CodSpeed comparison, or durable run links, so
none are claimed here. A future production attempt still requires the final
remote aggregate CodSpeed comparison.

Before closeout, the benchmark release build, CodSpeed simulation build and
discovery, formatting, strict clippy, workspace tests, and real inspector smoke
all passed. The production index and its oracle tests were then removed, while
the benchmark-only coverage remained.

## Why this matters

Each statement and function currently scans the entire width-sorted V8 range
list until it finds the narrowest containing range. With dense block coverage,
conversion work grows with coverage entries multiplied by V8 ranges. Valid V8
ranges are normally nested or disjoint, which allows an indexed query, but the
implementation must preserve exact smallest-container and tie behavior and
must safely fall back for malformed or partially overlapping input.

## Current state

- `apply_v8_coverage_inner` builds one source coordinate index, keeps inspector
  ranges in native UTF-16 units, sorts one flattened copy by width, partitions
  child block ranges for tight branch-arm queries, and retains outer range mode
  and provenance for inheritance.
- `apply_statement_counts` and `apply_function_counts` call
  `CoverageContext::count_for_location` for every entry.
- `smallest_containing_range_count` is the current correctness oracle:

```rust
for r in ranges {
    if r.start_offset <= start && r.end_offset >= end {
        return r.count;
    }
}
0
```

- `benches/apply.rs` already has dense ASCII, Unicode, and branch fixtures.
  Extend this harness instead of adding another benchmark framework.
- Plans 005, 011, and 012 provide real inspector validation, branch semantics,
  and UTF-16 normalization and must land first.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Focused tests | `cargo test -p oxc_coverage_v8` | exits 0 |
| Benchmark build | `cargo build --release -p oxc_coverage_v8 --bench apply --features codspeed` | exits 0 |
| CodSpeed discovery | `cargo codspeed run -m simulation apply` | benchmark is discovered; local timing may be unsupported |
| Full suite | `cargo test --workspace --all-targets` | exits 0 |
| Inspector smoke | `node scripts/v8-inspector-smoke.mjs` | exits 0 |

## Scope

**In scope**:

- `crates/oxc-coverage-v8/src/lib.rs`
- `crates/oxc-coverage-v8/benches/apply.rs`
- `crates/oxc-coverage-v8/tests/api_edges.rs`

**Out of scope**:

- Branch-arm tolerance lookup, already start-indexed.
- Changing wrapper-base or UTF-16 normalization behavior.
- Changing child block-range partitioning, outer inheritance provenance,
  block-coverage mode, strict containment, equal-span exclusion, or zero-width
  rejection.
- New dependencies.
- Keeping a rewrite that does not produce a measured CodSpeed improvement.

## Git workflow

- Branch: `codex/006-index-v8-range-lookups`
- Commit: `perf(v8): index containment lookups`
- Use `git commit -S`. Do not push or open a PR unless instructed.

## Steps

### Step 1: Add scaling benchmarks and capture baseline

Extend `v8_apply` with stable benchmark ids for increasing dense range sizes and
for three shapes:

- properly nested ranges
- disjoint ranges
- partially overlapping ranges that are not a valid nesting tree

Keep fixture construction outside `b.iter`. Record the current CodSpeed result
or CI comparison for the largest valid cases. Local simulation is discovery
only when the platform lacks the Valgrind executor.

**Verify**: benchmark binaries build and every new id is discovered.

### Step 2: Preserve the linear function as a test oracle

Keep a test-only linear implementation with the current width-sort and stable
tie behavior. Add deterministic generated cases covering nested, disjoint,
duplicate-span, zero-width, wrapper-shifted, and partially overlapping ranges.
For each query, assert the new index returns exactly the oracle count.

All queries and indexed ranges are absolute UTF-16 offsets. Normalize source
coordinates before the index and never convert an indexed result again.

**Verify**: the new equivalence tests fail before the index exists and pass
after it is wired.

### Step 3: Build a laminar range index with safe fallback

Implement a private `RangeIndex` without dependencies.

- Preserve original flattened order for equal-width ties.
- Sort by start ascending, end descending, and original order.
- Build a containment forest with a stack when intervals are nested or
  disjoint.
- Mark the index non-laminar when intervals partially overlap without
  containment.
- For laminar input, find the deepest containing child using binary search over
  disjoint children. The deepest node is the narrowest containing range.
- For non-laminar input, call the preserved width-sorted linear oracle.

Do not assume all caller-provided JSON is valid V8 output. The fallback is part
of the correctness contract.

**Verify**: property-style equivalence cases pass for every range shape.

### Step 4: Use the index for statements and functions

Construct `RangeIndex` once in `apply_v8_coverage_inner` and replace only
`count_for_location`'s statement/function lookup. Leave child-only
`arm_ranges`, `best_arm_range_count`, and provenance-bearing inheritance
ranges unchanged. Preserve block-coverage mode, strict parent containment,
equal-span exclusion, and zero-width body rejection exactly.

Avoid cloning the full range vector more than the branch path still requires.
Keep code single-purpose and document the laminar invariant and fallback.

**Verify**:

```bash
cargo test -p oxc_coverage_v8
node scripts/v8-inspector-smoke.mjs
```

Both commands exit 0.

### Step 5: Measure and retain only a win

Run the same CodSpeed benchmark ids before and after. Require a clear improvement
on the largest nested and disjoint cases, no meaningful regression on small
cases, and no regression on branch-heavy conversion. Verify the aggregate
CodSpeed analysis when CI is available, not only shard completion.

If results are neutral or worse, revert the production index and keep only the
useful benchmark coverage.

**Verify**: attach or record the benchmark comparison in the execution report.

### Step 6: Run full gates

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
node scripts/v8-inspector-smoke.mjs
```

**Verify**: every command exits 0.

## Test plan

- Exact equivalence with the old linear oracle.
- Nested, disjoint, duplicate, zero-width, non-laminar, Unicode, and wrapper
  cases.
- Real Node inspector output from Plans 005, 011, and 012.
- UTF-16 normalization before indexing, including astral, BMP, CRLF, and
  explicit shifted-producer wrapper-base cases.
- Child block-range partitioning, inheritance provenance, block mode, strict
  containment, equal-span exclusion, and zero-width rejection.
- Scaling benchmarks with fixed ids and sizes.
- Full workspace regression suite.

## Done criteria

- [ ] Valid nested/disjoint V8 ranges use an indexed lookup.
- [ ] Non-laminar input falls back to exact old semantics.
- [ ] Equivalence tests cover all named range shapes.
- [ ] Real inspector smoke passes unchanged.
- [ ] CodSpeed shows a clear large-input improvement and no meaningful
  regression.
- [ ] Full format, clippy, and workspace tests pass.
- [ ] Only in-scope files and `plans/README.md` are modified.

## STOP conditions

Stop and report if:

- Plan 005 is not complete.
- Plan 011 or Plan 012 is not complete.
- Real inspector ranges violate the proposed laminar model and make fallback
  the common path.
- Equal-span tie behavior differs from the linear oracle.
- The optimization is not a measured win.
- Correctness requires changing branch-arm lookup semantics.
- Correctness requires moving UTF-16 normalization after indexed lookup.
- A verification fails twice after a reasonable correction.

## Maintenance notes

Future V8 protocol changes may introduce new overlap shapes. Keep the fallback
and oracle tests even if current inspector output is always laminar. Reviewers
should reject micro-optimizations that weaken exact count equivalence.
