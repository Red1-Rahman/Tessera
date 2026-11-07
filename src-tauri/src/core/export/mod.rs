// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Document Model export renderers and dispatcher.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::docx::writer;
use crate::core::error::{ExportFormat, TesseraError};
use crate::core::latex::printer;
use crate::core::model::Document;

/// Markdown export renderer.
pub mod markdown;
/// PDF export renderer.
pub mod pdf;
/// Plain text export renderer.
pub mod plaintext;

/// PDF engine selected for an export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum PdfEngine {
    /// A locally installed TeX engine.
    SystemTeX { program: String },
    /// Tessera's limited built-in typesetter.
    BuiltIn,
}

/// Validated export request passed from the IPC layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportRequest {
    /// Requested output format.
    pub format: ExportFormat,
    /// Destination path, or `None` for a text clipboard payload.
    pub destination: Option<String>,
}

impl ExportRequest {
    /// Rejects blank paths and clipboard requests for binary formats.
    pub fn validate(&self) -> Result<(), TesseraError> {
        if self
            .destination
            .as_ref()
            .is_some_and(|path| path.trim().is_empty())
        {
            return Err(TesseraError::validation(
                "export.destination",
                "destination path must not be empty or whitespace-only when present",
            ));
        }
        if self.destination.is_none()
            && matches!(self.format, ExportFormat::Docx | ExportFormat::Pdf)
        {
            return Err(TesseraError::ExportError {
                format: self.format,
                message: "binary output requires a file destination".to_string(),
            });
        }
        Ok(())
    }
}

/// Result returned by the export command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum ExportResult {
    /// Output was written to the requested path.
    WrittenToFile {
        /// Destination path.
        path: String,
        /// Engine used for PDF, when applicable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pdf_engine: Option<PdfEngine>,
    },
    /// Text output suitable for clipboard use.
    ClipboardPayload {
        format: ExportFormat,
        content: String,
    },
}

/// Renders a document through the selected core exporter.
pub fn export_document(
    document: &Document,
    request: &ExportRequest,
    assets_dir: &Path,
) -> Result<ExportResult, TesseraError> {
    request.validate()?;
    let destination = request.destination.as_deref();
    match request.format {
        ExportFormat::Latex => text_output(
            request.format,
            printer::print_document(document),
            destination,
        ),
        ExportFormat::Docx => {
            let output = writer::write_docx(document, assets_dir)?;
            let path = destination.ok_or_else(|| TesseraError::ExportError {
                format: ExportFormat::Docx,
                message: "DOCX output requires a file destination".to_string(),
            })?;
            write_atomically(path, &output.bytes)?;
            Ok(ExportResult::WrittenToFile {
                path: path.to_string(),
                pdf_engine: None,
            })
        }
        ExportFormat::Pdf => {
            let path = destination.ok_or_else(|| TesseraError::ExportError {
                format: ExportFormat::Pdf,
                message: "PDF output requires a file destination".to_string(),
            })?;
            let engine = pdf::export_pdf(document, assets_dir, path)?;
            Ok(ExportResult::WrittenToFile {
                path: path.to_string(),
                pdf_engine: Some(engine),
            })
        }
        ExportFormat::Markdown => text_output(
            request.format,
            markdown::render_markdown(document)?,
            destination,
        ),
        ExportFormat::PlainText => text_output(
            request.format,
            plaintext::render_plaintext(document)?,
            destination,
        ),
    }
}

fn text_output(
    format: ExportFormat,
    content: String,
    destination: Option<&str>,
) -> Result<ExportResult, TesseraError> {
    if let Some(path) = destination {
        write_atomically(path, content.as_bytes())?;
        Ok(ExportResult::WrittenToFile {
            path: path.to_string(),
            pdf_engine: None,
        })
    } else {
        Ok(ExportResult::ClipboardPayload { format, content })
    }
}

fn write_atomically(path: &str, bytes: &[u8]) -> Result<(), TesseraError> {
    let destination = Path::new(path);
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| TesseraError::IoError {
        path: parent.display().to_string(),
        message: format!("failed to create export directory: {error}"),
    })?;
    let name = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let temporary = parent.join(format!(".{name}.tessera-{}.tmp", std::process::id()));
    fs::write(&temporary, bytes).map_err(|error| TesseraError::IoError {
        path: path.to_string(),
        message: format!("failed to write temporary export file: {error}"),
    })?;
    if let Err(error) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(TesseraError::IoError {
            path: path.to_string(),
            message: format!("failed to replace export destination: {error}"),
        });
    }
    Ok(())
}
