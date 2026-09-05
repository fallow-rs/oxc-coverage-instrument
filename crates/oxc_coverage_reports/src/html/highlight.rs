//! Server-side syntax highlighter for the html reporter's detail pages.
//!
//! Wraps [`syntect`] with a per-line facade: callers hand over the source
//! text and the file path, and get back a `Vec<String>` where each entry
//! is the highlighted HTML for the corresponding line of the input. The
//! per-line shape matches the line-numbered table layout in
//! [`super::detail_page`], so each line drops straight into a `<pre>` cell.
//!
//! Token spans use [`syntect::html::ClassStyle::SpacedPrefixed`] with the
//! prefix `stok-`, so emitted classes look like `class="stok-comment
//! stok-line stok-double-slash"`. CSS targets the leading-segment classes
//! (`stok-comment`, `stok-keyword`, `stok-string`, ...) and maps them to
//! the `--tok-*` custom properties in `coverage-tokens.css`; the trailing
//! scope-tail classes stay in the markup so a consumer can theme at a
//! finer grain.
//!
//! Scope state is carried across lines via a shared
//! [`syntect::parsing::ScopeStack`], so multi-line strings, block
//! comments, and template literals colour their continuation lines
//! correctly.
//!
//! The `SyntaxSet` is built on first use and cached behind a
//! [`std::sync::OnceLock`]; subsequent calls share the bundled
//! ~500 KB grammar archive.
//!
//! [syntect]: https://docs.rs/syntect

use std::path::Path;
use std::sync::OnceLock;

use syntect::html::{ClassStyle, line_tokens_to_classed_spans};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};
use syntect::util::LinesWithEndings;

use crate::escape::html_text;

/// Prefix applied to every emitted token class. Keeps the syntax-token
/// class namespace separate from the existing coverage-row classes
/// (`hit`, `miss`, `partial`, `no-stmt`, `sortable`, `tok-k`, ...).
const CLASS_PREFIX: &str = "stok-";

const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: CLASS_PREFIX };

/// The bundled grammar archive, parsed once per process.
///
/// [`two_face::syntax::extra_newlines`] extends syntect's default-fancy
/// bundle with TypeScript, TSX, JSX and many other grammars the stock
/// ship-set leaves out.
fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Render `source` as one HTML string per input line, with the trailing
/// newline stripped from each: the table layout re-adds line breaks via
/// row boundaries.
///
/// `file_path` is consulted only for the extension-based language lookup;
/// the file need not exist on disk. An unknown or absent extension falls
/// back to plain HTML-escaped text, so the report never breaks on an
/// obscure file type.
pub fn highlight_lines(source: &str, file_path: &Path) -> Vec<String> {
    let syntaxes = syntax_set();
    let Some(syntax) = file_path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(|e| syntaxes.find_syntax_by_extension(e))
    else {
        // Escaped-only fallback. Mirrors the shape of the highlighted path
        // so the caller does not branch on language availability.
        return source.split('\n').map(html_text).collect();
    };

    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let mut out: Vec<String> = Vec::new();

    for line in LinesWithEndings::from(source) {
        // `line_tokens_to_classed_spans` mutates `scope_stack` to the
        // post-line state, which is what carries a multi-line string or
        // block comment into its continuation lines.
        let ops = parse_state
            .parse_line(line, syntaxes)
            .expect("syntect line parsing should succeed on valid UTF-8");
        let (mut html, _delta) =
            line_tokens_to_classed_spans(line, ops.as_slice(), CLASS_STYLE, &mut scope_stack)
                .expect("syntect span emission should succeed on valid UTF-8");
        if html.ends_with('\n') {
            html.pop();
        }
        out.push(html);
    }

    // `LinesWithEndings` yields no entry for the empty region after a
    // trailing `\n`, and none at all for empty input. The caller numbers
    // lines as if it walked `source.split('\n')`, so pad to that shape.
    if source.ends_with('\n') || source.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_typescript_with_prefixed_classes() {
        let lines = highlight_lines("const x: number = 1;\n", Path::new("a.ts"));
        // The source line plus the empty region after its trailing `\n`.
        assert_eq!(lines.len(), 2, "got {} lines: {lines:?}", lines.len());
        assert!(lines[0].contains("stok-"), "expected prefixed class: {}", lines[0]);
        assert!(lines[0].contains("const"), "must keep source literal text");
        assert!(lines[0].contains("number"), "must keep source literal text");
        assert_eq!(lines[1], "");
    }

    #[test]
    fn unknown_extension_falls_back_to_escaped_text() {
        let lines = highlight_lines("hello <world> & friends", Path::new("file.unknown"));
        assert_eq!(lines.len(), 1, "single line, no trailing newline");
        assert!(lines[0].contains("hello"));
        assert!(lines[0].contains("&lt;world&gt;"));
        assert!(lines[0].contains("&amp;"));
        assert!(!lines[0].contains("stok-"), "fallback should not emit token spans: {}", lines[0]);

        // A path with no extension at all takes the same fallback branch.
        assert_eq!(highlight_lines("hello <world> & friends", Path::new("")), lines);
    }

    #[test]
    fn empty_source_yields_one_empty_line() {
        // Mirrors `"".split('\n')` which yields one empty string.
        assert_eq!(highlight_lines("", Path::new("a.ts")), vec![String::new()]);
        assert_eq!(highlight_lines("", Path::new("")), vec![String::new()]);
    }

    #[test]
    fn multi_line_string_carries_scope_state() {
        // A template literal opened on line 1 stays in string scope on line
        // 2, which only holds if the `ScopeStack` carries across lines.
        let src = "const s = `line1\nline2`;\n";
        let lines = highlight_lines(src, Path::new("a.ts"));
        assert!(lines[0].contains("stok-string"), "line 1 string scope missing: {}", lines[0]);
        assert!(
            lines[1].contains("stok-string"),
            "line 2 should continue string scope: {}",
            lines[1]
        );
    }

    #[test]
    fn hostile_source_is_escaped_under_highlight() {
        let lines = highlight_lines("// <script>alert(1)</script>\n", Path::new("a.js"));
        assert!(!lines[0].contains("<script>"), "raw <script> tag leaked: {}", lines[0]);
        assert!(lines[0].contains("&lt;script&gt;"), "expected escaped script tag: {}", lines[0]);
    }

    #[test]
    fn line_count_matches_split_newline() {
        let src = "a\nb\nc";
        let lines = highlight_lines(src, Path::new("a.ts"));
        assert_eq!(lines.len(), src.split('\n').count(), "line count must match `.split('\\n')`");
    }
}
