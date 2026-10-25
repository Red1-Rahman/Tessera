// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! DOCX writer.  The writer deliberately emits the small, conventional
//! OOXML package used by Word rather than depending on a Word installation.

use crate::core::error::{ExportFormat, TesseraError};
pub use crate::core::model::SyncWarning;
use crate::core::model::{
    Block, Document, Equation, Inline, ListKind, OpaqueOrigin, Table, TableCell,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Result of writing a document, including non-fatal conversion warnings.
#[derive(Debug, Clone)]
pub struct DocxOutput {
    /// Complete, ready-to-save DOCX bytes.
    pub bytes: Vec<u8>,
    /// Warnings for constructs that could not be represented exactly.
    pub warnings: Vec<SyncWarning>,
}

struct Render<'a> {
    assets: &'a Path,
    warnings: Vec<SyncWarning>,
    relationships: Vec<(String, String, String)>,
    media: Vec<(String, Vec<u8>, String)>,
    next_rel: usize,
    next_docpr: u32,
}

impl<'a> Render<'a> {
    fn new(assets: &'a Path) -> Self {
        Self {
            assets,
            warnings: Vec::new(),
            relationships: vec![(
                "rId1".to_string(),
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering"
                    .to_string(),
                "numbering.xml".to_string(),
            )],
            media: Vec::new(),
            next_rel: 2,
            next_docpr: 1,
        }
    }

    fn warning(&mut self, message: impl Into<String>, path: &[usize]) {
        self.warnings.push(SyncWarning {
            message: message.into(),
            affected_path: path.to_vec(),
        });
    }

    fn rel(&mut self, target: String, kind: &str) -> String {
        let id = format!("rId{}", self.next_rel);
        self.next_rel += 1;
        self.relationships
            .push((id.clone(), kind.to_string(), target));
        id
    }
}

/// Render `document` as a normal editable OOXML package.
///
/// `assets_dir` is the project's `.tessera/assets` directory.  Missing image
/// files are represented by a visible placeholder and a warning, never by a
/// silently omitted figure.
pub fn write_docx(
    document: &Document,
    assets_dir: impl AsRef<Path>,
) -> Result<DocxOutput, TesseraError> {
    let mut render = Render::new(assets_dir.as_ref());
    let mut body = String::new();
    if let Some(title) = document.metadata().title() {
        body.push_str(&paragraph("Title", &text_run(title)));
    }
    for author in document.metadata().authors() {
        body.push_str(&paragraph("Subtitle", &text_run(author.name())));
    }
    for (index, block) in document.body().iter().enumerate() {
        render_block(&mut render, block, &mut body, &[index])?;
    }
    body.push_str("<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr>");
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body>{body}</w:body></w:document>"#
    );
    let bytes = package(&document_xml, &render)?;
    Ok(DocxOutput {
        bytes,
        warnings: render.warnings,
    })
}

/// Write a DOCX directly to a path and return conversion warnings.
pub fn write_docx_file(
    document: &Document,
    destination: impl AsRef<Path>,
    assets_dir: impl AsRef<Path>,
) -> Result<Vec<SyncWarning>, TesseraError> {
    let output = write_docx(document, assets_dir)?;
    fs::write(destination.as_ref(), &output.bytes).map_err(|e| TesseraError::IoError {
        path: destination.as_ref().display().to_string(),
        message: e.to_string(),
    })?;
    Ok(output.warnings)
}

