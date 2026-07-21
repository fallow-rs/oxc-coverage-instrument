# Coding agent guides for `crates/oxc_coverage_instrument`

The crate README covers the public API and the documented divergences from
`istanbul-lib-instrument`. This file covers how to test a change to that
behavior.

## Running the tests

Every integration test compiles into one binary, `tests/instrument/main.rs`,
which declares its sibling modules. Run the whole crate with:

```sh
cargo test -p oxc_coverage_instrument
```

Scope to one module or one test with a filter passed to that binary:

```sh
cargo test -p oxc_coverage_instrument --test instrument conformance_suite::c06_if_else
```

Test the public API (`instrument(source, filename, &InstrumentOptions)` then
assert on `InstrumentResult`); do not reach into internals.

Changes behind the experimental `ast-api` feature have a separate compile,
test, lint, doctest, and documentation gate:

```sh
./scripts/check.sh ast-api
```

Before changing behavior that can affect production bundles, explicitly
prepare the pinned, hash-verified corpus and run both real-project gates:

```sh
node scripts/prepare-real-world-corpus.mjs
./scripts/check.sh real-world-gates
```

The check dispatcher never downloads corpus files. Review any manifest update
for its exact version, immutable URL, license metadata, and SHA-256 digest.

## Adding a conformance fixture

Conformance fixtures pin coverage-map shape against istanbul's canonical output
for the same source. Each fixture is three edits:

1. Add the source at `tests/conformance/fixtures/NN-name.js`.
2. Generate the istanbul reference (needs `npm install` at the repo root for
   `istanbul-lib-instrument`):

   ```sh
   node crates/oxc_coverage_instrument/tests/conformance/generate-reference.mjs
   ```

   This writes `tests/conformance/reference/NN-name.json` for every fixture.
3. Register it in `tests/instrument/conformance_suite.rs`:

   ```rust
   conformance_test!(cNN_name, "NN-name");
   ```

The `conformance_test!` macro emits one `#[test]` per dimension per fixture:
function count, branch count, branch types, per-branch location counts,
statement count, plus the serialized JSON shape and a re-parse of the emitted
code. Keep them separate. A merged `assert_eq!` stops at the first failure and
hides which dimension regressed. The fixture and reference loaders `panic!` on a
missing file, so a fixture added without its reference JSON fails loudly instead
of passing green.

## Reviewing snapshots

Coverage-map and instrumented-code snapshots live in
`tests/instrument/snapshots/` and are driven by `tests/instrument/snapshot.rs`
(`assert_json_snapshot!` for the map, `assert_snapshot!` for the code). A change
to either output writes `.snap.new` files; accept or reject them with:

```sh
cargo insta review
```

`cargo insta test -p oxc_coverage_instrument` runs the suite and collects the
pending snapshots in one step.

## Byte-for-byte istanbul parity

The count-based Rust tests miss span-level and counter-shape drift. The blocking
CI check that catches it is a leaf-by-leaf diff of the emitted coverage map
against `istanbul-lib-instrument` over the same fixture corpus:

```sh
./scripts/check.sh istanbul-diff
```

It runs `node scripts/istanbul-diff.mjs`, so it needs `npm install` and the
native binding built once:

```sh
npm --prefix crates/oxc_coverage_instrument_napi run build:debug
```

When a change makes the output diverge from istanbul on purpose, add a targeted
filter in `scripts/istanbul-diff.mjs` and document the divergence in the README
under "Differences from istanbul-lib-instrument". An unfiltered divergence fails
the check.
