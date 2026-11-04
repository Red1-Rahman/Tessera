// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Document and project commands: `open_project`, `new_project`,
//! `parse_latex`, `apply_docx_edit`, and `save_project` (see
//! `docs/contracts.md`, section 2).
//!
//! Every handler is a few lines of marshaling around
//! [`ProjectSession`]; the edit, version, and file logic lives in
//! `core/` so it is testable without Tauri.

use std::sync::{Mutex, MutexGuard};

use tauri::State;

use crate::core::diff::{DocSyncResult, DocxEditRequest, LatexEditRequest};
use crate::core::error::TesseraError;
use crate::core::latex::highlight::{HighlightRequest, HighlightToken};
use crate::core::project::{NewProjectRequest, ProjectSession, ProjectState};

/// Tauri-managed state for one application window.
pub type AppState = Mutex<ProjectSession>;

fn lock_session<'a>(session: &'a AppState) -> Result<MutexGuard<'a, ProjectSession>, TesseraError> {
    session.lock().map_err(|_| TesseraError::IoError {
        path: "session".to_string(),
        message: "the project session lock was poisoned".to_string(),
    })
}

/// Loads a project directory and returns its initial document.
#[tauri::command]
pub fn open_project(
    state: State<'_, AppState>,
    path: String,
) -> Result<ProjectState, TesseraError> {
    open_project_with_session(&state, &path)
}

/// Opens a project using the session shared with the Tauri command handler.
pub fn open_project_with_session(
    session: &AppState,
    path: &str,
) -> Result<ProjectState, TesseraError> {
    lock_session(session)?.open_project(path)
}

/// Creates a new project directory with a starter `main.tex`.
#[tauri::command]
pub fn new_project(
    state: State<'_, AppState>,
    request: NewProjectRequest,
) -> Result<ProjectState, TesseraError> {
    new_project_with_session(&state, &request)
}

/// Creates a project using the session shared with the Tauri command handler.
pub fn new_project_with_session(
    session: &AppState,
    request: &NewProjectRequest,
) -> Result<ProjectState, TesseraError> {
    lock_session(session)?.new_project(request)
}

/// Applies a LaTeX pane edit and returns the updated document.
#[tauri::command]
pub fn parse_latex(
    state: State<'_, AppState>,
    request: LatexEditRequest,
) -> Result<DocSyncResult, TesseraError> {
    parse_latex_with_session(&state, &request)
}

/// Applies a LaTeX edit using the session shared with the Tauri command handler.
pub fn parse_latex_with_session(
    session: &AppState,
    request: &LatexEditRequest,
) -> Result<DocSyncResult, TesseraError> {
    lock_session(session)?.parse_latex(request)
}

/// Applies DOCX pane patches and returns the refreshed LaTeX render.
#[tauri::command]
pub fn apply_docx_edit(
    state: State<'_, AppState>,
    request: DocxEditRequest,
) -> Result<DocSyncResult, TesseraError> {
    apply_docx_edit_with_session(&state, &request)
}

/// Applies DOCX patches using the session shared with the Tauri command handler.
pub fn apply_docx_edit_with_session(
    session: &AppState,
    request: &DocxEditRequest,
) -> Result<DocSyncResult, TesseraError> {
    lock_session(session)?.apply_docx_edit(request)
}

/// Writes `main.tex`, `main.docx`, and `.tessera/project.json`.
#[tauri::command]
pub fn save_project(state: State<'_, AppState>) -> Result<(), TesseraError> {
    save_project_with_session(&state)
}

/// Saves the current project using the session shared with the Tauri command handler.
pub fn save_project_with_session(session: &AppState) -> Result<(), TesseraError> {
    lock_session(session)?.save_project()
}

/// Returns Okabe-Ito token categories for a LaTeX source range.
#[tauri::command]
pub fn get_highlight_tokens(
    state: State<'_, AppState>,
    request: HighlightRequest,
) -> Result<Vec<HighlightToken>, TesseraError> {
    get_highlight_tokens_with_session(&state, &request)
}

/// Highlights source using the session shared with the Tauri command handler.
pub fn get_highlight_tokens_with_session(
    session: &AppState,
    request: &HighlightRequest,
) -> Result<Vec<HighlightToken>, TesseraError> {
    lock_session(session)?.get_highlight_tokens(request)
}
