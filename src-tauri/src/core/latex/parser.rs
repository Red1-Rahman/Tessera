// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! LaTeX to Document Model parsing.
//!
//! The parser deliberately uses a small, lossless token scanner rather than
//! trying to be a TeX engine.  TeX is an extensible programming language,
//! while Tessera supports the finite subset in `docs/specs.md`.  Unknown
//! commands and environments are retained as `Opaque` blocks.

use crate::core::error::{Pane, SourceSpan, TesseraError};
use crate::core::model::{
    Author, Block, DocMetadata, Document, Equation, Figure, HeadingLevel, Inline, ListKind,
    OpaqueBlock, OpaqueOrigin, RuleStyle, Table, TableCell, TableStyle,
};

/// Parses a complete LaTeX source into the Document Model.
///
/// For all first-class nodes, this preserves the semantic source required by
/// the LaTeX printer. Unsupported source is never discarded: it is represented
/// by an `Opaque` block. Parse and validation failures are returned with a
/// source span when the failing construct can be located.
pub fn parse_latex(source: &str) -> Result<Document, TesseraError> {
    let (options, class_name) = document_class(source)
        .ok_or_else(|| parse_error(source, "missing or malformed \\documentclass"))?;
    if !matches!(class_name.as_str(), "article" | "report" | "book") {
        return Err(parse_error(source, "unsupported document class"));
    }
    let body_start = source
        .find("\\begin{document}")
        .ok_or_else(|| parse_error(source, "missing \\begin{document}"))?;
    let body_end = source[body_start..]
        .find("\\end{document}")
        .map(|i| body_start + i)
        .ok_or_else(|| parse_error(source, "missing \\end{document}"))?;
    let title = command_arg(source, "title").map(|x| clean_metadata(&x.0));
    let date = command_arg(source, "date").map(|x| clean_metadata(&x.0));
    let mut authors = Vec::new();
    if let Some((raw, _)) = command_arg(source, "author") {
        for name in raw.split("\\and") {
            if !name.trim().is_empty() {
                authors.push(Author::new(clean_metadata(name), None)?);
            }
        }
    }
    let mut metadata = DocMetadata::new(title, authors, date, Some(class_name.clone()), options)?;
    let preamble_raw = source[..body_start]
        .split_inclusive('\n')
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("\\usepackage")
                || trimmed.starts_with("\\newcommand")
                || trimmed.starts_with("\\renewcommand")
                || trimmed.starts_with("\\def")
        })
        .map(str::to_string)
        .collect();
    metadata.set_preamble_raw(preamble_raw);
    let body_source = &source[body_start + "\\begin{document}".len()..body_end];
    let trailing_len = body_source.len() - body_source.trim_end().len();
    let body = parse_blocks(body_source, &class_name)?;
    let mut document = Document::new(metadata, body);
    document.set_trailing_trivia(body_source[body_source.len() - trailing_len..].to_string());
    Ok(document)
}

