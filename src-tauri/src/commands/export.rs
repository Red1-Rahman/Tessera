// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! IPC export handler. Conversion and file output live in `core/export`.

use tauri::State;

use crate::commands::document::AppState;
use crate::core::error::TesseraError;
use crate::core::export;
pub use crate::core::export::{ExportRequest, ExportResult};

/// Exports a snapshot of the current Document Model.
#[tauri::command]
pub async fn export(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, TesseraError> {
    let (document, assets_dir) = {
        let session = state.lock().map_err(|_| TesseraError::IoError {
            path: "session".to_string(),
            message: "the project session lock was poisoned".to_string(),
        })?;
        (session.document()?.clone(), session.assets_dir())
    };
    export::export_document(&document, &request, &assets_dir)
}
