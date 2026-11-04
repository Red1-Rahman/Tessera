// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! PDF export implementation (`parallel-task06-B`).
//!
//! Evaluates PDF export via a two-tier strategy:
//! 1. System TeX engine (`pdflatex`, `xelatex`, `lualatex`) discovered on `PATH`.
//! 2. Built-in typesetter fallback using `printpdf` when no TeX engine is present.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use printpdf::{BuiltinFont, Mm, PdfDocument, Pt};
use which::which_in;

use crate::core::error::{ExportFormat, TesseraError};
use crate::core::export::PdfEngine;
use crate::core::latex::printer;
use crate::core::model::{Block, Document, ListKind, OpaqueOrigin};

/// RAII guard to guarantee temporary directory deletion on success or error.
struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Result<Self, TesseraError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let folder_name = format!("{prefix}_{}_{}", std::process::id(), timestamp);
        let path = std::env::temp_dir().join(folder_name);
        fs::create_dir(&path).map_err(|e| TesseraError::IoError {
            path: path.to_string_lossy().to_string(),
            message: format!("failed to create temporary TeX build directory: {e}"),
        })?;
        Ok(Self { path })
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Detects available TeX engines on PATH (or a custom PATH string override for testing).
pub fn detect_tex_engine(path_override: Option<&str>) -> Option<(String, PathBuf)> {
    let engines = ["pdflatex", "xelatex", "lualatex"];
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for engine in engines {
        let result = if let Some(path_str) = path_override {
            which_in(engine, Some(path_str), &cwd)
        } else {
            which_in(engine, std::env::var_os("PATH"), &cwd)
        };
        if let Ok(executable_path) = result {
            return Some((engine.to_string(), executable_path));
        }
    }
    None
}

/// Exports a document to PDF, trying system TeX engines first, then falling back to `printpdf`.
pub fn export_pdf(
    document: &Document,
    assets_dir: &Path,
    destination: &str,
) -> Result<PdfEngine, TesseraError> {
    export_pdf_with_path_override(document, assets_dir, destination, None)
}

/// Export PDF with an optional custom PATH string for testing TeX detection.
pub fn export_pdf_with_path_override(
    document: &Document,
    assets_dir: &Path,
    destination: &str,
    path_override: Option<&str>,
) -> Result<PdfEngine, TesseraError> {
    if let Some((program_name, executable)) = detect_tex_engine(path_override) {
        export_system_tex(
            document,
            assets_dir,
            destination,
            &program_name,
            &executable,
        )?;
        Ok(PdfEngine::SystemTeX {
            program: program_name,
        })
    } else {
        export_builtin_typesetter(document, destination)?;
        Ok(PdfEngine::BuiltIn)
    }
}

fn export_system_tex(
    document: &Document,
    assets_dir: &Path,
    destination: &str,
    engine_name: &str,
    executable: &Path,
) -> Result<(), TesseraError> {
    let guard = TempDirGuard::new("tessera_pdf_build")?;
    let temp_dir = &guard.path;

    if assets_dir.exists() && assets_dir.is_dir() {
        copy_assets(assets_dir, temp_dir)?;
    }

    let tex_source = printer::print_document(document);
    let tex_path = temp_dir.join("main.tex");
    fs::write(&tex_path, &tex_source).map_err(|e| TesseraError::IoError {
        path: tex_path.to_string_lossy().to_string(),
        message: format!("failed to write temporary LaTeX source file: {e}"),
    })?;

    let run_engine = || {
        Command::new(executable)
            .current_dir(temp_dir)
            .stdin(Stdio::null())
            .arg("-no-shell-escape")
            .arg("-interaction=nonstopmode")
            .arg("-halt-on-error")
            .arg(format!("-output-directory={}", temp_dir.to_string_lossy()))
            .arg("main.tex")
            .output()
    };
    let output = run_engine().map_err(|e| TesseraError::ExportError {
        format: ExportFormat::Pdf,
        message: format!("failed to execute TeX engine '{engine_name}': {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(TesseraError::ExportError {
            format: ExportFormat::Pdf,
            message: format!(
                "TeX engine '{engine_name}' failed to compile PDF. Output:\n{stdout}\n{stderr}"
            ),
        });
    }

    // A second pass resolves references and the table of contents.
    let output = run_engine().map_err(|e| TesseraError::ExportError {
        format: ExportFormat::Pdf,
        message: format!("failed to execute TeX engine '{engine_name}' for its second pass: {e}"),
    })?;
    if !output.status.success() {
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        return Err(TesseraError::ExportError {
            format: ExportFormat::Pdf,
            message: format!("TeX engine '{engine_name}' failed on its second pass: {diagnostic}"),
        });
    }

    let generated_pdf = temp_dir.join("main.pdf");
    if !generated_pdf.exists() {
        return Err(TesseraError::ExportError {
            format: ExportFormat::Pdf,
            message: format!(
                "TeX engine '{engine_name}' completed but did not produce expected output main.pdf"
            ),
        });
    }

    fs::copy(&generated_pdf, destination).map_err(|e| TesseraError::IoError {
        path: destination.to_string(),
        message: format!("failed to copy generated PDF to final destination: {e}"),
    })?;

    Ok(())
}

