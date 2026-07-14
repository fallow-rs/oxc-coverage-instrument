# Release Prepublication Gate Design

## Objective

Prevent deterministic npm validation failures from occurring after crates.io publication has started. A release tag must pass one combined, read-only prepublication gate before native or WASI builds and before either registry can be mutated.

## Current problem

The release workflow currently runs platform builds first, publishes crates second, and performs the exact npm 11 clean install only inside the final npm publication job. This allowed v0.10.2 to reach crates.io before npm rejected stale nested lockfile records.

The existing `version-sync` profile already checks public version parity, internal Rust pins, release targets, the version-sync regression fixture, and npm pack metadata. The missing contract is execution order: those checks and the exact npm clean install must complete before release work proceeds.

## Workflow design

Add a dedicated `prepublish` job to `.github/workflows/release-npm.yml`. It runs on Ubuntu with read-only repository permissions and performs these steps in order:

1. Check out the tagged commit.
2. Configure Node.js 22.
3. Install the pinned trusted-publishing npm version, currently 11.12.1.
4. Run `./scripts/check.sh version-sync` from the repository root.
5. Run strict `npm ci` in `crates/oxc-coverage-instrument-napi`.

Make `build` depend on `prepublish`. The remaining sequence stays unchanged:

```text
prepublish -> build -> publish-crate -> publish
```

This ordering fails quickly before expensive cross-platform builds and prevents any registry publication when deterministic metadata, version, lockfile, or install checks fail.

## Policy enforcement

Add a focused workflow policy test that reads the checked-in release workflow and verifies:

- the `prepublish` job exists;
- it uses Node.js 22 and installs the exact trusted-publishing npm version;
- it runs the existing combined `version-sync` profile;
- it runs strict `npm ci` in the published N-API package directory;
- `build` depends on `prepublish`;
- the existing `publish-crate` and `publish` dependency chain remains intact.

Wire the policy test into the repository's normal version-sync verification so CI rejects future topology drift. The test must fail against the current workflow before the workflow change is implemented.

## Failure behavior

Any prepublication failure blocks every downstream job. Error output comes from the owning check, npm installation, or policy assertion. No fallback to `npm install`, mutable npm version, or ignored validation failure is allowed.

The final npm publication job keeps its own exact npm setup and strict clean install as defense in depth. Cross-registry publication cannot be fully atomic against registry outages, but the shared gate removes the known deterministic split-release path.

## Verification

Implementation follows a red-green cycle:

1. Add and run the workflow policy test against the current workflow, confirming the expected missing-gate failure.
2. Add the prepublication job and dependency, then confirm the policy test passes.
3. Run version-sync verification, workflow syntax validation, workflow security validation, formatting, and the repository's relevant full verification profile.
4. Inspect the final workflow dependency chain and Git diff before committing.

## Compatibility and scope

No package versions, registry credentials, publication commands, artifacts, or public APIs change. The release workflow gains one short validation stage and a small increase in tag-to-build latency. Publication ordering after successful validation remains unchanged.

Out of scope:

- attempting transactional publication across crates.io and npm;
- changing the existing package build matrix;
- changing npm provenance or registry authentication;
- publishing another release as part of this work.
