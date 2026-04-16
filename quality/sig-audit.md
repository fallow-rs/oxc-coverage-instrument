# SIG Audit

Date: 2026-04-17
Version: 1.1

## Scope

This audit covers production Rust code only:

- `src/*.rs`
- `cli/src/main.rs`
- `napi/src/lib.rs`

Excluded from code-level metrics:

- `tests/`
- `benches/`
- fixture files
- generated npm package metadata and wrapper files

The workspace is a small Rust system with three production components:

- `core_lib` (`src/`)
- `cli` (`cli/src/`)
- `napi` (`napi/src/`)

## Summary

| # | Property | Measurement | SIG 4-star reference | Rating | Reliability |
|---|---|---:|---:|---:|---|
| 1 | Volume | 2,054 LOC across 7 files | Medium systems are 100K-500K LOC | 5 | Exact |
| 2 | Duplication | Estimated `<2%`; only small repeated AST-builder helper patterns | `<5%` duplicated code | 4 | Estimated |
| 3 | Unit Size | `28.4%` >15 LOC, `9.0%` >30 LOC, `3.0%` >60 LOC | `47.1%`, `23.1%`, `8.3%` max | 4 | High |
| 4 | Unit Complexity | Estimated `83.6%` of functions at CC `<=5` | `>=75%` low-complexity units | 4 | Estimated |
| 5 | Unit Interfacing | `23.9%` with `>=3` params, `3.0%` with `>=5`, `0%` with `>=7` | `15.0%`, `3.3%`, `0.9%` max | 3 | High |
| 6 | Module Coupling | Max incoming deps `3`; no module >20 or >50 | `5.6%` / `1.9%` max | 5 | High |
| 7 | Component Balance | `core_lib 88.1%`, `cli 7.5%`, `napi 4.4%` | Even distribution | 3 | High |
| 8 | Component Independence | Estimated hidden code `97.2%` by LOC | `>=93.7%` hidden | 4 | High |
| 9 | Component Entanglement | No crate-level or file-level cycles detected | No cycles / no bypassing | 5 | High |
| 10 | Testability / Coverage | `98.36%` line (lib only), `96.8%` function, test ratio `137.6%` | `>80%` typically 5-star | 5 | Exact |

Estimated overall maintainability: **about 4.2 / 5**.

This is not the official SIG aggregate formula. It is a conservative synthesis of the measured properties above.

## 1. Volume

- Production files: `7`
- Production LOC: `2,054`
- Test files: `6`
- Test LOC: `2,826`
- Test-to-production ratio: `137.6%`

Interpretation:

- The codebase is small enough to remain highly analyzable.
- The test footprint is larger than production code, which is a positive signal for maintainability and regression safety.

Rating: **5/5**

## 2. Duplication

Method:

- Normalized rolling-window duplicate scan over production Rust files.

Result:

- No meaningful cross-file copy-paste blocks found.
- The only repeated material is a small pair of AST-builder sequences in `src/transform.rs` around the counter-expression helpers.

Interpretation:

- Duplicate volume is comfortably below the SIG 4-star threshold of `5%`.
- Because this is a lightweight detector rather than SIG's token-based tooling, treat the result as directional.

Rating: **4/5**

## 3. Unit Size

Measured over `67` Rust functions:

- `19` functions (`28.4%`) larger than `15` LOC
- `6` functions (`9.0%`) larger than `30` LOC
- `2` functions (`3.0%`) larger than `60` LOC

SIG 4-star thresholds:

- `>15 LOC`: max `47.1%`
- `>30 LOC`: max `23.1%`
- `>60 LOC`: max `8.3%`

Largest functions:

- `cli/src/main.rs:15` `main` (`107` LOC)
- `src/instrument.rs:106` `instrument` (`88` LOC)
- `src/transform.rs:835` `exit_statements` (`40` LOC)
- `src/transform.rs:766` `enter_statement` (`39` LOC)
- `src/transform.rs:239` `build_branch_counter_expr` (`37` LOC)

Interpretation:

