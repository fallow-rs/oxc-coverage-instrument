window.BENCHMARK_DATA = {
  "lastUpdate": 1779779887356,
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
        "date": 1779371942019,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85103784,
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
          "id": "9ab4ae49a0d820197c94c1d53a2760496d403149",
          "message": "chore: release v0.7.0",
          "timestamp": "2026-05-21T16:32:51+02:00",
          "tree_id": "a00631034241cb13a5cc0a723679bad4d8a4bf6f",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/9ab4ae49a0d820197c94c1d53a2760496d403149"
        },
        "date": 1779779384659,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85079496,
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
          "id": "c696091241b46b71e1a94b47e88b95ac3693a7a0",
          "message": "chore: release v0.7.1\n\nRepublishes after v0.7.0 left npm side unpublished. The release-npm.yml\nmatrix called dtolnay/rust-toolchain with toolchain: stable + the cross\ntarget, but rust-toolchain.toml pins the channel to 1.95, so the cross\ntarget rustlib landed in stable's sysroot while cargo (under the repo)\npicked up 1.95 and could not find the target's core crate.\n\n5 of 8 build matrix jobs failed (musl, aarch64-linux-gnu,\naarch64-pc-windows, x86_64-darwin, wasm32-wasip1-threads), the publish\nto npm step was skipped, and crates.io v0.7.0 went out without an\naccompanying npm release.\n\nFix: route both the matrix build and the publish-crate job through\n./.github/actions/setup-rust which honors rust-toolchain.toml first.",
          "timestamp": "2026-05-26T09:16:24+02:00",
          "tree_id": "76276305dcc2e6b6029dc4b8e3db65f2953a677e",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/c696091241b46b71e1a94b47e88b95ac3693a7a0"
        },
        "date": 1779779886388,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85082240,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}