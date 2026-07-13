# Plan 002: Preserve all coverage through multi-source remapping and collisions

> **Executor instructions**: Follow this plan exactly. This is a correctness
> migration, not a local cleanup. Run each verification gate before continuing.
> Stop and report instead of improvising when a STOP condition occurs. Update
> this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- crates/oxc-coverage-source-maps/src/lib.rs crates/oxc-coverage-source-maps/tests/lib_edges.rs crates/oxc-coverage-instrument-napi/src/lib.rs crates/oxc-coverage-instrument-napi/test.mjs README.md`
> Compare every changed in-scope file with the excerpts below. Stop if the
> remap return shapes or collision behavior changed.

## Status

- **Status**: DONE
- **Priority**: P1
- **Effort**: L
- **Risk**: HIGH
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

The remapper currently assigns an entire generated file to the first source in
its source map, even when locations map to several original files. At the map
level, two generated chunks that remap to the same original path overwrite one
another. Bundled and code-split applications can therefore receive coverage
under the wrong file and lose earlier hit counts. The corrected map API must
fan one generated entry out by original source and merge fan-in by Istanbul
location semantics without breaking the existing single-source API.

## Current state

- `crates/oxc-coverage-source-maps/src/lib.rs:197-200` documents destructive
  collision behavior:

```rust
/// When two entries remap to the same original path ... the
/// later entry replaces the earlier
```

- `remap_coverage_map_with_loader_and_options` currently performs plain
  replacement:

```rust
Some(remapped) => {
    out.insert(remapped.path.clone(), remapped);
}
```

- `apply_source_map` selects `sm.sources.first()` through
  `resolve_primary_source`, sets `out.path` once, then remaps every location
  without retaining its source index.
- `get_mapping_location` already proves the start and end of one `Location`
  map to the same source, but returns only `Location`; extend this invariant
  rather than duplicating lookup logic.
- `FileCoverage` uses separate id-keyed metadata and counter maps. Any split or
  merge must keep `statementMap/s`, `fnMap/f/x_fallow_functionMap`, and
  `branchMap/b/bT` aligned and contiguous.
- Tests in `lib_edges.rs` use `srcmap-generator = 0.3.9` as a dev dependency;
  follow the generator pattern in `benches/remap.rs` instead of hand-authoring
  complex VLQ mappings.
- N-API `remap_coverage_map` and `remap_coverage_map_with_loader` already call
  the Rust map-level functions, so they should gain correct behavior without a
  JavaScript API rename.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Source-map tests | `cargo test -p oxc_coverage_source_maps --test lib_edges` | exits 0 |
| Instrument integration | `cargo test -p oxc_coverage_instrument source_map` | exits 0 |
| N-API build | `npm --prefix crates/oxc-coverage-instrument-napi run build:debug` | exits 0 |
| N-API tests | `node crates/oxc-coverage-instrument-napi/test.mjs` | exits 0 |
| Full suite | `cargo test --workspace --all-targets` | exits 0 |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | exits 0 |

## Scope

**In scope**:

- `crates/oxc-coverage-source-maps/src/lib.rs`
- `crates/oxc-coverage-source-maps/tests/lib_edges.rs`
- `crates/oxc-coverage-instrument-napi/src/lib.rs` only for docs or adaptation
  required by the corrected Rust API
- `crates/oxc-coverage-instrument-napi/test.mjs`
- `README.md` for public remap semantics

**Out of scope**:

- Eager `composeInputSourceMap` during instrumentation. Its one-file output
  shape cannot represent a true multi-source split and must remain documented
  separately.
- A general-purpose coverage merge crate or new dependency.
- Changing the JSON names of existing N-API functions.
- Recomputing `x_fallow_functionMap` identities after remap. Preserve or drop
  entries consistently with existing documented behavior.

## Git workflow

- Branch: `codex/002-preserve-source-map-coverage`
- Commit: `fix(source-maps): preserve multi-source coverage`
- Use `git commit -S`. Do not push or open a PR unless instructed.

## Steps

### Step 1: Establish upstream behavior with failing repros

Add Rust fixtures generated with `SourceMapGenerator` for:

1. One generated file whose statements map to `src/a.ts` and `src/b.ts`.
2. Two generated chunks whose source maps both resolve to `src/shared.ts`,
   with one shared location and one distinct location per chunk.
3. Functions, branches, `bT`, and the function identity overlay, not only
   statements.

Add a small Node oracle in the N-API test or a dedicated test helper that feeds
the same fixtures to the installed `istanbul-lib-source-maps` and compares
paths, locations, and counts. Do not snapshot internal ids, because ids may be
renumbered while locations and counts remain equivalent.

**Verify**: before implementation, the Rust repro must show all multi-source
entries under the first path and the collision repro must lose one chunk.

### Step 2: Retain source identity in remapped locations

Introduce a private result such as:

```rust
struct MappedLocation {
    source: u32,
    location: Location,
}
```

Change the cached get-mapping path to retain the source index alongside the
location. A statement or function belongs to one source only when every
required span resolves to that source. A branch belongs to the source selected
by its retained mapped arms. Drop cross-source arms, fall back to the first
retained arm when the umbrella is unmapped, and retain a mapped umbrella as
metadata even when it resolves elsewhere, matching the upstream oracle.

Keep the existing location-only wrapper for callers that do not need source
identity, so `PositionRemapper::location_maps` and eager-gate behavior remain
stable.

**Verify**: focused tests prove source indices are retained without changing
single-source results.

### Step 3: Add a one-to-many remap primitive

Add a public map-returning primitive for one `FileCoverage`, for example
`remap_coverage_to_map_with_loader_and_options`. It must:

- return one `FileCoverage` per resolved original source
- clear `inputSourceMap` on every remapped output
- retain matching counter slots only
- renumber each metadata and counter family contiguously
- preserve the existing orphan-counter invariant

Keep the current `Option<FileCoverage>` functions source-compatible. They may
delegate to the new primitive and return `Some` only when exactly one remapped
file exists. For a true multi-source result, return `None` and document that
callers needing complete results must use the map-returning API. Do not pick one
source silently. When a multi-source map yields one surviving output, preserve
the legacy wrapper's original metadata IDs and aligned counter and overlay
shape rather than returning the map API's canonicalized IDs.

**Verify**: the multi-source Rust repro returns both original paths with the
expected entries and counts.

### Step 4: Merge remap collisions by location

Change every map-level remap function and `SourceMapStore` map transform to
merge into an existing original path rather than replace it. Implement the
merge locally without a new dependency.

Merge rules:

- Equivalent statement locations share one output id and their `s` counts sum.
- Equivalent functions use declaration location as identity; their `f` counts
  sum and the first entry supplies name and body metadata.
- Equivalent branches use ordered arm locations as identity; `b` and `bT` sum
  arm by arm and the first entry supplies type and umbrella metadata.
- Distinct entries append with new contiguous ids.
- Overlay entries follow the final function id. If equivalent functions carry
  conflicting overlay records, stop and follow the upstream or documented
  extension contract instead of choosing arbitrarily.
- Statement-only inputs do not participate in overlay completeness or conflict
  decisions.
- Use checked or saturating addition consistently with existing counter types;
  document the choice and test the boundary.

**Verify**: collision tests retain both distinct entries and sum the shared
entry exactly like the JavaScript oracle.

### Step 5: Verify N-API and real-project behavior

Add N-API assertions for the same multi-source and collision JSON shapes.
Then run the Vitest TypeScript example and remap its generated coverage through
the N-API API or CLI. Confirm the output retains its original TypeScript path
and remains consumable by the existing verification script.

```bash
npm --prefix crates/oxc-coverage-instrument-napi ci
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
npm --prefix examples/vitest-typescript ci
npm --prefix examples/vitest-typescript run coverage
npm --prefix examples/vitest-typescript run verify
```

**Verify**: every command exits 0.

### Step 6: Run full gates

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
node scripts/istanbul-diff.mjs
node crates/oxc-coverage-instrument-napi/test.mjs
```