fn parse_blocks(source: &str, class: &str) -> Result<Vec<Block>, TesseraError> {
    let mut blocks = Vec::new();
    let mut rest = source;
    while !rest.trim().is_empty() {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace());
        if rest.is_empty() {
            break;
        }
        if rest.starts_with("\\[") {
            let end = rest[2..]
                .find("\\]")
                .ok_or_else(|| parse_error(source, "unterminated display math"))?
                + 2;
            blocks.push(Block::equation(Equation::new(
                rest[2..end].to_string(),
                true,
                None,
            )?));
            rest = &rest[end + 2..];
            continue;
        }
        if let Some((env, content, consumed)) = take_environment(rest) {
            let block = match env.as_str() {
                "itemize" => parse_list(&content, ListKind::Unordered, class)?,
                "enumerate" => parse_list(&content, ListKind::Ordered, class)?,
                "verbatim" => Block::code_block(None, content)?,
                "lstlisting" => {
                    let language = option_value(rest, "language");
                    Block::code_block(language, content)?
                }
                "minted" => {
                    let language = first_group(rest).unwrap_or_default();
                    Block::code_block(Some(language), content)?
                }
                "equation" => {
                    Block::equation(Equation::new(content.clone(), true, find_label(&content))?)
                }
                "equation*" => {
                    Block::equation(Equation::new(content.clone(), false, find_label(&content))?)
                }
                "align" | "align*" | "gather" => {
                    for line in content.split("\\\\").filter(|x| !x.trim().is_empty()) {
                        blocks.push(Block::equation(Equation::new(
                            line.trim().to_string(),
                            !env.ends_with('*'),
                            find_label(line),
                        )?));
                    }
                    rest = &rest[consumed..];
                    continue;
                }
                "tabular" => Block::table(parse_tabular(&content)?),
                "table" => Block::table(parse_table_float(&content)?),
                "figure" => parse_figure(&content)?,
                "quote" => Block::quote(parse_blocks(&content, class)?),
                "tikzpicture" | "tikz" | "pgfpicture" | "pgfplots" => opaque(
                    &format!("\\begin{{{env}}}{content}\\end{{{env}}}"),
                    &format!("Unsupported LaTeX environment: {env}"),
                )?,
                _ => opaque(
                    &rest[..consumed],
                    &format!("Unsupported LaTeX environment: {env}"),
                )?,
            };
            blocks.push(block);
            rest = &rest[consumed..];
            continue;
        }
        if let Some((cmd, arg, consumed)) = take_command(rest) {
            let level = match cmd.as_str() {
                "part" if class == "report" || class == "book" => Some(HeadingLevel::H1),
                "chapter" if class == "report" || class == "book" => Some(HeadingLevel::H1),
                "section" => Some(if class == "report" || class == "book" {
                    HeadingLevel::H2
                } else {
                    HeadingLevel::H1
                }),
                "subsection" => Some(if class == "report" || class == "book" {
                    HeadingLevel::H3
                } else {
                    HeadingLevel::H2
                }),
                "subsubsection" => Some(if class == "report" || class == "book" {
                    HeadingLevel::H4
                } else {
                    HeadingLevel::H3
                }),
                "paragraph" => Some(if class == "report" || class == "book" {
                    HeadingLevel::H5
                } else {
                    HeadingLevel::H4
                }),
                _ => None,
            };
            if let Some(level) = level {
                let label = find_label(&rest[consumed..]);
                blocks.push(Block::heading(level, parse_inlines(&arg)?, label)?);
                rest = &rest[consumed..];
                continue;
            }
            if cmd == "label" || cmd == "caption" || cmd == "item" {
                return Err(parse_error(
                    source,
                    "command is only valid inside its enclosing construct",
                ));
            }
            if cmd == "input" || cmd == "include" {
                blocks.push(opaque(
                    &rest[..consumed],
                    "Multi-file input requires project resolution",
                )?);
                rest = &rest[consumed..];
                continue;
            }
            if matches!(
                cmd.as_str(),
                "textbf"
                    | "bfseries"
                    | "textit"
                    | "emph"
                    | "itshape"
                    | "underline"
                    | "texttt"
                    | "footnote"
                    | "cite"
                    | "citep"
                    | "citet"
                    | "ref"
                    | "cref"
                    | "autoref"
            ) {
                let end = next_block_boundary(rest);
                blocks.push(Block::paragraph(parse_inlines(rest[..end].trim())?));
                rest = &rest[end..];
                continue;
            }
            blocks.push(opaque(
                &rest[..consumed],
                &format!("Unsupported LaTeX command: \\{cmd}"),
            )?);
            rest = &rest[consumed..];
            continue;
        }
        let end = next_block_boundary(rest);
        let text = &rest[..end];
        if !text.trim().is_empty() {
            match parse_inlines(text.trim()) {
                Ok(inline) => blocks.push(Block::paragraph(inline)),
                Err(_) => blocks.push(opaque(text.trim(), "Unsupported LaTeX inline construct")?),
            }
        }
        rest = &rest[end..];
    }
    Ok(blocks)
}

