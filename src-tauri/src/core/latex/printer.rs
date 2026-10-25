// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Printer for the LaTeX projection of the Document Model.
//!
//! The printer uses canonical spellings for constructs which have multiple
//! supported LaTeX spellings.  `OpaqueOrigin::Latex` is the exception:
//! its raw source is appended without inspecting or modifying it.

use std::fmt::Write;

use crate::core::model::{
    Author, Block, DocMetadata, Document, Equation, Figure, HeadingLevel, Inline, ListKind,
    OpaqueOrigin, RuleStyle, Table, TableCell,
};

/// Prints a document as a complete LaTeX source file.
///
/// First-class nodes are emitted using the canonical spellings documented in
/// `docs/specs.md`.  Latex-origin opaque nodes are copied byte-for-byte.
pub fn print(document: &Document) -> String {
    let mut out = String::new();
    print_preamble(document.metadata(), &mut out);
    out.push_str("\\begin{document}\n");
    if document.metadata().title().is_some() {
        out.push_str("\\maketitle\n");
    }
    for (index, block) in document.body().iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        print_block(block, &mut out, 0);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(document.trailing_trivia());
    out.push_str("\\end{document}\n");
    out
}

/// Alias for [`print`], useful at export call sites.
pub fn print_document(document: &Document) -> String {
    print(document)
}

fn print_preamble(metadata: &DocMetadata, out: &mut String) {
    if let Some(class) = metadata.latex_class() {
        if metadata.latex_class_options().is_empty() {
            let _ = writeln!(out, "\\documentclass{{{class}}}");
        } else {
            let _ = writeln!(
                out,
                "\\documentclass[{}]{{{class}}}",
                metadata.latex_class_options().join(",")
            );
        }
    } else {
        out.push_str("\\documentclass{article}\n");
    }
    if let Some(title) = metadata.title() {
        write_command_arg(out, "title", title);
    }
    for author in metadata.authors() {
        print_author(author, out);
    }
    if let Some(date) = metadata.date() {
        write_command_arg(out, "date", date);
    }
    for raw in metadata.preamble_raw() {
        out.push_str(raw);
        if !raw.ends_with('\n') {
            out.push('\n');
        }
    }
}

fn print_author(author: &Author, out: &mut String) {
    out.push_str("\\author{");
    out.push_str(&escape_text(author.name()));
    if let Some(affiliation) = author.affiliation() {
        out.push_str("\\\\");
        out.push_str(&escape_text(affiliation));
    }
    out.push_str("}\n");
}

fn write_command_arg(out: &mut String, command: &str, value: &str) {
    let _ = writeln!(out, "\\{command}{{{}}}", escape_text(value));
}

fn print_block(block: &Block, out: &mut String, indent: usize) {
    if let Some(trivia) = block.trivia() {
        out.push_str(&trivia.leading);
    }
    match block {
        Block::Heading {
            level,
            inline,
            label,
            ..
        } => {
            indent_line(out, indent);
            let _ = write!(out, "\\{}{{", heading_command(*level));
            print_inlines(inline, out);
            out.push('}');
            print_label(label.as_deref(), out);
        }
        Block::Paragraph { inline, .. } => {
            indent_line(out, indent);
            print_inlines(inline, out);
        }
        Block::List { kind, items, .. } => print_list(*kind, items, out, indent),
        Block::Table(table) => print_table(table, out, indent),
        Block::Figure(figure) => print_figure(figure, out, indent),
        Block::Equation(equation) => print_equation(equation, out, indent),
        Block::CodeBlock { language, text, .. } => {
            indent_line(out, indent);
            if let Some(language) = language {
                let _ = writeln!(
                    out,
                    "\\begin{{lstlisting}}[language={}]",
                    escape_option(language)
                );
            } else {
                out.push_str("\\begin{verbatim}\n");
            }
            out.push_str(text);
            if !text.ends_with('\n') {
                out.push('\n');
            }
            indent_line(out, indent);
            out.push_str(if language.is_some() {
                "\\end{lstlisting}"
            } else {
                "\\end{verbatim}"
            });
        }
        Block::Quote { blocks, .. } => {
            indent_line(out, indent);
            out.push_str("\\begin{quote}\n");
            print_blocks(blocks, out, indent + 1);
            indent_line(out, indent);
            out.push_str("\\end{quote}");
        }
        Block::Opaque(opaque) => match opaque.origin() {
            OpaqueOrigin::Latex => out.push_str(opaque.raw()),
            OpaqueOrigin::Docx => {
                indent_line(out, indent);
                let _ = write!(out, "% Opaque DOCX block: {}\n% ", opaque.summary());
                out.push_str(&opaque.raw().replace('\n', "\n% "));
            }
        },
    }
}

