# Contract And Evidence Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make public docs, completed-plan state, and retained benchmark evidence match the checked-in behavior.

**Architecture:** Correct only claims disproven by code or workflows. Add a machine-readable companion to Plan 006 without inventing missing candidate commits, remote runs, or significance values.

**Tech Stack:** Markdown, JSON, Node.js JSON parsing, strict TypeScript, repository checks.

## Global Constraints

- Do not change runtime behavior.
- Do not fabricate benchmark SHAs, raw artifacts, run links, or p-values.
- Do not add volatile test totals.
- Keep the Rust single-file sparse-ID contract distinct from map-level N-API behavior.

---

### Task 1: Resolve every remaining contract and evidence finding

**Files:**
- Modify: `README.md`
- Test: `crates/oxc-coverage-instrument-napi/test.mjs`

- [x] Add or identify a runtime assertion showing whether N-API
  `remapCoverageMap` returns contiguous IDs after a dropped entry.
- [x] Run the assertion before docs edits and record the live behavior.
- [x] Rewrite the README sentence to describe map-level contiguous IDs and
  separately note sparse-ID compatibility in the Rust single-file helper.
- [x] Run the N-API runtime oracle and stale-contract search.

#### Add durable Plan 006 evidence

**Files:**
- Create: `plans/evidence/006-v8-range-index-results.json`
- Modify: `plans/006-index-v8-range-lookups.md`

- [x] Encode the original baseline and all three candidate rounds from the
  execution report as JSON objects with stable benchmark IDs, mean
  microseconds, and deltas from the original baseline.
- [x] Identify candidates as `unconditional_laminar_index`,
  `guarded_eager_preflight`, and `lazy_two_stage_gate`, with their documented
  policies and `decision: "reverted"`.
- [x] Add explicit metadata stating that candidate code was not committed,
  local Criterion means are not remote CodSpeed evidence, and significance
  comparisons against the original baseline are unavailable.
- [x] Link the JSON evidence from Plan 006 and validate JSON parsing.

Run:

```bash
node -e "JSON.parse(require('node:fs').readFileSync('plans/evidence/006-v8-range-index-results.json', 'utf8'))"
```

Expected: exits successfully.

#### Correct remaining roadmap and plan state

**Files:**
- Modify: `ROADMAP.md`
- Modify: `plans/003-synchronize-vitest-types.md`

- [x] Change the release matrix description to seven native targets plus two
  WASI variants.
- [x] Check every Plan 003 done criterion only after mapping it to current
  strict typecheck, runtime, package-surface, Vitest, and workspace evidence.
- [x] Add a combined-branch resolution note naming the canonical profiles that
  prove the completed criteria.
- [x] Run strict TypeScript, typos, stale wording searches, and diff checks.
- [x] Commit with a signed Conventional Commit subject.

Run:

```bash
./scripts/check.sh vitest-typecheck
./scripts/check.sh napi-test
./scripts/check.sh package-surface
./scripts/check.sh typos
git diff --check
```

Expected: every command exits successfully.