fn parse_list(content: &str, kind: ListKind, class: &str) -> Result<Block, TesseraError> {
    let mut items = Vec::new();
    for piece in content.split("\\item").skip(1) {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        if piece.starts_with('[') {
            let end = piece
                .find(']')
                .ok_or_else(|| parse_error(content, "unterminated custom list label"))?;
            items.push(vec![opaque(&piece[..end + 1], "Custom list item label")?]);
            let rest = piece[end + 1..].trim();
            if !rest.is_empty() {
                if let Some(item) = items.last_mut() {
                    item.push(Block::paragraph(parse_inlines(rest)?));
                }
            }
        } else {
            items.push(parse_blocks(piece, class)?);
        }
    }
    Ok(Block::list(kind, items))
}

fn parse_tabular(content: &str) -> Result<Table, TesseraError> {
    let trimmed = content.trim_start();
    let (column_specs, content) = if trimmed.starts_with('{') {
        balanced_group(trimmed)
            .map(|(spec, end)| (vec![spec], &trimmed[end..]))
            .unwrap_or_else(|| (Vec::new(), content))
    } else {
        (Vec::new(), content)
    };
    let hline_before_rows = content
        .match_indices("\\hline")
        .map(|(at, _)| content[..at].matches("\\\\").count())
        .collect();
    let cleaned = content
        .replace("\\toprule", "")
        .replace("\\midrule", "")
        .replace("\\bottomrule", "")
        .replace("\\hline", "");
    let mut rows = Vec::new();
    for raw_row in cleaned.split("\\\\").filter(|r| !r.trim().is_empty()) {
        let mut cells = Vec::new();
        for raw in raw_row.split('&') {
            let (col_span, row_span, text) = if raw.trim_start().starts_with("\\multicolumn") {
                let args = groups(raw);
                let span = args
                    .first()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(1);
                (span, 1, args.get(2).cloned().unwrap_or_default())
            } else if raw.trim_start().starts_with("\\multirow") {
                let args = groups(raw);
                let span = args
                    .first()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(1);
                (1, span, args.get(2).cloned().unwrap_or_default())
            } else {
                (1, 1, raw.trim().to_string())
            };
            cells.push(TableCell::new(
                vec![Block::paragraph(parse_inlines(&text)?)],
                col_span,
                row_span,
            )?);
        }
        rows.push(cells);
    }
    let rule_style = if content.contains("\\toprule")
        || content.contains("\\midrule")
        || content.contains("\\bottomrule")
    {
        RuleStyle::Booktabs
    } else if content.contains("\\hline") {
        RuleStyle::Hline
    } else {
        RuleStyle::None
    };
    let mut table = Table::new(rows, None, None)?;
    table.set_latex_style(Some(TableStyle::new(
        column_specs,
        rule_style,
        hline_before_rows,
    )));
    Ok(table)
}

fn parse_table_float(content: &str) -> Result<Table, TesseraError> {
    let inner = take_environment(content)
        .map(|(_, c, _)| c)
        .unwrap_or_else(|| content.to_string());
    let mut table = parse_tabular(&inner)?;
    let caption = command_arg(&inner, "caption")
        .map(|x| parse_inlines(&x.0))
        .transpose()?;
    let label = find_label(&inner);
    let style = table.latex_style().cloned();
    table = Table::new(table.rows().to_vec(), caption, label)?;
    table.set_latex_style(style);
    Ok(table)
}

fn parse_figure(content: &str) -> Result<Block, TesseraError> {
    let image = command_with_options_arg(content, "includegraphics")
        .ok_or_else(|| parse_error(content, "figure has no \\includegraphics"))?;
    let path = image.0.trim().to_string();
    let width = option_value(content, "width").and_then(|x| parse_width(&x));
    let caption = command_arg(content, "caption")
        .map(|x| parse_inlines(&x.0))
        .transpose()?;
    Figure::new(path, caption, find_label(content), width).map(Block::figure)
}