fn render_block(
    render: &mut Render<'_>,
    block: &Block,
    out: &mut String,
    path: &[usize],
) -> Result<(), TesseraError> {
    match block {
        Block::Heading { level, inline, .. } => out.push_str(&paragraph(
            &format!("Heading {}", level.depth()),
            &inlines(render, inline, path),
        )),
        Block::Paragraph { inline, .. } => {
            out.push_str(&paragraph("Normal", &inlines(render, inline, path)))
        }
        Block::List { kind, items, .. } => {
            for item in items {
                let style = if *kind == ListKind::Ordered {
                    "List Number"
                } else {
                    "List Bullet"
                };
                let mut content = String::new();
                for b in item {
                    render_block(render, b, &mut content, path)?;
                }
                if !apply_list_paragraph_style(&mut content, style) {
                    content.push_str(&paragraph(style, ""));
                }
                out.push_str(&content);
            }
        }
        Block::Table(table) => out.push_str(&render_table(render, table, path)),
        Block::Figure(figure) => {
            let (rel, dimensions) = render_image(render, figure.asset_path(), path);
            let docpr = render.next_docpr;
            render.next_docpr += 1;
            out.push_str(&paragraph(
                "Normal",
                &figure_xml(rel, figure.width_percent(), dimensions, docpr),
            ));
            if let Some(caption) = figure.caption() {
                out.push_str(&paragraph("Caption", &inlines(render, caption, path)));
            }
        }
        Block::Equation(equation) => out.push_str(&equation_xml(render, equation, path)),
        Block::CodeBlock { text, .. } => out.push_str(&paragraph("No Spacing", &text_run(text))),
        Block::Quote { blocks, .. } => {
            for b in blocks {
                render_block(render, b, out, path)?;
            }
        }
        Block::Opaque(value) => match value.origin() {
            OpaqueOrigin::Docx => {
                if value.raw().contains("<w:p") {
                    out.push_str(value.raw());
                } else {
                    out.push_str(&paragraph("Normal", &text_run(value.raw())));
                }
            }
            OpaqueOrigin::Latex => {
                out.push_str(&format!(
                    r#"<w:p><w:pPr><w:shd w:fill="EADCF0"/></w:pPr>{}<w:r><w:br/></w:r>{}</w:p>"#,
                    text_run(&format!("Opaque LaTeX content: {}", value.summary())),
                    text_run(value.raw())
                ));
            }
        },
    }
    Ok(())
}

fn apply_list_paragraph_style(content: &mut String, style: &str) -> bool {
    let Some(paragraph_start) = content.find("<w:p>") else {
        return false;
    };
    let insert_at = paragraph_start + "<w:p>".len();
    let style_property = format!("<w:pStyle w:val=\"{}\"/>", style_id(style));
    if let Some(properties_start) = content[insert_at..].find("<w:pPr>") {
        let start = insert_at + properties_start;
        if let Some(properties_end) = content[start..].find("</w:pPr>") {
            let end = start + properties_end;
            if let Some(style_start) = content[start..end].find("<w:pStyle") {
                let style_start = start + style_start;
                if let Some(style_end) = content[style_start..end].find("/>") {
                    content
                        .replace_range(style_start..style_start + style_end + 2, &style_property);
                    return true;
                }
            }
            content.insert_str(end, &style_property);
            return true;
        }
    }
    content.insert_str(insert_at, &format!("<w:pPr>{style_property}</w:pPr>"));
    true
}

fn inlines(render: &mut Render<'_>, values: &[Inline], path: &[usize]) -> String {
    inlines_with_properties(render, values, path, &[])
}

fn inlines_with_properties(
    render: &mut Render<'_>,
    values: &[Inline],
    path: &[usize],
    properties: &[&str],
) -> String {
    let mut output = String::new();
    for value in values {
        match value {
            Inline::Text { text } => output.push_str(&styled_text_run(text, properties)),
            Inline::Bold { inline } => output.push_str(&inlines_with_properties(
                render,
                inline,
                path,
                &append_property(properties, "<w:b/>"),
            )),
            Inline::Italic { inline } => output.push_str(&inlines_with_properties(
                render,
                inline,
                path,
                &append_property(properties, "<w:i/>"),
            )),
            Inline::Underline { inline } => output.push_str(&inlines_with_properties(
                render,
                inline,
                path,
                &append_property(properties, "<w:u w:val=\"single\"/>"),
            )),
            Inline::Code { text } => output.push_str(&styled_text_run(
                text,
                &append_property(
                    properties,
                    "<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>",
                ),
            )),
            Inline::Citation { keys } => output.push_str(&styled_text_run(
                &format!("[{}]", keys.join(", ")),
                properties,
            )),
            Inline::CrossRef { label } => {
                output.push_str(&styled_text_run(&format!("({label})"), properties))
            }
            Inline::Footnote { inline } => {
                render.warning(
                    "Footnote rendered inline because no footnotes part is packaged",
                    path,
                );
                output.push_str(&inlines_with_properties(render, inline, path, properties));
            }
            Inline::InlineMath { latex_source } => {
                output.push_str(&omml(render, latex_source, false, path));
            }
            Inline::LineBreak => output.push_str("<w:r><w:br/></w:r>"),
        }
    }
    output
}