- The project clears the 4-star thresholds on all three size bins with wide margin on the `>15` and `>30` bins.
- Size risk remains concentrated in a handful of orchestration functions, not spread across the codebase.

Rating: **4/5**

## 4. Unit Complexity

Method:

- Estimated using a Rust-oriented branch heuristic over `67` functions.
- Low-complexity threshold used: estimated CC `<=5`.

Result:

- `83.6%` of functions fall in the low-complexity bucket.

Highest estimated complexity:

- `cli/src/main.rs:15` `main` (CC `~33`)
- `src/transform.rs:835` `exit_statements` (CC `~13`)
- `src/pragma.rs:90` `parse_pragma` (CC `~9`)
- `src/instrument.rs:106` `instrument` (CC `~9`)

Interpretation:

- Above the SIG 4-star reference of `>=75%` low-complexity units.
- The main complexity hotspot (`cli::main`) is argument parsing plus file-output branching, which is inherent to a small argv-based CLI.

Rating: **4/5**

## 5. Unit Interfacing

Measured over `67` Rust functions:

- `16` functions (`23.9%`) have `>=3` parameters
- `2` functions (`3.0%`) have `>=5` parameters
- `0` functions (`0%`) have `>=7` parameters

SIG 4-star thresholds:

- `>=3 params`: max `15.0%`
- `>=5 params`: max `3.3%`
- `>=7 params`: max `0.9%`

Highest-arity functions:

- `src/transform.rs:315` `generate_preamble_source` (`5`)
- `src/transform.rs:1075` `inject_branch_counter_into_statement` (`5`)
- `src/transform.rs:85` `CoverageTransform::new` (`4`)
- `src/transform.rs:201` `build_counter_expr` (`4`)
- `src/transform.rs:239` `build_branch_counter_expr` (`4`)

Interpretation:

- This is the weakest measured maintainability property: the `>=3` bucket sits above the SIG 4-star ceiling.
- Parameter count is concentrated in internal transformation helpers, not in the public API surface.
- Most 4-param helpers thread `cov_fn`, a counter name/id, and a `TraverseCtx` together; a small context struct would bring the `>=3` bucket under the ceiling.

Rating: **3/5**

## 6. Module Coupling

Measured at production-module level (core_lib):

- `transform` → `pragma`, `types`
- `instrument` → `pragma`, `transform`, `types`
- `pragma` → `types`
- `types` → nothing

Incoming dependencies per module:

- `types`: `3`
- `pragma`: `2`
- `transform`: `1`
- `instrument`: `0` (re-exported via `lib`)

- Modules above `20` incoming deps: `0`
- Modules above `50` incoming deps: `0`

Interpretation:

- Coupling is low and very far from the SIG risk thresholds.
- The system has one obvious center of gravity, which is expected for a library with thin adapters.

Rating: **5/5**

## 7. Component Balance

Production LOC by component:

- `core_lib`: `1,810` LOC (`88.1%`)
- `cli`: `154` LOC (`7.5%`)
- `napi`: `90` LOC (`4.4%`)

Interpretation:

- Intentionally adapter-shaped: almost all logic lives in the core library.
- Architecturally sensible, but not "balanced" in the SIG sense.
- The imbalance is acceptable because the small components are wrappers, not neglected domains.

Rating: **3/5**

## 8. Component Independence

Method:

- Hidden code estimated by LOC, using workspace entrypoints as the exposed surface.
- `src/lib.rs` is the only core entrypoint referenced from outside the core component.

Result:

- Exposed entrypoint LOC: `57` (`src/lib.rs`)
- Hidden production LOC: `1,997`
- Hidden share: `97.2%`

SIG 4-star threshold:

- Hidden code should be at least `93.7%`

Interpretation:

- The internal implementation is well hidden behind a very small public surface.
- A strong maintainability property for a library project.

Rating: **4/5**

## 9. Component Entanglement

Measured result:

- No crate-level cycles
- No file-level cycles in the production module graph
- No layer-bypassing between `core_lib`, `cli`, and `napi`

