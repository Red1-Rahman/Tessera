// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Okabe-Ito LaTeX token classification (`parallel-task06-A`).
//!
//! Tokens come from `core/latex/parser.rs`'s own inline tokenizer, so
//! highlighting and parsing cannot disagree about token boundaries (see
//! `docs/specs.md`, section 5). This module only assigns categories; the
//! frontend maps categories to the palette in `docs/conventions.md`.
//!
//! Offsets in [`HighlightToken`] are UTF-16 code units, matching the
//! offsets a JavaScript editor such as CodeMirror uses, not Rust byte
//! offsets. That keeps non-ASCII LaTeX source aligned in the webview.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::core::error::TesseraError;
use crate::core::latex::parser::{tokenize_inline_lenient, InlineToken, InlineTokenKind};

/// The seven token categories that receive a palette color. The eighth
/// Okabe-Ito color is the `Opaque` block tint, a block-level treatment
/// rather than a token category (see `docs/conventions.md`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HighlightKind {
    /// Commands such as `\section`, `\textbf`, and escaped punctuation.
    Command,
    /// Braces and brackets, including command arguments and options.
    Bracket,
    /// Environment names inside `\begin{...}` and `\end{...}`.
    Environment,
    /// `%` comments, colored to the end of the line.
    Comment,
    /// Math delimiters and content.
    Math,
    /// Unresolved cross-references and other parse-level errors.
    Error,
    /// Everything else, rendered in the default text color.
    Text,
}

/// One classified source range. `start` and `end` are half-open UTF-16
/// code-unit offsets into the source passed to `get_highlight_tokens`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct HighlightToken {
    pub start: u32,
    pub end: u32,
    pub kind: HighlightKind,
}

/// Input to `get_highlight_tokens`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRequest {
    pub source: String,
}

/// Classifies every token in `source`.
///
/// `known_labels` are the document's current cross-reference keys; a
/// `\ref`/`\cref`/`\autoref` target missing from that set is reported as
/// [`HighlightKind::Error`] (Vermillion in the palette).
pub fn highlight(
    source: &str,
    known_labels: &HashSet<String>,
) -> Result<Vec<HighlightToken>, TesseraError> {
    let tokens = tokenize_inline_lenient(source);
    let offsets = Utf16Offsets::new(source);
    let mut output = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let token = &tokens[index];
        match &token.kind {
            InlineTokenKind::Error(_) => {
                push(&mut output, &offsets, token, HighlightKind::Error);
            }
            InlineTokenKind::Text(_) => {
                push(&mut output, &offsets, token, HighlightKind::Text);
            }
            InlineTokenKind::Comment => {
                push(&mut output, &offsets, token, HighlightKind::Comment);
            }
            InlineTokenKind::Math(_) => {
                push(&mut output, &offsets, token, HighlightKind::Math);
            }
            InlineTokenKind::LineBreak => {
                push(&mut output, &offsets, token, HighlightKind::Command);
            }
            InlineTokenKind::Code(_) => {
                push(&mut output, &offsets, token, HighlightKind::Text);
            }
            InlineTokenKind::OpenBrace
            | InlineTokenKind::CloseBrace
            | InlineTokenKind::OpenBracket
            | InlineTokenKind::CloseBracket => {
                push(&mut output, &offsets, token, HighlightKind::Bracket);
            }
            InlineTokenKind::Command { name } => {
                if matches!(name.as_str(), "begin" | "end") {
                    if let Some(group) = group_after(&tokens, index) {
                        push(&mut output, &offsets, token, HighlightKind::Command);
                        push(
                            &mut output,
                            &offsets,
                            &tokens[group.open],
                            HighlightKind::Bracket,
                        );
                        let environment_start = tokens[group.open].span.end;
                        let environment_end = tokens[group.close].span.start;
                        push_range(
                            &mut output,
                            &offsets,
                            environment_start,
                            environment_end,
                            HighlightKind::Environment,
                        );
                        push(
                            &mut output,
                            &offsets,
                            &tokens[group.close],
                            HighlightKind::Bracket,
                        );
                        index = group.close + 1;
                        continue;
                    }
                }

                if matches!(name.as_str(), "ref" | "cref" | "autoref") {
                    if let Some(group) = group_after(&tokens, index) {
                        let label = source
                            [tokens[group.open].span.end..tokens[group.close].span.start]
                            .trim();
                        if !known_labels.contains(label) {
                            if let (Some(start), Some(end)) = (
                                offsets.get(token.span.start),
                                offsets.get(tokens[group.close].span.end),
                            ) {
                                push_span(&mut output, start, end, HighlightKind::Error);
                            }
                            index = group.close + 1;
                            continue;
                        }
                    }
                }

                push(&mut output, &offsets, token, HighlightKind::Command);
            }
        }
        index += 1;
    }

    Ok(output)
}

