window.BENCHMARK_DATA = {
  "lastUpdate": 1779831519049,
  "repoUrl": "https://github.com/fallow-rs/oxc-coverage-instrument",
  "entries": {
    "oxc-coverage-instrument benchmarks": [
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
        "date": 1779305633241,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 20757,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 44234,
            "range": "± 311",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 123656,
            "range": "± 499",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 273980,
            "range": "± 2365",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 95620,
            "range": "± 954",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 483006,
            "range": "± 10852",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 273720,
            "range": "± 3251",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 419148,
            "range": "± 5127",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 56202,
            "range": "± 184",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 273699,
            "range": "± 5716",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 543726,
            "range": "± 3812",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2854648,
            "range": "± 9825",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 36105,
            "range": "± 548",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 32360,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 464216,
            "range": "± 7916",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 414698,
            "range": "± 7854",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 160292,
            "range": "± 2256",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 146023,
            "range": "± 1105",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 859162,
            "range": "± 19371",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 773086,
            "range": "± 3624",
            "unit": "ns/iter"
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
        "date": 1779305928682,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 16544,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 35261,
            "range": "± 256",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 96794,
            "range": "± 591",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 216166,
            "range": "± 5305",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 75257,
            "range": "± 694",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 375854,
            "range": "± 3434",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 213973,
            "range": "± 2096",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 322308,
            "range": "± 3216",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 43754,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 217145,
            "range": "± 2551",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 425486,
            "range": "± 14157",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2205786,
            "range": "± 5202",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 27953,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 25590,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 368211,
            "range": "± 3505",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 318336,
            "range": "± 5954",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 124883,
            "range": "± 1244",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 113558,
            "range": "± 1019",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 670534,
            "range": "± 2304",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 602633,
            "range": "± 15225",
            "unit": "ns/iter"
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
        "date": 1779347137884,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 23028,
            "range": "± 270",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 51045,
            "range": "± 1627",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 149686,
            "range": "± 4437",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 314854,
            "range": "± 5338",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 128091,
            "range": "± 2641",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 527510,
            "range": "± 4731",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 315654,
            "range": "± 4841",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 470584,
            "range": "± 4588",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 57771,
            "range": "± 200",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 282713,
            "range": "± 2939",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 558553,
            "range": "± 4782",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2831061,
            "range": "± 30274",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 41846,
            "range": "± 324",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 37670,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 521437,
            "range": "± 2745",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 473367,
            "range": "± 3511",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 198886,
            "range": "± 1534",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 183499,
            "range": "± 1720",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 877621,
            "range": "± 4811",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 791215,
            "range": "± 8453",
            "unit": "ns/iter"
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
        "date": 1779347518321,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 21131,
            "range": "± 620",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 45510,
            "range": "± 257",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 124206,
            "range": "± 3656",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 275348,
            "range": "± 2484",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 97653,
            "range": "± 558",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 492218,
            "range": "± 3342",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 276660,
            "range": "± 2257",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 424913,
            "range": "± 3649",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 56238,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 275020,
            "range": "± 1594",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 546731,
            "range": "± 4744",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2842192,
            "range": "± 16503",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 36369,
            "range": "± 197",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 32866,
            "range": "± 669",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 480631,
            "range": "± 9603",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 437299,
            "range": "± 6381",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 162664,
            "range": "± 2583",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 147807,
            "range": "± 1318",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 860838,
            "range": "± 3861",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 775761,
            "range": "± 23180",
            "unit": "ns/iter"
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
        "date": 1779356761687,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 22568,
            "range": "± 308",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 52193,
            "range": "± 782",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 149416,
            "range": "± 2658",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 307219,
            "range": "± 8724",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 126248,
            "range": "± 934",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 530375,
            "range": "± 5064",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 312308,
            "range": "± 2458",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 464180,
            "range": "± 4426",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 58378,
            "range": "± 549",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 286427,
            "range": "± 1389",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 556138,
            "range": "± 3230",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2847067,
            "range": "± 19556",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 42658,
            "range": "± 372",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 37407,
            "range": "± 297",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 514364,
            "range": "± 24748",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 469548,
            "range": "± 7871",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 194141,
            "range": "± 1114",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 179700,
            "range": "± 1113",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 864929,
            "range": "± 4665",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 779057,
            "range": "± 14964",
            "unit": "ns/iter"
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
          "id": "3adf3eb38e9bef9105f6052efa7f0ee9a8e6b807",
          "message": "feat(napi): add wasm32-wasi build target (#91)\n\n* feat(napi): add wasm32-wasi build target\n\nShips @oxc-coverage-instrument/binding-wasm32-wasi as an automatic\nfallback when no matching native binding is available. Target is\nwasm32-wasip1-threads (napi-rs 3's officially-supported variant).\n\nBuild matrix:\n- New ubuntu-latest entry in release-npm.yml producing the wasm binary\n  plus four shim files (coverage-instrument.wasi.cjs, wasi-browser.js,\n  wasi-worker.mjs, wasi-worker-browser.mjs)\n- New artifact upload globs cover the .wasm + shim files\n- napi artifacts now uses --build-output-dir so the shim files are\n  correctly moved into npm/wasm32-wasi/ during publish\n- Explicit \"Restore wasm shim files to root package\" step ensures the\n  five wasm shims are present in the root tarball before npm publish\n  (the default napi-rs publish flow only restores index.js/index.d.ts)\n\nCI gate:\n- New napi-wasm job builds the wasm target and runs the full 36-test\n  napi suite under NAPI_RS_FORCE_WASI=error on every PR\n- Hard size ceiling at 2 MB brotli enforces issue #46's <2 MB\n  compressed acceptance target (current baseline 0.58 MB, 80% headroom)\n\nPackage surface:\n- package.json declares browser export pointing at browser.js\n- @napi-rs/wasm-runtime added as a runtime dependency (matches the\n  oxlint / rspack napi-rs 3 ecosystem precedent)\n- @oxc-coverage-instrument/binding-wasm32-wasi added to optionalDeps\n- npm/wasm32-wasi/ ships committed scaffolding (package.json + README)\n\nRefs #46\n\n* docs(readme): runtime matrix and NAPI_RS_FORCE_WASI usage\n\nAdds a \"Runtime matrix\" subsection to Compatibility documenting which\nruntimes get the native binding, which get the wasm fallback, and which\nare not yet supported.\n\n- Browser row calls out the COOP/COEP + SharedArrayBuffer requirement\n  and the bundler matrix (Vite 2+, webpack 5 with topLevelAwait,\n  esbuild, rollup). webpack 4 and Parcel 1 are explicitly unsupported\n  due to top-level await in the generated wasi-browser shim.\n- Cloudflare Workers and Deno Deploy / StackBlitz rows link to the\n  follow-up tracker issues (#87 and #88).\n- Bun row references oven-sh/bun#16156 (incomplete node:wasi) as the\n  reason the native binding is preferred there.\n\nAdds a \"Forcing the WASM binding\" subsection with a table mapping the\ntwo NAPI_RS_FORCE_WASI values: =1 (soft-fall to native if wasm fails)\nvs =error (hard-fail with diagnostic; what CI uses).\n\nRefs #46\n\n* test(examples): wasm-node end-to-end smoke\n\nShips examples/wasm-node/ as a runnable smoke for the wasm binding.\n\n- package.json defines two scripts: `smoke` (uses native binding when\n  available) and `smoke:wasi` (forces NAPI_RS_FORCE_WASI=error and\n  fails fast if the wasi binding cannot load).\n- scripts/smoke.mjs calls instrument() on a small TypeScript-flavored\n  fixture and asserts the coverage map carries statements, functions,\n  and branches, plus a source map when sourceMap: true.\n- README.md documents the supported runtimes per binding, with a\n  per-runtime status table mirroring the project README.\n\nUsed as the cookbook entry point for downstream consumers verifying\nthat the wasm fallback works in their environment. The same smoke can\nserve as the basis for a future CI gate (tracked in #90).\n\nRefs #46",
          "timestamp": "2026-05-21T14:54:52+01:00",
          "tree_id": "6b75a6a852f378ea84fc282a8608bfd29b2993c9",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/3adf3eb38e9bef9105f6052efa7f0ee9a8e6b807"
        },
        "date": 1779372054075,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 20943,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 45041,
            "range": "± 314",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 122194,
            "range": "± 2252",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 275343,
            "range": "± 1722",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 95047,
            "range": "± 618",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 477438,
            "range": "± 5072",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 273898,
            "range": "± 4496",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 414907,
            "range": "± 3258",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 55647,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 273573,
            "range": "± 8007",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 537298,
            "range": "± 1952",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2817598,
            "range": "± 10530",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 36667,
            "range": "± 472",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 32998,
            "range": "± 284",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 463298,
            "range": "± 7192",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 419464,
            "range": "± 4356",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 161176,
            "range": "± 2259",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 146082,
            "range": "± 1047",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 858953,
            "range": "± 4739",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 771350,
            "range": "± 5983",
            "unit": "ns/iter"
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
          "id": "e3a5077a9917dab8dfcc4e4e6436b9d6eac7fa4b",
          "message": "feat(types): opt-in x_fallow_functionMap identity overlay (#86)\n\n* feat(types): opt-in x_fallow_functionMap identity overlay\n\nAdd an optional non-Istanbul extension on `FileCoverage` that carries a\nstable `fallow:fn:<hex>` identity per function, keyed by the same ids as\n`fnMap`. The id is a `djb31_hex` digest of `(path, name, decl span, loc\nspan)` so two runs over byte-identical source produce identical ids and\nrenames / body edits / line shifts all change the id.\n\nGated by a new `InstrumentOptions::function_identity_overlay: bool`\n(default false) on the Rust API and `functionIdentityOverlay?: boolean`\non the napi surface. With the option off the JSON output stays\nbyte-identical to what Istanbul consumers (nyc, Vitest, Jest, Codecov)\nexpect; the `x_`-prefixed key on the overlay also makes the field a\nno-op for spec-compliant Istanbul parsers when present.\n\nDesigned for downstream code-quality tools (Fallow et al.) that need a\nlong-lived join key across AST inventories, runtime coverage beacons,\nand source-mapped positions without reconstructing identity from\n`(path, name, line, column)` after the fact. When `inputSourceMap` is\nconsumed the overlay still references pre-remap positions; consumers\nthat remap downstream must recompute identity against the post-remap\npositions (documented in the README + the `FunctionIdentity` rustdoc).\n\nTests cover named / anonymous / same-line / class-method / object-method\n/ arrow function shapes, the TS-direct strip path (positions remain\noriginal TS offsets), determinism across runs, identity change on\nposition shift, identity change on path change, and overlay omission /\npresence at the JSON-key level.\n\nCloses #78\n\n* fix(napi): forward function identity overlay in vitest adapter\n\n* fix(overlay): JSON-encode hash input + document path normalization\n\nSwitch `build_function_identity_map` from a `|`-delimited\n`format!(\"{path}|{name}|{lines}...\")` hash input to\n`serde_json::to_string(&json!([path, name, ...]))`. The flat-string\nshape would collide on adversarial pairs like `(path=\"a\", name=\"b|c\")`\nvs `(path=\"a|b\", name=\"c\")` because both flatten to `\"a|b|c|...\"`.\nJSON-encoding quotes every string so the field boundaries survive any\ncharacter, including the computed-key methods that empirically land in\n`fn_map[].name` with `|` literally inside (verified on\n`class C { ['x|y']() {} }`).\n\nRegression test lives next to the helper as a unit test: two\nsynthetic `FnEntry` inputs whose `|`-concat would tie, asserted to\nhash to distinct ids. The integration test in\n`function_identity_overlay_test.rs` is reframed as a \"weird names\ndon't crash\" smoke since the upstream-shape smoke alone cannot prove\nboundary-resistance (different source lengths shift other parts of\nthe hash and mask the collision).\n\nAlso document the path-normalization caveat on both the README and\nthe `FunctionIdentity` rustdoc: the path enters the hash verbatim\nfrom the `filename` argument, so callers that need stable ids across\ntools must normalise paths (`./app.js` and `app.js` hash differently).",
          "timestamp": "2026-05-21T14:55:52+01:00",
          "tree_id": "0f8c7332209e7b436fa38793100beeda732418fb",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/e3a5077a9917dab8dfcc4e4e6436b9d6eac7fa4b"
        },
        "date": 1779372377393,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 23295,
            "range": "± 341",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 51384,
            "range": "± 301",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 148843,
            "range": "± 820",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 312978,
            "range": "± 2244",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 125651,
            "range": "± 976",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 521788,
            "range": "± 15764",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 312921,
            "range": "± 3091",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 462918,
            "range": "± 7383",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 57273,
            "range": "± 588",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 282006,
            "range": "± 1634",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 553250,
            "range": "± 15113",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2780159,
            "range": "± 18849",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 41676,
            "range": "± 320",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 37494,
            "range": "± 233",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 514248,
            "range": "± 3603",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 467121,
            "range": "± 2193",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 195704,
            "range": "± 1924",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 181147,
            "range": "± 1716",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 860107,
            "range": "± 3098",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 776496,
            "range": "± 6739",
            "unit": "ns/iter"
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
          "id": "9ab4ae49a0d820197c94c1d53a2760496d403149",
          "message": "chore: release v0.7.0",
          "timestamp": "2026-05-21T16:32:51+02:00",
          "tree_id": "a00631034241cb13a5cc0a723679bad4d8a4bf6f",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/9ab4ae49a0d820197c94c1d53a2760496d403149"
        },
        "date": 1779779600824,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 21037,
            "range": "± 380",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 45342,
            "range": "± 303",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 124691,
            "range": "± 1548",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 274022,
            "range": "± 6902",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 96911,
            "range": "± 1952",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 491460,
            "range": "± 5496",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 278496,
            "range": "± 2308",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 417009,
            "range": "± 7820",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 56308,
            "range": "± 1942",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 276913,
            "range": "± 3581",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 547284,
            "range": "± 2819",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2826349,
            "range": "± 7767",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 36409,
            "range": "± 480",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 32855,
            "range": "± 578",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 474604,
            "range": "± 13365",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 413576,
            "range": "± 6198",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 164361,
            "range": "± 2388",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 149080,
            "range": "± 2057",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 870161,
            "range": "± 7146",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 785566,
            "range": "± 12754",
            "unit": "ns/iter"
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
          "id": "c696091241b46b71e1a94b47e88b95ac3693a7a0",
          "message": "chore: release v0.7.1\n\nRepublishes after v0.7.0 left npm side unpublished. The release-npm.yml\nmatrix called dtolnay/rust-toolchain with toolchain: stable + the cross\ntarget, but rust-toolchain.toml pins the channel to 1.95, so the cross\ntarget rustlib landed in stable's sysroot while cargo (under the repo)\npicked up 1.95 and could not find the target's core crate.\n\n5 of 8 build matrix jobs failed (musl, aarch64-linux-gnu,\naarch64-pc-windows, x86_64-darwin, wasm32-wasip1-threads), the publish\nto npm step was skipped, and crates.io v0.7.0 went out without an\naccompanying npm release.\n\nFix: route both the matrix build and the publish-crate job through\n./.github/actions/setup-rust which honors rust-toolchain.toml first.",
          "timestamp": "2026-05-26T09:16:24+02:00",
          "tree_id": "76276305dcc2e6b6029dc4b8e3db65f2953a677e",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/c696091241b46b71e1a94b47e88b95ac3693a7a0"
        },
        "date": 1779780150954,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 20920,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 50541,
            "range": "± 154",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 138906,
            "range": "± 1994",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 280303,
            "range": "± 892",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 116285,
            "range": "± 463",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 483222,
            "range": "± 2372",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 282006,
            "range": "± 2982",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 447246,
            "range": "± 3959",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 53921,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 255111,
            "range": "± 12507",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 502014,
            "range": "± 1967",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2668106,
            "range": "± 8984",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 37714,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 34166,
            "range": "± 953",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 490505,
            "range": "± 3472",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 448554,
            "range": "± 3666",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 181036,
            "range": "± 1107",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 168285,
            "range": "± 832",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 839709,
            "range": "± 1633",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 768041,
            "range": "± 3015",
            "unit": "ns/iter"
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
          "id": "071c0a0aeafd0de298caf6cc611bb4fbffb8d73e",
          "message": "chore: release v0.7.2",
          "timestamp": "2026-05-26T11:09:49+02:00",
          "tree_id": "8613231d3307709c20506380e7ff0fb532fd43d2",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/071c0a0aeafd0de298caf6cc611bb4fbffb8d73e"
        },
        "date": 1779786927487,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 22608,
            "range": "± 163",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 51880,
            "range": "± 429",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 147752,
            "range": "± 3139",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 313497,
            "range": "± 2043",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 126312,
            "range": "± 1211",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 533591,
            "range": "± 23171",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 315924,
            "range": "± 2759",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 469899,
            "range": "± 4068",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 57751,
            "range": "± 424",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 285677,
            "range": "± 1511",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 557878,
            "range": "± 3131",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2848367,
            "range": "± 30319",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 41654,
            "range": "± 755",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 37227,
            "range": "± 1137",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 519876,
            "range": "± 5314",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 473057,
            "range": "± 4223",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 199757,
            "range": "± 1620",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 185779,
            "range": "± 4506",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 869560,
            "range": "± 4672",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 787486,
            "range": "± 5345",
            "unit": "ns/iter"
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
          "id": "043b6d96c838d5e6794cd271b79bef650f3eea58",
          "message": "fix(napi): clearer error when remapCoverageMap receives a FileCoverage\n\nremapCoverageMap and remapCoverageMapWithLoader expect an Istanbul\nCoverageMap shape ({[path]: FileCoverage}), but the raw serde_json error\nreads \"expected struct FileCoverage\" when the caller passes a single\nFileCoverage. The message is technically correct but unhelpful: it\npoints at the FileCoverage shape rather than the wrong outer container,\nso users end up rechecking their FileCoverage shape instead of wrapping\ntheir input.\n\nWhen parse_coverage_map fails, peek the JSON for a single FileCoverage\nshape and append a hint pointing at `{ [fc.path]: fc }` wrapping.\n\nCaught during the v0.7.2 smoke test. Adds oxc_coverage_types as a path\ndep on the napi crate (already in the workspace, just not previously\nreferenced from napi); napi crate is publish = false so no version\nconstraint needed.",
          "timestamp": "2026-05-26T11:36:10+02:00",
          "tree_id": "a91801d8cd0c15d3a7e298696dbb6c60e0780d19",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/043b6d96c838d5e6794cd271b79bef650f3eea58"
        },
        "date": 1779788493618,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 22588,
            "range": "± 862",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 50794,
            "range": "± 475",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 144700,
            "range": "± 624",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 310424,
            "range": "± 1513",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 124472,
            "range": "± 3247",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 523391,
            "range": "± 3529",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 308342,
            "range": "± 1697",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 466032,
            "range": "± 4668",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 57454,
            "range": "± 470",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 283287,
            "range": "± 2450",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 553150,
            "range": "± 3167",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2807418,
            "range": "± 9690",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 41901,
            "range": "± 202",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 37588,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 516622,
            "range": "± 6731",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 461680,
            "range": "± 4015",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 196665,
            "range": "± 1234",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 181734,
            "range": "± 816",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 864886,
            "range": "± 2498",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 784429,
            "range": "± 10116",
            "unit": "ns/iter"
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
          "id": "27c6c49a05080bf319eacc8326f817d5c9d48662",
          "message": "feat(source-maps): RemapOptions { drop_unmapped } for remap helpers (#93)\n\nAdds an opt-in flag that prunes statement / function / branch entries\n(and their matching s / f / b / bT slots) when their positions cannot\nbe looked up in the inputSourceMap, instead of silently keeping the\ngenerated-output coordinates. Drop semantics match\nistanbul-lib-source-maps's transformer.js: statements drop when start\nor end fails; functions drop when any of decl / loc start or end\nfails; branch arms drop per arm, the whole branch drops only when no\narms survive or the umbrella loc start / end fails.\n\nSurfaced via RemapOptions on both the Rust and napi APIs. Existing\nfns stay as zero-overhead wrappers; the new third arg on\nremapCoverageMap / remapCoverageMapWithLoader is optional so the JS\nsurface is backwards compatible.\n\nUse case (from the issue): Vue 3 + Vite coverage where OXC instruments\ncompiler-emitted boilerplate in the ?vue&type=script chunk that has\nno mapping back to the .vue source. The unmapped positions otherwise\nsurvive at chunk-line coordinates and istanbul-reports renders them\nagainst the .vue path on lines that belong to <template> or CSS.\n\nCloses #92",
          "timestamp": "2026-05-26T11:04:44+01:00",
          "tree_id": "2949f177bb31490e2ba8e040cee014f00e2f54bd",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/27c6c49a05080bf319eacc8326f817d5c9d48662"
        },
        "date": 1779790203811,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 20859,
            "range": "± 699",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 44401,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 124051,
            "range": "± 2576",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 270715,
            "range": "± 2466",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 97959,
            "range": "± 775",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 475239,
            "range": "± 3399",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 272233,
            "range": "± 2391",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 415475,
            "range": "± 3217",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 55497,
            "range": "± 187",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 274749,
            "range": "± 1567",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 549425,
            "range": "± 1704",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2809822,
            "range": "± 26693",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 36164,
            "range": "± 312",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 32656,
            "range": "± 124",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 467476,
            "range": "± 5683",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 417658,
            "range": "± 3962",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 163827,
            "range": "± 1385",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 150474,
            "range": "± 2215",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 856892,
            "range": "± 5422",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 771173,
            "range": "± 7090",
            "unit": "ns/iter"
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
          "id": "2cf6f2c0f34e8bd0455aa8be3854152e5bfb0ff6",
          "message": "fix(napi): make remapCoverageMap hint detection wasi-safe (#94)\n\n* fix(napi): make remapCoverageMap hint detection wasi-safe\n\nThe hint that nudges callers from `remapCoverageMap(fileCoverage)` to\n`remapCoverageMap({ [fc.path]: fc })` (added in #67 / v0.7.2) was gated\non `serde_json::from_str::<FileCoverage>(coverage_json).is_ok()`. That\npredicate returned Err under the wasm32-wasi napi binding even when\nthe same JSON deserialized fine on the native binding, so the hint\nsilently disappeared on wasi. The Napi Test (wasm32-wasi) CI job has\nbeen red on main since v0.7.2 because of this.\n\nSwitch to a shape check on the parsed `serde_json::Value`: top-level\nobject with a string `path` plus the canonical Istanbul map keys\n(`statementMap`, `fnMap`, `branchMap`). This is platform-agnostic, much\ncheaper than a full struct parse, and the false-positive rate is\neffectively zero for realistic CoverageMap inputs since every\nCoverageMap key is a path string whose value is an object (the outer\ncontainer itself never has a `path: string` field at the top level).\n\nAlso drop `[lib].test = false` and add `rlib` to crate-type so the new\npure helper can be unit-tested in-process (5 tests covering single\nFileCoverage detection, CoverageMap rejection, non-object roots,\nmalformed JSON, and partial-shape rejection).\n\nRefs #67\n\n* fix(napi): drop unused oxc_coverage_types dep\n\nThe refactor to use a serde_json::Value-based heuristic in\nlooks_like_single_file_coverage removed the only consumer of\noxc_coverage_types in this crate. Drop the dep so the\nUnused Dependencies CI job stays green.\n\n* fix: regenerate Cargo.lock after dropping oxc_coverage_types from napi crate\n\n* debug(napi): instrument hint detection to surface wasi divergence\n\nTemporarily prepend [shape_check=true|false] to invalid-coverage-JSON\nerrors so CI logs reveal whether looks_like_single_file_coverage is\nreturning false on wasi (suggesting the Value-based check itself\nfails) or returning true (suggesting napi/wasi truncates the hint\nstring in the rendered error message). Will be reverted once the\nroot cause is identified.\n\n* test(napi): log presence of local napi binaries to diagnose wasi loader fallback\n\nSurface which local artifacts exist at test time so CI logs reveal\nwhen the loader silently falls back to a published optionalDependency\nbecause the freshly-built wasm wasn't placed at the expected path.\n\n* ci: nudge GitHub Actions\n\n* fix(ci): force wasi job to load freshly-built local wasm\n\nThe wasm32-wasi CI job has been loading the previously published\n@oxc-coverage-instrument/binding-wasm32-wasi (the pinned\noptionalDependency) instead of the freshly-built local artifact. The\nloader in coverage-instrument.wasi.cjs falls back to the published\npackage's wasm whenever it cannot find the local file at the expected\npath, so PR changes to the napi crate never reached the wasi test.\nThis is why Napi Test (wasm32-wasi) reported red on the original v0.7.2\nhint regression even after the Rust-side fix landed in this PR's\nearlier commits.\n\nRemove the published wasm from node_modules between the build and\ntest steps so the loader can only use the freshly-built local\nartifact. If napi build failed to produce it, the loader fails loudly\ninstead of silently exercising the previous release.\n\nAlso revert the diagnostic [shape_check=<bool>] prefix on the error\nmessage; with the loader fix in place it is no longer needed to tell\nwhich binary is in use.\n\n* ci: re-trigger workflows after GitHub Actions delay",
          "timestamp": "2026-05-26T14:15:50+01:00",
          "tree_id": "792de65693455ff3127c9f82c828441404e562e0",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/2cf6f2c0f34e8bd0455aa8be3854152e5bfb0ff6"
        },
        "date": 1779801676962,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 20680,
            "range": "± 206",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 44470,
            "range": "± 872",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 124060,
            "range": "± 5079",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 271949,
            "range": "± 4401",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 97217,
            "range": "± 1604",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 486629,
            "range": "± 3929",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 274013,
            "range": "± 2163",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 423974,
            "range": "± 4286",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 55647,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 276421,
            "range": "± 5558",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 547025,
            "range": "± 1936",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2815254,
            "range": "± 43186",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 36319,
            "range": "± 213",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 33130,
            "range": "± 530",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 467899,
            "range": "± 4054",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 425557,
            "range": "± 11322",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 163464,
            "range": "± 2618",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 148686,
            "range": "± 1092",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 869772,
            "range": "± 3763",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 785527,
            "range": "± 4001",
            "unit": "ns/iter"
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
          "id": "df0eb0c85c719dfc7dc379af289955e685ca2bec",
          "message": "chore(napi): rebrand startup binary-presence log from [diag] to [napi-artifact]\n\nThe temporary [diag] tag added during PR #94's wasi-loader debugging\nread as debug output, but the underlying check (logging which local\nnapi artifacts exist before the binding loader runs) is load-bearing:\nit surfaces silent fallbacks to the published optionalDependency\nbinary on the first failing CI run instead of after hours of remote\ndiagnosis. Keep the log, drop the debug-flavoured tag, and document\nthe safeguard in a comment so future readers don't strip it.",
          "timestamp": "2026-05-26T15:25:43+02:00",
          "tree_id": "66728a10048caac10a891e20548f66b236deb121",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/df0eb0c85c719dfc7dc379af289955e685ca2bec"
        },
        "date": 1779802249921,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 16268,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 34978,
            "range": "± 107",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 96661,
            "range": "± 440",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 211284,
            "range": "± 6796",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 75031,
            "range": "± 431",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 368738,
            "range": "± 4160",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 214133,
            "range": "± 1257",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 317172,
            "range": "± 2456",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 43457,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 215224,
            "range": "± 800",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 424074,
            "range": "± 2481",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2205899,
            "range": "± 4812",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 28051,
            "range": "± 835",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 25644,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 360830,
            "range": "± 3056",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 319778,
            "range": "± 4121",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 124265,
            "range": "± 911",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 113644,
            "range": "± 656",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 664955,
            "range": "± 3026",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 599210,
            "range": "± 5960",
            "unit": "ns/iter"
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
          "id": "8922226afbe7dc754c4e7c3e62118f8f4dcd08b6",
          "message": "feat(napi): guard SharedArrayBuffer before WASM instantiation (#96)\n\nThe browser WASM shim regenerated by napi build allocates new\nWebAssembly.Memory({ shared: true }) at module top level, which throws\nan opaque TypeError when the host page lacks cross-origin isolation\n(no COOP/COEP headers, no SharedArrayBuffer).\n\nInject a guard at the top of coverage-instrument.wasi-browser.js via a\npostbuild script so the failure surfaces a precise diagnostic pointing\nat the host-page fix. Wire the script into the napi crate's build /\nbuild:debug / test pipelines and into the wasm matrix entries of\nrelease-npm.yml and ci.yml. The script is idempotent (sentinel\ncomment) and a no-op on non-wasm targets.",
          "timestamp": "2026-05-26T14:58:50+01:00",
          "tree_id": "755ebe68d4fc7551470246fc974065d2bc84acf1",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/8922226afbe7dc754c4e7c3e62118f8f4dcd08b6"
        },
        "date": 1779804252882,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 20795,
            "range": "± 161",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 44922,
            "range": "± 337",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 125256,
            "range": "± 4619",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 273304,
            "range": "± 2277",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 97482,
            "range": "± 2077",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 487882,
            "range": "± 5253",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 277064,
            "range": "± 2948",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 415389,
            "range": "± 4790",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 55581,
            "range": "± 518",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 274471,
            "range": "± 2886",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 546482,
            "range": "± 2642",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2851811,
            "range": "± 9747",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 36016,
            "range": "± 311",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 32735,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 472044,
            "range": "± 9211",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 423795,
            "range": "± 4804",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 163080,
            "range": "± 3157",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 147678,
            "range": "± 876",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 867306,
            "range": "± 4790",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 779635,
            "range": "± 8139",
            "unit": "ns/iter"
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
          "id": "fe1c0cd7f1a60af2201305c16e56e180c4c3fab9",
          "message": "feat: add single-threaded WASI binding for Workers\n\nCloses #87.",
          "timestamp": "2026-05-26T22:30:20+01:00",
          "tree_id": "fcc0acea8da9f0ec7fb7755b943af17ca42ccee5",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/fe1c0cd7f1a60af2201305c16e56e180c4c3fab9"
        },
        "date": 1779831518771,
        "tool": "cargo",
        "benches": [
          {
            "name": "instrument/file/small_pragma",
            "value": 22611,
            "range": "± 208",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/small_while",
            "value": 52117,
            "range": "± 325",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_react",
            "value": 146212,
            "range": "± 1701",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_app",
            "value": 310060,
            "range": "± 3075",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/medium_typescript",
            "value": 124814,
            "range": "± 682",
            "unit": "ns/iter"
          },
          {
            "name": "instrument/file/large_module",
            "value": 527162,
            "range": "± 2027",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/without_source_map",
            "value": 309852,
            "range": "± 1234",
            "unit": "ns/iter"
          },
          {
            "name": "source_map/with_source_map",
            "value": 466755,
            "range": "± 2709",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/10",
            "value": 57676,
            "range": "± 676",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/50",
            "value": 281059,
            "range": "± 980",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/100",
            "value": 552088,
            "range": "± 1868",
            "unit": "ns/iter"
          },
          {
            "name": "scaling/functions/500",
            "value": 2796958,
            "range": "± 12459",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/small_pragma",
            "value": 41325,
            "range": "± 757",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/small_pragma",
            "value": 37096,
            "range": "± 496",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_app",
            "value": 515105,
            "range": "± 2974",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_app",
            "value": 464748,
            "range": "± 1763",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/medium_typescript",
            "value": 195119,
            "range": "± 944",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/medium_typescript",
            "value": 180602,
            "range": "± 1968",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/legacy/large_module",
            "value": 858520,
            "range": "± 29498",
            "unit": "ns/iter"
          },
          {
            "name": "napi_path/cached/large_module",
            "value": 774465,
            "range": "± 2402",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}