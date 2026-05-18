//! Server-side syntax highlighter for the html reporter's detail pages.
//!
//! Wraps [`syntect`] with a per-line facade: callers hand over the source
//! text and the file path, and get back a `Vec<String>` where each entry
//! is the highlighted HTML for the corresponding line of the input. The
//! per-line shape matches the line-numbered table layout in `html.rs`,
//! so each line drops straight into a `<pre>` cell.
//!
//! Token spans use [`syntect::html::ClassStyle::SpacedPrefixed`] with the
//! prefix `stok-`, so emitted classes look like `class="stok-comment
//! stok-line stok-double-slash"`. CSS targets the leading-segment
//! classes (`stok-comment`, `stok-keyword`, `stok-string`, ...) and maps
//! them to our `--tok-*` vars (G3) or fallow's `--fallow-syntax-token-*`
//! vars (G4). Trailing scope-tail classes are kept in markup for future
//! finer-grained theming.
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

use crate::escape::html_text;

use syntect::html::{ClassStyle, line_tokens_to_classed_spans};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Prefix applied to every emitted token class. Keeps the syntax-token
/// class namespace separate from the existing coverage-row classes
/// (`hit`, `miss`, `partial`, `no-stmt`, `sortable`, `tok-k`, ...).
const CLASS_PREFIX: &str = "stok-";

const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: CLASS_PREFIX };

/// Cache the syntect `SyntaxSet`. Parsing the bundled grammar archive on
/// first use takes ~5 ms; we want to pay that exactly once per process.
///
/// Uses [`two_face::syntax::extra_newlines`] which extends syntect's
/// default-fancy bundle with TypeScript, TSX, JSX, and many other
/// grammars not included in the stock syntect ship-set.
fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Render `source` as a `Vec<String>` of HTML lines.
///
/// Returns one entry per input line. Trailing newlines are stripped
/// from each entry; the caller chooses whether and how to re-insert
/// them (the line-numbered table layout in `html.rs` does not need
/// them).
///
/// `file_path` is consulted only for the extension-based language
/// lookup; the file does not need to exist on disk. Pass `Path::new("")`
/// if no path hint is available, and lines will be rendered as plain
/// HTML-escaped text (no colours), which is what we also do for unknown
/// extensions so the report never breaks on obscure file types.
pub fn highlight_lines(source: &str, file_path: &Path) -> Vec<String> {
    let ss = syntax_set();
    let Some(syntax) = pick_syntax(ss, file_path) else {
        // Plain HTML-escaped fallback, one line per `\n`. Mirrors the
        // shape of the highlighted path so the caller does not branch
        // on language availability.
        return source.split('\n').map(html_text).collect();
    };

    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let mut out: Vec<String> = Vec::new();

    for line in LinesWithEndings::from(source) {
        // syntect produces tokens for `line` including the trailing `\n`.
        // `line_tokens_to_classed_spans` returns the HTML for the line
        // and mutates `scope_stack` to reflect the post-line state.
        let ops = parse_state
            .parse_line(line, ss)
            .expect("syntect line parsing should succeed on valid UTF-8");
        let (mut html, _delta) =
            line_tokens_to_classed_spans(line, ops.as_slice(), CLASS_STYLE, &mut scope_stack)
                .expect("syntect span emission should succeed on valid UTF-8");
        // Drop the trailing newline syntect echoes from the input; the
        // table cell uses `<pre>` per line and re-adds newlines via row
        // boundaries.
        if html.ends_with('\n') {
            html.pop();
        }
        out.push(html);
    }

    // `LinesWithEndings::from` skips a final trailing empty line if the
    // input ends in `\n`. Coverage rendering walks `source.split('\n')`,
    // which produces one extra empty trailing line in that case. Match
    // that shape so line numbers line up with the caller's iteration.
    if source.ends_with('\n') {
        out.push(String::new());
    }
    // If the input is empty entirely (no lines at all), produce one
    // empty entry so the caller's `enumerate` yields the same shape as
    // `"".split('\n')`.
    if source.is_empty() {
        out.push(String::new());
    }
    out
}

/// Look up a syntect [`SyntaxReference`] for `file_path`.
///
/// Tries the file extension first (covers `.ts`, `.tsx`, `.js`, `.jsx`,
/// `.rs`, `.py`, `.json`, ... out of the box). Returns `None` if no
/// matching grammar is bundled.
fn pick_syntax<'a>(ss: &'a SyntaxSet, file_path: &Path) -> Option<&'a SyntaxReference> {
    let ext = file_path.extension().and_then(|e| e.to_str())?;
    ss.find_syntax_by_extension(ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_typescript_with_prefixed_classes() {
        let lines = highlight_lines("const x: number = 1;\n", Path::new("a.ts"));
        // Two entries: the actual line + a trailing empty (matches
        // `.split('\n')` behavior on trailing-newline input).
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
    }

    #[test]
    fn empty_source_yields_one_empty_line() {
        // Mirrors `"".split('\n')` which yields one empty string.
        assert_eq!(highlight_lines("", Path::new("a.ts")), vec![String::new()]);
        assert_eq!(highlight_lines("", Path::new("")), vec![String::new()]);
    }

    #[test]
    fn no_path_hint_falls_back_to_plain() {
        let lines = highlight_lines("const x = 1;", Path::new(""));
        assert!(!lines[0].contains("stok-"), "no path hint -> no spans");
        assert!(lines[0].contains("const x = 1;"));
    }

    #[test]
    fn multi_line_string_carries_scope_state() {
        // Template literal spans multiple lines. With per-line ScopeStack
        // continuity, the second line should also carry a `stok-string`
        // class somewhere in its tokens, NOT plain identifier markup.
        let src = "const s = `line1\nline2`;\n";
        let lines = highlight_lines(src, Path::new("a.ts"));
        // Line 1 contains the opening backtick + "line1" -> string scope.
        assert!(lines[0].contains("stok-string"), "line 1 string scope missing: {}", lines[0]);
        // Line 2 is the continuation of the template literal; the
        // continuation should still be in string scope until the closing
        // backtick.
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
