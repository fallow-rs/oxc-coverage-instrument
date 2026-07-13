# Plan 010: Refresh roadmap API and benchmark contracts

> **Executor instructions**: Change only statements disproven by checked-in
> code or workflows. Preserve unfinished roadmap ideas and run documentation
> gates before updating this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- ROADMAP.md crates/oxc-coverage-reports/src/html/mod.rs crates/oxc-coverage-reports/src/lib.rs .github/workflows/bench.yml .github/workflows/ci.yml`
> Re-check every changed contract. Stop if public API or benchmark workflow is
> mid-migration and the checked-in state is not the intended steady state.

## Status

- **Status**: DONE
- **Resolution note**: `bench.yml` now documents CodSpeed simulation, while
  `bloat.yml` retains its separately verified `gh-pages` size history.
- **Priority**: P3
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: docs
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

`ROADMAP.md` describes superseded HTML entrypoints and the benchmark system
that preceded CodSpeed. It also presents volatile test and coverage totals as a
durable project milestone. These claims misdirect API users and contributors,
and they make otherwise completed roadmap sections look unmaintained.

## Current state

The roadmap currently describes an API that does not exist:

```text
HtmlOptions { high_threshold: f64 }
html::write_with_options()
Format::write_to_dir_with_options()
```

The checked-in API instead has a private `green_threshold` field, validates
construction through `HtmlOptions::new`, exposes `green_threshold()`, and passes
`&HtmlOptions` through the existing `html::write` and `Format::write_to_dir`
entrypoints.

The roadmap also says `bench.yml` uses `github-action-benchmark`, pushes a
dashboard to `gh-pages`, and serves that dashboard from GitHub Pages. The active
workflow instead installs `cargo-codspeed`, builds benchmark shards, and runs
`CodSpeedHQ/action` in simulation mode.

Finally, the historical capability list includes exact automated test and line
coverage totals. Those values drift independently of the durable claim that the
project has broad tests, conformance checks, and strict linting.

## Commands you will need

```bash
sed -n '1,130p' ROADMAP.md
sed -n '115,210p' crates/oxc-coverage-reports/src/html/mod.rs
sed -n '140,180p' crates/oxc-coverage-reports/src/lib.rs
sed -n '1,130p' .github/workflows/bench.yml
rg -n "high_threshold|write_with_options|write_to_dir_with_options|github-action-benchmark|gh-pages|tests|coverage" README.md ROADMAP.md crates
```

## Scope

### In scope

- Stale HTML option and writer descriptions in `ROADMAP.md`.
- Stale benchmark workflow and dashboard descriptions in `ROADMAP.md`.
- Volatile historical test and coverage totals in `ROADMAP.md`.
- The same stale contract elsewhere in public docs only if the search finds it.

### Out of scope

- Public API, benchmark code, CI, or workflow changes.
- New roadmap promises, dates, or commitments.
- Rewriting unrelated roadmap prose.
- Removing future directions that remain genuinely unfinished.
- Refreshing unrelated volatile workflow version pins or fixture totals.

## Git workflow

- Branch: `codex/010-refresh-roadmap`
- Commit: `docs: refresh roadmap contracts`
- Use a signed commit.

## Steps

### Step 1: Verify the live HTML contract

Re-read `HtmlOptions`, `html::write`, `Format::write_to_dir`, their rustdoc, and
call sites. Confirm:

- `HtmlOptions::new(f64)` validates finite values in the inclusive valid range.
- `HtmlOptions::default()` uses the traditional threshold.
- `green_threshold()` is the public accessor.
- Existing write entrypoints require `&HtmlOptions`.
- The medium-to-low boundary remains fixed.

If any statement is false on the live tree, document the live contract rather
than following this planned snapshot mechanically.

### Step 2: Replace stale HTML roadmap prose

Rewrite the completed threshold item to describe the actual constructor,
accessor, default, validation, and configured write path. Do not show struct
literal construction because the field is private. Prefer symbol links or a
small compilable example over speculative compatibility history.

Keep the user-facing threshold behavior and CLI validation description only
where live tests or code still support them.

### Step 3: Verify and describe the active benchmark workflow

Read the complete `bench.yml` matrix, triggers, permissions, build commands,
CodSpeed action configuration, and benchmark crates. Replace references to the
old dashboard with the active CodSpeed simulation design. Describe stable
coverage categories, not volatile shard totals or action versions.

Remove the GitHub Pages URL only after confirming no other active workflow owns
that benchmark dashboard. Do not claim that CodSpeed produces a public
dashboard unless checked-in documentation or provider configuration proves it.

### Step 4: Replace volatile historical totals

Replace exact test and line coverage totals with a durable statement such as
broad unit, conformance, integration, and strict lint validation. Do not change
meaningful compatibility guarantees or milestone status.

Search public documentation for the same obsolete symbols and benchmark names.
Fix only matches that describe this same contract. Leave historical changelog
or dependency references intact when they are accurate in context.

### Step 5: Run documentation and doctest gates

Run stale-name searches, rustdoc tests, typos, and diff checks. Inspect the
rendered Markdown structure and confirm the diff contains no unrelated roadmap
rewriting.

## Test plan

```bash
if rg -n "high_threshold|write_with_options|write_to_dir_with_options|175 automated checks|282 tests|97%" ROADMAP.md README.md; then exit 1; fi
if rg -n '\*\*`bench\.yml`\*\*:.*(github-action-benchmark|gh-pages|dashboard)' ROADMAP.md; then exit 1; fi
rg -n "HtmlOptions::new|green_threshold|CodSpeed|simulation|bloat.*gh-pages" ROADMAP.md
cargo test --workspace --doc > /tmp/oxc-roadmap-doctest.log 2>&1
tail -80 /tmp/oxc-roadmap-doctest.log
typos ROADMAP.md README.md
git diff --check
git diff -- ROADMAP.md
```

Expected result: obsolete claims are absent, current API and benchmark terms
are present, doctests and documentation gates pass, and only proven stale
roadmap text changed.

## Done criteria

- HTML roadmap prose matches public symbols and threshold semantics.
- Benchmark prose matches the active CodSpeed simulation workflow.
- Obsolete GitHub Pages performance benchmark dashboard claims are removed,
  while the separately verified binary-size history remains documented.
- Volatile test and coverage totals are replaced with durable capabilities.
- Unfinished roadmap items remain intact unless live evidence justifies a
  status change.
- Documentation and doctest checks pass.

## STOP conditions

- The HTML API is mid-migration or its intended steady state is unclear.
- The benchmark workflow and maintainer intent disagree.
- Another active workflow still owns the documented GitHub Pages dashboard.
- Correcting a claim would require changing implementation or making a new
  product commitment.

## Maintenance notes

Treat public API examples and benchmark infrastructure descriptions as
release-adjacent documentation. Update them with the code or workflow change,
and prefer durable capabilities over totals that decay after the next commit.
