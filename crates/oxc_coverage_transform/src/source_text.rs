//! ECMAScript source-text location helpers for the transform kernel.

/// Find the first ECMAScript line terminator and return its byte offset and
/// byte width. CRLF is one logical terminator spanning two bytes.
pub fn find_line_terminator(source: &str) -> Option<(usize, usize)> {
    for (index, ch) in source.char_indices() {
        match ch {
            '\r' => {
                let width = if source.as_bytes().get(index + 1) == Some(&b'\n') { 2 } else { 1 };
                return Some((index, width));
            }
            '\n' => return Some((index, 1)),
            '\u{2028}' | '\u{2029}' => return Some((index, ch.len_utf8())),
            _ => {}
        }
    }
    None
}

/// Compute the byte offset at which each logical ECMAScript line starts.
pub fn line_starts(source: &str) -> Vec<u32> {
    let mut starts = vec![0];
    let mut cursor = 0usize;
    while let Some((relative, width)) = find_line_terminator(&source[cursor..]) {
        cursor += relative + width;
        starts.push(u32::try_from(cursor).unwrap_or(u32::MAX));
    }
    starts
}

/// Convert a byte offset to a one-based line and zero-based UTF-16 column.
pub fn line_column(source: &str, offset: u32) -> (u32, u32) {
    let starts = line_starts(source);
    let offset = (offset as usize).min(source.len());
    let line = starts.partition_point(|start| *start as usize <= offset).saturating_sub(1);
    let start = starts[line] as usize;
    let column = u32::try_from(source[start..offset].encode_utf16().count()).unwrap_or(u32::MAX);
    (u32::try_from(line + 1).unwrap_or(u32::MAX), column)
}

#[cfg(test)]
mod tests {
    use super::{find_line_terminator, line_column, line_starts};

    #[test]
    fn line_starts_treat_crlf_as_one_break() {
        assert_eq!(line_starts("a\r\nb"), vec![0, 3]);
        assert_eq!(find_line_terminator("a\r\nb"), Some((1, 2)));
    }

    #[test]
    fn columns_are_utf16_code_units() {
        let source = "a\u{2028}😀x";
        let offset = u32::try_from(source.find('x').unwrap()).unwrap();
        assert_eq!(line_column(source, offset), (2, 2));
    }
}
