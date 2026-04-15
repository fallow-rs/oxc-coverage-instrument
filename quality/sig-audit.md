# SIG Audit

Date: 2026-04-15
Version: 1.0

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

The workspace is a small Rust system with three production crates/components:

- `core_lib` (`src/`)
- `cli` (`cli/src/`)
- `napi` (`napi/src/`)

## Summary

| # | Property | Measurement | SIG 4-star reference | Rating | Reliability |
|---|---|---:|---:|---:|---|
| 1 | Volume | 2,038 LOC across 7 files | Medium systems are 100K-500K LOC | 5 | Exact |
| 2 | Duplication | Estimated `<2%`; only two small repeated helper patterns found | `<5%` duplicated code | 4 | Estimated |
| 3 | Unit Size | `43.3%` >15 LOC, `17.9%` >30 LOC, `3.0%` >60 LOC | `47.1%`, `23.1%`, `8.3%` max | 4 | High |
| 4 | Unit Complexity | Estimated `85.1%` of functions at CC `<=5` | `>=75%` low-complexity units | 4 | Estimated |
| 5 | Unit Interfacing | `23.9%` with `>=3` params, `3.0%` with `>=5`, `0%` with `>=7` | `15.0%`, `3.3%`, `0.9%` max | 3 | High |
| 6 | Module Coupling | Max incoming deps `3`; no module >20 or >50 | `5.6%` / `1.9%` max | 5 | High |
| 7 | Component Balance | `core_lib 88.6%`, `cli 7.0%`, `napi 4.4%` | Even distribution | 3 | High |
| 8 | Component Independence | Estimated hidden code `97.1%` by LOC | `>=93.7%` hidden | 4 | High |
| 9 | Component Entanglement | No crate-level or file-level cycles detected | No cycles / no bypassing | 5 | High |
| 10 | Testability / Coverage | `96.95%` line, `95.74%` function, test ratio `136.1%` | `>80%` typically 5-star | 5 | Exact |

Estimated overall maintainability: **about 4.2 / 5**.

This is not the official SIG aggregate formula. It is a conservative synthesis of the measured properties above.

## 1. Volume

- Production files: `7`
- Production LOC: `2,038`
- Test files: `7`
- Test LOC: `2,773`
- Test-to-production ratio: `136.1%`

Interpretation:

- The codebase is small enough to remain highly analyzable.
- The test code footprint is larger than production code, which is usually a positive signal for maintainability and regression safety.

Rating: **5/5**

## 2. Duplication

Method:

- Normalized 6-line rolling-window duplicate scan over production Rust files.

Result:

- No meaningful cross-file copy-paste blocks were found.
- The only repeated material was a small pair of AST-builder sequences in `src/transform.rs`, around the counter-expression helpers.

Interpretation:

- Duplicate volume appears comfortably below the SIG 4-star threshold of `5%`.
- Because this is a lightweight detector rather than token-based SIG tooling, this result should be treated as directional.

Rating: **4/5**

## 3. Unit Size

Measured over `67` Rust functions:

- `29` functions (`43.3%`) are larger than `15` LOC
- `12` functions (`17.9%`) are larger than `30` LOC
- `2` functions (`3.0%`) are larger than `60` LOC

SIG 4-star thresholds:

- `>15 LOC`: max `47.1%`
- `>30 LOC`: max `23.1%`
- `>60 LOC`: max `8.3%`

Largest functions:

- `cli/src/main.rs:14` `main` (`97` LOC)
- `src/instrument.rs:106` `instrument` (`94` LOC)
- `src/transform.rs:813` `exit_statements` (`46` LOC)
- `src/transform.rs:758` `enter_statement` (`45` LOC)
- `src/transform.rs:232` `build_branch_counter_expr` (`44` LOC)

Interpretation:

- The project clears the 4-star thresholds on all three size bins.
- The size risk is concentrated in a handful of orchestration functions rather than spread across the whole codebase.

Rating: **4/5**

## 4. Unit Complexity

Method:

- Estimated using a Rust-oriented branch heuristic over `67` functions.
- Low-complexity threshold used: estimated CC `<=5`.

Result:

- `85.1%` of functions are in the low-complexity bucket.

Highest estimated complexity:

- `cli/src/main.rs:14` `main` (`17`)
- `src/transform.rs:813` `exit_statements` (`11`)
- `src/transform.rs:308` `generate_preamble_source` (`10`)
- `src/instrument.rs:106` `instrument` (`9`)

Interpretation:

- The project is above the SIG 4-star reference of `>=75%` low-complexity units.
- Complexity hotspots are narrow and understandable: command dispatch, AST rewrite orchestration, and preamble generation.

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

- `src/transform.rs:308` `generate_preamble_source` (`5`)
- `src/transform.rs:1057` `inject_branch_counter_into_statement` (`5`)
- `src/transform.rs:232` `build_branch_counter_expr` (`4`)
- `src/transform.rs:194` `build_counter_expr` (`4`)
- `src/transform.rs:293` `build_branch_counter_stmt` (`4`)

Interpretation:

- This remains the weakest measured maintainability property.
- The parameter count is concentrated in internal transformation helpers, not in public API surface.
- The refactor removed the worst 6-parameter recursion chain, which brought the `>=5` bucket back under the SIG 4-star threshold.
- What still keeps this property below 4 stars is the number of 3- and 4-parameter internal helpers in `src/transform.rs`.

