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
    /// Whether the entire file should be ignored.
    pub ignore_file: bool,
}

impl PragmaMap {
    /// Build a pragma map from the program's comments and source text.
    pub fn from_program(program: &Program, source: &str) -> (Self, Vec<UnhandledPragma>) {
        let mut ignores = BTreeMap::new();
        let mut ignore_file = false;
        let mut unhandled = Vec::new();

        for comment in &program.comments {
            // Only process block comments that Oxc tagged as coverage-related,
            // or manually scan for istanbul/v8 patterns in all comments
            let text = Self::comment_text(comment, source);

            if let Some(ignore_type) = Self::parse_pragma(&text) {
                match ignore_type {
                    PragmaResult::Ignore(it) => {
                        ignores.insert(comment.attached_to, it);
                    }
                    PragmaResult::File => {
                        ignore_file = true;
                    }
                    PragmaResult::Unknown(comment_text) => {
                        let prefix = &source[..comment.span.start as usize];
                        let line = prefix.chars().filter(|&c| c == '\n').count() as u32 + 1;
                        let line_start = prefix.rfind('\n').map_or(0, |p| p + 1);
                        // Istanbul reports columns as UTF-16 code units, matching Babel.
                        let column = source[line_start..comment.span.start as usize]
                            .chars()
                            .map(char::len_utf16)
                            .sum::<usize>() as u32;
                        unhandled.push(UnhandledPragma { comment: comment_text, line, column });
                    }
                }
            }
        }

        (Self { ignores, ignore_file }, unhandled)
    }

    /// Get the ignore type for a given token start offset.
    pub fn get(&self, token_start: u32) -> Option<IgnoreType> {
        self.ignores.get(&token_start).copied()
    }

    /// Extract comment text from source.
    fn comment_text(comment: &Comment, source: &str) -> String {
        let content_span = comment.content_span();
        source[content_span.start as usize..content_span.end as usize].to_string()
    }

    /// Parse a pragma comment text into an ignore type.
    ///
    /// Matches `<tool> ignore <kind>` where `<tool>` is one of `istanbul`, `v8`, `c8`,
    /// and `<kind>` is one of `next`, `if`, `else`, `file`. Any ASCII whitespace run
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
            _ => PragmaResult::Unknown(trimmed.to_string()),
        })
    }
}

enum PragmaResult {
    Ignore(IgnoreType),
    File,
    Unknown(String),
}
