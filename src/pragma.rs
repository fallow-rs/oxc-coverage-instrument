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
                        let line = source[..comment.span.start as usize]
                            .chars()
                            .filter(|&c| c == '\n')
                            .count() as u32
                            + 1;
                        let line_start =
                            source[..comment.span.start as usize].rfind('\n').map_or(0, |p| p + 1);
                        let column = comment.span.start as usize - line_start;
                        unhandled.push(UnhandledPragma {
                            comment: comment_text,
                            line,
                            column: column as u32,
                        });
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
    fn parse_pragma(text: &str) -> Option<PragmaResult> {
        let trimmed = text.trim();

        // Match: istanbul ignore (next|if|else|file)
        if let Some(rest) = trimmed.strip_prefix("istanbul ignore ") {
            return Some(Self::parse_ignore_kind(rest.trim_start(), trimmed));
        }
        if let Some(rest) = trimmed.strip_prefix("istanbul ignore\t") {
            return Some(Self::parse_ignore_kind(rest.trim_start(), trimmed));
        }

        // Match: v8 ignore (next|if|else|file)
        if let Some(rest) = trimmed.strip_prefix("v8 ignore ") {
            return Some(Self::parse_ignore_kind(rest.trim_start(), trimmed));
        }
        if let Some(rest) = trimmed.strip_prefix("v8 ignore\t") {
            return Some(Self::parse_ignore_kind(rest.trim_start(), trimmed));
        }

        // Match: c8 ignore (next|if|else|file)
        if let Some(rest) = trimmed.strip_prefix("c8 ignore ") {
            return Some(Self::parse_ignore_kind(rest.trim_start(), trimmed));
        }
        if let Some(rest) = trimmed.strip_prefix("c8 ignore\t") {
            return Some(Self::parse_ignore_kind(rest.trim_start(), trimmed));
        }

        None
    }

    fn parse_ignore_kind(kind_str: &str, full_text: &str) -> PragmaResult {
        let keyword = kind_str.split_whitespace().next().unwrap_or("");

        match keyword {
            "next" => PragmaResult::Ignore(IgnoreType::Next),
            "if" => PragmaResult::Ignore(IgnoreType::If),
            "else" => PragmaResult::Ignore(IgnoreType::Else),
            "file" => PragmaResult::File,
            _ => PragmaResult::Unknown(full_text.to_string()),
        }
    }
}

enum PragmaResult {
    Ignore(IgnoreType),
    File,
    Unknown(String),
}