fn append_property<'a>(properties: &[&'a str], property: &'a str) -> Vec<&'a str> {
    let mut nested = properties.to_vec();
    nested.push(property);
    nested
}

fn styled_text_run(text: &str, properties: &[&str]) -> String {
    let run_properties = if properties.is_empty() {
        String::new()
    } else {
        format!("<w:rPr>{}</w:rPr>", properties.concat())
    };
    format!(
        "<w:r>{run_properties}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
        xml(text)
    )
}

fn render_table(render: &mut Render<'_>, table: &Table, path: &[usize]) -> String {
    let width = table
        .rows()
        .iter()
        .map(|r| r.iter().map(TableCell::col_span).sum::<u32>())
        .max()
        .unwrap_or(1);
    let mut active: Vec<u32> = vec![0; width as usize];
    let mut xml = format!(
        r#"<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/></w:tblPr><w:tblGrid>{}</w:tblGrid>"#,
        (0..width)
            .map(|_| "<w:gridCol w:w=\"1440\"/>")
            .collect::<String>()
    );
    for (ri, row) in table.rows().iter().enumerate() {
        xml.push_str("<w:tr>");
        let mut col = 0usize;
        let mut source = 0usize;
        while col < width as usize {
            if active[col] > 0 {
                xml.push_str(&cell_xml("", 1, Some("continue"), false));
                active[col] -= 1;
                col += 1;
                continue;
            }
            let cell = row.get(source);
            source += 1;
            if let Some(cell) = cell {
                let span = cell.col_span().min(width - col as u32);
                let mut content = String::new();
                for b in cell.blocks() {
                    let _ = render_block(render, b, &mut content, path);
                }
                xml.push_str(&cell_xml(
                    &content,
                    span,
                    if cell.row_span() > 1 {
                        Some("restart")
                    } else {
                        None
                    },
                    true,
                ));
                if cell.row_span() > 1 {
                    for slot in active.iter_mut().skip(col).take(span as usize) {
                        *slot = cell.row_span() - 1;
                    }
                }
                col += span as usize;
            } else {
                xml.push_str(&cell_xml("", 1, None, false));
                col += 1;
            }
        }
        xml.push_str("</w:tr>");
        let _ = ri;
    }
    xml.push_str("</w:tbl>");
    if let Some(caption) = table.caption() {
        xml.push_str(&paragraph("Caption", &inlines(render, caption, path)));
    }
    xml
}

