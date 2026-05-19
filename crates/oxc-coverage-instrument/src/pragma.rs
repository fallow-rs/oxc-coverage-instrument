//! Istanbul/v8 coverage pragma comment handling.
//!
//! Scans AST comments for `istanbul ignore` and `v8 ignore` directives,
//! building a lookup table that the coverage transform uses to skip
//! instrumentation for specific nodes.

use std::collections::BTreeMap;

use oxc_ast::ast::{Comment, Program};

use oxc_coverage_types::UnhandledPragma;

/// Type of coverage ignore directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreType {
    /// `/* istanbul ignore next */` or `/* v8 ignore next */`
    /// Skip the next node (statement, function, class, etc.)
    Next,
    /// `/* istanbul ignore if */`
    /// Skip the if branch of an if statement.
    If,
    /// `/* istanbul ignore else */`
    /// Skip the else branch of an if statement.
    Else,
}

/// Lookup table of coverage ignore directives, keyed by the start offset
/// of the token the comment is attached to.
pub struct PragmaMap {
    /// Maps token start offset → ignore type.
    ignores: BTreeMap<u32, IgnoreType>,
    /// Ranges ignored by `ignore start` / `ignore stop` block pragmas.
    ignored_ranges: Vec<(u32, u32)>,
    /// Whether the entire file should be ignored.
    pub ignore_file: bool,
}

impl PragmaMap {
    /// Build a pragma map from the program's comments and source text.
    pub fn from_program(program: &Program, source: &str) -> (Self, Vec<UnhandledPragma>) {
        let mut state = PragmaCollect::default();
        let mut comments: Vec<_> = program.comments.iter().collect();
        comments.sort_by_key(|comment| comment.span.start);

        for comment in comments {
            let text = Self::comment_text(comment, source);
            if let Some(result) = Self::parse_pragma(&text) {
                state.apply(result, comment, source);
            }
        }

        if let Some(start) = state.active_ignore_start {
            state.ignored_ranges.push((start, source.len() as u32));
        }

        (
            Self {
                ignores: state.ignores,
                ignored_ranges: state.ignored_ranges,
                ignore_file: state.ignore_file,
            },
            state.unhandled,
        )
    }

    /// Get the ignore type for a given token start offset.
    pub fn get(&self, token_start: u32) -> Option<IgnoreType> {
        self.ignores.get(&token_start).copied().or_else(|| {
            self.ignored_ranges
                .iter()
                .any(|&(start, end)| token_start >= start && token_start < end)
                .then_some(IgnoreType::Next)
        })
    }

    /// Extract comment text from source.
    fn comment_text(comment: &Comment, source: &str) -> String {
        let content_span = comment.content_span();
        source[content_span.start as usize..content_span.end as usize].to_string()
    }

    fn line_column(source: &str, offset: u32) -> (u32, u32) {
        let prefix = &source[..offset as usize];
        let line = prefix.chars().filter(|&c| c == '\n').count() as u32 + 1;
        let line_start = prefix.rfind('\n').map_or(0, |p| p + 1);
        // Istanbul reports columns as UTF-16 code units, matching Babel.
        let column =
            source[line_start..offset as usize].chars().map(char::len_utf16).sum::<usize>() as u32;
        (line, column)
    }

    fn next_token_start(source: &str, offset: u32) -> Option<u32> {
        let mut cursor = offset as usize;
        while cursor < source.len() {
            let ch = source[cursor..].chars().next()?;
            if ch.is_whitespace() {
                cursor += ch.len_utf8();
                continue;
            }

            let rest = &source[cursor..];
            if rest.starts_with("//") {
                if let Some(newline) = rest.find('\n') {
                    cursor += newline + 1;
                } else {
                    return None;
                }
                continue;
            }
            if rest.starts_with("/*") {
                if let Some(end) = rest.find("*/") {
                    cursor += end + 2;
                    continue;
                }
                return None;
            }

            return Some(cursor as u32);
        }
        None
    }