fn print_blocks(blocks: &[Block], out: &mut String, indent: usize) {
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        print_block(block, out, indent);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
}

fn print_list(kind: ListKind, items: &[Vec<Block>], out: &mut String, indent: usize) {
    indent_line(out, indent);
    let environment = match kind {
        ListKind::Ordered => "enumerate",
        ListKind::Unordered => "itemize",
    };
    let _ = writeln!(out, "\\begin{{{environment}}}");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        indent_line(out, indent + 1);
        out.push_str("\\item ");
        if let Some((first, rest)) = item.split_first() {
            print_block(first, out, 0);
            for block in rest {
                out.push('\n');
                print_block(block, out, indent + 2);
            }
        }
        out.push('\n');
    }
    indent_line(out, indent);
    let _ = write!(out, "\\end{{{environment}}}");
}

fn print_table(table: &Table, out: &mut String, indent: usize) {
    out.push_str(table.leading_trivia());
    indent_line(out, indent);
    out.push_str("\\begin{table}\n");
    if let Some(caption) = table.caption() {
        indent_line(out, indent + 1);
        out.push_str("\\caption{");
        print_inlines(caption, out);
        out.push_str("}\n");
    }
    if let Some(label) = table.label() {
        indent_line(out, indent + 1);
        let _ = writeln!(out, "\\label{{{label}}}");
    }
    let columns = table.rows().iter().map(Vec::len).max().unwrap_or(1);
    let inferred = "l".repeat(columns);
    let column_specs = table
        .latex_style()
        .map(|style| style.column_specs().join(""))
        .filter(|s| !s.is_empty())
        .unwrap_or(inferred);
    indent_line(out, indent + 1);
    let _ = writeln!(out, "\\begin{{tabular}}{{{column_specs}}}");
    if let Some(style) = table.latex_style() {
        if style.rule_style() == RuleStyle::Booktabs {
            out.push_str("\\toprule\n");
        }
        for row in style.hline_before_rows() {
            if *row == 0 {
                out.push_str("\\hline\n");
            }
        }
    }
    for (row_index, row) in table.rows().iter().enumerate() {
        indent_line(out, indent + 2);
        for (cell_index, cell) in row.iter().enumerate() {
            if cell_index > 0 {
                out.push_str(" & ");
            }
            print_cell(cell, out);
        }
        out.push_str(" \\\\");
        if let Some(style) = table.latex_style() {
            if style.hline_before_rows().contains(&(row_index + 1)) {
                out.push_str("\\hline");
            } else if style.rule_style() == RuleStyle::Booktabs
                && row_index + 1 < table.rows().len()
            {
                out.push_str("\\midrule");
            }
        }
        if row_index + 1 < table.rows().len() {
            out.push('\n');
        }
    }
    out.push('\n');
    if table
        .latex_style()
        .is_some_and(|style| style.rule_style() == RuleStyle::Booktabs)
    {
        out.push_str("\\bottomrule\n");
    }
    indent_line(out, indent + 1);
    out.push_str("\\end{tabular}\n");
    indent_line(out, indent);
    out.push_str("\\end{table}");
}

fn print_cell(cell: &TableCell, out: &mut String) {
    if cell.col_span() > 1 {
        let _ = write!(out, "\\multicolumn{{{}}}{{l}}{{", cell.col_span());
        print_cell_contents(cell, out);
        out.push('}');
    } else if cell.row_span() > 1 {
        let _ = write!(out, "\\multirow{{{}}}{{*}}{{", cell.row_span());
        print_cell_contents(cell, out);
        out.push('}');
    } else {
        print_cell_contents(cell, out);
    }
}

fn print_cell_contents(cell: &TableCell, out: &mut String) {
    for (index, block) in cell.blocks().iter().enumerate() {
        if index > 0 {
            out.push_str("\\par ");
        }
        print_block(block, out, 0);
    }
}

fn print_figure(figure: &Figure, out: &mut String, indent: usize) {
    out.push_str(figure.leading_trivia());
    indent_line(out, indent);
    out.push_str("\\begin{figure}\n");
    indent_line(out, indent + 1);
    out.push_str("\\includegraphics");
    if let Some(width) = figure.width_percent() {
        let _ = write!(out, "[width={width:.2}\\%\\textwidth]");
    }
    let _ = writeln!(out, "{{{}}}", escape_path(figure.asset_path()));
    if let Some(caption) = figure.caption() {
        indent_line(out, indent + 1);
        out.push_str("\\caption{");
        print_inlines(caption, out);
        out.push_str("}\n");
    }
    if let Some(label) = figure.label() {
        indent_line(out, indent + 1);
        let _ = writeln!(out, "\\label{{{label}}}");
    }
    indent_line(out, indent);
    out.push_str("\\end{figure}");
}