Rating: **3/5**

## 6. Module Coupling

Measured at production-module/crate level:

- Max incoming dependencies to any module: `3`
- Modules above `20` incoming deps: `0`
- Modules above `50` incoming deps: `0`

Observed dependency shape:

- `cli` depends on `core_lib`
- `napi` depends on `core_lib`
- Core source modules are modestly connected, with no fan-in hotspot anywhere near SIG risk levels

Interpretation:

- Coupling is low and very far from the SIG risk thresholds.
- The system has one obvious center of gravity, which is expected for a library with thin adapters.

Rating: **5/5**

## 7. Component Balance

Production LOC by component:

- `core_lib`: `1,806` LOC (`88.6%`)
- `cli`: `142` LOC (`7.0%`)
- `napi`: `90` LOC (`4.4%`)

Interpretation:

- The system is intentionally adapter-shaped: almost all logic lives in the core library.
- That is architecturally sensible, but it is not “balanced” in the SIG sense.
- The imbalance is acceptable because the small components are wrappers, not neglected domains.

Rating: **3/5**

## 8. Component Independence

Method:

- Hidden code estimated by LOC, using workspace entrypoints as the exposed surface.
- `src/lib.rs` is the only core entrypoint referenced from outside the core component.

Result:

- Exposed entrypoint LOC: `59`
- Hidden production LOC: `1,979`
- Hidden share: `97.1%`

SIG 4-star threshold:

- Hidden code should be at least `93.7%`

Interpretation:

- The internal implementation is well hidden behind a very small public surface.
- This is a strong maintainability property for a library project.

Rating: **4/5**

## 9. Component Entanglement

Measured result:

- No crate-level cycles
- No file-level cycles detected in the production module graph
- No layer-bypassing pattern between `core_lib`, `cli`, and `napi`

Interpretation:

- At the architectural level, the workspace is clean.
- The previous `instrument.rs <-> pragma.rs` entanglement is gone after moving `UnhandledPragma` into `src/types.rs`.
- This property no longer needs immediate attention.

Rating: **5/5**

## 10. Testability And Coverage

Command:

```bash
cargo llvm-cov --json
```

Fresh coverage summary:

- Line coverage: `96.95%`
- Function coverage: `95.74%`
- Region coverage: `96.20%`
- Test/production LOC ratio: `136.1%`

Per-file line coverage:

- `src/lib.rs`: `100.0%`
- `src/types.rs`: `100.0%`
- `src/transform.rs`: `98.50%`
- `src/pragma.rs`: `92.86%`
- `src/instrument.rs`: `89.81%`

Per-file function coverage:

- `src/lib.rs`: `100.0%`
- `src/types.rs`: `100.0%`
- `src/transform.rs`: `100.0%`
- `src/instrument.rs`: `85.71%`
- `src/pragma.rs`: `71.43%`

Interpretation:

- Coverage is well into 5-star territory.
- The remaining gap is concentrated in the orchestration and pragma-parsing paths, not in the core AST transformation engine.

Rating: **5/5**

## Methodology And Limitations

| Property | Method | Reliability |
|---|---|---|
| Volume | Direct file and line counts over production Rust files | Exact |
| Duplication | Normalized 6-line duplicate-window scan | Estimated |
| Unit Size | Rust function extraction and LOC binning | High |
| Unit Complexity | Branch-keyword heuristic per Rust function | Estimated |
| Unit Interfacing | Rust signature parsing with `self` excluded | High |
| Module Coupling | Workspace/module dependency scan from imports and module edges | High |
| Component Balance | LOC split across production crates | High |
| Component Independence | Hidden-entrypoint approximation using externally referenced surfaces | High |
| Component Entanglement | Crate dependency graph plus file-level cycle inspection | High |
| Coverage | `cargo llvm-cov --json` | Exact |

Important limitations:

- This is not a licensed SIG analysis run, so the overall score is an informed estimate, not an official benchmark output.
- Duplication and complexity were measured with repo-local heuristics rather than SIG’s internal tooling.
- Component independence was approximated from workspace entrypoints; in Rust this is usually a strong proxy because public crate surface is explicit.

## Priority Actions

1. Reduce helper arity in `src/transform.rs`.
   The parameter-count profile is the clearest measured weakness. A small context struct for logical-branch wrapping would improve readability and reduce call-site noise.

2. Break up the largest orchestration functions.
   `cli/src/main.rs:14`, `src/instrument.rs:106`, and `src/transform.rs:813` carry most of the size and complexity risk. Splitting command parsing, instrumentation pipeline setup, and statement-exit handling would improve analyzability.

3. Split one more orchestration function only if new features land there.
   The next likely candidates are `generate_preamble_source` and `instrument`, but neither is urgent after the current cleanup.

## Conclusion

The code quality is strong. The workspace is small, heavily tested, low in coupling, and structurally centered around a clean core library with thin adapters. On a SIG-style maintainability lens, it is clearly above average and plausibly in the low-4-star range.

The main drag factors are local rather than systemic:

- high helper arity in `src/transform.rs`
- a few oversized orchestration functions

The previously identified file-level dependency cycle has been removed.

If those are cleaned up, the codebase would move from “strong and safe” toward “exceptionally easy to change.”
