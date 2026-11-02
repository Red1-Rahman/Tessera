// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Document Model to Markdown (`parallel-task06-C`).
//!
//! The output targets Pandoc-compatible Markdown: math keeps its raw
//! LaTeX source between `$...$` or `$$...$$`, citations use `[@key]`,
//! and cross-references use `[@label]` so pandoc-crossref can resolve
//! them. `Opaque` blocks become visible HTML comments, never silently
//! omitted content.
//!
//! Table spans are flattened visibly: a cell whose `col_span` is greater
//! than one repeats its content across the spanned columns, and a cell
//! whose `row_span` is greater than one shows its content in the first
//! row with empty continuation cells below. Plain Markdown tables have
//! no span syntax, and `docs/specs.md`, section 4 records this
//! convention.

use std::fmt::Write;

use crate::core::error::TesseraError;
use crate::core::model::{
    Block, Document, Inline, ListKind, OpaqueOrigin, Table, TableCell,
};

/// Renders a document as Markdown.
pub fn render_markdown(document: &Document) -> Result<String, TesseraError> {
    let mut renderer = Renderer::default();
    let mut out = String::new();

    if let Some(title) = document.metadata().title() {
        let _ = writeln!(out, "# {}\n", escape_text(title));
    }
    for (index, block) in document.body().iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        renderer.block(block, &mut out, 0);
    }

    if !renderer.footnotes.is_empty() {
        out.push('\n');
        for (index, footnote) in renderer.footnotes.iter().enumerate() {
            let _ = writeln!(out, "[^{}]: {}", index + 1, footnote.trim());
        }
    }

    Ok(out)
}

#[derive(Default)]
struct Renderer {
    footnotes: Vec<String>,
}

impl Renderer {
    fn block(&mut self, block: &Block, out: &mut String, indent: usize) {
        match block {
            Block::Heading {
                level,
                inline,
                label,
                ..
            } => {
                let hashes = "#".repeat(usize::from(level.depth()));
                let heading = self.inlines(inline);
                if let Some(label) = label {
                    let _ = writeln!(out, "{hashes} {heading} {{#{label}}}");
                } else {
                    let _ = writeln!(out, "{hashes} {heading}");
                }
            }
            Block::Paragraph { inline, .. } => {
                let text = self.inlines(inline);
                for line in text.lines() {
                    indent_line(out, indent);
                    let _ = writeln!(out, "{line}");
                }
                if text.is_empty() {
                    out.push('\n');
                }
            }
            Block::List { kind, items, .. } => self.list(*kind, items, out, indent),
            Block::Table(table) => self.table(table, out, indent),
            Block::Figure(figure) => {
                indent_line(out, indent);
                let caption = figure
                    .caption()
                    .map(|caption| self.inlines(caption))
                    .unwrap_or_default();
                let _ = write!(out, "![{caption}]({})", figure.asset_path());
                if let Some(width) = figure.width_percent() {
                    let _ = write!(out, "{{width={width}%}}");
                }
                out.push('\n');
            }
            Block::Equation(equation) => {
                indent_line(out, indent);
                let fence = if equation.display_mode() { "$$" } else { "$" };
                let _ = writeln!(out, "{fence}{}{fence}", equation.latex_source());
            }
            Block::CodeBlock {
                language, text, ..
            } => {
                indent_line(out, indent);
                let fence = code_fence(text);
                let _ = writeln!(out, "{fence}{}", language.as_deref().unwrap_or_default());
                out.push_str(text);
                if !text.ends_with('\n') {
                    out.push('\n');
                }
                indent_line(out, indent);
                out.push_str(&fence);
                out.push('\n');
            }
            Block::Quote { blocks, .. } => {
                let mut inner = String::new();
                for (index, child) in blocks.iter().enumerate() {
                    if index > 0 {
                        inner.push('\n');
                    }
                    self.block(child, &mut inner, 0);
                }
                for line in inner.trim_end().lines() {
                    indent_line(out, indent);
                    if line.is_empty() {
                        out.push_str(">\n");
                    } else {
                        let _ = writeln!(out, "> {line}");
                    }
                }
            }
            Block::Opaque(opaque) => {
                indent_line(out, indent);
                let origin = match opaque.origin() {
                    OpaqueOrigin::Latex => "LaTeX",
                    OpaqueOrigin::Docx => "DOCX",
                };
                let summary = opaque.summary().replace("--", "- -");
                let _ = writeln!(
                    out,
                    "<!-- Opaque {origin} block omitted from Markdown export: {summary} -->"
                );
            }
        }
    }

    fn list(&mut self, kind: ListKind, items: &[Vec<Block>], out: &mut String, indent: usize) {
        for (index, item) in items.iter().enumerate() {
            let marker = match kind {
                ListKind::Ordered => format!("{}.", index + 1),
                ListKind::Unordered => "-".to_string(),
            };
            let mut inner = String::new();
            for (child_index, child) in item.iter().enumerate() {
                if child_index > 0 {
                    inner.push('\n');
                }
                self.block(child, &mut inner, 0);
            }
            let inner = inner.trim_end();
            let mut lines = inner.lines();
            indent_line(out, indent);
            let _ = writeln!(
                out,
                "{marker} {}",
                lines.next().unwrap_or_default().trim_start()
            );
            let continuation_indent = indent + marker.chars().count() + 1;
            for line in lines {
                if line.is_empty() {
                    out.push('\n');
                } else {
                    indent_line(out, continuation_indent);
                    let _ = writeln!(out, "{line}");
                }
            }
        }
    }