**Verify**: every command exits 0.

## Test plan

- Minimal bug repro: one generated file maps to two original sources.
- Collision repro: two chunks map to one original source with shared and
  distinct coverage entries.
- All metadata families: statements, functions, branches, `bT`, and overlay.
- Edge cases: unmapped entries under both drop modes, invalid maps, count
  addition boundary, and conflicting overlay data.
- Oracle: compare location and count semantics with `istanbul-lib-source-maps`.
- Real project: Vitest TypeScript example.

## Done criteria

- [x] Map-level remapping emits every original source represented by mappings.
- [x] No remapped chunk overwrites existing coverage for the same source path.
- [x] Single-source public APIs remain source-compatible.
- [x] Metadata, counters, and overlays retain their invariants.
- [x] Rust and N-API repros match the upstream oracle.
- [x] Real-project smoke and full repository gates pass.
- [x] Only in-scope files and `plans/README.md` are modified.

## STOP conditions

Stop and report if:

- The JavaScript oracle intentionally drops or reshapes a case differently
  from this plan.
- Correct support requires a breaking change to an existing public signature.
- Cross-source branch semantics are ambiguous after reading upstream behavior.
- Overlay collisions cannot be reconciled without changing the documented id
  contract.
- A verification command fails twice after a reasonable correction.

## Maintenance notes

Reviewers should focus on id/counter alignment and source identity, not the
specific internal data structure. Any future performance work must benchmark
the fan-out and merge paths separately from single-source remapping.
