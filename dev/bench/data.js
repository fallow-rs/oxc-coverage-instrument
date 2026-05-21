window.BENCHMARK_DATA = {
  "lastUpdate": 1779356762260,
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
      }
    ]
  }
}