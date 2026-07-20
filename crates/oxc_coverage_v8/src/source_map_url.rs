//! `//# sourceMappingURL=` trailer parsing.
//!
//! V8 coverage data carries no source-map information; the source does, in a
//! trailing directive comment. This module reads that comment and decodes the
//! inline `data:application/json` form.

const SOURCE_MAPPING_URL_PREFIX: &str = "//# sourceMappingURL=";

fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// The directive counts only on the last non-whitespace physical line, and a
/// URL containing a quote character is rejected: a multi-line template literal
/// whose closing line opens with the marker would otherwise parse as one.
fn source_mapping_url_trailer(source: &str) -> Option<&str> {
    let source = source.trim_end();
    let line_start = source
        .char_indices()
        .rev()
        .find_map(|(index, ch)| is_line_terminator(ch).then_some(index + ch.len_utf8()))
        .unwrap_or(0);
    let line = source.get(line_start..)?.trim_start();
    let url = line.strip_prefix(SOURCE_MAPPING_URL_PREFIX)?.trim();
    if url.is_empty() || url.chars().any(|ch| matches!(ch, '\'' | '"' | '`')) {
        return None;
    }
    Some(url)
}

/// Read the URL of a trailing `//# sourceMappingURL=<url>` comment.
///
/// Returns `None` when `source` has no trailer, or when the trailer is a
/// `data:` URI: the inline form is decoded by [`extract_inline_source_map`].
#[must_use] // A pure parse; discarding the URL leaves the call with no effect
pub fn extract_external_source_mapping_url(source: &str) -> Option<&str> {
    let url = source_mapping_url_trailer(source)?;
    (!url.starts_with("data:")).then_some(url)
}

/// Decode the source map embedded in a trailing
/// `//# sourceMappingURL=data:application/json,...` comment.
///
/// Both the `;base64,` payload and the percent-encoded payload are accepted.
/// Only the `data:` form is decoded here; an external URL is reported by
/// [`extract_external_source_mapping_url`] and loaded by the caller.
#[must_use] // A pure parse; discarding the map leaves the call with no effect
pub fn extract_inline_source_map(source: &str) -> Option<serde_json::Value> {
    let url = source_mapping_url_trailer(source)?;
    let data = url.strip_prefix("data:application/json")?;
    let comma = data.find(',')?;
    let metadata = &data[..comma];
    let payload = &data[comma + 1..];
    let json = if metadata.split(';').any(|part| part == "base64") {
        let bytes = decode_base64(payload)?;
        String::from_utf8(bytes).ok()?
    } else {
        decode_percent_encoded(payload)?
    };
    serde_json::from_str(&json).ok()
}

/// Accepts both the standard (RFC 4648 §4) and the URL-safe (RFC 4648 §5)
/// alphabet, and treats padding as optional.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn value(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0u8; 4];
    let mut len = 0usize;
    for byte in input.bytes() {
        if byte == b'=' || byte.is_ascii_whitespace() {
            continue;
        }
        chunk[len] = byte;
        len += 1;
        if len == 4 {
            let n0 = value(chunk[0])?;
            let n1 = value(chunk[1])?;
            let n2 = value(chunk[2])?;
            let n3 = value(chunk[3])?;
            out.push((n0 << 2) | (n1 >> 4));
            out.push((n1 << 4) | (n2 >> 2));
            out.push((n2 << 6) | n3);
            len = 0;
        }
    }
    // A lone trailing sextet cannot encode a whole byte.
    if len == 1 {
        return None;
    }
    if len > 1 {
        let n0 = value(chunk[0])?;
        let n1 = value(chunk[1])?;
        out.push((n0 << 2) | (n1 >> 4));
        if len > 2 {
            let n2 = value(chunk[2])?;
            out.push((n1 << 4) | (n2 >> 2));
        }
    }
    Some(out)
}

fn decode_percent_encoded(input: &str) -> Option<String> {
    fn hex_digit(byte: u8) -> Option<u8> {
        char::from(byte).to_digit(16).and_then(|digit| u8::try_from(digit).ok())
    }

    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = hex_digit(*bytes.get(i + 1)?)?;
            let lo = hex_digit(*bytes.get(i + 2)?)?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::decode_percent_encoded;

    #[test]
    fn rejects_truncated_percent_escape_at_payload_end() {
        assert_eq!(decode_percent_encoded("payload%"), None);
        assert_eq!(decode_percent_encoded("payload%4"), None);
    }
}
