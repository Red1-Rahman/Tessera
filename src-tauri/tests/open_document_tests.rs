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

use tessera_lib::commands::document;
use tessera_lib::core::diff::{DmPatch, DocxEditRequest, LatexEditRequest};
use tessera_lib::core::error::{ExportFormat, TesseraError};
use tessera_lib::core::export::{ExportRequest, ExportResult};
use tessera_lib::core::model::{Block, Inline};
use tessera_lib::core::project::ProjectSession;

type AppState = Mutex<ProjectSession>;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tessera_open_document_test_{}_{}",
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

const SOURCE: &str = "\\documentclass{article}\n\\begin{document}\n\\section{Loose}\nA standalone file.\n\\end{document}\n";

#[test]
fn open_document_loads_tex_and_save_writes_back_only_that_file() {
    let temporary = TestDirectory::new();
    let file = temporary.path().join("notes.tex");
    fs::write(&file, SOURCE).expect("write fixture");
    let session = empty_session();

    let state = document::open_document_with_session(&session, &file.to_string_lossy())
        .expect("open_document succeeds for .tex");
    assert_eq!(state.path, file.display().to_string());
    assert!(!state.document.body().is_empty());

    let base_version = state.document.version();
    let edited = SOURCE.replace("A standalone file.", "Edited text.");
    document::parse_latex_with_session(
        &session,
        &LatexEditRequest {
            base_version,
            full_source: edited,
        },
    )
    .expect("parse_latex works in file mode");
    document::save_project_with_session(&session).expect("save writes back");

    let saved = fs::read_to_string(&file).expect("read saved file");
    assert!(saved.contains("Edited text."));
    assert!(!temporary.path().join("main.tex").exists());
    assert!(!temporary.path().join("main.docx").exists());
    assert!(!temporary.path().join(".tessera").exists());
}

#[test]
fn open_document_round_trips_docx_exported_by_tessera() {
    let temporary = TestDirectory::new();
    let tex = temporary.path().join("source.tex");
    fs::write(&tex, SOURCE).expect("write fixture");
    let session = empty_session();
    document::open_document_with_session(&session, &tex.to_string_lossy()).expect("open tex");

    let docx = temporary.path().join("loose.docx");
    let result = tessera_lib::commands::export::export_with_session(
        &session,
        &ExportRequest {
            format: ExportFormat::Docx,
            destination: Some(docx.to_string_lossy().to_string()),
        },
    )
    .expect("export docx");
    assert!(matches!(result, ExportResult::WrittenToFile { .. }));

    let state = document::open_document_with_session(&session, &docx.to_string_lossy())
        .expect("open_document succeeds for .docx");
    assert_eq!(state.path, docx.display().to_string());
    assert!(!state.document.body().is_empty());

    let (index, first) = state
        .document
        .body()
        .iter()
        .enumerate()
        .find(|(_, block)| matches!(block, Block::Paragraph { .. }))
        .expect("opened docx contains a paragraph");
    let edited = match first.clone() {
        Block::Paragraph { trivia, .. } => Block::Paragraph {
            inline: vec![Inline::text("Edited DOCX content.".to_string()).expect("valid text")],
            trivia,
        },
        other => panic!("expected paragraph, got {other:?}"),
    };
    document::apply_docx_edit_with_session(
        &session,
        &DocxEditRequest {
            base_version: state.document.version(),
            patches: vec![DmPatch::Replace {
                path: vec![index],
                block: edited,
            }],
        },
    )
    .expect("edit opened docx");
    document::save_project_with_session(&session).expect("save writes docx back");
    let reopened = document::open_document_with_session(&session, &docx.to_string_lossy())
        .expect("reopen saved docx");
    assert!(reopened.document.body().iter().any(|block| {
        matches!(
            block,
            Block::Paragraph { inline, .. }
                if inline.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Edited DOCX content."))
        )
    }));
    assert!(!temporary.path().join(".tessera").exists());
}

#[test]
fn open_document_rejects_blank_path_bad_extension_and_missing_file() {
    let temporary = TestDirectory::new();
    let session = empty_session();

    let blank = document::open_document_with_session(&session, "  ");
    assert!(matches!(blank, Err(TesseraError::ValidationError { .. })));

    let unsupported = temporary.path().join("notes.txt");
    fs::write(&unsupported, "x").expect("write fixture");
    let wrong = document::open_document_with_session(&session, &unsupported.to_string_lossy());
    assert!(matches!(wrong, Err(TesseraError::ValidationError { .. })));

    let missing = temporary.path().join("absent.tex");
    let io = document::open_document_with_session(&session, &missing.to_string_lossy());
    assert!(matches!(io, Err(TesseraError::IoError { .. })));

    let garbage = temporary.path().join("broken.docx");
    fs::write(&garbage, b"not a zip").expect("write fixture");
    let parse = document::open_document_with_session(&session, &garbage.to_string_lossy());
    assert!(matches!(parse, Err(TesseraError::ParseError { .. })));
}