    fn table(&mut self, table: &Table, out: &mut String, indent: usize) {
        if let Some(caption) = table.caption() {
            indent_line(out, indent);
            let caption_text = self.inlines(caption);
            if let Some(label) = table.label() {
                let _ = writeln!(out, "*{caption_text}* {{#{label}}}");
            } else {
                let _ = writeln!(out, "*{caption_text}*");
            }
        }

        let rows = table.rows();
        if rows.is_empty() {
            indent_line(out, indent);
            out.push_str("<!-- empty table -->\n");
            return;
        }

        let width = rows
            .iter()
            .map(|row| row.iter().map(|cell| cell.col_span() as usize).sum::<usize>())
            .max()
            .unwrap_or(1)
            .max(1);
        let mut grid: Vec<Vec<Option<String>>> =
            vec![vec![None; width]; rows.len()];

        for (row_index, row) in rows.iter().enumerate() {
            let mut column = 0usize;
            for cell in row {
                let content = self.cell(cell);
                let col_span = cell.col_span() as usize;
                let row_span = cell.row_span() as usize;
                for offset in 0..col_span {
                    if column + offset < width {
                        grid[row_index][column + offset] = Some(content.clone());
                    }
                }
                for row_offset in 1..row_span {
                    let target = row_index + row_offset;
                    if target < grid.len() {
                        for offset in 0..col_span {
                            if column + offset < width {
                                grid[target][column + offset] = Some(String::new());
                            }
                        }
                    }
                }
                column += col_span;
            }
        }

        for (row_index, row) in grid.iter().enumerate() {
            indent_line(out, indent);
            out.push('|');
            for cell in row {
                let value = cell.as_deref().unwrap_or_default().replace('\n', " ");
                let _ = write!(out, " {} |", value.trim());
            }
            out.push('\n');
            if row_index == 0 {
                indent_line(out, indent);
                out.push('|');
                for _ in 0..width {
                    out.push_str(" --- |");
                }
                out.push('\n');
            }
        }

    }

    fn cell(&mut self, cell: &TableCell) -> String {
        let mut text = String::new();
        for block in cell.blocks() {
            match block {
                Block::Paragraph { inline, .. } => {
                    text.push_str(&self.inlines(inline));
                }
                _ => {
                    let mut nested = String::new();
                    self.block(block, &mut nested, 0);
                    text.push_str(nested.trim());
                }
            }
        }
        text
    }

    fn inlines(&mut self, inlines: &[Inline]) -> String {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text { text } => out.push_str(&escape_text(text)),
                Inline::Bold { inline } => {
                    let _ = write!(out, "**{}**", self.inlines(inline));
                }
                Inline::Italic { inline } => {
                    let _ = write!(out, "*{}*", self.inlines(inline));
                }
                Inline::Underline { inline } => {
                    let _ = write!(out, "<u>{}</u>", self.inlines(inline));
                }
                Inline::Code { text } => out.push_str(&code_span(text)),
                Inline::Citation { keys } => {
                    let rendered: Vec<String> =
                        keys.iter().map(|key| format!("@{key}")).collect();
                    let _ = write!(out, "[{}]", rendered.join("; "));
                }
                Inline::CrossRef { label } => {
                    let _ = write!(out, "[@{label}]");
                }
                Inline::Footnote { inline } => {
                    let number = self.footnotes.len() + 1;
                    let content = self.inlines(inline);
                    self.footnotes.push(content);
                    let _ = write!(out, "[^{number}]");
                }
                Inline::InlineMath { latex_source } => {
                    let _ = write!(out, "${latex_source}$");
                }
                Inline::LineBreak => out.push_str("  \n"),
            }
        }
        out
    }
}

fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for (line_index, line) in text.split_inclusive('\n').enumerate() {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let marker_len = list_marker_len(content);
        for (index, character) in content.chars().enumerate() {
            if (index == 0 && matches!(content.as_bytes().first(), Some(b'-' | b'+'))
                && content.as_bytes().get(1) == Some(&b' '))
                || (marker_len > 0 && index == marker_len)
            {
                escaped.push('\\');
            }
            escape_character(&mut escaped, character);
        }
        if line_index + 1 < text.split_inclusive('\n').count() {
            escaped.push('\n');
        }
    }
    escaped
}

fn escape_character(escaped: &mut String, character: char) {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '#' | '|' | '$' | '@'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
}

fn list_marker_len(line: &str) -> usize {
    let bytes = line.as_bytes();
    if matches!(bytes, [b'-', b' ', ..] | [b'+', b' ', ..]) {
        return 0;
    }
    let digits = bytes.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if digits > 0 && bytes.get(digits..digits + 2) == Some(b". ") {
        digits
    } else {
        usize::MAX
    }
}

fn code_fence(text: &str) -> String {
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    "`".repeat((longest + 1).max(3))
}

fn code_span(text: &str) -> String {
    let mut fence = "`".to_string();
    while text.contains(&fence) {
        fence.push('`');
    }
    if text.starts_with('`') || text.ends_with('`') {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

fn indent_line(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}