fn parse_inlines(source: &str) -> Result<Vec<Inline>, TesseraError> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = source.as_bytes();
    while i < source.len() {
        if bytes[i] == b'%' {
            i = source[i..]
                .find('\n')
                .map(|n| i + n + 1)
                .unwrap_or(source.len());
            continue;
        }
        if bytes[i] == b'$' {
            let end = source[i + 1..]
                .find('$')
                .ok_or_else(|| parse_error(source, "unterminated inline math"))?
                + i
                + 1;
            out.push(Inline::inline_math(source[i + 1..end].to_string())?);
            i = end + 1;
            continue;
        }
        if source[i..].starts_with("\\(") {
            let end = source[i + 2..]
                .find("\\)")
                .ok_or_else(|| parse_error(source, "unterminated inline math"))?
                + i
                + 2;
            out.push(Inline::inline_math(source[i + 2..end].to_string())?);
            i = end + 2;
            continue;
        }
        if bytes[i] == b'\\' {
            if source[i..].starts_with("\\\\") {
                out.push(Inline::line_break());
                i += 2;
                continue;
            }
            if source[i..].starts_with("\\verb") {
                let delimiter = source
                    .as_bytes()
                    .get(i + 5)
                    .copied()
                    .ok_or_else(|| parse_error(source, "missing verbatim delimiter"))?
                    as char;
                let start = i + 6;
                let end = source[start..]
                    .find(delimiter)
                    .ok_or_else(|| parse_error(source, "unterminated verbatim text"))?
                    + start;
                out.push(Inline::code(source[start..end].to_string())?);
                i = end + 1;
                continue;
            }
            let (cmd, arg, consumed) = take_command(&source[i..])
                .ok_or_else(|| parse_error(source, "malformed command"))?;
            let node = match cmd.as_str() {
                "textbf" | "bfseries" => Inline::bold(parse_inlines(&arg)?),
                "textit" | "emph" | "itshape" => Inline::italic(parse_inlines(&arg)?),
                "underline" => Inline::underline(parse_inlines(&arg)?),
                "texttt" => Inline::code(arg)?,
                "footnote" => Inline::footnote(parse_inlines(&arg)?),
                "cite" | "citep" | "citet" => {
                    Inline::citation(arg.split(',').map(|s| s.trim().to_string()).collect())?
                }
                "ref" | "cref" | "autoref" => Inline::cross_ref(arg.trim().to_string())?,
                " " => Inline::text(" ".to_string())?,
                _ => {
                    return Err(parse_error(
                        source,
                        &format!("unsupported inline command: \\{cmd}"),
                    ))
                }
            };
            out.push(node);
            i += consumed;
            continue;
        }
        let mut end = i + 1;
        while end < source.len() && !matches!(source.as_bytes()[end], b'\\' | b'$' | b'%') {
            end += 1;
        }
        let text = source[i..end].to_string();
        if !text.is_empty() && !text.trim().is_empty() {
            out.push(Inline::text(text)?);
        }
        i = end;
    }
    Ok(out)
}

fn take_command(source: &str) -> Option<(String, String, usize)> {
    if !source.starts_with('\\') {
        return None;
    }
    let mut end = 1;
    while end < source.len() && source.as_bytes()[end].is_ascii_alphabetic() {
        end += 1;
    }
    if end == 1 {
        return Some((source[1..2].to_string(), String::new(), 2));
    }
    let name = source[1..end].to_string();
    let mut consumed = end;
    let arg = if source[consumed..].trim_start().starts_with('{') {
        consumed += source[consumed..].find('{').unwrap_or(0);
        let (value, n) = balanced_group(&source[consumed..])?;
        consumed += n;
        value
    } else {
        String::new()
    };
    Some((name, arg, consumed))
}

fn command_arg(source: &str, command: &str) -> Option<(String, usize)> {
    let needle = format!("\\{command}");
    let pos = source.find(&needle)?;
    let (_, arg, _) = take_command(&source[pos..])?;
    Some((arg, pos))
}

