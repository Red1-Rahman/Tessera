// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Export command handlers: `export` (see `docs/contracts.md`, section 2).
//!
//! Marshals format export calls to the corresponding core renderers
//! and manages file output vs. clipboard payload creation.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::commands::document::AppState;
use crate::core::docx::writer;
use crate::core::error::{ExportFormat, TesseraError};
use crate::core::export::{markdown, pdf, plaintext};
use crate::core::latex::printer;

use serde::{Deserialize, Serialize};

/// IPC export request payload (`docs/contracts.md`, section 2).
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    /// `None` means return string bytes for clipboard payload,
    /// `Some` means write to the specified disk path.
    pub destination: Option<String>,
}

/// IPC export result payload (`docs/contracts.md`, section 2).
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ExportResult {
    WrittenToFile { path: String },
    ClipboardPayload { format: ExportFormat, content: String },
}

/// Asynchronously renders the current Document Model state into the requested `ExportFormat`.
///
/// Runs off the main thread and releases the session lock before rendering or writing files.
#[tauri::command]
pub async fn export(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, TesseraError> {
    // Acquire session lock briefly to snapshot document state and assets directory.
    // Lock guard drops immediately at the end of this block.
    let (document, assets_dir) = {
        let session = state.lock().map_err(|_| TesseraError::IoError {
            path: "session".to_string(),
            message: "the project session lock was poisoned".to_string(),
        })?;

        let doc = session.document()?.clone();
        let assets = session.assets_dir();
        (doc, assets)
    };

    match request.format {
        ExportFormat::Latex => {
            let content = printer::print_document(&document)?;
            handle_text_output(request.format, content, request.destination)
        }
        ExportFormat::Docx => {
            let bytes = writer::write_document(&document, &assets_dir)?;
            if let Some(path) = request.destination {
                write_bytes_atomically(&path, &bytes)?;
                Ok(ExportResult::WrittenToFile { path })
            } else {
                Err(TesseraError::ExportError {
                    format: ExportFormat::Docx,
                    message: "binary DOCX output cannot be returned as text clipboard payload"
                        .to_string(),
                })
            }
        }
        ExportFormat::Pdf => {
            if let Some(path) = request.destination {
                export_pdf_atomically(&document, &assets_dir, &path)?;
                Ok(ExportResult::WrittenToFile { path })
            } else {
                Err(TesseraError::ExportError {
                    format: ExportFormat::Pdf,
                    message: "binary PDF output cannot be returned as text clipboard payload"
                        .to_string(),
                })
            }
        }
        ExportFormat::Markdown => {
            let content = markdown::render_markdown(&document)?;
            handle_text_output(request.format, content, request.destination)
        }
        ExportFormat::PlainText => {
            let content = plaintext::render_plaintext(&document)?;
            handle_text_output(request.format, content, request.destination)
        }
    }
}

fn handle_text_output(
    format: ExportFormat,
    content: String,
    destination: Option<String>,
) -> Result<ExportResult, TesseraError> {
    if let Some(path) = destination {
        write_bytes_atomically(&path, content.as_bytes())?;
        Ok(ExportResult::WrittenToFile { path })
    } else {
        Ok(ExportResult::ClipboardPayload { format, content })
    }
}

/// Writes byte slice to a temporary file in destination's parent folder,
/// then atomically renames it to replace destination.
fn write_bytes_atomically(dest_path_str: &str, bytes: &[u8]) -> Result<(), TesseraError> {
    let dest_path = Path::new(dest_path_str);
    let temp_path = generate_temp_path(dest_path);

    if let Err(e) = fs::write(&temp_path, bytes) {
        let _ = fs::remove_file(&temp_path);
        return Err(TesseraError::IoError {
            path: dest_path_str.to_string(),
            message: format!("failed to write temporary export file: {e}"),
        });
    }

    if let Err(e) = fs::rename(&temp_path, dest_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(TesseraError::IoError {
            path: dest_path_str.to_string(),
            message: format!("failed to replace target destination file: {e}"),
        });
    }

    Ok(())
}

/// Renders PDF output into a temporary file in destination's parent folder,
/// then atomically renames it to replace destination.
fn export_pdf_atomically(
    document: &crate::core::model::Document,
    assets_dir: &Path,
    dest_path_str: &str,
) -> Result<(), TesseraError> {
    let dest_path = Path::new(dest_path_str);
    let temp_path = generate_temp_path(dest_path);

    let temp_path_str = temp_path.to_string_lossy().to_string();
    if let Err(err) = pdf::export_pdf(document, assets_dir, &temp_path_str) {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }

    if let Err(e) = fs::rename(&temp_path, dest_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(TesseraError::IoError {
            path: dest_path_str.to_string(),
            message: format!("failed to replace target destination PDF file: {e}"),
        });
    }

    Ok(())
}

fn generate_temp_path(dest_path: &Path) -> std::path::PathBuf {
    let parent = dest_path.parent().unwrap_or_else(|| Path::new("."));
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_filename = format!(".tessera_export_tmp_{}_{}", std::process::id(), timestamp);
    parent.join(temp_filename)
}