fn copy_assets(source: &Path, destination: &Path) -> Result<(), TesseraError> {
    for entry in fs::read_dir(source).map_err(|error| TesseraError::IoError {
        path: source.display().to_string(),
        message: format!("failed to read project assets: {error}"),
    })? {
        let entry = entry.map_err(|error| TesseraError::IoError {
            path: source.display().to_string(),
            message: format!("failed to enumerate project assets: {error}"),
        })?;
        let target = destination.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| TesseraError::IoError {
            path: entry.path().display().to_string(),
            message: format!("failed to inspect project asset: {error}"),
        })?;
        if kind.is_dir() {
            fs::create_dir_all(&target).map_err(|error| TesseraError::IoError {
                path: target.display().to_string(),
                message: format!("failed to create temporary asset directory: {error}"),
            })?;
            copy_assets(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| TesseraError::IoError {
                path: entry.path().display().to_string(),
                message: format!("failed to copy project asset: {error}"),
            })?;
        }
    }
    Ok(())
}

fn export_builtin_typesetter(document: &Document, destination: &str) -> Result<(), TesseraError> {
    let (doc, page1, layer1) =
        PdfDocument::new("Tessera Document", Mm(210.0), Mm(297.0), "Layer 1");
    let mut current_layer = doc.get_page(page1).get_layer(layer1);

    let font_regular =
        doc.add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| TesseraError::ExportError {
                format: ExportFormat::Pdf,
                message: format!("failed to load built-in Helvetica font: {e}"),
            })?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| TesseraError::ExportError {
            format: ExportFormat::Pdf,
            message: format!("failed to load built-in HelveticaBold font: {e}"),
        })?;
    let fonts = PdfFonts {
        regular: &font_regular,
        bold: &font_bold,
    };

    let left_margin_pt = 56.69; // ~20mm
    let page_width_pt = 595.28; // ~210mm
    let content_width_pt = page_width_pt - (left_margin_pt * 2.0);
    let mut y_pt = 785.0; // ~277mm start top

    if let Some(title) = document.metadata().title() {
        draw_text_line(
            &mut current_layer,
            title,
            20.0,
            &font_bold,
            left_margin_pt,
            &mut y_pt,
            content_width_pt,
        );
        y_pt -= 12.0;
    }

    for current_block in document.body() {
        if y_pt < 70.0 {
            let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
            current_layer = doc.get_page(new_page).get_layer(new_layer);
            y_pt = 785.0;
        }

        render_block(
            &mut current_layer,
            current_block,
            &fonts,
            left_margin_pt,
            &mut y_pt,
            content_width_pt,
        );
    }

    let bytes = doc.save_to_bytes().map_err(|e| TesseraError::ExportError {
        format: ExportFormat::Pdf,
        message: format!("failed to serialize generated PDF document: {e}"),
    })?;
    fs::write(destination, bytes).map_err(|e| TesseraError::IoError {
        path: destination.to_string(),
        message: format!("failed to write output PDF file: {e}"),
    })?;

    Ok(())
}