fn cell_xml(content: &str, span: u32, merge: Option<&str>, real: bool) -> String {
    let grid = if span > 1 {
        format!(r#"<w:gridSpan w:val="{}"/>"#, span)
    } else {
        String::new()
    };
    let vm = merge
        .map(|v| format!(r#"<w:vMerge w:val="{}"/>"#, v))
        .unwrap_or_default();
    format!(
        "<w:tc><w:tcPr>{grid}{vm}</w:tcPr>{}</w:tc>",
        if real && !content.is_empty() {
            content.to_string()
        } else {
            "<w:p/>".to_string()
        }
    )
}

fn equation_xml(render: &mut Render<'_>, equation: &Equation, path: &[usize]) -> String {
    let omml = omml(render, equation.latex_source(), true, path);
    format!("<w:p>{}</w:p>", omml)
}

fn omml(render: &mut Render<'_>, source: &str, display: bool, path: &[usize]) -> String {
    let (body, unsupported) = math_content(source);
    if unsupported {
        render.warning(
            format!(
                "Math source is outside the supported OMML subset: {}",
                source
            ),
            path,
        );
    }
    let prefix = if display { "<m:oMathPara>" } else { "" };
    let suffix = if display { "</m:oMathPara>" } else { "" };
    format!(r#"{prefix}<m:oMath>{body}</m:oMath>{suffix}"#)
}

fn math_content(source: &str) -> (String, bool) {
    let chars: Vec<char> = source.chars().collect();
    let (atoms, unsupported) = math_atoms(&chars, 0, chars.len());
    (atoms.concat(), unsupported)
}

fn math_atoms(chars: &[char], mut index: usize, end: usize) -> (Vec<String>, bool) {
    let mut atoms = Vec::new();
    let mut unsupported = false;
    while index < end {
        let character = chars[index];
        match character {
            '^' | '_' => {
                let tag = if character == '^' { "m:sSup" } else { "m:sSub" };
                let (script, next, script_unsupported) = math_script(chars, index + 1, end);
                let base = atoms.pop().unwrap_or_else(|| {
                    unsupported = true;
                    omml_run("?")
                });
                atoms.push(format!(
                    "<{tag}><m:e>{base}</m:e><m:{}>{script}</m:{}></{tag}>",
                    if character == '^' { "sup" } else { "sub" },
                    if character == '^' { "sup" } else { "sub" },
                ));
                unsupported |= script_unsupported;
                index = next;
            }
            '\\' => {
                let (command, next) = math_command(chars, index + 1, end);
                if let Some(greek) = greek_character(&command) {
                    atoms.push(omml_run(&greek.to_string()));
                } else {
                    atoms.push(omml_run(&format!("\\{command}")));
                    unsupported = true;
                }
                index = next;
            }
            '{' => {
                let (close, depth) = matching_brace(chars, index, end);
                if let Some(close) = close {
                    let (inner, inner_unsupported) = math_atoms(chars, index + 1, close);
                    atoms.extend(inner);
                    unsupported |= inner_unsupported;
                    index = close + 1;
                    let _ = depth;
                } else {
                    atoms.push(omml_run("{"));
                    unsupported = true;
                    index += 1;
                }
            }
            '}' => index += 1,
            value if value.is_whitespace() => index += 1,
            value => {
                atoms.push(omml_run(&value.to_string()));
                index += 1;
            }
        }
    }
    (atoms, unsupported)
}

fn math_script(chars: &[char], index: usize, end: usize) -> (String, usize, bool) {
    if index >= end {
        return (omml_run("?"), index, true);
    }
    if chars[index] == '{' {
        if let (Some(close), _) = matching_brace(chars, index, end) {
            let (atoms, unsupported) = math_atoms(chars, index + 1, close);
            return (atoms.concat(), close + 1, unsupported);
        }
        return (omml_run("{"), index + 1, true);
    }
    let script_end = if chars[index] == '\\' {
        math_command(chars, index + 1, end).1
    } else {
        index + 1
    };
    let (atoms, unsupported) = math_atoms(chars, index, script_end);
    (atoms.concat(), script_end, unsupported)
}

fn math_command(chars: &[char], mut index: usize, end: usize) -> (String, usize) {
    let start = index;
    while index < end && chars[index].is_ascii_alphabetic() {
        index += 1;
    }
    if start == index && index < end {
        index += 1;
    }
    (chars[start..index].iter().collect(), index)
}

fn matching_brace(chars: &[char], start: usize, end: usize) -> (Option<usize>, usize) {
    let mut depth = 0usize;
    let mut index = start;
    while index < end {
        if chars[index] == '\\' {
            index = (index + 2).min(end);
            continue;
        }
        match chars[index] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return (Some(index), depth);
                }
            }
            _ => {}
        }
        index += 1;
    }
    (None, depth)
}

fn greek_character(command: &str) -> Option<char> {
    Some(match command {
        "alpha" => 'α',
        "beta" => 'β',
        "gamma" => 'γ',
        "delta" => 'δ',
        "epsilon" => 'ε',
        "theta" => 'θ',
        "lambda" => 'λ',
        "mu" => 'μ',
        "pi" => 'π',
        "sigma" => 'σ',
        "phi" => 'φ',
        "omega" => 'ω',
        _ => return None,
    })
}

fn omml_run(text: &str) -> String {
    format!(
        r#"<m:r><m:t xml:space="preserve">{}</m:t></m:r>"#,
        xml(text)
    )
}

fn render_image(
    render: &mut Render<'_>,
    path: &str,
    affected: &[usize],
) -> (String, Option<(u32, u32)>) {
    let assets_root = match render.assets.canonicalize() {
        Ok(root) => root,
        Err(error) => {
            render.warning(
                format!("Figure assets directory is unavailable: {error}"),
                affected,
            );
            return (String::new(), None);
        }
    };
    let requested = render.assets.join(PathBuf::from(path));
    let source = match requested.canonicalize() {
        Ok(source) if source.starts_with(&assets_root) => source,
        Ok(_) => {
            render.warning(
                format!("Figure asset `{path}` escapes the assets directory"),
                affected,
            );
            return (String::new(), None);
        }
        Err(error) => {
            render.warning(
                format!("Figure asset `{path}` could not be opened: {error}"),
                affected,
            );
            return (String::new(), None);
        }
    };
    let ext = source
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ext.is_empty() {
        render.warning(
            format!("Figure asset `{path}` has no file extension"),
            affected,
        );
        return (String::new(), None);
    }
    let data = match fs::read(&source) {
        Ok(bytes) => bytes,
        Err(error) => {
            render.warning(
                format!("Figure asset `{}` could not be embedded: {}", path, error),
                affected,
            );
            return (String::new(), None);
        }
    };
    let content_type = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };
    let name = format!("image{}.{}", render.media.len() + 1, ext);
    let dimensions = image_dimensions(&data, &ext);
    render
        .media
        .push((name.clone(), data, content_type.to_string()));
    let rel = render.rel(
        format!("media/{}", name),
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
    );
    (rel, dimensions)
}

