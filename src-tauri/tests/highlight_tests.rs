// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Integration tests for Okabe-Ito token classification
//! (`parallel-task06-A`).

use std::collections::HashSet;

use tessera_lib::core::latex::highlight::{highlight, HighlightKind, HighlightToken};

fn labels() -> HashSet<String> {
    ["sec:intro".to_string()].into_iter().collect()
}

fn text(source: &str, token: HighlightToken) -> String {
    let utf16: Vec<u16> = source.encode_utf16().collect();
    String::from_utf16(&utf16[token.start as usize..token.end as usize])
        .expect("valid UTF-16 range")
}

#[test]
fn fixture_tokens_cover_all_parser_categories() {
    let source = "% a comment\n\\begin{itemize}\\item $x^2$ \\ref{sec:intro}\\end{itemize}";
    let tokens = highlight(source, &labels()).expect("highlight");

    let classified: Vec<(HighlightKind, String)> = tokens
        .into_iter()
        .map(|token| (token.kind, text(source, token)))
        .collect();

    assert!(classified
        .iter()
        .any(|(kind, value)| *kind == HighlightKind::Comment && value.starts_with('%')));
    assert!(classified
        .iter()
        .any(|(kind, value)| *kind == HighlightKind::Environment && value == "itemize"));
    assert!(classified
        .iter()
        .any(|(kind, value)| *kind == HighlightKind::Math && value == "$x^2$"));
    assert!(classified
        .iter()
        .any(|(kind, value)| *kind == HighlightKind::Command && value == "\\ref"));
    assert!(classified
        .iter()
        .any(|(kind, value)| *kind == HighlightKind::Bracket && value == "{"));
    assert!(!classified
        .iter()
        .any(|(kind, _)| *kind == HighlightKind::Error));
}

#[test]
fn unresolved_reference_is_an_error_token() {
    let source = "see \\ref{sec:missing}";
    let tokens = highlight(source, &labels()).expect("highlight");
    assert!(tokens.iter().any(|token| {
        token.kind == HighlightKind::Error && text(source, *token) == "\\ref{sec:missing}"
    }));
}

#[test]
fn unicode_offsets_are_utf16_not_bytes() {
    let source = "é\\textit{café}";
    let tokens = highlight(source, &labels()).expect("highlight");
    let command = tokens
        .iter()
        .find(|token| token.kind == HighlightKind::Command)
        .expect("command token");
    assert_eq!(command.start, 1);
    assert_eq!(text(source, *command), "\\textit");
}

#[test]
fn malformed_source_returns_error_tokens_and_continues() {
    let tokens = highlight("$unterminated\\textbf{x}", &labels()).expect("highlight");
    assert!(tokens
        .iter()
        .any(|token| token.kind == HighlightKind::Error));
    assert!(tokens
        .iter()
        .any(|token| token.kind == HighlightKind::Command));
}
