//! Coordinate conversion for the V8-visible source.
//!
//! Three coordinate systems meet here: V8 reports UTF-16 code-unit offsets,
//! Istanbul positions are 1-based line plus 0-based UTF-16 column, and oxc
//! spans are UTF-8 byte offsets. [`SourceIndex`] precomputes the line table and
//! the runs of characters whose UTF-8 and UTF-16 widths differ, so both of the
//! source-side systems can be mapped onto V8's.

#[derive(Clone, Copy)]
struct NonAsciiSpan {
    byte_start: u32,
    byte_end: u32,
    utf16_start: u32,
    utf16_end: u32,
}

pub struct SourceIndex {
    line_starts_utf16: Vec<u32>,
    line_ends_utf16: Vec<u32>,
    non_ascii_spans: Vec<NonAsciiSpan>,
    byte_len: u32,
    utf16_len: u32,
}

impl SourceIndex {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "`char::len_utf8` and `char::len_utf16` both return a value in `1..=4`"
    )]
    pub fn new(source: &str) -> Self {
        let mut line_starts_utf16 = vec![0];
        let mut line_ends_utf16 = Vec::new();
        let mut non_ascii_spans = Vec::new();
        let mut utf16_offset = 0u32;

        let mut chars = source.char_indices().peekable();
        while let Some((byte_start, ch)) = chars.next() {
            let byte_start = u32::try_from(byte_start).unwrap_or(u32::MAX);
            let byte_width = ch.len_utf8() as u32;
            let utf16_width = ch.len_utf16() as u32;
            if byte_width != utf16_width {
                non_ascii_spans.push(NonAsciiSpan {
                    byte_start,
                    byte_end: byte_start.saturating_add(byte_width),
                    utf16_start: utf16_offset,
                    utf16_end: utf16_offset.saturating_add(utf16_width),
                });
            }

            match ch {
                '\r' => {
                    line_ends_utf16.push(utf16_offset);
                    utf16_offset = utf16_offset.saturating_add(1);
                    if chars.peek().is_some_and(|(_, next)| *next == '\n') {
                        chars.next();
                        utf16_offset = utf16_offset.saturating_add(1);
                    }
                    line_starts_utf16.push(utf16_offset);
                }
                '\n' | '\u{2028}' | '\u{2029}' => {
                    line_ends_utf16.push(utf16_offset);
                    utf16_offset = utf16_offset.saturating_add(utf16_width);
                    line_starts_utf16.push(utf16_offset);
                }
                _ => utf16_offset = utf16_offset.saturating_add(utf16_width),
            }
        }
        line_ends_utf16.push(utf16_offset);

        Self {
            line_starts_utf16,
            line_ends_utf16,
            non_ascii_spans,
            byte_len: u32::try_from(source.len()).unwrap_or(u32::MAX),
            utf16_len: utf16_offset,
        }
    }

    /// A byte offset inside a multi-byte character resolves to the start of
    /// that character, so a span boundary can never land mid-code-point.
    pub fn byte_to_utf16(&self, byte_offset: u32) -> u32 {
        let byte_offset = byte_offset.min(self.byte_len);
        let index = self.non_ascii_spans.partition_point(|span| span.byte_end <= byte_offset);
        if let Some(span) = self.non_ascii_spans.get(index)
            && byte_offset > span.byte_start
            && byte_offset < span.byte_end
        {
            return span.utf16_start;
        }
        let byte_utf16_delta = index
            .checked_sub(1)
            .and_then(|previous| self.non_ascii_spans.get(previous))
            .map_or(0, |span| span.byte_end.saturating_sub(span.utf16_end));
        byte_offset.saturating_sub(byte_utf16_delta).min(self.utf16_len)
    }

    /// A column past the end of its line clamps to that line's end rather than
    /// spilling into the next one.
    pub fn position_to_utf16(&self, line_1based: u32, column_utf16: u32) -> u32 {
        if line_1based == 0 {
            return 0;
        }
        let line_index = (line_1based - 1) as usize;
        let Some(line_start) = self.line_starts_utf16.get(line_index).copied() else {
            return self.utf16_len;
        };
        let line_end = self.line_ends_utf16.get(line_index).copied().unwrap_or(self.utf16_len);
        line_start.saturating_add(column_utf16.min(line_end.saturating_sub(line_start)))
    }
}