fn image_dimensions(data: &[u8], extension: &str) -> Option<(u32, u32)> {
    match extension {
        "png" if data.len() >= 24 && data.starts_with(b"\x89PNG\r\n\x1a\n") => Some((
            u32::from_be_bytes(data[16..20].try_into().ok()?),
            u32::from_be_bytes(data[20..24].try_into().ok()?),
        )),
        "gif" if data.len() >= 10 && data.starts_with(b"GIF8") => Some((
            u16::from_le_bytes(data[6..8].try_into().ok()?) as u32,
            u16::from_le_bytes(data[8..10].try_into().ok()?) as u32,
        )),
        "bmp" if data.len() >= 26 && data.starts_with(b"BM") => Some((
            u32::from_le_bytes(data[18..22].try_into().ok()?),
            i32::from_le_bytes(data[22..26].try_into().ok()?).unsigned_abs(),
        )),
        "jpg" | "jpeg" => jpeg_dimensions(data),
        _ => None,
    }
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.get(..2)? != [0xff, 0xd8] {
        return None;
    }
    let mut cursor = 2;
    while cursor + 4 <= data.len() {
        if data[cursor] != 0xff {
            return None;
        }
        let marker = data[cursor + 1];
        cursor += 2;
        if matches!(marker, 0xd8 | 0xd9 | 0x01 | 0xd0..=0xd7) {
            continue;
        }
        let length = u16::from_be_bytes([*data.get(cursor)?, *data.get(cursor + 1)?]) as usize;
        if length < 2 || cursor + length > data.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            return Some((
                u16::from_be_bytes([*data.get(cursor + 5)?, *data.get(cursor + 6)?]) as u32,
                u16::from_be_bytes([*data.get(cursor + 3)?, *data.get(cursor + 4)?]) as u32,
            ));
        }
        cursor += length;
    }
    None
}

fn figure_xml(
    rel: String,
    width: Option<f32>,
    dimensions: Option<(u32, u32)>,
    docpr: u32,
) -> String {
    if rel.is_empty() {
        return text_run("[Missing figure asset]");
    }
    let cx = ((width.unwrap_or(100.0) / 100.0) * 6_858_000.0) as u64;
    let cy = dimensions
        .filter(|(image_width, image_height)| *image_width > 0 && *image_height > 0)
        .map(|(image_width, image_height)| cx * image_height as u64 / image_width as u64)
        .unwrap_or(4_000_000);
    format!(
        r#"<w:r><w:drawing><wp:inline><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{docpr}" name="Figure {docpr}"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="0" name="Figure {docpr}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="{rel}"/></pic:blipFill><pic:spPr><a:xfrm><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
    )
}

