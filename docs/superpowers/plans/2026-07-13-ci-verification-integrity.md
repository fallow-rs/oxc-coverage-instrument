# CI Verification Integrity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure runner self-tests and version synchronization can block merges.

**Architecture:** Reuse the existing cross-platform check matrix and aggregate. Run the Bash self-test only on Ubuntu, and add the existing version-sync job to the aggregate dependency set.

**Tech Stack:** GitHub Actions, Bash, actionlint, existing `scripts/check.sh`.

## Global Constraints

- Do not change job permissions, matrices, caches, or artifact sequencing.
- Do not impose Bash execution on Windows.
- Preserve all existing required job names.

---

### Task 1: Enforce the verification and aggregate contracts

**Files:**
- Modify: `scripts/check-self-test.sh`

- [x] Add a static workflow assertion that the Ubuntu check path invokes
  `./scripts/check.sh self-test`.
- [x] Add an assertion that `ci-ok.needs` contains `version-sync`.
- [x] Run the self-test and confirm RED against the current workflow.

Run: `./scripts/check-self-test.sh`

Expected: failure names the missing CI self-test or aggregate dependency.

#### Wire both gates into CI

**Files:**
- Modify: `.github/workflows/ci.yml`

- [x] Add a `Verification runner self-test` step to `jobs.check.steps` with
  `if: runner.os == 'Linux'` and command `./scripts/check.sh self-test`.
- [x] Add `version-sync` to `jobs.ci-ok.needs`.
- [x] Run self-test, actionlint, and zizmor until GREEN.
- [x] Commit with a signed Conventional Commit subject.

Run:

```bash
./scripts/check-self-test.sh
./scripts/check.sh actionlint
./scripts/check.sh zizmor
```

Expected: every command exits successfully.
