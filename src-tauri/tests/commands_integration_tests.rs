// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tessera_lib::commands::{document, export};
use tessera_lib::core::diff::{DocxEditRequest, LatexEditRequest};
use tessera_lib::core::error::{ExportFormat, TesseraError};
use tessera_lib::core::export::{ExportRequest, ExportResult};
use tessera_lib::core::latex::highlight::HighlightRequest;
use tessera_lib::core::project::{NewProjectRequest, ProjectSession};

type AppState = Mutex<ProjectSession>;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tessera_commands_test_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn empty_session() -> AppState {
    Mutex::new(ProjectSession::new())
}

fn create_and_open_project(session: &AppState, path: &Path) {
    let project_path = path.to_string_lossy().to_string();
    document::new_project_with_session(
        session,
        &NewProjectRequest {
            path: project_path.clone(),
            title: Some("Command integration".to_string()),
        },
    )
    .expect("new_project command succeeds");
    document::open_project_with_session(session, &project_path)
        .expect("open_project command succeeds");
}

#[test]
fn commands_complete_open_edit_save_highlight_and_five_format_export_cycle() {
    let temporary = TestDirectory::new();
    let project_path = temporary.path().join("project");
    let session = empty_session();
    create_and_open_project(&session, &project_path);

    let base_version = session
        .lock()
        .expect("session lock")
        .document()
        .expect("open document")
        .version();
    let source = "\\documentclass{article}\n\\title{Command integration}\n\\begin{document}\n\\section{Edited}\nThe edited content.\n\\end{document}\n";
    document::parse_latex_with_session(
        &session,
        &LatexEditRequest {
            base_version,
            full_source: source.to_string(),
        },
    )
    .expect("parse_latex command succeeds");

    let current_version = session
        .lock()
        .expect("session lock")
        .document()
        .expect("edited document")
        .version();
    document::apply_docx_edit_with_session(
        &session,
        &DocxEditRequest {
            base_version: current_version,
            patches: Vec::new(),
        },
    )
    .expect("apply_docx_edit command succeeds");
    assert!(document::get_highlight_tokens_with_session(
        &session,
        &HighlightRequest {
            source: source.to_string(),
        },
    )
    .is_ok());
    document::save_project_with_session(&session).expect("save_project command succeeds");
    assert!(project_path.join("main.docx").is_file());

    for format in [
        ExportFormat::Latex,
        ExportFormat::Docx,
        ExportFormat::Pdf,
        ExportFormat::Markdown,
        ExportFormat::PlainText,
    ] {
        let output_path = temporary.path().join(format!("output_{format:?}"));
        let result = export::export_with_session(
            &session,
            &ExportRequest {
                format,
                destination: Some(output_path.to_string_lossy().to_string()),
            },
        )
        .expect("export command succeeds for every format");
        assert!(matches!(result, ExportResult::WrittenToFile { .. }));
        assert!(output_path.is_file(), "missing output for {format:?}");
    }

    let clipboard = export::export_with_session(
        &session,
        &ExportRequest {
            format: ExportFormat::Markdown,
            destination: None,
        },
    )
    .expect("text clipboard export succeeds");
    assert!(matches!(
        clipboard,
        ExportResult::ClipboardPayload {
            format: ExportFormat::Markdown,
            content,
        } if content.contains("The edited content.")
    ));
}

#[test]
fn export_command_returns_errors_for_missing_project_invalid_destination_and_binary_clipboard() {
    let temporary = TestDirectory::new();
    let session = empty_session();

    let no_project = export::export_with_session(
        &session,
        &ExportRequest {
            format: ExportFormat::Markdown,
            destination: None,
        },
    );
    assert!(matches!(
        no_project,
        Err(TesseraError::ValidationError { .. })
    ));

    create_and_open_project(&session, &temporary.path().join("project"));
    for format in [ExportFormat::Docx, ExportFormat::Pdf] {
        let clipboard = export::export_with_session(
            &session,
            &ExportRequest {
                format,
                destination: None,
            },
        );
        assert!(matches!(clipboard, Err(TesseraError::ExportError { .. })));
    }

    let empty_destination = export::export_with_session(
        &session,
        &ExportRequest {
            format: ExportFormat::Markdown,
            destination: Some(" \t ".to_string()),
        },
    );
    assert!(matches!(
        empty_destination,
        Err(TesseraError::ValidationError { .. })
    ));
}
