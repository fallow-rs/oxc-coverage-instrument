# Plan 006: Make V8 containment lookup scale sublinearly on valid range trees

> **Executor instructions**: Execute only after Plan 005 is DONE. Measure before
> changing code and retain only a measured win. Run every verification gate and
> stop on any STOP condition. Update `plans/README.md` when complete.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- crates/oxc-coverage-v8/src/lib.rs crates/oxc-coverage-v8/benches/apply.rs crates/oxc-coverage-v8/tests/api_edges.rs scripts/v8-inspector-smoke.mjs`
> Plan 005 is expected to add the inspector script. Stop if Plan 005 is not DONE
> or the lookup semantics in `lib.rs` changed.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: `plans/005-test-real-v8-inspector-output.md`
- **Category**: perf
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

Each statement and function currently scans the entire width-sorted V8 range
list until it finds the narrowest containing range. With dense block coverage,
conversion work grows with coverage entries multiplied by V8 ranges. Valid V8
ranges are normally nested or disjoint, which allows an indexed query, but the
implementation must preserve exact smallest-container and tie behavior and
must safely fall back for malformed or partially overlapping input.

## Current state

- `apply_v8_coverage_inner` flattens every function range, sorts one copy by
  width, and clones another copy sorted by start for branch-arm queries.
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
- Plan 005 provides real Node inspector validation and must land first.

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
- Changing wrapper-offset or UTF-16 conversion behavior.
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
`count_for_location`'s statement/function lookup. Leave `arm_ranges` and
`best_arm_range_count` unchanged.

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
- Real Node inspector output from Plan 005.
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
- Real inspector ranges violate the proposed laminar model and make fallback
  the common path.
- Equal-span tie behavior differs from the linear oracle.
- The optimization is not a measured win.
- Correctness requires changing branch-arm lookup semantics.
- A verification fails twice after a reasonable correction.

## Maintenance notes

Future V8 protocol changes may introduce new overlap shapes. Keep the fallback
and oracle tests even if current inspector output is always laminar. Reviewers
should reject micro-optimizations that weaken exact count equivalence.
