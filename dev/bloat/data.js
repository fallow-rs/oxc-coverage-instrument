window.BENCHMARK_DATA = {
  "lastUpdate": 1784555213178,
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
        "date": 1779786706207,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85414120,
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
          "id": "043b6d96c838d5e6794cd271b79bef650f3eea58",
          "message": "fix(napi): clearer error when remapCoverageMap receives a FileCoverage\n\nremapCoverageMap and remapCoverageMapWithLoader expect an Istanbul\nCoverageMap shape ({[path]: FileCoverage}), but the raw serde_json error\nreads \"expected struct FileCoverage\" when the caller passes a single\nFileCoverage. The message is technically correct but unhelpful: it\npoints at the FileCoverage shape rather than the wrong outer container,\nso users end up rechecking their FileCoverage shape instead of wrapping\ntheir input.\n\nWhen parse_coverage_map fails, peek the JSON for a single FileCoverage\nshape and append a hint pointing at `{ [fc.path]: fc }` wrapping.\n\nCaught during the v0.7.2 smoke test. Adds oxc_coverage_types as a path\ndep on the napi crate (already in the workspace, just not previously\nreferenced from napi); napi crate is publish = false so no version\nconstraint needed.",
          "timestamp": "2026-05-26T11:36:10+02:00",
          "tree_id": "a91801d8cd0c15d3a7e298696dbb6c60e0780d19",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/043b6d96c838d5e6794cd271b79bef650f3eea58"
        },
        "date": 1779788277887,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85414120,
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
          "id": "27c6c49a05080bf319eacc8326f817d5c9d48662",
          "message": "feat(source-maps): RemapOptions { drop_unmapped } for remap helpers (#93)\n\nAdds an opt-in flag that prunes statement / function / branch entries\n(and their matching s / f / b / bT slots) when their positions cannot\nbe looked up in the inputSourceMap, instead of silently keeping the\ngenerated-output coordinates. Drop semantics match\nistanbul-lib-source-maps's transformer.js: statements drop when start\nor end fails; functions drop when any of decl / loc start or end\nfails; branch arms drop per arm, the whole branch drops only when no\narms survive or the umbrella loc start / end fails.\n\nSurfaced via RemapOptions on both the Rust and napi APIs. Existing\nfns stay as zero-overhead wrappers; the new third arg on\nremapCoverageMap / remapCoverageMapWithLoader is optional so the JS\nsurface is backwards compatible.\n\nUse case (from the issue): Vue 3 + Vite coverage where OXC instruments\ncompiler-emitted boilerplate in the ?vue&type=script chunk that has\nno mapping back to the .vue source. The unmapped positions otherwise\nsurvive at chunk-line coordinates and istanbul-reports renders them\nagainst the .vue path on lines that belong to <template> or CSS.\n\nCloses #92",
          "timestamp": "2026-05-26T11:04:44+01:00",
          "tree_id": "2949f177bb31490e2ba8e040cee014f00e2f54bd",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/27c6c49a05080bf319eacc8326f817d5c9d48662"
        },
        "date": 1779789983982,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85427152,
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
          "id": "2cf6f2c0f34e8bd0455aa8be3854152e5bfb0ff6",
          "message": "fix(napi): make remapCoverageMap hint detection wasi-safe (#94)\n\n* fix(napi): make remapCoverageMap hint detection wasi-safe\n\nThe hint that nudges callers from `remapCoverageMap(fileCoverage)` to\n`remapCoverageMap({ [fc.path]: fc })` (added in #67 / v0.7.2) was gated\non `serde_json::from_str::<FileCoverage>(coverage_json).is_ok()`. That\npredicate returned Err under the wasm32-wasi napi binding even when\nthe same JSON deserialized fine on the native binding, so the hint\nsilently disappeared on wasi. The Napi Test (wasm32-wasi) CI job has\nbeen red on main since v0.7.2 because of this.\n\nSwitch to a shape check on the parsed `serde_json::Value`: top-level\nobject with a string `path` plus the canonical Istanbul map keys\n(`statementMap`, `fnMap`, `branchMap`). This is platform-agnostic, much\ncheaper than a full struct parse, and the false-positive rate is\neffectively zero for realistic CoverageMap inputs since every\nCoverageMap key is a path string whose value is an object (the outer\ncontainer itself never has a `path: string` field at the top level).\n\nAlso drop `[lib].test = false` and add `rlib` to crate-type so the new\npure helper can be unit-tested in-process (5 tests covering single\nFileCoverage detection, CoverageMap rejection, non-object roots,\nmalformed JSON, and partial-shape rejection).\n\nRefs #67\n\n* fix(napi): drop unused oxc_coverage_types dep\n\nThe refactor to use a serde_json::Value-based heuristic in\nlooks_like_single_file_coverage removed the only consumer of\noxc_coverage_types in this crate. Drop the dep so the\nUnused Dependencies CI job stays green.\n\n* fix: regenerate Cargo.lock after dropping oxc_coverage_types from napi crate\n\n* debug(napi): instrument hint detection to surface wasi divergence\n\nTemporarily prepend [shape_check=true|false] to invalid-coverage-JSON\nerrors so CI logs reveal whether looks_like_single_file_coverage is\nreturning false on wasi (suggesting the Value-based check itself\nfails) or returning true (suggesting napi/wasi truncates the hint\nstring in the rendered error message). Will be reverted once the\nroot cause is identified.\n\n* test(napi): log presence of local napi binaries to diagnose wasi loader fallback\n\nSurface which local artifacts exist at test time so CI logs reveal\nwhen the loader silently falls back to a published optionalDependency\nbecause the freshly-built wasm wasn't placed at the expected path.\n\n* ci: nudge GitHub Actions\n\n* fix(ci): force wasi job to load freshly-built local wasm\n\nThe wasm32-wasi CI job has been loading the previously published\n@oxc-coverage-instrument/binding-wasm32-wasi (the pinned\noptionalDependency) instead of the freshly-built local artifact. The\nloader in coverage-instrument.wasi.cjs falls back to the published\npackage's wasm whenever it cannot find the local file at the expected\npath, so PR changes to the napi crate never reached the wasi test.\nThis is why Napi Test (wasm32-wasi) reported red on the original v0.7.2\nhint regression even after the Rust-side fix landed in this PR's\nearlier commits.\n\nRemove the published wasm from node_modules between the build and\ntest steps so the loader can only use the freshly-built local\nartifact. If napi build failed to produce it, the loader fails loudly\ninstead of silently exercising the previous release.\n\nAlso revert the diagnostic [shape_check=<bool>] prefix on the error\nmessage; with the loader fix in place it is no longer needed to tell\nwhich binary is in use.\n\n* ci: re-trigger workflows after GitHub Actions delay",
          "timestamp": "2026-05-26T14:15:50+01:00",
          "tree_id": "792de65693455ff3127c9f82c828441404e562e0",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/2cf6f2c0f34e8bd0455aa8be3854152e5bfb0ff6"
        },
        "date": 1779801460331,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85427152,
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
          "id": "df0eb0c85c719dfc7dc379af289955e685ca2bec",
          "message": "chore(napi): rebrand startup binary-presence log from [diag] to [napi-artifact]\n\nThe temporary [diag] tag added during PR #94's wasi-loader debugging\nread as debug output, but the underlying check (logging which local\nnapi artifacts exist before the binding loader runs) is load-bearing:\nit surfaces silent fallbacks to the published optionalDependency\nbinary on the first failing CI run instead of after hours of remote\ndiagnosis. Keep the log, drop the debug-flavoured tag, and document\nthe safeguard in a comment so future readers don't strip it.",
          "timestamp": "2026-05-26T15:25:43+02:00",
          "tree_id": "66728a10048caac10a891e20548f66b236deb121",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/df0eb0c85c719dfc7dc379af289955e685ca2bec"
        },
        "date": 1779802058175,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85427152,
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
          "id": "8922226afbe7dc754c4e7c3e62118f8f4dcd08b6",
          "message": "feat(napi): guard SharedArrayBuffer before WASM instantiation (#96)\n\nThe browser WASM shim regenerated by napi build allocates new\nWebAssembly.Memory({ shared: true }) at module top level, which throws\nan opaque TypeError when the host page lacks cross-origin isolation\n(no COOP/COEP headers, no SharedArrayBuffer).\n\nInject a guard at the top of coverage-instrument.wasi-browser.js via a\npostbuild script so the failure surfaces a precise diagnostic pointing\nat the host-page fix. Wire the script into the napi crate's build /\nbuild:debug / test pipelines and into the wasm matrix entries of\nrelease-npm.yml and ci.yml. The script is idempotent (sentinel\ncomment) and a no-op on non-wasm targets.",
          "timestamp": "2026-05-26T14:58:50+01:00",
          "tree_id": "755ebe68d4fc7551470246fc974065d2bc84acf1",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/8922226afbe7dc754c4e7c3e62118f8f4dcd08b6"
        },
        "date": 1779804050422,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85427152,
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
          "id": "fe1c0cd7f1a60af2201305c16e56e180c4c3fab9",
          "message": "feat: add single-threaded WASI binding for Workers\n\nCloses #87.",
          "timestamp": "2026-05-26T22:30:20+01:00",
          "tree_id": "fcc0acea8da9f0ec7fb7755b943af17ca42ccee5",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/fe1c0cd7f1a60af2201305c16e56e180c4c3fab9"
        },
        "date": 1779831321746,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85427152,
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
          "id": "e0a22a75131dbb60eb56e7c9197f40cbd6a25841",
          "message": "chore: release v0.7.3",
          "timestamp": "2026-05-27T07:28:34+02:00",
          "tree_id": "9d39166e6ef1dd42500b5b2c7c39aa32567291ee",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/e0a22a75131dbb60eb56e7c9197f40cbd6a25841"
        },
        "date": 1779861166420,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85429072,
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
          "id": "eca4b5a388fb6270dce1ca08da92fd623ed339bf",
          "message": "feat: compose inputSourceMap eagerly during instrument() (#101)\n\nAdd an opt-in composeInputSourceMap flag to InstrumentOptions (napi:\ncomposeInputSourceMap). When true and inputSourceMap is set, instrument()\nfolds the input source map into the coverage map during instrumentation\nvia remap_coverage, so the returned coverageMap and the runtime\n__coverage__ baked into the preamble carry original-source positions,\nare keyed by the original source path, and embed no inputSourceMap.\nremapCoverageMap on the result is then a no-op.\n\nThis removes the per-collection remap round-trip for E2E collectors\n(Playwright et al.) that dump window.__coverage__ directly. Composition\nruns after the function-identity overlay attaches and before the coverage\nmap is serialized into the preamble, so the overlay keeps its pre-remap\nids and the eager path is bit-for-bit equal to instrument-then-remap.\nWhen the input map is unusable the embedded map is retained so the lazy\npath still works. The output source map is unaffected (still composed\nonce by finalize_source_map). Core + napi only; CLI and the Vitest\nadapter are out of scope.",
          "timestamp": "2026-05-28T19:18:42+01:00",
          "tree_id": "8a5f6b98f96bd7274f241ed029a55aa9e32bbbdb",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/eca4b5a388fb6270dce1ca08da92fd623ed339bf"
        },
        "date": 1779992452078,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85519856,
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
          "id": "55ade84f741ba9d06dda2b4f3e0768c82926a2b3",
          "message": "chore: release v0.7.4",
          "timestamp": "2026-05-28T20:22:13+02:00",
          "tree_id": "85f37052c41d09d9f8253ba155702f6bd55c1834",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/55ade84f741ba9d06dda2b4f3e0768c82926a2b3"
        },
        "date": 1779992653187,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85516488,
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
          "id": "3d3c742b81e9aa65587603ede0e654932e18e0f6",
          "message": "chore: release v0.7.5",
          "timestamp": "2026-05-28T21:26:04+02:00",
          "tree_id": "984e0d4d92064e81ab0c7a432affbfdb70896372",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/3d3c742b81e9aa65587603ede0e654932e18e0f6"
        },
        "date": 1779996486860,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85505312,
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
          "id": "814fda7bc415a6ac931f6c2dbdadb5bc8f4bb370",
          "message": "refactor: split oversized parser/builder units and cover CLI error paths (#102)\n\nSplits parse_instrument_args, prune_unmapped, and build_file_coverage into focused helpers (no behavior change) and adds CLI integration tests for previously uncovered error branches. CLI main.rs 85.1%->92.0% line; workspace total 95.7%->96.3%. Closes #103.",
          "timestamp": "2026-05-29T22:55:21+01:00",
          "tree_id": "bb9bbe523ecf00a3d601ab7944dc4a425d168407",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/814fda7bc415a6ac931f6c2dbdadb5bc8f4bb370"
        },
        "date": 1780091822545,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85509040,
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
          "id": "8faba6df9f994c911401ad394b50351b8ca7f943",
          "message": "chore: release oxc coverage instrument 0.7.6",
          "timestamp": "2026-06-01T18:58:47+02:00",
          "tree_id": "241eb7476f3c8b3be45e9d680d35c9bb65b8f739",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/8faba6df9f994c911401ad394b50351b8ca7f943"
        },
        "date": 1780333266098,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85492616,
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
          "id": "67f4958f4a4e4980bdffedb6be197492ef770d93",
          "message": "fix: composeInputSourceMap drops unmapped positions\n\nThe eager composeInputSourceMap path folded the input source map into the\ncoverage map via the no-drop remap_coverage, so positions with no mapping\nwere stranded at generated coordinates and re-keyed past the end of the\noriginal file (e.g. Vue SFC compiler boilerplate with no mapping back to\nthe .vue, ~20% of .vue statements past EOF in a real Vite + Playwright\nrun). The eager path bakes positions into the runtime __coverage__ literal\nwith no later remap opportunity, so those entries can never be recovered.\n\nRoute eager compose through remap_coverage_with_options with\ndrop_unmapped: true so it agrees with the lazy remapCoverageMap path's\ndropUnmapped behavior and never emits past-EOF entries. No new public\nflag: an entry with no original position is never meaningful on the eager\npath. The unusable-map back-off is unchanged.\n\nAlso keep the x_fallow_functionMap overlay 1:1 with fnMap when a function\nis dropped (prune_functions now removes the matching overlay entry), a\nlatent orphan-key issue on both the lazy and eager drop paths.\n\nCloses #105",
          "timestamp": "2026-06-02T11:24:47+02:00",
          "tree_id": "52b32473723b4d24fa53246896452caa6870a727",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/67f4958f4a4e4980bdffedb6be197492ef770d93"
        },
        "date": 1780392402519,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86531824,
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
          "id": "1d0f441149fd8dc4ce58e8eb4665e223a5049907",
          "message": "chore: release v0.7.7",
          "timestamp": "2026-06-02T11:28:52+02:00",
          "tree_id": "12c51871c171f47b0ddfa3e19cb3703dcecb413c",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/1d0f441149fd8dc4ce58e8eb4665e223a5049907"
        },
        "date": 1780392684915,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86539512,
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
          "id": "7754d97cadb6a39c73f0d7f3ae0ed741ad86397f",
          "message": "chore: bump oxc-coverage-reports source_maps pin to 0.3.2\n\nAlign the declared pin with the version published this release. Caret\nsemantics already resolved it (Cargo.lock unchanged); this keeps the\nmanifest honest for the next time reports is republished.",
          "timestamp": "2026-06-02T12:08:56+02:00",
          "tree_id": "673ac0573110c3818cafb8f8909a44fbe6bfd460",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/7754d97cadb6a39c73f0d7f3ae0ed741ad86397f"
        },
        "date": 1780395041215,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86539512,
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
          "id": "935539412783f9cf34db4d0312b2214eea301ada",
          "message": "chore: bump oxc_coverage_types to 0.2.1 to resync published docstring\n\ncheck-version-sync.sh --mode=published flagged that types' local src/\ndiverged from the published 0.2.0: commit 8debd7f refined the\nFunctionIdentity docstring (to describe the SHA-256 id formula) without\nbumping types, so docs.rs for 0.2.0 carries the old, inaccurate\nderivation. Bump types 0.2.0 -> 0.2.1 and cascade the five internal\npins so the corrected docstring publishes on the next release.",
          "timestamp": "2026-06-02T12:19:57+02:00",
          "tree_id": "df90ccece6b21f4c4fc6c058d5828e2ffefd5786",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/935539412783f9cf34db4d0312b2214eea301ada"
        },
        "date": 1780395733861,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86535976,
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
          "id": "5800d27e132992701747c44857ee1ed1580747b1",
          "message": "fix: composeInputSourceMap no longer emits dangling counters (issue #106)\n\nv0.7.7 made eager composeInputSourceMap drop coverage entries whose\npositions have no source-map mapping (the #105 fix), but only trimmed the\ncoverage data: the instrumented code still incremented those dropped\ncounters. At runtime that ran ++cov.b[id][..] against a pruned slot\n(undefined), throwing TypeError and crashing any app using\ncomposeInputSourceMap (e.g. Vue 3 + Vite E2E).\n\nFix at the AST level (issue #106): in eager mode a coverage point whose\npositions do not remap through the input source map is never instrumented,\nno map entry and no counter, so the runtime coverage object and the\nemitted counters are derived from the same decision and agree by\nconstruction. add_function/add_statement/add_branch/add_branch_path now\nreturn Option and register nothing when a point does not remap; callers\nskip only the counter and continue traversing (nested mappable statements\nare still instrumented). Branches with fixed arm indices (logical /\nbinary / optional-chain / logical-assign) gate at the whole-branch level;\nif / ternary / switch gate per arm with contiguous indices. Compose\nreverts to a plain no-drop remap since the transform now owns the drop.\n\nThe gate is a strict no-op unless composeInputSourceMap is set with a\nusable input map, so all non-eager output is byte-identical.\n\nCompanion oxc_coverage_source_maps gains a public PositionRemapper used by\nthe gate; its predicate mirrors try_remap_position exactly.\n\nCloses #106",
          "timestamp": "2026-06-02T14:41:17+02:00",
          "tree_id": "70876e186768e54131f227a755814201ec04b8aa",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/5800d27e132992701747c44857ee1ed1580747b1"
        },
        "date": 1780404187758,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85521280,
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
          "id": "5417025677c72b4642468351243c9fe6454fe04f",
          "message": "chore: release v0.7.8",
          "timestamp": "2026-06-02T14:45:14+02:00",
          "tree_id": "b7573bce5de75ca8991e76110195f2c7c7cbb6e1",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/5417025677c72b4642468351243c9fe6454fe04f"
        },
        "date": 1780404433607,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85528128,
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
          "id": "8e642d633bf4f4875c01bc35bd034d6fc284aa2c",
          "message": "ci: commit napi package-lock.json and add fetch-retry .npmrc\n\nThe napi package-lock.json had been gitignored since the #45 workspace\nrestructure (lumped in with the napi dir's genuinely-generated artifacts\nlike index.js / *.wasi.cjs). With no committed lockfile, the release\nmatrix's `npm ci || npm install` always fell through to `npm install`,\nwhich hits the live registry for resolution + download on all seven\nplatform runners. v0.7.8's musl build failed on a transient ECONNRESET\nthere, which skipped the npm publish (recovered via rerun).\n\nCommit the lockfile so `npm ci` takes the deterministic fast path\n(fewer network round-trips), and add an .npmrc with fetch retries +\nbackoff so a transient registry blip is retried instead of failing the\nbuild. The .npmrc carries only fetch tuning; registry auth / OIDC\ntrusted-publishing config stays in the release workflow. The existing\n`npm ci || npm install` steps are unchanged: npm ci now succeeds, with\nnpm install kept as a last-resort fallback.",
          "timestamp": "2026-06-02T14:57:25+02:00",
          "tree_id": "43ef871abf8a00d6b7b89ddbf7aad8a616250d66",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/8e642d633bf4f4875c01bc35bd034d6fc284aa2c"
        },
        "date": 1780405161518,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 85528128,
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
          "id": "1111d69a28fb54f41352df23fb1cc986e0b537bc",
          "message": "fix: reconcile orphan counters in remap so nyc merge never crashes (#107)\n\n* fix: drop orphan counters in remap so nyc merge never crashes (issue #107)\n\nAn orphan counter, an s/f/b key with no matching statementMap/fnMap/branchMap\nentry, is fatal to istanbul-lib-coverage's CoverageMap.merge: mergeProp iterates\nevery s key, looks up statementMap[key], and keyFromLoc destructures start of the\nundefined entry, throwing \"Cannot destructure property 'start' of 'undefined'\".\nA single orphan anywhere aborts the whole nyc report step.\n\nThe v0.7.8 composeInputSourceMap path cannot itself emit an orphan: the #106\nAST-level gate makes the emitted code, the embedded literal (s = statementMap\nkeys), and the location maps consistent by construction, and compose is a\nno-drop remap (verified by code review plus fuzzing multi-segment and NO_SOURCE\nmaps). A null-valued orphan is the runtime signature of a dangling cov.s[id]++\nagainst a pruned slot (undefined + 1 = NaN, serialized back as null), the\npre-#106 shape, so it reaches our pipeline only via runtime-collected coverage\nfrom an upstream/older instrumenter.\n\nThe remap helpers previously propagated such an orphan unchanged. Now every\nremap exit point reconciles to the Istanbul merge invariant:\n\n- New FileCoverage::prune_orphan_counters() drops s/f/b/bT keys absent from\n  their location maps and keeps the x_fallow_functionMap overlay 1:1 with fnMap.\n- Wired into apply_source_map (after remap/prune) and both map-level None\n  passthrough branches (remap_coverage_map* and SourceMapStore variants), so an\n  already-composed entry with no embedded map is reconciled too.\n\nA no-op on already-consistent coverage; s deserializes null to 0, so a coverage\nobject carrying the exact issue-107 shape ingests and cleans rather than\ncrashing the consumer.\n\nCloses #107\n\n* refactor: count overlay prunes in prune_orphan_counters return (review)\n\nReview CONCERN: the returned count excluded x_fallow_functionMap overlay\nprunes, so a caller using removed>0 as a was-this-dirty signal could get a\nfalse negative when an orphan lived only in the overlay. Count overlay removals\ntoo, so the return is the total orphan entries removed across every map.\n\n* test: add verbatim issue #107 FileCoverage as a regression fixture\n\nThe reporter shared the exact malformed coverage object (names mocked, shape\nverbatim) straight from window.__coverage__: statementMap jumps 2 -> 4 with\ns[\"3\"] = null, while fnMap/branchMap stay consistent. Capture it as a fixture\nand assert (a) the raw shape crashes istanbul-lib-coverage merge, (b)\nremapCoverageMap reconciles it via the passthrough branch (drops only the single\norphan, preserves the other 35 statement counters), and (c) the cleaned object\nmerges without throwing. This is the real-world consumer crash, not a synthetic\nconstruction.",
          "timestamp": "2026-06-03T10:26:08+02:00",
          "tree_id": "4374282c6747dba6b6b6db0e9f72656689c5abf8",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/1111d69a28fb54f41352df23fb1cc986e0b537bc"
        },
        "date": 1780475272468,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86069448,
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
          "id": "d122c107d4465ffbffc1778bea170431976452cf",
          "message": "feat: add trackOptionalChainBranches option to disable optional-chain branch tracking (#108)\n\n* feat: add trackOptionalChainBranches option to disable ?. branch tracking (issue #108)\n\nin a runtime _oc helper call. That is more complete than istanbul-lib-instrument\n(which leaves ?. native), but it is unconditional and adds per-operand call\noverhead in optional-chain-dense hot paths, and it diverges from istanbul for\nprojects that want byte-comparable reports or gate only on line coverage.\n\nAdd a boolean to opt out, defaulting to true so existing behavior is unchanged:\n\n- Rust: InstrumentOptions::track_optional_chain (default true).\n- napi: InstrumentOptions.trackOptionalChainBranches (default true).\n- vitest adapter: createOxcInstrumenter({ trackOptionalChainBranches }), validated\n  as a strict boolean (an explicit false is honored, not coerced).\n\nWhen false, the three optional-link dispatch points (static member, computed\nmember, call) skip wrap_optional_chain_link, so no optional-chain branch is\nregistered and the _oc helper append self-disables. The chain is left native,\nmatching istanbul: item?.a?.b?.c ?? 0 instruments only the surrounding ??.\nStatement, function, and other branch coverage are unaffected. The flag mirrors\nthe existing report_logic plumbing through TransformInit / CoverageTransform;\nthe v8-collect path keeps tracking on (it only builds location maps).\n\nCloses #108\n\n* refactor: only intern cov_fn_oc_name when optional-chain tracking is on (review)\n\nReview CONCERN: cov_fn_oc_name was interned unconditionally, unlike its sibling\ncov_fn_bt_name which allocates only when report_logic is on. With\ntrack_optional_chain off the string was never referenced. Make it\nOption<&str> gated on track_optional_chain, matching cov_fn_bt_name; the single\nuse site in wrap_optional_chain_link is only reached when tracking is on (gated\nat the three dispatch points), so it expects Some.",
          "timestamp": "2026-06-03T10:29:38+02:00",
          "tree_id": "d2cae01ea53b9986a27c8f90ac21c0cc5d76b226",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/d122c107d4465ffbffc1778bea170431976452cf"
        },
        "date": 1780475579014,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86072608,
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
          "id": "5b413f49faab74e18f21d67862b35062ee685867",
          "message": "feat(source-maps): resolve remap positions via istanbul getMapping semantics (#112)\n\nremapCoverageMap and instrument(..., { composeInputSourceMap: true }) now\nresolve surviving coverage positions through an istanbul getMapping-equivalent\nrange remap instead of a direct original_position_for lookup, finishing the\ndocumented createSourceMapStore().transformCoverage equivalence the drop\nsemantics already claim.\n\nSource maps carry segments only at token starts, so the old direct lookup (1)\ntruncated exclusive ends backward to the previous segment (a statement covering\nstate.someVeryLongPropertyName1 came back covering only state.) and (2) never\nballooned a span smaller than its enclosing segment (a 1-char arrow decl stayed\n1 char). Both are one root cause: starts now resolve with greatest-lower-bound\nand ends resolve to the next original segment via originalEndPositionFor (or the\nend of the original line). The end-of-line case (istanbul's column: Infinity)\nclamps to the original line's UTF-16 length from sourcesContent, falling back to\nthe rightmost mapped column when content is absent. The degenerate-span branch\nis ported so zero-width spans cannot corrupt keyFromLoc merge dedup.\n\nBehavior change: ends widen to the full token and 1-char decls balloon to their\nenclosing span. Line numbers and coverage percentages are unchanged. Because\nistanbul-lib-coverage keyFromLoc includes columns, pre-upgrade coverage caches\nor artifacts must be flushed before merging with post-upgrade runs, and snapshot\nassertions on exact columns will diff. oxc_coverage_source_maps is bumped to\n0.4.0 to signal the position-shape change to semver-range pinners.\n\nAdds an end-to-end byte-parity test against istanbul-lib-source-maps@5.0.6\n(createSourceMapStore().transformCoverage), the reporter's two concrete cases as\nfixtures, and Rust unit tests for end widening, the 1-char balloon, the\nend-of-line clamp, and the degenerate-span guard.\n\nCloses #111",
          "timestamp": "2026-06-03T11:45:10+02:00",
          "tree_id": "7b266f23bc69e21584414fc8233924b7aabd675d",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/5b413f49faab74e18f21d67862b35062ee685867"
        },
        "date": 1780480052043,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86428560,
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
          "id": "bc7b890a5fef41e17bb7f85ea9f290aa57deab8e",
          "message": "chore: release v0.8.0",
          "timestamp": "2026-06-03T12:18:00+02:00",
          "tree_id": "2f789e4d3ca2f2a6a97505c047c37c80bc26e342",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/bc7b890a5fef41e17bb7f85ea9f290aa57deab8e"
        },
        "date": 1780482153244,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86445200,
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
          "id": "2f4740f757b3fcefe4ce5f63ee124d4cd8a2f70c",
          "message": "test(napi): real @babel/core-emitted map in getMapping parity harness (#113)\n\nThe #111 byte-parity test previously remapped only hand-built (@jridgewell/gen-mapping)\nmaps through both istanbul-lib-source-maps and us. Hand-built maps do not reproduce the\nsegment density, column shifts, and sourcesContent shape a real transpiler emits, which\nis exactly where the truncation #111 fixed bites.\n\nAdds a case that runs a real @babel/core transform (an inline rename plugin shortens long\nidentifiers, collapsing member chains and shifting columns so exclusive ends land between\nsegments), emits a real source map with sourcesContent, instruments the output, and remaps\nthrough both istanbul transformCoverage and remapCoverageMap, asserting the surviving spans\nmatch column-by-column plus at least one widened multi-column statement span. parityCheck is\nrefactored to share a compareRemap core with the new case. @babel/core is promoted from a\ntransitive to a declared devDependency.",
          "timestamp": "2026-06-03T12:48:38+02:00",
          "tree_id": "45c7a363d61bbb2bdfadf0aa5f0d7418ab185de3",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/2f4740f757b3fcefe4ce5f63ee124d4cd8a2f70c"
        },
        "date": 1780483899518,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86445200,
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
          "id": "f043c9af7efb52b42cb80d290361db68a607d891",
          "message": "perf: optimize coverage remap and v8 hot paths",
          "timestamp": "2026-06-03T15:39:05+02:00",
          "tree_id": "56d1bab29387ab94f801f28d55e3aa30ab738717",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/f043c9af7efb52b42cb80d290361db68a607d891"
        },
        "date": 1780495871263,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86445200,
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
          "id": "df4c4b8d396abbb8745f9f93dbb470f5c379d8d5",
          "message": "fix: count inline export const function declaration statements (#114)\n\nAn inline `export const fn = () => {}` registered its per-declarator\nstatement in statementMap but never emitted the `++cov.s[N]` increment, so\nthe declaration line reported as uncovered even after the module ran.\n\nFunction/arrow/class-valued declarator inits hoist their statement counter to\na sibling statement before the enclosing declaration (the `(++s, fn)`\nsequence-wrap would break Function.name inference). The hoist target was the\ninner VariableDeclaration start, but for an exported declaration the\nExportNamedDeclaration occupies the statement slot, so exit_statements (which\nmatches by target_start == stmt.span().start) never matched and the counter\nwas dropped. enclosing_var_decl_hoist_target now returns the export node's\nstart when the declaration's parent is an ExportNamedDeclaration.\n\nVerified at byte-parity with istanbul-lib-instrument across export\nconst/let/var arrow, function, class, multi-declarator, and mixed forms, both\nat module evaluation and after call. Adds Rust emit-order regression tests and\na napi runtime test that evaluates the module and asserts s[0] === 1.\n\nCloses #114",
          "timestamp": "2026-06-04T09:40:03+02:00",
          "tree_id": "acd2924a1ec979e5749953e7962ace786c04f600",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/df4c4b8d396abbb8745f9f93dbb470f5c379d8d5"
        },
        "date": 1780558961228,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86433512,
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
          "id": "1416c9a223a6fbcecf1ad724690bad254cfd3411",
          "message": "chore(clippy): resolve rust 1.95 workspace lints\n\n`cargo clippy --workspace --all-targets -- -D warnings` was red under the\npinned 1.95 toolchain on lints that predate this toolchain bump:\n\n- needless_pass_by_value: `SourceMapStore::add_map` now takes\n  `&serde_json::Value` (it only serializes the value, never stores it).\n  All callers updated to pass a reference. Minor public API change.\n- format_push_string: `push_str(&format!(..))` -> `writeln!` in the v8 and\n  reports benches.\n- type_complexity: extracted a `BranchFixture` type alias in the v8 bench.\n- if-bool-to-int and manual loop counter: `u32::from(..)` and `.enumerate()`\n  in the reports bench.\n\nNo behavior change; full test suite, fmt, and napi runtime tests stay green.",
          "timestamp": "2026-06-04T09:55:35+02:00",
          "tree_id": "c730539e7baa713c87643a8ba0959b8b1111102f",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/1416c9a223a6fbcecf1ad724690bad254cfd3411"
        },
        "date": 1780559843171,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86433512,
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
          "id": "33e1fbb9294d2325e84c34c52003c81495ab0965",
          "message": "fix: align emnapi wasm dependency versions",
          "timestamp": "2026-06-04T10:04:32+02:00",
          "tree_id": "065c3ca4e32449bb7b4ca90a4c93d2fb1c9c0469",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/33e1fbb9294d2325e84c34c52003c81495ab0965"
        },
        "date": 1780560381491,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86433512,
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
          "id": "2b6cb853f2f4de61f8bde5378fb70c3826552a36",
          "message": "chore: release v0.8.1",
          "timestamp": "2026-06-04T10:48:23+02:00",
          "tree_id": "2045ae8740407569ae81bd74036beca30013e5ff",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/2b6cb853f2f4de61f8bde5378fb70c3826552a36"
        },
        "date": 1780563141816,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86430528,
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
          "id": "7e7fddf8d7985a6729fb1fb48093d762be6361b3",
          "message": "chore: release v0.8.2",
          "timestamp": "2026-06-04T10:55:15+02:00",
          "tree_id": "97bffcf2cf24a3d44bf9a547444bc269a5058488",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/7e7fddf8d7985a6729fb1fb48093d762be6361b3"
        },
        "date": 1780563443771,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86426904,
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
          "id": "e82bb7f5b818bcfc8b132d2c1ec611e3958d5ca6",
          "message": "refactor: split coverage map finalization",
          "timestamp": "2026-06-04T11:35:39+02:00",
          "tree_id": "ada3a80fb7562dfe113cc3ca106de41d1537aa59",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/e82bb7f5b818bcfc8b132d2c1ec611e3958d5ca6"
        },
        "date": 1780570055694,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86429104,
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
          "id": "551eb8a7841fcf5d2fac8d42651341147144f4ad",
          "message": "fix: align eager composeInputSourceMap drop gate with getMapping keep-decision\n\nThe eager AST-level drop gate resolved each Location endpoint with a single greatest-lower-bound lookup, so it dropped a coverage point whose generated column sits just before its line's first mapping. The deferred remapCoverageMap({ dropUnmapped: true }) path keeps that entry via getMapping's least-upper-bound fallback, so eager composition silently dropped statements, functions, and branches that the lazy path (and istanbul-lib-source-maps) retain. This hit compiled Vue render functions at scale.\n\nReplace PositionRemapper::maps(line, column) with location_maps(&Location), which returns get_mapping_location(loc).is_some() (mirroring the deferred drop keep-decision try_remap_location, including the line-0 sentinel). The eager gate and the deferred prune now agree by construction.\n\nRemoves the pub PositionRemapper::maps method from oxc-coverage-source-maps (its only caller was the in-crate gate); a pre-1.0 API change to fold into the next version bump.\n\nCloses #122",
          "timestamp": "2026-06-05T09:48:09+02:00",
          "tree_id": "76798eabb1fdb1a54cf31bbc1c368923e1b1d718",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/551eb8a7841fcf5d2fac8d42651341147144f4ad"
        },
        "date": 1780645804709,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86440000,
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
          "id": "bce77875e81522bdb582456b394154cbdefdedc8",
          "message": "chore: release v0.9.0",
          "timestamp": "2026-06-05T11:25:26+02:00",
          "tree_id": "8d092137194f05454a4e130c89b026d0f3053c83",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/bce77875e81522bdb582456b394154cbdefdedc8"
        },
        "date": 1780651858723,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86439848,
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
          "id": "615671aefcf6ed639ab154540dbbed411dcdbe8e",
          "message": "chore(deps): bump srcmap-sourcemap to 0.3.8\n\nBumps [srcmap-sourcemap](https://github.com/fallow-rs/srcmap) from 0.3.7 to 0.3.8.\n- [Release notes](https://github.com/fallow-rs/srcmap/releases)\n- [Changelog](https://github.com/fallow-rs/srcmap/blob/main/release.toml)\n- [Commits](https://github.com/fallow-rs/srcmap/compare/v0.3.7...v0.3.8)\n\n---\nupdated-dependencies:\n- dependency-name: srcmap-sourcemap\n  dependency-version: 0.3.8\n  dependency-type: direct:production\n  update-type: version-update:semver-patch\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-06-10T09:18:45+02:00",
          "tree_id": "fba936253182158227bcf01934f4b3ab2a4a786f",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/615671aefcf6ed639ab154540dbbed411dcdbe8e"
        },
        "date": 1781076071750,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86442032,
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
          "id": "466d517113e916bef7e10cb3b36e9d1bc1e836f3",
          "message": "chore(deps): bump oxc_sourcemap to 7.0.0\n\n* chore(deps): bump oxc_sourcemap from 6.1.1 to 7.0.0 in the oxc group\n\nBumps the oxc group with 1 update: [oxc_sourcemap](https://github.com/oxc-project/oxc-sourcemap).\n\n\nUpdates `oxc_sourcemap` from 6.1.1 to 7.0.0\n- [Release notes](https://github.com/oxc-project/oxc-sourcemap/releases)\n- [Changelog](https://github.com/oxc-project/oxc-sourcemap/blob/main/CHANGELOG.md)\n- [Commits](https://github.com/oxc-project/oxc-sourcemap/compare/v6.1.1...v7.0.0)\n\n---\nupdated-dependencies:\n- dependency-name: oxc_sourcemap\n  dependency-version: 7.0.0\n  dependency-type: direct:production\n  update-type: version-update:semver-major\n  dependency-group: oxc\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\n\n* fix: adapt oxc sourcemap bump\n\n---------\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\nCo-authored-by: Bart Waardenburg <bart@waardenburg.dev>",
          "timestamp": "2026-06-10T09:33:09+02:00",
          "tree_id": "401f81d834c25aed70caee8f72ef37d4ad7ca1fb",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/466d517113e916bef7e10cb3b36e9d1bc1e836f3"
        },
        "date": 1781076943223,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86443928,
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
          "id": "3fcbf840d06f038fe7f7f7fa6c7f25eba0c809ab",
          "message": "ci: migrate benchmarks to codspeed",
          "timestamp": "2026-06-17T15:00:04+02:00",
          "tree_id": "dbb193d8993539728a984a133fc5d64bfa29afde",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/3fcbf840d06f038fe7f7f7fa6c7f25eba0c809ab"
        },
        "date": 1781701612308,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86519160,
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
          "id": "460edf68d2bae20690b7340993f4c547a8c62f58",
          "message": "ci: switch benchmarks to criterion2",
          "timestamp": "2026-06-17T15:37:30+02:00",
          "tree_id": "606570274f44386bee6166e40be711a2dc90ccb2",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/460edf68d2bae20690b7340993f4c547a8c62f58"
        },
        "date": 1781703639468,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86519160,
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
          "id": "7b3c6c83bbe8368fa04a7b08465b5970b975d047",
          "message": "perf: index source map original columns",
          "timestamp": "2026-06-17T20:49:23+02:00",
          "tree_id": "d303abf623e2d4d998a8214b442b9fa5cd1b904c",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/7b3c6c83bbe8368fa04a7b08465b5970b975d047"
        },
        "date": 1781723046250,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970112,
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
          "id": "203673da4f929e43ae4cb0905c691ca65e62ea12",
          "message": "perf: restore v8 range ordering",
          "timestamp": "2026-06-17T21:06:22+02:00",
          "tree_id": "421ba1d4516ee4dbf7ff695bc438fe4e3da9c2b4",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/203673da4f929e43ae4cb0905c691ca65e62ea12"
        },
        "date": 1781723320011,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970112,
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
          "id": "1f632f31324353a9da5ed7f7400928a7d78b1df7",
          "message": "test: cover napi remap wrappers",
          "timestamp": "2026-06-17T21:20:28+02:00",
          "tree_id": "dd71ce7fdbb656762e55ac17fe77cf8154050ab0",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/1f632f31324353a9da5ed7f7400928a7d78b1df7"
        },
        "date": 1781724157240,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970112,
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
          "id": "08fab76386b9f9ba3fb41cef5ca790b45948d3a2",
          "message": "test: expand codspeed report benchmarks",
          "timestamp": "2026-06-17T21:32:46+02:00",
          "tree_id": "1b2d2b9ccb50b58b2c9d378b356e076bddea5d60",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/08fab76386b9f9ba3fb41cef5ca790b45948d3a2"
        },
        "date": 1781724880007,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970112,
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
          "id": "b862a818d88b39e4d6ccb5081c942cb3211901a5",
          "message": "test: keep html codspeed benchmark compact",
          "timestamp": "2026-06-17T21:40:19+02:00",
          "tree_id": "10236d7e7b8cb688a5285267ba9eaae17d7ea050",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/b862a818d88b39e4d6ccb5081c942cb3211901a5"
        },
        "date": 1781725327338,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970112,
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
          "id": "b7a81671e5563135ebb5d7836a56634114fdaa77",
          "message": "perf: streamline v8 source map helpers",
          "timestamp": "2026-06-18T11:03:06+02:00",
          "tree_id": "ce1b3b64709872532e467780e390442fb4f853b6",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/b7a81671e5563135ebb5d7836a56634114fdaa77"
        },
        "date": 1781773497303,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970112,
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
          "id": "be38e0fb656ada69d2cbc99986b18ce0ad862fb2",
          "message": "chore: release v0.9.1",
          "timestamp": "2026-06-18T11:06:36+02:00",
          "tree_id": "78d9dc6d1593f7c0e1fa47398b412074021acc5e",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/be38e0fb656ada69d2cbc99986b18ce0ad862fb2"
        },
        "date": 1781773802477,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970096,
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
          "id": "fcfc8196da6aa4ee7c2bde35a967e337adc8d32a",
          "message": "fix(ci): unbreak wasm builds (emnapi pin) and Actions Security (zizmor) (#149)\n\n* fix(napi): pin emnapi packages to keep wasm builds consistent\n\nThe napi lockfile falls out of sync once the binding optionalDependencies\nare bumped at release time (the sibling binding packages publish after the\nrelease commit, so the release-time lockfile cannot resolve them). CI\ninstalls with `npm ci || npm install`, so the failed sync check falls back\nto `npm install`, which re-resolves the floating `emnapi ^1.10.0` to a\nnewer minor while `@emnapi/core`/`@emnapi/runtime` stay pinned. napi-rs\nthen aborts every wasm build with \"emnapi version mismatch\".\n\nPin emnapi, @emnapi/core and @emnapi/runtime to the same lockstep version\nvia overrides so the install fallback can never split them again, and\nregenerate the lockfile so `npm ci` passes cleanly.\n\n* ci: ignore zizmor adhoc-packages for release-npm bootstrap\n\nzizmor 1.26.x added the adhoc-packages audit, which flags the\n`npm install -g npm@latest` step in release-npm.yml. That step bootstraps\na recent npm so OIDC trusted publishing / provenance works (the runner's\nnpm is too old); it is a first-party toolchain bootstrap, not an untrusted\nad-hoc dependency install. Ignore the audit for that workflow, matching the\nexisting per-file rationale entries. The workflow runs zizmor via unpinned\n`uvx`, so the new audit started failing the Actions Security job with no\ncode change.",
          "timestamp": "2026-07-01T11:26:53+02:00",
          "tree_id": "ac297f25d0826cc0c8e22210c1327288b54d26fb",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/fcfc8196da6aa4ee7c2bde35a967e337adc8d32a"
        },
        "date": 1782898177012,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86970096,
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
          "id": "df3d11baed087caa619cd97a1ef3f63cf2767bca",
          "message": "chore(deps): bump actions/checkout from 6.0.3 to 7.0.0 (#140)\n\nBumps [actions/checkout](https://github.com/actions/checkout) from 6.0.3 to 7.0.0.\n- [Release notes](https://github.com/actions/checkout/releases)\n- [Changelog](https://github.com/actions/checkout/blob/main/CHANGELOG.md)\n- [Commits](https://github.com/actions/checkout/compare/df4cb1c069e1874edd31b4311f1884172cec0e10...9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0)\n\n---\nupdated-dependencies:\n- dependency-name: actions/checkout\n  dependency-version: 7.0.0\n  dependency-type: direct:production\n  update-type: version-update:semver-major\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-07-01T11:29:44+02:00",
          "tree_id": "971d9ce340fdd73ded14a5fa7f3d869e645b89b1",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/df3d11baed087caa619cd97a1ef3f63cf2767bca"
        },
        "date": 1782898353698,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86968056,
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
          "id": "3be071cc268d4983c99b627ca61fe2d1a01830be",
          "message": "chore(deps-dev): support @babel/core 7 and 8 (#148)\n\n@babel/core is a test-only devDependency (test.mjs uses it to emit a real\ntranspiler source map for the remap smoke). Babel 8 dropped the CommonJS\ndefault export, so `(await import('@babel/core')).default` is undefined and\nthe real-transpiler test threw \"Cannot read properties of undefined\".\n\nSince we only touch `transformSync` (a named export on both majors), make\nthe import version-agnostic (`mod.default ?? mod`) and widen the range to\n`^7.29.0 || ^8.0.0` instead of pinning to 8, so both majors keep working.\nLockfile moves to 8.0.1 so CI exercises the new major. Validated: test.mjs\npasses on both 7.29.7 and 8.0.1.\n\nCo-authored-by: Bart Waardenburg <bart@waardenburg.dev>",
          "timestamp": "2026-07-01T11:53:38+02:00",
          "tree_id": "af2e7d9ee5f73392467d64d1a9d6707a70e25dc6",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/3be071cc268d4983c99b627ca61fe2d1a01830be"
        },
        "date": 1782899763633,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86968056,
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
          "id": "8a210549b14a036dbf6a7c58b59a323641369c3e",
          "message": "chore(deps): bump srcmap-sourcemap from 0.3.8 to 0.3.9 (#143)\n\nRebased onto current main (the original dependabot branch conflicted on\nCargo.lock after the srcmap-remapping bump landed). Bumps the exact pins in\nboth consumer crates and refreshes srcmap-codec / srcmap-scopes to 0.3.9.\n\nCo-authored-by: Bart Waardenburg <bart@waardenburg.dev>",
          "timestamp": "2026-07-01T11:57:40+02:00",
          "tree_id": "36b6dc6be34fff2e7002a36decc05bf056a21a52",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/8a210549b14a036dbf6a7c58b59a323641369c3e"
        },
        "date": 1782899997484,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86984976,
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
          "id": "4708b72592ad5b217604dbd18005ca4c3358cc81",
          "message": "feat(instrument): name callback arguments from the callee (opt-in) (#151)\n\nA function or arrow passed directly as a call or `new` argument has no\nbinding to inherit a name from, so both istanbul-lib-instrument and this\ninstrumenter fall back to `(anonymous_N)`. In callback-heavy code (route\nhandlers, `.map`/`.filter`, promise `.then`, `describe`/`it`,\n`new Promise`) that fallback dominates the fnMap.\n\nAdd an opt-in `name_callback_arguments` option (napi:\n`nameCallbackArguments`, vitest adapter: same) that names these from the\ncallee: `arr.map(cb)` -> \"map\", `new Promise(cb)` -> \"Promise\",\n`el.addEventListener(\"click\", cb)` -> \"addEventListener\". A binding name\nand an explicit named function expression still take precedence; this only\nreplaces the `(anonymous_N)` fallback.\n\nOnly the callee is used, never a sibling string argument: the traversal\nancestor for an argument position exposes the callee but not the other\narguments. The name is also stable across rebuilds (the `(anonymous_N)`\ncounter renumbers when unrelated functions are added), which matters for\ntools that key function identity on the name.\n\nDefaults to false, so default output stays byte-identical to Istanbul.\nDocumented under README \"Differences from istanbul-lib-instrument\".",
          "timestamp": "2026-07-01T15:26:55+02:00",
          "tree_id": "3c1a76cef758b2529696438c81eaec91828be1df",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/4708b72592ad5b217604dbd18005ca4c3358cc81"
        },
        "date": 1782912575167,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86991128,
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
          "id": "04b3095d769084694d458ec9c5df982bee01ccb5",
          "message": "chore: release v0.10.0",
          "timestamp": "2026-07-01T15:31:23+02:00",
          "tree_id": "8bc826938c5bfd16b6ff5288ac9f813847777867",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/04b3095d769084694d458ec9c5df982bee01ccb5"
        },
        "date": 1782912964320,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 86982368,
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
          "id": "b316b48de1e30972a568f4a5b3bce3dd8fb18568",
          "message": "perf(source-maps): reuse getMapping caches across eager-gate calls (#152)\n\n* perf(source-maps): reuse getMapping caches across eager-gate calls\n\nThe eager AST-level drop gate (issue #106/#122) calls\n`PositionRemapper::location_maps` once per coverage node on a hot\ntraversal path. Each call built a fresh `RemapContext`, so the per-(source,\nline) original-column index (issue #122 getMapping, populated by scanning\nevery mapping in the map) was rebuilt and thrown away on every node, making\nthe b5e1264/7b3c6c8 caches dead weight on this path.\n\nGive `PositionRemapper` a `RefCell<RemapCaches>` (mapping cache +\ncolumn index) that persists across every `location_maps` call for one map,\nand route the gate through `get_mapping_location_cached`. The caches are a\npure function of (location, source map) and the source map is fixed per\nremapper, so results are unchanged; only redundant per-node work is\nremoved. `apply_source_map` keeps a fresh per-call cache set. `RefCell`\nbecause `location_maps` takes `&self` (the transform visits with `&self`).\n\n* docs(reports): note html skip path intentionally omits orphan prune\n\nThe no-source-map fast path (3f10800) renders from the borrowed original\ncoverage map and skips `remap_coverage_map`, so it also skips\n`FileCoverage::prune_orphan_counters`. Document why that is safe (every\nhtml counter consumer looks up `s`/`f`/`b` via\n`.get(id).unwrap_or(0)` keyed by the corresponding map, so orphan slots\nare never observed) and when to revisit (if `write` ever re-serializes a\nraw `FileCoverage`). No behavior change.",
          "timestamp": "2026-07-01T15:48:15+02:00",
          "tree_id": "2ee8172b6fc46349379cbb345ac77e191be4e7e3",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/b316b48de1e30972a568f4a5b3bce3dd8fb18568"
        },
        "date": 1782913978998,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 87035376,
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
          "id": "2a57a92feff7df2ba8798f6971849a7d637faaca",
          "message": "chore: release v0.10.1",
          "timestamp": "2026-07-01T16:33:33+02:00",
          "tree_id": "3f8daf3aba3776572a2828043d35111ba1b2d54c",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/2a57a92feff7df2ba8798f6971849a7d637faaca"
        },
        "date": 1782916629560,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 87046616,
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
          "id": "498f9613c8aeac7e1f207b753c6a52c1e4ca5dd8",
          "message": "fix(instrument): resolve parenthesized callees in callback naming (#153)\n\n* fix(instrument): resolve parenthesized callees in callback naming\n\nThe opt-in `name_callback_arguments` (#151) left `(foo)(cb)` and\n`foo((cb))` as `(anonymous_N)` because Oxc keeps `ParenthesizedExpression`\nas a real AST node (Babel strips it). Unwrap parens in `callee_name`\n(`(foo)(cb)` -> foo, `(a.b)(cb)` -> b) and skip `ParenthesizedExpression`\nancestors in `callback_argument_name` (`foo((function(){}))` -> foo). An\nIIFE whose parenthesized callee is a function stays anonymous (the callee\nposition is not an argument), guarded by a new test. Naming only; no counter,\nspan, or default-path change.\n\n* test(instrument): reword IIFE comment to satisfy typos",
          "timestamp": "2026-07-01T17:22:11+02:00",
          "tree_id": "9f1899e235b0cf42a59afd7b9a8cd353e4964286",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/498f9613c8aeac7e1f207b753c6a52c1e4ca5dd8"
        },
        "date": 1782919488517,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 87047216,
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
          "id": "af6d5ec71ac781ffc4f7b2b402e18954c6266436",
          "message": "chore(deps): update Dependabot queue",
          "timestamp": "2026-07-12T20:32:08+02:00",
          "tree_id": "6fb4c2b84ffb6f7a1c9c253b7587de8aac1ff9f7",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/af6d5ec71ac781ffc4f7b2b402e18954c6266436"
        },
        "date": 1783881332798,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 87063280,
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
          "id": "61aac68cba874eedfcdb6c5e29b7523dcc555efb",
          "message": "docs(source-maps): fix lookup rustdoc typo",
          "timestamp": "2026-07-13T23:09:42+02:00",
          "tree_id": "1b9b22c9ae4483079798c55661ace6aa40e86be7",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/61aac68cba874eedfcdb6c5e29b7523dcc555efb"
        },
        "date": 1783977230744,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 90292792,
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
          "id": "4081e20c79ba043b8cf97c9e59f45c91ab3a4e15",
          "message": "test(cli): crawl portable html reports",
          "timestamp": "2026-07-14T13:11:27+02:00",
          "tree_id": "befbbbd5c175a47a8eb964a2de2d8e2e503def94",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/4081e20c79ba043b8cf97c9e59f45c91ab3a4e15"
        },
        "date": 1784032745065,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 91714360,
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
          "id": "0784209c4050414a53affacde225bb34e8fe6aa8",
          "message": "chore: release v0.10.2",
          "timestamp": "2026-07-14T15:31:39+02:00",
          "tree_id": "d9bd159a7fe889a962d27835f695179df1d26b8e",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/0784209c4050414a53affacde225bb34e8fe6aa8"
        },
        "date": 1784036622073,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 91714264,
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
          "id": "8a0d2fffdeb8ce1704d83cf3e59d4d46079f3e58",
          "message": "chore: release v0.10.3",
          "timestamp": "2026-07-14T16:44:57+02:00",
          "tree_id": "0f4b5b4c31e502bb7207c350459dcaae711f5778",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/8a0d2fffdeb8ce1704d83cf3e59d4d46079f3e58"
        },
        "date": 1784040762043,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 91710232,
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
          "distinct": false,
          "id": "b4b359c7e67f443e6ce8630faac9eebff27df858",
          "message": "chore(deps): bump oxc to 0.140 and refresh the dependency tree\n\noxc 0.140 requires Rust 1.95, so the workspace MSRV moves 1.92 -> 1.95.\nThat gate matters beyond the pin: while rust-version stayed at 1.92,\ncargo's resolver silently held oxc_sourcemap at 8.1.0 and\noxc-browserslist at 3.0.9.\n\nThree API breaks came with the bump:\n\n- DecoratorOptions gained strict_null_checks, set to the oxc and tsc\n  default of true. It is only consulted when emit_decorator_metadata is\n  on.\n- ParserReturn/TransformerReturn `errors` is now `diagnostics`, which is\n  not a pure rename: the new Diagnostics type carries warnings as well.\n  Both checks gate on has_errors()/errors() so a warning-only source is\n  not newly reported as a failure.\n- The transformer now asserts on SemanticBuilder::with_enum_eval(true)\n  when lowering a TypeScript enum. It is enabled only on the scoping fed\n  to the strip pass; the V8-collect path builds its own scoping for the\n  traverse pass and never reaches the transformer.\n\nThe old ctx.ast.* builder methods are deprecated in favour of the\ntype-associated interface (oxc#23043). With clippy running under\n-D warnings that migration is mandatory, so all 29 call sites in\ntransform.rs move to Type::new_xxx(.., ctx) and ArenaVec::new_in(ctx).\n\nOn the npm side: @commitlint/cli 21.2.1, @napi-rs/cli 3.7.3, and the\nemnapi trio to 1.11.2 in lockstep so the override from #149 holds. The\ntracked Cloudflare Workers lockfile still pinned the linked napi package\nat 0.7.2 and is resynced.",
          "timestamp": "2026-07-20T15:42:37+02:00",
          "tree_id": "136b91a35fc307d092139f1e23023226207bb2da",
          "url": "https://github.com/fallow-rs/oxc-coverage-instrument/commit/b4b359c7e67f443e6ce8630faac9eebff27df858"
        },
        "date": 1784555212186,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (oxc-coverage-instrument CLI)",
            "value": 90675016,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}