fn paragraph(style: &str, content: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:pStyle w:val="{}"/></w:pPr>{}</w:p>"#,
        xml(style_id(style)),
        content
    )
}

fn text_run(text: &str) -> String {
    format!("<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r>", xml(text))
}

fn style_id(style: &str) -> &str {
    match style {
        "Heading 1" => "Heading1",
        "Heading 2" => "Heading2",
        "Heading 3" => "Heading3",
        "Heading 4" => "Heading4",
        "Heading 5" => "Heading5",
        "Heading 6" => "Heading6",
        "List Bullet" => "ListBullet",
        "List Number" => "ListNumber",
        "No Spacing" => "NoSpacing",
        other => other,
    }
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn package(document: &str, render: &Render<'_>) -> Result<Vec<u8>, TesseraError> {
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = SimpleFileOptions::default();
    let content_types = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>{}<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#,
        media_content_types(render)
    );
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let doc_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{}</Relationships>"#,
        render
            .relationships
            .iter()
            .map(|(id, typ, target)| format!(
                r#"<Relationship Id="{id}" Type="{typ}" Target="{target}"/>"#
            ))
            .collect::<String>()
    );
    let styles = r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults/><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/></w:style><w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/></w:style><w:style w:type="paragraph" w:styleId="Caption"><w:name w:val="Caption"/></w:style><w:style w:type="paragraph" w:styleId="NoSpacing"><w:name w:val="No Spacing"/></w:style><w:style w:type="paragraph" w:styleId="ListBullet"><w:name w:val="List Bullet"/><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr></w:pPr></w:style><w:style w:type="paragraph" w:styleId="ListNumber"><w:name w:val="List Number"/><w:pPr><w:numPr><w:numId w:val="2"/></w:numPr></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/></w:style><w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/></w:style><w:style w:type="paragraph" w:styleId="Heading4"><w:name w:val="heading 4"/></w:style><w:style w:type="paragraph" w:styleId="Heading5"><w:name w:val="heading 5"/></w:style><w:style w:type="paragraph" w:styleId="Heading6"><w:name w:val="heading 6"/></w:style></w:styles>"#;
    let numbering = r#"<?xml version="1.0" encoding="UTF-8"?><w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="singleLevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId="1"><w:multiLevelType w:val="singleLevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num></w:numbering>"#;
    for (name, data) in [
        ("[Content_Types].xml", content_types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("word/document.xml", document.as_bytes()),
        ("word/styles.xml", styles.as_bytes()),
        ("word/numbering.xml", numbering.as_bytes()),
        ("word/_rels/document.xml.rels", doc_rels.as_bytes()),
    ] {
        zip.start_file(name, options).map_err(zip_error)?;
        zip.write_all(data).map_err(io_error)?;
    }
    for (name, data, _) in &render.media {
        zip.start_file(format!("word/media/{}", name), options)
            .map_err(zip_error)?;
        zip.write_all(data).map_err(io_error)?;
    }
    zip.finish().map_err(zip_error)?;
    Ok(cursor.into_inner())
}

fn zip_error(error: zip::result::ZipError) -> TesseraError {
    TesseraError::ExportError {
        format: ExportFormat::Docx,
        message: error.to_string(),
    }
}

fn io_error(error: std::io::Error) -> TesseraError {
    TesseraError::ExportError {
        format: ExportFormat::Docx,
        message: error.to_string(),
    }
}