struct PdfFonts<'a> {
    regular: &'a printpdf::IndirectFontRef,
    bold: &'a printpdf::IndirectFontRef,
}

fn render_block(
    layer: &mut printpdf::PdfLayerReference,
    block: &Block,
    fonts: &PdfFonts<'_>,
    x_pt: f32,
    y_pt: &mut f32,
    content_width_pt: f32,
) {
    match block {
        Block::Heading { level, inline, .. } => {
            let text = flatten_inlines(inline);
            let font_size = match level.depth() {
                1 => 16.0,
                2 => 14.0,
                3 => 12.0,
                _ => 11.0,
            };
            *y_pt -= 6.0;
            draw_text_line(
                layer,
                &text,
                font_size,
                fonts.bold,
                x_pt,
                y_pt,
                content_width_pt,
            );
            *y_pt -= 4.0;
        }
        Block::Paragraph { inline, .. } => {
            let text = flatten_inlines(inline);
            draw_text_line(
                layer,
                &text,
                10.0,
                fonts.regular,
                x_pt,
                y_pt,
                content_width_pt,
            );
            *y_pt -= 4.0;
        }
        Block::List { kind, items, .. } => {
            for (idx, item) in items.iter().enumerate() {
                let prefix = match kind {
                    ListKind::Ordered => format!("({}) ", idx + 1),
                    ListKind::Unordered => "  * ".to_string(),
                };
                let mut inner_text = String::new();
                inner_text.push_str(&flatten_blocks(item));
                draw_text_line(
                    layer,
                    &format!("{prefix}{inner_text}"),
                    10.0,
                    fonts.regular,
                    x_pt + 10.0,
                    y_pt,
                    content_width_pt - 10.0,
                );
            }
            *y_pt -= 4.0;
        }
        Block::Table(table) => {
            let caption = table
                .caption()
                .map(flatten_inlines)
                .unwrap_or_else(|| "Table".to_string());
            draw_text_line(
                layer,
                &format!("Table: {caption}"),
                10.0,
                fonts.bold,
                x_pt,
                y_pt,
                content_width_pt,
            );
            for row in table.rows() {
                let cells = row
                    .iter()
                    .map(|cell| flatten_blocks(cell.blocks()))
                    .collect::<Vec<_>>();
                draw_text_line(
                    layer,
                    &cells.join(" | "),
                    9.0,
                    fonts.regular,
                    x_pt,
                    y_pt,
                    content_width_pt,
                );
            }
            *y_pt -= 4.0;
        }
        Block::Figure(figure) => {
            let caption = figure
                .caption()
                .map(flatten_inlines)
                .unwrap_or_else(|| "Figure".to_string());
            draw_text_line(
                layer,
                &format!("[Figure: {caption} ({})]", figure.asset_path()),
                10.0,
                fonts.bold,
                x_pt,
                y_pt,
                content_width_pt,
            );
            *y_pt -= 4.0;
        }
        Block::Equation(equation) => {
            draw_text_line(
                layer,
                equation.latex_source(),
                10.0,
                fonts.regular,
                x_pt + 20.0,
                y_pt,
                content_width_pt - 20.0,
            );
            *y_pt -= 4.0;
        }
        Block::CodeBlock { text, .. } => {
            for line in text.lines() {
                draw_text_line(
                    layer,
                    line,
                    9.0,
                    fonts.regular,
                    x_pt + 15.0,
                    y_pt,
                    content_width_pt - 15.0,
                );
            }
            *y_pt -= 4.0;
        }
        Block::Quote { blocks, .. } => {
            for child in blocks {
                render_block(
                    layer,
                    child,
                    fonts,
                    x_pt + 15.0,
                    y_pt,
                    content_width_pt - 15.0,
                );
            }
        }
        Block::Opaque(opaque) => {
            let origin = match opaque.origin() {
                OpaqueOrigin::Latex => "LaTeX",
                OpaqueOrigin::Docx => "DOCX",
            };
            draw_text_line(
                layer,
                &format!("[{origin} content omitted: {}]", opaque.summary()),
                9.0,
                fonts.bold,
                x_pt,
                y_pt,
                content_width_pt,
            );
            *y_pt -= 4.0;
        }
    }
}

