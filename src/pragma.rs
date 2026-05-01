//! Istanbul/v8 coverage pragma comment handling.
//!
//! Scans AST comments for `istanbul ignore` and `v8 ignore` directives,
//! building a lookup table that the coverage transform uses to skip
//! instrumentation for specific nodes.

use std::collections::BTreeMap;

use oxc_ast::ast::{Comment, Program};

use crate::types::UnhandledPragma;

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
        let mut ignores = BTreeMap::new();
        let mut ignored_ranges = Vec::new();
        let mut active_ignore_start = None;
        let mut ignore_file = false;
        let mut unhandled = Vec::new();

        let mut comments: Vec<_> = program.comments.iter().collect();
        comments.sort_by_key(|comment| comment.span.start);

        for comment in comments {
            // Only process block comments that Oxc tagged as coverage-related,
            // or manually scan for istanbul/v8 patterns in all comments
            let text = Self::comment_text(comment, source);

            if let Some(ignore_type) = Self::parse_pragma(&text) {
                match ignore_type {
                    PragmaResult::Ignore(it) => {
                        let token_start = Self::next_token_start(source, comment.span.end)
                            .unwrap_or(comment.attached_to);
                        ignores.insert(token_start, it);
                    }
                    PragmaResult::File => {
                        ignore_file = true;
                    }
                    PragmaResult::Start => {
                        active_ignore_start.get_or_insert(comment.span.end);
                    }
                    PragmaResult::Stop => {
                        if let Some(start) = active_ignore_start.take()
                            && start <= comment.span.start
                        {
                            ignored_ranges.push((start, comment.span.start));
                        }
                    }
                    PragmaResult::Unknown(comment_text) => {
                        let (line, column) = Self::line_column(source, comment.span.start);
                        unhandled.push(UnhandledPragma { comment: comment_text, line, column });
                    }
                }
            }
        }

        if let Some(start) = active_ignore_start {
            ignored_ranges.push((start, source.len() as u32));
        }

        (Self { ignores, ignored_ranges, ignore_file }, unhandled)
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

    /// Parse a pragma comment text into an ignore type.
    ///
    /// Matches `<tool> ignore <kind>` where `<tool>` is one of `istanbul`, `v8`, `c8`,
    /// and `<kind>` is one of `next`, `if`, `else`, `file`, `start`, or `stop`. Any ASCII whitespace run
    /// (spaces, tabs, newlines) between tokens is accepted, matching Istanbul's behavior.
    fn parse_pragma(text: &str) -> Option<PragmaResult> {
        let trimmed = text.trim();
        let mut tokens = trimmed.split_whitespace();
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
