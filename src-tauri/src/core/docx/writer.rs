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
}

impl<'a> Render<'a> {
    fn new(assets: &'a Path) -> Self {
        Self {
            assets,
            warnings: Vec::new(),
            relationships: Vec::new(),
            media: Vec::new(),
            next_rel: 1,
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
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}</w:body></w:document>"#
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
                out.push_str(&paragraph(style, ""));
                out.push_str(&content);
            }
        }
        Block::Table(table) => out.push_str(&render_table(render, table, path)),
        Block::Figure(figure) => {
            let rel = render_image(render, figure.asset_path(), path);
            out.push_str(&paragraph(
                "Normal",
                &figure_xml(rel, figure.width_percent()),
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
                let marked = format!(
                    "Opaque LaTeX content: {}\\n{}",
                    value.summary(),
                    value.raw()
                );
                out.push_str(&format!(
                    r#"<w:p><w:pPr><w:shd w:fill="EADCF0"/></w:pPr>{}</w:p>"#,
                    text_run(&marked)
                ));
            }
        },
    }
    Ok(())
}

fn inlines(render: &mut Render<'_>, values: &[Inline], path: &[usize]) -> String {
    values.iter().map(|v| match v {
        Inline::Text { text } => text_run(text),
        Inline::Bold { inline } => format!("<w:r><w:rPr><w:b/></w:rPr>{}</w:r>", inlines(render, inline, path)),
        Inline::Italic { inline } => format!("<w:r><w:rPr><w:i/></w:rPr>{}</w:r>", inlines(render, inline, path)),
        Inline::Underline { inline } => format!(r#"<w:r><w:rPr><w:u w:val="single"/></w:rPr>{}</w:r>"#, inlines(render, inline, path)),
        Inline::Code { text } => format!(r#"<w:r><w:rPr><w:rFonts w:ascii="Courier New" w:hAnsi="Courier New"/></w:rPr>{}</w:r>"#, text_run(text)),
        Inline::Citation { keys } => text_run(&format!("[{}]", keys.join(", "))),
        Inline::CrossRef { label } => text_run(&format!("({})", label)),
        Inline::Footnote { inline } => format!("<w:r><w:footnoteReference w:id=\"1\"/></w:r>{}", inlines(render, inline, path)),
        Inline::InlineMath { latex_source } => omml(render, latex_source, false, path),
        Inline::LineBreak => "<w:r><w:br/></w:r>".to_string(),
    }).collect()
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
    let mut body = String::new();
    let mut chars = source.chars().peekable();
    let mut unsupported = false;
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut command = String::new();
            while chars.peek().is_some_and(|x| x.is_ascii_alphabetic()) {
                command.push(chars.next().unwrap_or_default());
            }
            let greek = matches!(
                command.as_str(),
                "alpha"
                    | "beta"
                    | "gamma"
                    | "delta"
                    | "epsilon"
                    | "theta"
                    | "lambda"
                    | "mu"
                    | "pi"
                    | "sigma"
                    | "phi"
                    | "omega"
            );
            if greek {
                body.push_str(&format!(
                    r#"<m:r><m:t>{}</m:t></m:r>"#,
                    xml(&format!("\\{}", command))
                ));
            } else if command == "frac" {
                body.push_str(r#"<m:f><m:num><m:r><m:t>?</m:t></m:r></m:num><m:den><m:r><m:t>?</m:t></m:r></m:den></m:f>"#);
                unsupported = true;
            } else {
                unsupported = true;
                body.push_str(&text_run(&format!("\\{}", command)));
            }
        } else if c == '^' || c == '_' {
            let value = chars.next().unwrap_or('?');
            let tag = if c == '^' { "m:sup" } else { "m:sub" };
            body.push_str(&format!(
                r#"<{tag}><m:e><m:r><m:t>{}</m:t></m:r></m:e></{tag}>"#,
                xml(&value.to_string())
            ));
        } else if !c.is_whitespace() {
            body.push_str(&format!(r#"<m:r><m:t>{}</m:t></m:r>"#, xml(&c.to_string())));
        }
    }
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
    format!(
        r#"{prefix}<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">{body}</m:oMath>{suffix}"#
    )
}

fn render_image(render: &mut Render<'_>, path: &str, affected: &[usize]) -> String {
    let source = render.assets.join(PathBuf::from(path));
    let data = match fs::read(&source) {
        Ok(bytes) => bytes,
        Err(error) => {
            render.warning(
                format!("Figure asset `{}` could not be embedded: {}", path, error),
                affected,
            );
            return String::new();
        }
    };
    let ext = Path::new(path)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let content_type = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };
    let name = format!("image{}.{}", render.media.len() + 1, ext);
    render
        .media
        .push((name.clone(), data, content_type.to_string()));
    render.rel(
        format!("media/{}", name),
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
    )
}

fn figure_xml(rel: String, width: Option<f32>) -> String {
    if rel.is_empty() {
        return text_run("[Missing figure asset]");
    }
    let cx = ((width.unwrap_or(100.0) / 100.0) * 6_858_000.0) as u64;
    format!(
        r#"<w:r><w:drawing><wp:inline><wp:extent cx="{cx}" cy="400000"/><wp:docPr id="1" name="Figure"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:blipFill><a:blip r:embed="{rel}"/></pic:blipFill><pic:spPr><a:xfrm><a:ext cx="{cx}" cy="400000"/></a:xfrm><a:prstGeom prst="rect"/></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
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
        r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>{}<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#,
        render
            .media
            .iter()
            .map(|(_, _, ct)| format!(
                r#"<Default Extension="{}" ContentType="{}"/>"#,
                ct.rsplit('/').next().unwrap_or("png"),
                ct
            ))
            .collect::<String>()
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
    let styles = r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults/><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/></w:style><w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/></w:style><w:style w:type="paragraph" w:styleId="Caption"><w:name w:val="Caption"/></w:style><w:style w:type="paragraph" w:styleId="NoSpacing"><w:name w:val="No Spacing"/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/></w:style><w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/></w:style><w:style w:type="paragraph" w:styleId="Heading4"><w:name w:val="heading 4"/></w:style><w:style w:type="paragraph" w:styleId="Heading5"><w:name w:val="heading 5"/></w:style><w:style w:type="paragraph" w:styleId="Heading6"><w:name w:val="heading 6"/></w:style></w:styles>"#;
    for (name, data) in [
        ("[Content_Types].xml", content_types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("word/document.xml", document.as_bytes()),
        ("word/styles.xml", styles.as_bytes()),
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
}