Interpretation:

- At the architectural level, the workspace is clean.
- `UnhandledPragma` lives in `src/types.rs`, so no latent entanglement between `pragma.rs` and `instrument.rs`.

Rating: **5/5**

## 10. Testability And Coverage

Command:

```bash
cargo llvm-cov --summary-only --workspace
```

Fresh coverage summary (library-only, excluding the untested `cli/src/main.rs` binary):

- Line coverage: `98.36%` (1019 of 1036 lines)
- Function coverage: `96.8%` (91 of 94 functions)
- Test/production LOC ratio: `137.6%`

Per-file line coverage:

- `src/lib.rs`: `100.0%`
- `src/types.rs`: `100.0%`
- `src/transform.rs`: `98.79%`
- `src/pragma.rs`: `96.23%`
- `src/instrument.rs`: `95.97%`
- `cli/src/main.rs`: `0%` (CLI binary not exercised in the Rust test suite)

Interpretation:

- Library coverage is well into 5-star territory.
- The CLI binary is an argv-based wrapper around the library and is covered indirectly through shell invocations, not `cargo test`. Reported whole-workspace coverage (`90.02%`) reflects that gap.
- The remaining library gap is concentrated in the orchestration and pragma-parsing error paths, not in the core AST transformation engine.

Rating: **5/5**

## Methodology And Limitations

| Property | Method | Reliability |
|---|---|---|
| Volume | Direct file and line counts over production Rust files | Exact |
| Duplication | Normalized rolling-window duplicate-window scan | Estimated |
| Unit Size | Rust function extraction and LOC binning | High |
| Unit Complexity | Branch-keyword heuristic per Rust function | Estimated |
| Unit Interfacing | Rust signature parsing with `self` excluded | High |
| Module Coupling | Workspace and module dependency scan from imports | High |
| Component Balance | LOC split across production crates | High |
| Component Independence | Hidden-entrypoint approximation using externally referenced surfaces | High |
| Component Entanglement | Crate dependency graph plus file-level cycle inspection | High |
| Coverage | `cargo llvm-cov --summary-only --workspace` | Exact |

Important limitations:

- This is not a licensed SIG analysis run, so the overall score is an informed estimate, not an official benchmark output.
- Duplication and complexity were measured with repo-local heuristics rather than SIG's internal tooling.
- Component independence was approximated from workspace entrypoints; in Rust this is usually a strong proxy because public crate surface is explicit.
- Unit-metric heuristics count body LOC via "non-empty, non-comment line" rules that differ slightly from SIG's proprietary definitions. Treat absolute percentages as directional.

## Priority Actions

1. Reduce helper arity in `src/transform.rs`.
   The parameter-count profile is the clearest measured weakness. A small context struct bundling `cov_fn`, counter kind, and counter id would cut the `>=3` bucket toward the 4-star ceiling without changing behavior.

2. Break up the two largest orchestration functions.
   `cli/src/main.rs:15` `main` (`107` LOC, CC ~33) and `src/instrument.rs:106` `instrument` (`88` LOC) carry most of the size and complexity risk. Splitting argv handling from output-file handling in the CLI, and splitting parse/transform/codegen phases in `instrument`, would lower both size and complexity.

3. Exercise the CLI binary in tests.
   The CLI is currently untested from Rust's perspective (0% llvm-cov), which drags the whole-workspace coverage down to `90.02%`. A handful of `assert_cmd`-style integration tests would lift both the binary and whole-workspace numbers into 5-star territory.

## Conclusion

The code quality is strong. The workspace is small, heavily tested, low in coupling, and structurally centered around a clean core library with thin adapters. On a SIG-style maintainability lens it is clearly above average and plausibly in the low-4-star range.

The main drag factors are local rather than systemic:

- high helper arity in `src/transform.rs`
- a few oversized orchestration functions
- an untested CLI binary that pulls the whole-workspace coverage below the library's own 98%

No file-level dependency cycles remain, and the new UTF-16 column handling removed the last known correctness gap relative to Istanbul's semantics.