struct Group {
    open: usize,
    close: usize,
}

/// Finds the `{...}` group following a command token, skipping only
/// whitespace text, the same way the parser associates a command with
/// its mandatory argument.
fn group_after(tokens: &[InlineToken], command_index: usize) -> Option<Group> {
    let mut open = command_index + 1;
    while open < tokens.len() {
        match &tokens[open].kind {
            InlineTokenKind::Text(text) if text.trim().is_empty() => open += 1,
            InlineTokenKind::OpenBrace => break,
            _ => return None,
        }
    }
    if open >= tokens.len() {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.kind {
            InlineTokenKind::OpenBrace => depth += 1,
            InlineTokenKind::CloseBrace => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(Group { open, close: index });
                }
            }
            _ => {}
        }
    }
    None
}

struct Utf16Offsets {
    offsets: Vec<Option<u32>>,
}

impl Utf16Offsets {
    fn new(source: &str) -> Self {
        let mut offsets = vec![None; source.len() + 1];
        let mut utf16 = 0u32;
        for (byte, character) in source.char_indices() {
            offsets[byte] = Some(utf16);
            utf16 += character.len_utf16() as u32;
        }
        offsets[source.len()] = Some(utf16);
        Self { offsets }
    }

    fn get(&self, byte: usize) -> Option<u32> {
        self.offsets.get(byte).copied().flatten()
    }
}

fn push(
    output: &mut Vec<HighlightToken>,
    offsets: &Utf16Offsets,
    token: &InlineToken,
    kind: HighlightKind,
) {
    push_range(output, offsets, token.span.start, token.span.end, kind);
}

fn push_range(
    output: &mut Vec<HighlightToken>,
    offsets: &Utf16Offsets,
    start_byte: usize,
    end_byte: usize,
    kind: HighlightKind,
) {
    if start_byte >= end_byte {
        return;
    }
    if let (Some(start), Some(end)) = (offsets.get(start_byte), offsets.get(end_byte)) {
        push_span(output, start, end, kind);
    }
}

fn push_span(output: &mut Vec<HighlightToken>, start: u32, end: u32, kind: HighlightKind) {
    if start >= end {
        return;
    }
    if let Some(previous) = output.last_mut() {
        if previous.kind == kind && previous.end == start {
            previous.end = end;
            return;
        }
    }
    output.push(HighlightToken { start, end, kind });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> HashSet<String> {
        ["sec:intro".to_string()].into_iter().collect()
    }

    fn kinds(source: &str) -> Vec<(HighlightKind, String)> {
        let tokens = highlight(source, &labels()).expect("highlight");
        tokens
            .into_iter()
            .map(|token| {
                (
                    token.kind,
                    source[token.start as usize..token.end as usize].to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn classifies_commands_brackets_and_environment_names() {
        let classified = kinds("\\begin{itemize}\\item text\\end{itemize}");
        assert!(classified.contains(&(HighlightKind::Command, "\\begin".into())));
        assert!(classified.contains(&(HighlightKind::Environment, "itemize".into())));
        assert!(classified.contains(&(HighlightKind::Bracket, "{".into())));
        assert!(classified.contains(&(HighlightKind::Text, " text".into())));
    }

    #[test]
    fn classifies_comments_and_math() {
        let classified = kinds("text % note\n$x^2$");
        assert!(classified
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::Comment && text.starts_with('%')));
        assert!(classified
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::Math && text == "$x^2$"));
    }

    #[test]
    fn resolved_and_unresolved_references_differ() {
        let resolved = kinds("see \\ref{sec:intro}");
        assert!(!resolved
            .iter()
            .any(|(kind, _)| *kind == HighlightKind::Error));

        let unresolved = kinds("see \\ref{sec:missing}");
        assert!(unresolved
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::Error && text == "\\ref{sec:missing}"));
    }

    #[test]
    fn offsets_are_utf16_code_units() {
        let source = "é\\textbf{x}";
        let tokens = highlight(source, &labels()).expect("highlight");
        let command = tokens
            .iter()
            .find(|token| token.kind == HighlightKind::Command)
            .expect("command token");
        assert_eq!(command.start, 1);
        assert_eq!(command.end, 8);
        assert_eq!("\\textbf".encode_utf16().count(), 7);
    }
}
