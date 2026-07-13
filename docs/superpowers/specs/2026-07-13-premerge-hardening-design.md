# Pre-merge hardening design

## Goal

Close every finding from the final review before merging
`codex/fix-all-improvements` into `main`.

## Scope

The hardening pass has three independent parts:

1. Make HTML report writes resistant to symlink replacement and hard-link
   truncation races while preserving the public API and report layout.
2. Make the CI aggregate enforce version synchronization and the verification
   runner self-tests.
3. Correct the remaining public documentation, plan status, and benchmark
   evidence contracts.

## Secure HTML output architecture

Use `cap-std` and `cap-fs-ext` version `4.0.2`. The HTML reporter opens the
selected output directory once as a capability directory and performs every
later traversal relative to retained directory handles.

For each path component:

- validate that it is a single normal component;
- open it with `DirExt::open_dir_nofollow`;
- if missing, create it relative to the parent handle and reopen it without
  following symlinks;
- reject files, symlinks, reparse points, and other non-directory objects.

For each leaf file:

- create an unpredictable temporary sibling with `create_new` and no-follow;
- write the complete content to that new inode;
- replace the destination by a handle-relative rename;
- on platforms where rename cannot replace an existing leaf, unlink the
  destination entry through the held parent handle and retry the rename;
- remove the temporary entry on every error path.

This prevents writes through swapped path components. It also avoids
truncating an outside inode through a hard-linked destination. A concurrent
replacement can make a report fail, but cannot redirect its bytes outside the
held capability tree.

The output directory path itself is resolved once when the capability is
opened. After that point, renaming or replacing its ambient pathname does not
change the directory object used by the report.

## CI and verification architecture

The existing `check` matrix remains cross-platform and keeps its direct Rust
commands. The Ubuntu matrix member additionally runs
`./scripts/check.sh self-test`, because the self-test uses Bash fixtures and
must protect merges without imposing a Bash contract on Windows.

The `ci-ok` aggregate adds `version-sync` to `needs`, so a version failure
cannot leave the branch-protection aggregate green.

## Contract and evidence corrections

- Verify the actual N-API `remapCoverageMap` ID behavior, then align README
  wording with its map-level contiguous ID contract. Keep the separate Rust
  single-file sparse-ID compatibility statement explicit.
- Record Plan 006 benchmark measurements in a checked-in machine-readable JSON
  file. Identify each candidate by algorithm and policy, include every retained
  mean and delta, and state that candidate code was not committed and no remote
  significance data exists.
- Correct the release matrix description to seven native targets plus two WASI
  variants.
- Mark the verified Plan 003 completion criteria and add a combined-branch
  resolution note.

## Testing

The HTML regression layer must show RED before implementation for:

- replacing a previously valid child directory with a symlink before a write;
- a destination leaf hard-linked to an outside sentinel.

GREEN requires the outside sentinel to remain unchanged, the symlink race to
fail safely, ordinary recursive output to remain correct, and a real Vitest
coverage report to render through the CLI.

The CI and contract layer uses actionlint, the runner self-test, strict
TypeScript, JSON parsing of benchmark evidence, stale wording searches, and
the repository's full `all-local` profile.

## Non-goals

- No public HTML API or layout change.
- No custom unsafe platform syscall layer.
- No performance implementation is restored from Plan 006.
- No new release promises or workflow topology changes beyond enforcing the
  existing jobs.

