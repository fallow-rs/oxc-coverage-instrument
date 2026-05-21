window.BENCHMARK_DATA = {
  "lastUpdate": 1779347138366,
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
      }
    ]
  }
}