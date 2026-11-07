// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Document Model to plain text (`parallel-task06-D`).
//!
//! This export is intentionally lossy, as `docs/specs.md`, section 4
//! states. All inline formatting is flattened, math keeps its LaTeX
//! source, and no ad hoc markup characters are emitted, so the result
//! cannot be confused with the Markdown export.
//!
//! Heading convention: level 1 headings are underlined with `=`, level
//! 2 with `-`, and deeper headings are indented plain lines. Lists use
//! `(1) ` for ordered items and indentation for unordered items; neither
//! is a Markdown list marker.

use std::fmt::Write;

use crate::core::error::TesseraError;
use crate::core::model::{Block, Document, Inline, ListKind, OpaqueOrigin, Table};

/// Renders a document as plain text.
pub fn render_plaintext(document: &Document) -> Result<String, TesseraError> {
    let mut out = String::new();
    if let Some(title) = document.metadata().title() {
        out.push_str(title);
        out.push('\n');
        out.push_str(&"=".repeat(title.chars().count()));
        out.push_str("\n\n");
    }
    for (index, document_block) in document.body().iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        block(document_block, &mut out, 0);
    }
    Ok(out)
}

fn block(current_block: &Block, out: &mut String, indent: usize) {
    match current_block {
        Block::Heading { level, inline, .. } => {
            let text = inlines(inline);
            let depth = usize::from(level.depth());
            match depth {
                1 | 2 => {
                    indent_line(out, indent);
                    let _ = writeln!(out, "{text}");
                    indent_line(out, indent);
                    let underline = if depth == 1 { '=' } else { '-' };
                    out.push_str(&underline.to_string().repeat(text.chars().count().max(1)));
                    out.push('\n');
                }
                _ => {
                    indent_line(out, indent + depth.saturating_sub(2));
                    let _ = writeln!(out, "{text}");
                }
            }
        }
        Block::Paragraph { inline, .. } => {
            indent_line(out, indent);
            let _ = writeln!(out, "{}", inlines(inline));
        }
        Block::List { kind, items, .. } => list(*kind, items, out, indent),
        Block::Table(table_value) => table(table_value, out, indent),
        Block::Figure(figure) => {
            indent_line(out, indent);
            let caption = figure
                .caption()
                .map(inlines)
                .unwrap_or_else(|| "Untitled figure".to_string());
            let _ = writeln!(out, "Figure: {caption} ({})", figure.asset_path());
        }
        Block::Equation(equation) => {
            indent_line(out, indent);
            let _ = writeln!(out, "{}", equation.latex_source());
        }
        Block::CodeBlock { text, .. } => {
            indent_line(out, indent);
            out.push_str(text);
            if !text.ends_with('\n') {
                out.push('\n');
            }
        }
        Block::Quote { blocks, .. } => {
            for (index, child) in blocks.iter().enumerate() {
                if index > 0 {
                    out.push('\n');
                }
                block(child, out, indent + 1);
            }
        }
        Block::Opaque(opaque) => {
            indent_line(out, indent);
            let origin = match opaque.origin() {
                OpaqueOrigin::Latex => "LaTeX",
                OpaqueOrigin::Docx => "DOCX",
            };
            let _ = writeln!(out, "[{origin} content not shown: {}]", opaque.summary());
        }
    }
}

fn list(kind: ListKind, items: &[Vec<Block>], out: &mut String, indent: usize) {
    for (index, item) in items.iter().enumerate() {
        let mut inner = String::new();
        for (child_index, child) in item.iter().enumerate() {
            if child_index > 0 {
                inner.push('\n');
            }
            block(child, &mut inner, 0);
        }
        let inner = inner.trim_end();
        let mut lines = inner.lines();
        indent_line(out, indent);
        match kind {
            ListKind::Ordered => {
                let _ = write!(out, "({}) ", index + 1);
            }
            ListKind::Unordered => {
                out.push_str("  ");
            }
        }
        let _ = writeln!(out, "{}", lines.next().unwrap_or_default());
        for line in lines {
            indent_line(out, indent + 2);
            let _ = writeln!(out, "{line}");
        }
    }
}

fn table(table: &Table, out: &mut String, indent: usize) {
    if let Some(caption) = table.caption() {
        indent_line(out, indent);
        let _ = writeln!(out, "Table: {}", inlines(caption));
    }
    for row in table.rows() {
        indent_line(out, indent);
        let cells: Vec<String> = row.iter().map(|cell| cell_text(cell.blocks())).collect();
        let _ = writeln!(out, "{}", cells.join("\t"));
    }
}

fn cell_text(blocks: &[Block]) -> String {
    let mut text = String::new();
    for current_block in blocks {
        if let Block::Paragraph { inline, .. } = current_block {
            text.push_str(&inlines(inline));
        } else {
            block(current_block, &mut text, 0);
        }
    }
    text.replace(['\n', '\t'], " ").trim().to_string()
}

fn inlines(inline_nodes: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inline_nodes {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Bold { inline } => out.push_str(&inlines(inline)),
            Inline::Italic { inline } => out.push_str(&inlines(inline)),
            Inline::Underline { inline } => out.push_str(&inlines(inline)),
            Inline::Code { text } => out.push_str(text),
            Inline::Citation { keys } => {
                let _ = write!(out, "[{}]", keys.join(", "));
            }
            Inline::CrossRef { label } => {
                let _ = write!(out, "[{label}]");
            }
            Inline::Footnote { inline } => {
                let _ = write!(out, " ({})", inlines(inline));
            }
            Inline::InlineMath { latex_source } => out.push_str(latex_source),
            Inline::LineBreak => out.push('\n'),
            Inline::Opaque { raw, .. } => out.push_str(raw),
        }
    }
    out
}

fn indent_line(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}
