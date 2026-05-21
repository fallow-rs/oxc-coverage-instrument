//! Edge cases for the V8 conversion entry points and source-map helpers.
//!
//! The integration suite in `oxc-coverage-instrument/tests/v8_to_istanbul_test.rs`
//! drives the converter through the high-level `v8_to_istanbul` wrapper, so it
//! only exercises paths that real instrumenter output hits. This file targets
//! the public surface of `oxc_coverage_v8` directly: the URL-safe base64 and
//! percent-encoded inline source-map decoders, the external-URL trailer parser,
//! and the V8 arm tie-breaker that has to pick a tighter range when two V8
//! ranges sit equidistant from a branch arm location.
//!
//! All tests go through the public API so they survive renames or internal
//! refactors of `decode_base64`, `urlencoding_decode`, and friends.

use std::collections::BTreeMap;

use oxc_coverage_types::{BranchEntry, FileCoverage, Location, Position};
use oxc_coverage_v8::{
    V8CoverageRange, V8FunctionCoverage, apply_v8_coverage, extract_external_source_mapping_url,
    extract_inline_source_map,
};

// The `~~~~~~` payload guarantees the URL-safe encoder emits `-` (alphabet
// index 62), so the decoder's `b'+' | b'-'` arm gets exercised. Six tildes
// pack into sextets `011111 100111 111001 111110 011111 100111 111001 111110`,
// and slot 62 maps to `-` in the URL-safe alphabet.
const SAMPLE_MAP: &str = r#"{"version":3,"sources":["foo.ts"],"mappings":"","note":"~~~~~~"}"#;

/// Tiny base64 encoder using the URL-safe alphabet (`-` / `_`) and no
/// padding. We bring our own so the test does not pull in a base64 crate
/// just to round-trip eight characters; equally importantly, hitting the
/// URL-safe alphabet on the decode side is exactly what we want to cover.
fn b64_urlsafe(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(((b1 & 0b1111) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(b2 & 0b11_1111) as usize] as char);
        }
    }
    out
}

#[test]
fn extracts_inline_source_map_with_urlsafe_base64_alphabet() {
    let payload = b64_urlsafe(SAMPLE_MAP.as_bytes());
    assert!(
        payload.contains('-') || payload.contains('_'),
        "test fixture must exercise the URL-safe alphabet; got {payload}",
    );
    let source =
        format!("const x = 1;\n//# sourceMappingURL=data:application/json;base64,{payload}");
    let map = extract_inline_source_map(&source).expect("URL-safe base64 should decode");
    assert_eq!(map["version"], 3);
    assert_eq!(map["sources"][0], "foo.ts");
}

#[test]
fn extracts_inline_source_map_from_percent_encoded_payload() {
    // The non-base64 inline form: esbuild emits this when `sourcemap=inline`
    // is paired with the no-base64 mode. The payload is `encodeURIComponent`
    // applied to the JSON, so the decoder has to walk `%HH` escapes.
    let percent =
        "%7B%22version%22%3A3%2C%22sources%22%3A%5B%22foo.ts%22%5D%2C%22mappings%22%3A%22%22%7D";
    let source = format!("const x = 1;\n//# sourceMappingURL=data:application/json,{percent}");
    let map = extract_inline_source_map(&source).expect("percent payload should decode");
    assert_eq!(map["version"], 3);
    assert_eq!(map["sources"][0], "foo.ts");
}

#[test]
fn inline_source_map_returns_none_on_malformed_payloads() {
    // Trailer says `data:application/json` but stops before the payload
    // separator. An editor truncation or a hand-rolled bundler bug looks
    // like this; the helper should fail closed without panicking.
    assert!(extract_inline_source_map("//# sourceMappingURL=data:application/json").is_none());

    // Garbage after the comma: not valid JSON in either alphabet.
    assert!(extract_inline_source_map("//# sourceMappingURL=data:application/json,@@@@").is_none(),);
}

#[test]
fn external_source_mapping_url_filters_data_urls_and_blanks() {
    // Inline data URL is the inline-form's job, not this one.
    assert!(
        extract_external_source_mapping_url(
            "x;\n//# sourceMappingURL=data:application/json;base64,e30=",
        )
        .is_none(),
    );

    // Blank URL (trailer with nothing after the `=`).
    assert!(extract_external_source_mapping_url("x;\n//# sourceMappingURL=").is_none());

    // Whitespace-only URL after trimming.
    assert!(extract_external_source_mapping_url("x;\n//# sourceMappingURL=   ").is_none());

    // Valid external URL is returned verbatim (trimmed).
    assert_eq!(
        extract_external_source_mapping_url("x;\n//# sourceMappingURL=foo.js.map"),
        Some("foo.js.map"),
    );
}

#[test]
fn arm_tiebreaker_prefers_tighter_v8_range_at_equal_distance() {
    // Branch arm on line 3, columns 4..6, mapping to byte offsets [26, 28).
    // Two V8 block ranges flank it within the 4-byte tolerance:
    //   A: [25, 28) -> distance 1+0 = 1, width 3
    //   B: [26, 27) -> distance 0+1 = 1, width 1
    // Same total distance, but B is the narrower (more block-specific) range,
    // so its count must win. Ranges are appended in `A, B` order so the
    // tie-breaker actually has to replace `best`.
    let source = "let x = 1;\nlet y = 2;\nif (xx) y;\n";

    let mut fc = FileCoverage {
        path: "tiebreak.js".to_string(),
        statement_map: BTreeMap::new(),
        fn_map: BTreeMap::new(),
        branch_map: BTreeMap::new(),
        s: BTreeMap::new(),
        f: BTreeMap::new(),
        b: BTreeMap::new(),
        b_t: None,
        input_source_map: None,
    };
    fc.branch_map.insert(
        "0".to_string(),
        BranchEntry {
            loc: Location {
                start: Position { line: 3, column: 0 },
                end: Position { line: 3, column: 11 },
            },
            line: 3,
            branch_type: "if".to_string(),
            locations: vec![Location {
                start: Position { line: 3, column: 4 },
                end: Position { line: 3, column: 6 },
            }],
        },
    );
    fc.b.insert("0".to_string(), vec![0]);

    let wider = V8CoverageRange { start_offset: 25, end_offset: 28, count: 7 };
    let tighter = V8CoverageRange { start_offset: 26, end_offset: 27, count: 99 };
    let functions = vec![V8FunctionCoverage {
        function_name: String::new(),
        ranges: vec![wider, tighter],
        is_block_coverage: true,
    }];

    apply_v8_coverage(&mut fc, source, &functions, 0, &BTreeMap::new());
    assert_eq!(fc.b["0"][0], 99, "tighter range must win the tie-breaker for arm count");
}