fn command_with_options_arg(source: &str, command: &str) -> Option<(String, usize)> {
    let needle = format!("\\{command}");
    let pos = source.find(&needle)?;
    let mut cursor = pos + needle.len();
    let after = source[cursor..].trim_start();
    cursor += source[cursor..].len() - after.len();
    if after.starts_with('[') {
        cursor += after.find(']')? + 1;
    }
    let (value, _) = balanced_group(&source[cursor..])?;
    Some((value, cursor))
}

fn document_class(source: &str) -> Option<(Vec<String>, String)> {
    let start = source.find("\\documentclass")?;
    let mut rest = source[start + "\\documentclass".len()..].trim_start();
    let options = if rest.starts_with('[') {
        let end = rest.find(']')?;
        let values = rest[1..end]
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect();
        rest = rest[end + 1..].trim_start();
        values
    } else {
        Vec::new()
    };
    let (class, _) = balanced_group(rest)?;
    Some((options, class.trim().to_string()))
}

fn balanced_group(source: &str) -> Option<(String, usize)> {
    if !source.starts_with('{') {
        return None;
    }
    let mut depth = 0;
    for (i, c) in source.char_indices() {
        if c == '{' {
            depth += 1;
        }
        if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some((source[1..i].to_string(), i + 1));
            }
        }
    }
    None
}

fn first_group(source: &str) -> Option<String> {
    let start = source.find('{')?;
    balanced_group(&source[start..]).map(|x| x.0)
}
fn groups(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find('{') {
        if let Some((v, n)) = balanced_group(&rest[start..]) {
            out.push(v);
            rest = &rest[start + n..];
        } else {
            break;
        }
    }
    out
}
fn clean_metadata(value: &str) -> String {
    value
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn find_label(source: &str) -> Option<String> {
    command_arg(source, "label")
        .map(|x| x.0.trim().to_string())
        .filter(|x| !x.is_empty())
}
fn next_block_boundary(source: &str) -> usize {
    source.find("\n\n").map(|i| i + 2).unwrap_or(source.len())
}
fn option_value(source: &str, key: &str) -> Option<String> {
    let start = source.find('[')?;
    let end = source[start..].find(']')? + start;
    source[start + 1..end].split(',').find_map(|p| {
        p.trim()
            .strip_prefix(&format!("{key}="))
            .map(str::to_string)
    })
}
fn parse_width(value: &str) -> Option<f32> {
    let n = value
        .trim()
        .trim_end_matches("\\linewidth")
        .trim_end_matches("\\textwidth")
        .trim_end_matches("pt")
        .parse::<f32>()
        .ok()?;
    if value.contains("linewidth") || value.contains("textwidth") {
        Some(n * 100.0)
    } else {
        Some(n)
    }
}
fn opaque(raw: &str, summary: &str) -> Result<Block, TesseraError> {
    Ok(Block::opaque(OpaqueBlock::new(
        OpaqueOrigin::Latex,
        raw.to_string(),
        summary.to_string(),
    )?))
}
fn parse_error(source: &str, message: &str) -> TesseraError {
    let line = source.lines().count().max(1) as u32;
    TesseraError::ParseError {
        pane: Pane::Latex,
        message: message.to_string(),
        span: Some(SourceSpan {
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col: 1,
        }),
    }
}

fn take_environment(source: &str) -> Option<(String, String, usize)> {
    if !source.starts_with("\\begin{") {
        return None;
    }
    let (env, begin_len) = balanced_group(&source[6..]).map(|(e, n)| (e, n + 6))?;
    let end_marker = format!("\\end{{{env}}}");
    let end = source[begin_len..].find(&end_marker)? + begin_len;
    Some((
        env,
        source[begin_len..end].to_string(),
        end + end_marker.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_basic_document() {
        let doc = parse_latex("\\documentclass[12pt]{article}\\title{T}\\begin{document}\\section{Intro}\\textbf{hello}\\end{document}").expect("valid fixture");
        assert_eq!(doc.metadata().title(), Some("T"));
        assert_eq!(doc.body().len(), 2);
    }
}
