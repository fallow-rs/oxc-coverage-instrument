# Plan 007: Add repository-specific agent guidance

> **Executor instructions**: Follow this plan step by step. Keep the guidance
> concise and public-repository safe. Run every verification command before
> marking the plan complete, then update its row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- CONTRIBUTING.md .githooks/pre-push package.json Cargo.toml .github/workflows/release-npm.yml`
> Re-read every changed policy surface before writing `AGENTS.md`. Stop if two
> authoritative files disagree about generated files or release ownership.

## Status

- **Status**: DONE
- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: DX
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

The repository has no local `AGENTS.md`. Automated contributors must infer
layout, generated-file ownership, version synchronization, and required gates
from several documents and workflows. That makes companion package drift and
incomplete validation more likely. A short repository-specific guide should
route agents to the authoritative sources without duplicating global rules.

## Current state

`CONTRIBUTING.md` identifies the public and internal package boundaries:

```text
- `oxc_coverage_instrument` and the published npm package move together
- Internal adapter crates with `publish = false` are implementation packages
- The root `package.json` is a private helper manifest
- The published npm manifest is `crates/oxc-coverage-instrument-napi/package.json`
```

It also defines separate Rust, N-API, conformance, workflow-security, and
dependency checks. `.githooks/pre-push` runs only a fast subset by default, and
the release workflow owns the multi-package publication sequence. No root file
currently tells an agent which source is authoritative or which outputs are
generated.

## Commands you will need

```bash
test ! -e AGENTS.md
sed -n '1,220p' CONTRIBUTING.md
sed -n '1,180p' .githooks/pre-push
rg -n "generated|sync|package.json|publish|release" CONTRIBUTING.md scripts .github/workflows/release-npm.yml
```

## Scope

### In scope

- New root `AGENTS.md`.
- Import of `@~/.Codex/rules/release-companion-repos.md`.
- Repository layout and authoritative-source guidance.
- Generated artifact and companion manifest ownership.
- Validation expectations, including real-world bug validation.
- Links to existing contributor and release documentation.

### Out of scope

- Copying global agent instructions into this repository.
- Machine-specific paths other than the required rule import.
- Private project names or real private-project output.
- Changes to hooks, CI, release automation, source, or manifests.
- Replacing `CONTRIBUTING.md` with agent-only documentation.

## Git workflow

- Branch: `codex/007-add-agent-guidance`
- Commit: `docs: add repository agent guidance`
- Use a signed commit.

## Steps

### Step 1: Establish authoritative ownership

Re-read the drift-check files immediately before editing. Build a small mapping
for the guide:

- Rust crates and tests: workspace manifests and crate-local source.
- Published Node package: `crates/oxc-coverage-instrument-napi`.
- Platform manifests: the N-API package's `npm/` directory.
- Generated N-API loaders and declarations: N-API build tooling, not manual
  edits unless the generator itself is updated.
- Version synchronization: `CONTRIBUTING.md`, sync scripts, and release checks.
- Verification: `CONTRIBUTING.md`, hooks, and CI.

If ownership cannot be proven from checked-in files, do not state it as fact.

### Step 2: Add concise root guidance

Create `AGENTS.md`. Put this import first:

```text
@~/.Codex/rules/release-companion-repos.md
```

Then add short sections for repository structure, generated files, version
policy, verification, and git hygiene. Link to `CONTRIBUTING.md` for detailed
commands. State that a bug fix requires a minimal repro, validation against a
real user project, and the full relevant suite before completion.

Make clear that:

- Changes stay scoped and preserve unrelated worktree edits.
- Public and platform npm manifests must stay synchronized when their shared
  contract changes.
- Generated files are updated through their owner and verified for drift.
- Commits are signed and use Conventional Commits.
- Private project output is never copied into public fixtures or docs.

### Step 3: Check portability and contradictions

Compare every new statement with `CONTRIBUTING.md` and the release workflow.
Remove duplicated command lists that could drift. Search for absolute home
paths, private project names, secrets, and policy contradictions.

### Step 4: Run documentation gates

Run the commands below, inspect the diff, and confirm `AGENTS.md` is the only
changed file for this plan.

## Test plan

```bash
rg -n "release-companion-repos|CONTRIBUTING.md|generated|real user project|signed" AGENTS.md
if rg -n "/Users/|private client|api[_-]?key|token=" AGENTS.md; then exit 1; fi
typos AGENTS.md CONTRIBUTING.md
git diff --check
git status --short
```

Expected result: the required policy topics are present, the private-data
search is empty, documentation checks pass, and only `AGENTS.md` is changed.
If `typos` is unavailable, install the documented tool or let CI run the gate.

## Done criteria

- Root `AGENTS.md` imports the companion-repository rules.
- It identifies the important repository surfaces and authoritative documents.
- Generated artifacts and version synchronization have clear owners.
- Bug-fix validation includes a repro, real user project, and full suite.
- Guidance contains no private, secret, or non-portable repository data.
- Documentation checks pass.

## STOP conditions

- An instruction conflicts with an existing repository policy.
- A generated artifact or manifest has no provable owner.
- The required personal rule import is not acceptable for the repository.
  Resolve that policy decision before committing a partial guide.

## Maintenance notes

Review `AGENTS.md` whenever repository layout, release synchronization, or the
canonical verification entrypoint changes. Keep detailed procedures in their
authoritative documents and keep this file as a concise routing layer.