fn media_content_types(render: &Render<'_>) -> String {
    let mut extensions = BTreeMap::new();
    for (name, _, content_type) in &render.media {
        if let Some(extension) = Path::new(name).extension().and_then(|value| value.to_str()) {
            extensions
                .entry(extension.to_ascii_lowercase())
                .or_insert_with(|| content_type.clone());
        }
    }
    extensions
        .into_iter()
        .map(|(extension, content_type)| {
            format!(r#"<Default Extension="{extension}" ContentType="{content_type}"/>"#)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{DocMetadata, Inline};
    use std::io::Read;
    use zip::ZipArchive;

    #[test]
    fn writes_an_ooxml_package_with_builtin_heading_style() {
        let heading = Block::heading(
            crate::core::model::HeadingLevel::H1,
            vec![Inline::text("Title".into()).expect("text")],
            None,
        )
        .expect("heading");
        let document = Document::new(DocMetadata::default(), vec![heading]);
        let output = write_docx(&document, std::env::temp_dir()).expect("write docx");
        let mut archive = ZipArchive::new(Cursor::new(output.bytes)).expect("zip archive");
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .expect("document part")
            .read_to_string(&mut xml)
            .expect("read document part");
        assert!(xml.contains("w:pStyle w:val=\"Heading1\""));
        assert!(xml.contains(">Title</w:t>"));
        assert!(archive.by_name("word/styles.xml").is_ok());
    }

    #[test]
    fn list_items_apply_numbering_style_to_content_paragraphs() {
        let item = Block::paragraph(vec![Inline::text("Item one".into()).expect("text")]);
        let document = Document::new(
            DocMetadata::default(),
            vec![Block::list(ListKind::Unordered, vec![vec![item]])],
        );
        let output = write_docx(&document, std::env::temp_dir()).expect("write docx");
        let mut archive = ZipArchive::new(Cursor::new(output.bytes)).expect("zip");
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .expect("document")
            .read_to_string(&mut xml)
            .expect("read");
        assert_eq!(xml.matches("<w:p>").count(), 1);
        assert!(xml.contains("<w:pStyle w:val=\"ListBullet\"/>"));
        let mut styles = String::new();
        archive
            .by_name("word/styles.xml")
            .expect("styles")
            .read_to_string(&mut styles)
            .expect("read");
        assert!(styles.contains("w:styleId=\"ListBullet\""));
        assert!(styles.contains("w:styleId=\"ListNumber\""));
        assert!(archive.by_name("word/numbering.xml").is_ok());
    }

    #[test]
    fn inline_formatting_uses_one_leaf_run_and_footnotes_warn_inline() {
        let document = Document::new(
            DocMetadata::default(),
            vec![Block::paragraph(vec![Inline::bold(vec![Inline::italic(
                vec![Inline::footnote(vec![
                    Inline::text("note".into()).expect("text")
                ])],
            )])])],
        );
        let output = write_docx(&document, std::env::temp_dir()).expect("write");
        let mut archive = ZipArchive::new(Cursor::new(output.bytes)).expect("zip");
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .expect("document")
            .read_to_string(&mut xml)
            .expect("read");
        assert_eq!(xml.matches("<w:r>").count(), 1);
        assert!(xml.contains("<w:b/><w:i/>"));
        assert!(!xml.contains("footnoteReference"));
        assert_eq!(output.warnings.len(), 1);
    }

    #[test]
    fn omml_uses_base_runs_for_scripts_and_unicode_greek() {
        let document = Document::new(
            DocMetadata::default(),
            vec![Block::equation(
                Equation::new("x^{n+1} + \\alpha".into(), true, None).expect("equation"),
            )],
        );
        let output = write_docx(&document, std::env::temp_dir()).expect("write");
        let mut archive = ZipArchive::new(Cursor::new(output.bytes)).expect("zip");
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .expect("document")
            .read_to_string(&mut xml)
            .expect("read");
        assert!(xml.contains("<m:sSup><m:e><m:r><m:t xml:space=\"preserve\">x</m:t></m:r></m:e>"));
        let alpha = char::from_u32(0x03b1).expect("Greek alpha");
        assert!(xml.contains(&format!("<m:t xml:space=\"preserve\">{alpha}</m:t>")));
        assert!(
            xml.contains("xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"")
        );
        assert!(!xml.contains("<m:oMath xmlns:m="));
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn media_defaults_use_unique_filename_extensions() {
        let mut render = Render::new(Path::new("."));
        render.media = vec![
            ("a.png".into(), Vec::new(), "image/png".into()),
            ("b.png".into(), Vec::new(), "image/png".into()),
            ("c.jpeg".into(), Vec::new(), "image/jpeg".into()),
            ("without-extension".into(), Vec::new(), "image/png".into()),
        ];
        let defaults = media_content_types(&render);
        assert_eq!(defaults.matches("Extension=\"png\"").count(), 1);
        assert_eq!(defaults.matches("Extension=\"jpeg\"").count(), 1);
        assert!(!defaults.contains("without-extension"));
    }
}
