window.BENCHMARK_DATA = {
  "lastUpdate": 1780483900530,
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
      }
    ]
  }
}