fn print_equation(equation: &Equation, out: &mut String, indent: usize) {
    out.push_str(equation.leading_trivia());
    indent_line(out, indent);
    let environment = if equation.display_mode() {
        "equation"
    } else {
        "equation*"
    };
    let _ = writeln!(out, "\\begin{{{environment}}}");
    indent_line(out, indent + 1);
    out.push_str(equation.latex_source());
    if let Some(label) = equation.label() {
        let _ = write!(out, "\n\\label{{{label}}}");
    }
    out.push('\n');
    indent_line(out, indent);
    let _ = write!(out, "\\end{{{environment}}}");
}

fn print_inlines(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(&escape_text(text)),
            Inline::Bold { inline } => wrapped_inlines(out, "textbf", inline),
            Inline::Italic { inline } => wrapped_inlines(out, "textit", inline),
            Inline::Underline { inline } => wrapped_inlines(out, "underline", inline),
            Inline::Code { text } => print_verbatim_inline(text, out),
            Inline::Citation { keys } => {
                let _ = write!(out, "\\cite{{{}}}", keys.join(","));
            }
            Inline::CrossRef { label } => {
                let _ = write!(out, "\\ref{{{label}}}");
            }
            Inline::Footnote { inline } => wrapped_inlines(out, "footnote", inline),
            Inline::InlineMath { latex_source } => {
                let _ = write!(out, "\\({latex_source}\\)");
            }
            Inline::LineBreak => out.push_str("\\\\"),
        }
    }
}

fn wrapped_inlines(out: &mut String, command: &str, inlines: &[Inline]) {
    let _ = write!(out, "\\{command}{{");
    print_inlines(inlines, out);
    out.push('}');
}

fn print_verbatim_inline(text: &str, out: &mut String) {
    for delimiter in ['|', '!', '+', ';', ':', '/'] {
        if !text.contains(delimiter) && !text.contains('\n') {
            let _ = write!(out, "\\verb{delimiter}{text}{delimiter}");
            return;
        }
    }
    wrapped_inlines(
        out,
        "texttt",
        &[Inline::Text {
            text: text.to_string(),
        }],
    );
}

fn heading_command(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => "section",
        HeadingLevel::H2 => "subsection",
        HeadingLevel::H3 => "subsubsection",
        HeadingLevel::H4 => "paragraph",
        HeadingLevel::H5 => "subparagraph",
        HeadingLevel::H6 => "subparagraph",
    }
}

fn print_label(label: Option<&str>, out: &mut String) {
    if let Some(label) = label {
        let _ = write!(out, "\\label{{{label}}}");
    }
}

fn indent_line(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn escape_path(value: &str) -> String {
    value.replace('\\', "/").replace(' ', "\\ ")
}

fn escape_option(value: &str) -> String {
    value.replace(',', "{,}")
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '#' => escaped.push_str("\\#"),
            '$' => escaped.push_str("\\$"),
            '%' => escaped.push_str("\\%"),
            '&' => escaped.push_str("\\&"),
            '_' => escaped.push_str("\\_"),
            '^' => escaped.push_str("\\^{}"),
            '~' => escaped.push_str("\\~{}"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{DocMetadata, OpaqueBlock};

    #[test]
    fn latex_opaque_is_copied_byte_for_byte() {
        let raw = "\\customcommand{a}% comment\n\\begin{unknown}\n".to_string();
        let opaque = OpaqueBlock::new(OpaqueOrigin::Latex, raw.clone(), "raw macro".to_string())
            .expect("valid opaque");
        let document = Document::new(DocMetadata::default(), vec![Block::opaque(opaque)]);
        assert!(print(&document).contains(&raw));
    }

    #[test]
    fn prints_nested_formatting_and_escaping() {
        let document = Document::new(
            DocMetadata::default(),
            vec![Block::paragraph(vec![
                Inline::bold(vec![Inline::text("a_b".to_string()).expect("valid text")]),
                Inline::inline_math("x^2".to_string()).expect("valid math"),
            ])],
        );
        assert!(print(&document).contains("\\textbf{a\\_b}\\(x^2\\)"));
    }

    #[test]
    fn table_uses_preserved_column_spec_and_rule_style() {
        use crate::core::model::{RuleStyle, TableStyle};
        let cell = TableCell::new(vec![], 1, 1).expect("cell");
        let mut table = Table::new(vec![vec![cell]], None, None).expect("table");
        table.set_latex_style(Some(TableStyle::new(
            vec!["|p{4cm}|r|".into()],
            RuleStyle::Booktabs,
            vec![0],
        )));
        let document = Document::new(DocMetadata::default(), vec![Block::table(table)]);
        let output = print(&document);
        assert!(output.contains("\\begin{tabular}{|p{4cm}|r|}"));
        assert!(output.contains("\\toprule"));
    }
}
