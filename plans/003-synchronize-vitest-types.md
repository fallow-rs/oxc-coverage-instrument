# Plan 003: Make the Vitest declarations match the runtime adapter

> **Executor instructions**: Execute each step and verification in order. Stop
> on a STOP condition and report it. Update this plan's status in
> `plans/README.md` when complete.
>
> **Drift check (run first)**: `git diff --stat 321630c..HEAD -- crates/oxc-coverage-instrument-napi/vitest.d.ts crates/oxc-coverage-instrument-napi/vitest.js crates/oxc-coverage-instrument-napi/package.json examples/vitest-typescript/vitest.config.ts examples/vitest-typescript/tsconfig.json`
> Stop if runtime options or returned values no longer match the current-state
> excerpts.

## Status

- **Status**: DONE
- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `321630c`, 2026-07-13

## Why this matters

The JavaScript adapter accepts `trackOptionalChainBranches`,
`experimentalDecorators`, and `emitDecoratorMetadata`, and the README tells
TypeScript users to pass them. The published declaration omits those options,
so valid documented configurations fail typechecking. Its public return types
also use `any`, removing useful safety from strict TypeScript consumers.

## Current state

- `vitest.js:42-80` documents all runtime options and `vitest.js:91-179`
  validates and forwards them.
- `vitest.d.ts:7-35` declares several options but omits the three named above.
- `vitest.d.ts:57-62` currently exposes:

```ts
instrumentSync(code: string, filename: string, inputSourceMap?: any): string;
lastSourceMap(): any;
lastFileCoverage(): any;
readonly fileCoverage: any;
```

- `examples/vitest-typescript` already depends on TypeScript and uses strict
  compiler settings, but `skipLibCheck: true` means the example does not
  currently validate this declaration file.
- Follow the repository TypeScript rules: no `any`, named exports only,
  explicit return types, `null` for intentional absence.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build binding | `npm --prefix crates/oxc-coverage-instrument-napi run build:debug` | exits 0 |
| Runtime tests | `node crates/oxc-coverage-instrument-napi/test.mjs` | exits 0 |
| Typecheck example | `npm --prefix examples/vitest-typescript exec tsc -- --noEmit` | exits 0 |
| Package surface | `node scripts/npm-pack-surface-check.mjs` | exits 0 |
| Full suite | `cargo test --workspace --all-targets` | exits 0 |

## Scope

**In scope**:

- `crates/oxc-coverage-instrument-napi/vitest.d.ts`
- `examples/vitest-typescript/vitest.config.ts`
- `examples/vitest-typescript/tsconfig.json` only if needed to ensure library
  declarations are checked
- `crates/oxc-coverage-instrument-napi/test.mjs` only for a package-surface or
  runtime assertion that directly protects this contract

**Out of scope**:

- Generated `index.d.ts` from napi-rs.
- Changing runtime defaults or validation.
- Adding a new TypeScript dependency. The example already has TypeScript.
- Broadly modeling every napi-rs export in handwritten types.

## Git workflow

- Branch: `codex/003-synchronize-vitest-types`
- Commit: `fix(napi): synchronize vitest types`
- Use `git commit -S`. Do not push or open a PR unless instructed.

## Steps

### Step 1: Add a strict consumer compile fixture

Update the Vitest TypeScript example so `vitest.config.ts` passes all runtime
options through `createOxcInstrumenter`, including explicit `false` values.
Add type assertions that call `lastSourceMap`, `lastFileCoverage`, and read
`fileCoverage` without casts.

Ensure the example typecheck examines dependency declarations. Prefer a focused
consumer file and command over globally disabling `skipLibCheck` if third-party
packages contain unrelated errors.

**Verify**: before changing `vitest.d.ts`, the typecheck fails specifically on
the missing options or `any` assertions.

### Step 2: Declare every supported option

Add these optional boolean properties with JSDoc matching `vitest.js`:

- `trackOptionalChainBranches`
- `experimentalDecorators`
- `emitDecoratorMetadata`

Do not add options that the adapter does not read. Preserve
`emitDecoratorMetadata` auto-promotion semantics in its documentation.

**Verify**: the missing-option errors disappear.

### Step 3: Replace public `any` with local structural types

Define and export only the JSON and Istanbul structures required by the adapter:

- JSON primitive/value/object types for input source maps
- source-map result type with required `version`, `sources`, `names`, and
  `mappings`, plus JSON-compatible extension fields
- Istanbul `Position`, `Location`, function, branch, and file-coverage shapes
- an explicit returned instrumenter interface

Use `null` for `lastSourceMap`, `lastFileCoverage`, and `fileCoverage` before an
instrumentation result exists. Do not claim a field is always present when
runtime JSON can omit it.

**Verify**:

```bash
rg -n '\bany\b' crates/oxc-coverage-instrument-napi/vitest.d.ts
npm --prefix examples/vitest-typescript exec tsc -- --noEmit
```

The search returns no matches and the typecheck exits 0.

### Step 4: Run runtime and real-project checks

```bash
npm --prefix crates/oxc-coverage-instrument-napi ci
npm --prefix crates/oxc-coverage-instrument-napi run build:debug
node crates/oxc-coverage-instrument-napi/test.mjs
npm --prefix examples/vitest-typescript ci
npm --prefix examples/vitest-typescript run coverage
npm --prefix examples/vitest-typescript run verify
node scripts/npm-pack-surface-check.mjs
cargo test --workspace --all-targets
```

**Verify**: every command exits 0.

## Test plan

- Compile every supported option from a strict TypeScript consumer.
- Include explicit false values so optional defaults are represented correctly.
- Exercise return values both before and after `instrumentSync` in types.
- Run existing runtime adapter tests to prove declarations did not motivate a
  runtime change.
- Use the existing Vitest TypeScript example as real-project validation.

## Done criteria

- [x] Every option read by `createOxcInstrumenter` is declared.
- [x] `vitest.d.ts` contains no `any`.
- [x] A strict consumer compiles all options and return values without casts.
- [x] N-API runtime tests and package-surface checks pass.
- [x] The Vitest TypeScript coverage example passes.
- [x] Full workspace tests pass.
- [x] Only in-scope files and `plans/README.md` are modified.

### Combined-branch resolution

The integrated branch resolves these criteria through the canonical
`vitest-typecheck`, `napi-test`, `package-surface`, `vitest-coverage`,
`vitest-verify`, and `rust-test` profiles. The runtime option list was also
matched field by field against `OxcInstrumenterOptions`, and `vitest.d.ts` was
searched directly for `any`. The final scope criterion applies to the Plan 003
implementation slice, which changed only its listed files and
`plans/README.md`; the combined branch intentionally contains the other plan
slices and does not redefine that completed scope.

## STOP conditions

Stop and report if:

- Accurate types require importing an unpublished generated declaration.
- Third-party declaration failures prevent a focused consumer typecheck.
- Runtime can return a shape that the proposed structural types cannot express
  without `any`; use `unknown` only at a genuinely opaque boundary and report
  the exact boundary.
- A verification command fails twice after a reasonable correction.

## Maintenance notes

Every future option added to `vitest.js` must land with the matching declaration
and strict consumer fixture in the same change. Reviewers should compare the
runtime forwarding object and `OxcInstrumenterOptions` field by field.