fn draw_text_line(
    layer: &mut printpdf::PdfLayerReference,
    text: &str,
    font_size: f32,
    font: &printpdf::IndirectFontRef,
    x_pt: f32,
    y_pt: &mut f32,
    max_width_pt: f32,
) {
    let char_width_estimate_pt = font_size * 0.55;
    let max_chars = ((max_width_pt / char_width_estimate_pt).max(10.0)) as usize;

    for line in wrap_text(text, max_chars) {
        layer.use_text(
            &line,
            font_size,
            Mm::from(Pt(x_pt)),
            Mm::from(Pt(*y_pt)),
            font,
        );
        *y_pt -= font_size * 1.25;
    }
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.chars().count() + 1 + word.chars().count() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn flatten_inlines(inlines: &[crate::core::model::Inline]) -> String {
    use crate::core::model::Inline;
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Bold { inline }
            | Inline::Italic { inline }
            | Inline::Underline { inline }
            | Inline::Footnote { inline } => out.push_str(&flatten_inlines(inline)),
            Inline::Code { text } => out.push_str(text),
            Inline::Citation { keys } => out.push_str(&format!("[{}]", keys.join(", "))),
            Inline::CrossRef { label } => out.push_str(&format!("[{label}]")),
            Inline::InlineMath { latex_source } => out.push_str(latex_source),
            Inline::LineBreak => out.push(' '),
        }
    }
    out
}

fn flatten_blocks(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(flatten_block)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn flatten_block(block: &Block) -> String {
    match block {
        Block::Heading { inline, .. } | Block::Paragraph { inline, .. } => flatten_inlines(inline),
        Block::List { items, .. } => items
            .iter()
            .map(|item| flatten_blocks(item))
            .collect::<Vec<_>>()
            .join("; "),
        Block::Table(table) => table
            .rows()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| flatten_blocks(cell.blocks()))
                    .collect::<Vec<_>>()
                    .join(" | ")
            })
            .collect::<Vec<_>>()
            .join("; "),
        Block::Figure(figure) => figure.caption().map(flatten_inlines).unwrap_or_default(),
        Block::Equation(equation) => equation.latex_source().to_string(),
        Block::CodeBlock { text, .. } => text.clone(),
        Block::Quote { blocks, .. } => flatten_blocks(blocks),
        Block::Opaque(opaque) => format!(
            "[{} content omitted: {}]",
            match opaque.origin() {
                OpaqueOrigin::Latex => "LaTeX",
                OpaqueOrigin::Docx => "DOCX",
            },
            opaque.summary()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{copy_assets, TempDirGuard};
    use std::fs;

    #[test]
    fn copied_assets_keep_paths_relative_to_the_printed_latex_source() {
        let temporary = TempDirGuard::new("tessera_pdf_assets_test").unwrap();
        let source = temporary.path.join("source-assets");
        let build = temporary.path.join("build");
        fs::create_dir_all(source.join("figures")).unwrap();
        fs::create_dir_all(&build).unwrap();
        fs::write(source.join("figures").join("plot.png"), b"figure").unwrap();

        copy_assets(&source, &build).unwrap();

        assert_eq!(
            fs::read(build.join("figures").join("plot.png")).unwrap(),
            b"figure"
        );
        assert!(!build.join(".tessera").exists());
    }

    #[test]
    fn temporary_build_directory_is_removed_when_its_guard_drops() {
        let path = {
            let temporary = TempDirGuard::new("tessera_pdf_cleanup_test").unwrap();
            temporary.path.clone()
        };

        assert!(!path.exists());
    }
}