    fn pragma_target_start(source: &str, comment: &Comment) -> Option<u32> {
        let token_start = Self::next_token_start(source, comment.span.end)?;
        if source[token_start as usize..].starts_with('}')
            && Self::previous_non_whitespace(source, comment.span.start) == Some('{')
            && let Some(after_close) = Self::next_token_start(source, token_start + 1)
            && source[after_close as usize..].starts_with(['{', '<'])
        {
            return Some(after_close);
        }
        // `/* istanbul ignore <kind> */` placed between `else` and a chained
        // `else if` anchors to the inner `if`, not to the `else` keyword
        // itself. `IfStatement::span.start` is the `if` keyword; without
        // this hop the pragma never reaches the visitor. `word_at` matches
        // ASCII letters only, so `Some("else")` guarantees that the four
        // bytes starting at `token_start` are `b'e'`, `b'l'`, `b's'`, `b'e'`
        // and `token_start + 4` is a valid UTF-8 boundary.
        if Self::word_at(source, token_start) == Some("else")
            && let Some(after_else) = Self::next_token_start(source, token_start + 4)
            && Self::word_at(source, after_else) == Some("if")
        {
            return Some(after_else);
        }
        Some(token_start)
    }

    fn word_at(source: &str, offset: u32) -> Option<&str> {
        let rest = source.get(offset as usize..)?;
        let end = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
        if end == 0 { None } else { Some(&rest[..end]) }
    }

    fn previous_non_whitespace(source: &str, offset: u32) -> Option<char> {
        source[..offset as usize].chars().rev().find(|ch| !ch.is_whitespace())
    }

    /// Parse a pragma comment text into an ignore type.
    ///
    /// Matches `<tool> ignore <kind>` where `<tool>` is one of `istanbul`, `v8`, `c8`,
    /// and `<kind>` is one of `next`, `if`, `else`, `file`, `start`, or `stop`. Any ASCII whitespace run
    /// (spaces, tabs, newlines) between tokens is accepted, matching Istanbul's behavior.
    ///
    /// A single leading `!` (the legal-comment marker preserved by esbuild,
    /// terser, swc, and most other minifiers) is skipped before parsing so
    /// `/*! istanbul ignore next */` and `//! istanbul ignore next` are
    /// honored identically to their plain forms.
    fn parse_pragma(text: &str) -> Option<PragmaResult> {
        let trimmed = text.trim();
        let after_legal_marker = trimmed.strip_prefix('!').unwrap_or(trimmed);
        let mut tokens = after_legal_marker.split_whitespace();
        let tool = tokens.next()?;
        if !matches!(tool, "istanbul" | "v8" | "c8") {
            return None;
        }
        if tokens.next()? != "ignore" {
            return None;
        }
        let kind = tokens.next().unwrap_or("");
        Some(match kind {
            "next" => PragmaResult::Ignore(IgnoreType::Next),
            "if" => PragmaResult::Ignore(IgnoreType::If),
            "else" => PragmaResult::Ignore(IgnoreType::Else),
            "file" => PragmaResult::File,
            "start" => PragmaResult::Start,
            "stop" => PragmaResult::Stop,
            _ => PragmaResult::Unknown(trimmed.to_string()),
        })
    }
}

enum PragmaResult {
    Ignore(IgnoreType),
    File,
    Start,
    Stop,
    Unknown(String),
}

#[derive(Default)]
struct PragmaCollect {
    ignores: BTreeMap<u32, IgnoreType>,
    ignored_ranges: Vec<(u32, u32)>,
    active_ignore_start: Option<u32>,
    ignore_file: bool,
    unhandled: Vec<UnhandledPragma>,
}

