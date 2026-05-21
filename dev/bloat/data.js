window.BENCHMARK_DATA = {
  "lastUpdate": 1779356541963,
  "repoUrl": "https://github.com/fallow-rs/oxc-coverage-instrument",
  "entries": {
    "oxc-coverage-instrument Binary Size": [
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "1896029372ce0f44eb659d0cc6b55e19e33a7c1e",
          "message": "ci: pin dtolnay/rust-toolchain to current stable HEAD\n\nThe previous SHA (631a55b) was the stable HEAD at the time fallow pinned\nit, but dtolnay/rust-toolchain force-pushes its `stable` branch and that\ncommit is no longer reachable from any branch ref. Dependabot's\ngithub-actions scanner does a shallow clone and looks up the container\nbranch for the pinned SHA; with the SHA orphaned, the scan errored out\non every run with `error: no such commit 631a55b...`.\n\nRe-pinning to the current stable HEAD (29eef33) keeps the action working\nand lets dependabot resolve the SHA against `stable` again. We will need\nto re-pin whenever stable advances; tracked as an open follow-up.",
          "timestamp": "2026-05-20T21:27:50+02:00",
          "tree_id": "fa89a6c77dd19392c15e84679f97b3eb4a87b1cc",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/1896029372ce0f44eb659d0cc6b55e19e33a7c1e"
        },
        "date": 1779305432564,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 84243504,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "3df1004b836b1d565132bdcd9309ca28d0ce01c5",
          "message": "chore(deps): bump rayon from 1.11.0 to 1.12.0 (#64)\n\nBumps [rayon](https://github.com/rayon-rs/rayon) from 1.11.0 to 1.12.0.\n- [Changelog](https://github.com/rayon-rs/rayon/blob/main/RELEASES.md)\n- [Commits](https://github.com/rayon-rs/rayon/compare/rayon-core-v1.11.0...rayon-core-v1.12.0)\n\n---\nupdated-dependencies:\n- dependency-name: rayon\n  dependency-version: 1.12.0\n  dependency-type: direct:production\n  update-type: version-update:semver-minor\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-20T20:31:24+01:00",
          "tree_id": "c6950732c80ebbdb45ca67e0092d704e20b91b8a",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/3df1004b836b1d565132bdcd9309ca28d0ce01c5"
        },
        "date": 1779305587850,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 84243336,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "5437ae7eea307f8ff0bad349761e8fc1fc36856f",
          "message": "chore(clippy): port fallow's extra restriction lints + profile tuning\n\n[workspace.lints.rust] adds `unsafe_code = warn` to keep the workspace\nunsafe-free (no unsafe blocks today, no need to acquire one silently).\n\n[workspace.lints.clippy] enables the `cargo` group (catches missing\nmetadata, dependency hygiene drift) and adds 9 restriction lints fallow\ncarries: `empty_drop`, `empty_structs_with_brackets`, `infinite_loop`,\n`pathbuf_init_then_push`, `pub_underscore_fields`, `non_zero_suggestions`,\n`precedence_bits`, `map_with_unused_argument_over_ranges`,\n`filetype_is_file`. `multiple_crate_versions` is explicitly allowed\nbecause indexmap pulls both hashbrown 0.16 and 0.17 transitively and\nneither side is bumpable here.\n\nThe new `precedence_bits` lint caught a real ambiguity in the v8 crate's\nbase64 helper test (`a << 4 | b >> 4`): added explicit parentheses so the\nintent reads as `(a << 4) | (b >> 4)`.\n\nThe new `cargo_common_metadata` lint required each publishable crate to\ndeclare a `readme` path; added `readme = \"../../README.md\"` to the six\npublished library crates (matching fallow's per-crate pattern). The two\npublish=false adapter crates (cli, napi) don't need it.\n\n[profile.test] gains `debug = false` for faster test compile. Dev\nprofile gets per-package opt-level tuning for `serde_derive`,\n`napi-derive` (proc-macros compile once per workspace; bumping their\nopt-level pays back across every dependent crate) and `insta` / `similar`\n(snapshot diff work runs every test).",
          "timestamp": "2026-05-21T09:00:18+02:00",
          "tree_id": "cddacd07a94cfc49cecd34312b76786fdaabfd71",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/5437ae7eea307f8ff0bad349761e8fc1fc36856f"
        },
        "date": 1779346920434,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 84201208,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "96dbc341ffa878c530d2cbc2d0f3f0fb066b7e5d",
          "message": "chore: pin toolchain channel to 1.95\n\nMakes local builds use the same toolchain as CI by default, removing\nthe implicit fallback to whatever `dtolnay/rust-toolchain@stable` picks\non a given day. The MSRV gate stays at 1.92 (declared in\n`[workspace.package]` and enforced by the dedicated `MSRV` CI job),\nso this only pins the lint / formatter floor.",
          "timestamp": "2026-05-21T09:05:52+02:00",
          "tree_id": "28b25ee2c2ed9db3d87a2e102a340de629f6953b",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/96dbc341ffa878c530d2cbc2d0f3f0fb066b7e5d"
        },
        "date": 1779347315256,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 84201208,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "65d74776e65703ea23c007b44d40f05de242742c",
          "message": "feat(api)!: typed DecoratorMode enum + skip synthetic (0,0) branch spans (#85)\n\n* feat(api)!: typed DecoratorMode enum and synthetic-branch filter\n\nReplace the two-boolean decorator surface on `InstrumentOptions`\n(`experimental_decorators` + `emit_decorator_metadata`) with a single\n`DecoratorMode` enum (PassThrough / Experimental / ExperimentalWithMetadata)\nso the invalid combination \"emit metadata without lowering decorators\" is\nunrepresentable on the Rust side. The upstream `oxc_transformer` decorator\npass is gated on legacy mode being on, so emitting metadata without also\nlowering decorators was previously silently promoted.\n\nThe napi surface keeps the familiar two-optional-boolean shape\n(`experimentalDecorators` + `emitDecoratorMetadata`, mirroring\n`tsconfig.json`) and reconstructs the enum in the adapter. The invalid pair\nnow throws a JS `Error` at the boundary instead of being silently promoted.\n\nAlso filter ConditionalExpression and LogicalExpression nodes whose byte\nspan is `(0, 0)` from branch instrumentation. Those are the synthetic\n`typeof X === \"function\" ? X : Object` guards that `oxc_transformer`'s\nlegacy decorator pass injects inside `_decorateMetadata(\"design:paramtypes\",\n[...])` calls; registering them produced four phantom branch entries per\nparameter at L1:C0 and inflated the visible branch denominator for any\nNestJS / TypeORM user who enabled metadata emission. Matches the existing\n`(0, 0)` filter already in place for the synthesized else-arm of a\ntransformed enum IIFE.\n\nCloses #79\nCloses #81\n\n* fix(vitest): auto-promote emitDecoratorMetadata at the adapter layer\n\nWithout this fix, every vitest user who set `emitDecoratorMetadata: true`\non `createOxcInstrumenter({ ... })` without also setting\n`experimentalDecorators: true` would have hit a runtime throw on the first\ninstrumented file: the previous commit tightened the bare napi\n`instrument()` adapter to reject the invalid combination, but the vitest\nadapter kept passing both booleans through unchanged.\n\nAuto-promote at the vitest layer so the JS-facing API matches the\n`tsconfig.json` mental model (TypeScript itself silently enables\n`experimentalDecorators` when only `emitDecoratorMetadata` is set). The\nstrict-rejection contract still holds at the bare napi `instrument()`\nentry point, where the underlying Rust `DecoratorMode` enum keeps invalid\nstates unrepresentable.\n\nAlso update the docstring + README to describe the two-tier contract\n(adapter auto-promotes for ergonomics, bare API rejects for invariant).",
          "timestamp": "2026-05-21T10:40:38+01:00",
          "tree_id": "58cff0e3b859c46ed70ffe82c3eb2719cbface97",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/65d74776e65703ea23c007b44d40f05de242742c"
        },
        "date": 1779356541001,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 84248000,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}