impl PragmaCollect {
    fn apply(&mut self, result: PragmaResult, comment: &Comment, source: &str) {
        match result {
            PragmaResult::Ignore(it) => {
                let token_start =
                    PragmaMap::pragma_target_start(source, comment).unwrap_or(comment.attached_to);
                self.ignores.insert(token_start, it);
            }
            PragmaResult::File => self.ignore_file = true,
            PragmaResult::Start => {
                self.active_ignore_start.get_or_insert(comment.span.end);
            }
            PragmaResult::Stop => {
                if let Some(start) = self.active_ignore_start.take()
                    && start <= comment.span.start
                {
                    self.ignored_ranges.push((start, comment.span.start));
                }
            }
            PragmaResult::Unknown(comment_text) => {
                let (line, column) = PragmaMap::line_column(source, comment.span.start);
                self.unhandled.push(UnhandledPragma { comment: comment_text, line, column });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IgnoreType, PragmaMap, PragmaResult};
    use oxc_ast::ast::Comment;
    use oxc_span::Span;

    fn classify(text: &str) -> Option<PragmaResult> {
        PragmaMap::parse_pragma(text)
    }

    #[test]
    fn parses_plain_block_pragma() {
        assert!(matches!(
            classify(" istanbul ignore next "),
            Some(PragmaResult::Ignore(IgnoreType::Next))
        ));
    }

    #[test]
    fn parses_legal_block_pragma() {
        assert!(matches!(
            classify("! istanbul ignore next "),
            Some(PragmaResult::Ignore(IgnoreType::Next))
        ));
        assert!(matches!(
            classify("!istanbul ignore next"),
            Some(PragmaResult::Ignore(IgnoreType::Next))
        ));
    }

    #[test]
    fn parses_legal_line_pragma() {
        assert!(matches!(
            classify("! v8 ignore next"),
            Some(PragmaResult::Ignore(IgnoreType::Next))
        ));
        assert!(matches!(
            classify("! istanbul ignore else"),
            Some(PragmaResult::Ignore(IgnoreType::Else))
        ));
    }

    #[test]
    fn rejects_bang_only() {
        assert!(classify("!").is_none());
        assert!(classify("!!!").is_none());
    }

    #[test]
    fn rejects_non_pragma_legal_comment() {
        assert!(classify("! Copyright (c) 2026").is_none());
    }

    #[test]
    fn word_at_extracts_keyword() {
        assert_eq!(PragmaMap::word_at("else if (x)", 0), Some("else"));
        assert_eq!(PragmaMap::word_at("if (x)", 0), Some("if"));
        assert_eq!(PragmaMap::word_at("123abc", 0), None);
        assert_eq!(PragmaMap::word_at("", 0), None);
    }

    #[test]
    fn pragma_target_start_hops_past_else_to_chained_if() {
        let source = "if (a) {} /* istanbul ignore else */ else if (b) {}";
        let comment_start = source.find("/*").unwrap() as u32;
        let comment_end = (source.find("*/").unwrap() + 2) as u32;
        let comment = Comment { span: Span::new(comment_start, comment_end), ..Default::default() };
        let target = PragmaMap::pragma_target_start(source, &comment).unwrap();
        let if_offset = source[comment_end as usize..].find("if ").unwrap() as u32 + comment_end;
        assert_eq!(target, if_offset, "pragma must anchor on the chained `if`");
    }

    #[test]
    fn pragma_target_start_hops_past_else_through_inline_comment() {
        // The `next_token_start` helper already skips inline comments and
        // whitespace, so `else /*comment*/ if` should hop to `if` exactly
        // like `else if`.
        let source = "if (a) {} /* istanbul ignore else */ else /*c*/ if (b) {}";
        let comment_start = source.find("/* istanbul").unwrap() as u32;
        let comment_end = (source.find("*/").unwrap() + 2) as u32;
        let comment = Comment { span: Span::new(comment_start, comment_end), ..Default::default() };
        let target = PragmaMap::pragma_target_start(source, &comment).unwrap();
        let if_offset = source.find("if (b)").unwrap() as u32;
        assert_eq!(target, if_offset, "pragma must hop past `else` and the inline comment to `if`");
    }

    #[test]
    fn pragma_target_start_keeps_else_when_followed_by_block() {
        let source = "if (a) {} /* istanbul ignore else */ else { x }";
        let comment_start = source.find("/*").unwrap() as u32;
        let comment_end = (source.find("*/").unwrap() + 2) as u32;
        let comment = Comment { span: Span::new(comment_start, comment_end), ..Default::default() };
        let target = PragmaMap::pragma_target_start(source, &comment).unwrap();
        let else_offset = source[comment_end as usize..].find("else").unwrap() as u32 + comment_end;
        assert_eq!(target, else_offset, "pragma stays on `else` when no chained `if` follows");
    }
}
