// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Integration tests for the sync engine and project lifecycle
//! (`serial-task05`).
//!
//! These call the core session directly, not the Tauri command wrappers,
//! per `docs/conventions.md`'s testing policy.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tessera_lib::core::diff::{DmPatch, DocxEditRequest, LatexEditRequest};
use tessera_lib::core::error::TesseraError;
use tessera_lib::core::model::{Block, Inline};
use tessera_lib::core::project::{NewProjectRequest, ProjectSession};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tessera-sync-{label}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temporary project directory");
        TempDir { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn path_string(&self) -> String {
        self.path.display().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn source_with_heading() -> &'static str {
    "\\documentclass{article}\n\
     \\title{Paper}\n\
     \\begin{document}\n\
     \\section{Intro}\\label{sec:intro}\n\
     First paragraph.\n\
     \\end{document}\n"
}

fn new_session(temp: &TempDir) -> ProjectSession {
    let mut session = ProjectSession::new();
    session
        .new_project(&NewProjectRequest {
            path: temp.path_string(),
            title: Some("Paper".into()),
        })
        .expect("new project");
    session
}

fn parse(
    session: &mut ProjectSession,
    base: u64,
    source: &str,
) -> tessera_lib::core::diff::DocSyncResult {
    let current = session.document().expect("open document").version().value();
    assert_eq!(current, base, "test expected a different starting version");
    session
        .parse_latex(&LatexEditRequest {
            base_version: session.document().expect("open document").version(),
            full_source: source.to_string(),
        })
        .expect("LaTeX edit applies")
}

#[test]
fn new_project_creates_the_documented_project_files() {
    let temp = TempDir::new("new");
    let mut session = ProjectSession::new();
    let state = session
        .new_project(&NewProjectRequest {
            path: temp.path_string(),
            title: Some("Starter Paper".into()),
        })
        .expect("new project");

    assert_eq!(state.path, temp.path_string());
    assert_eq!(state.document.metadata().title(), Some("Starter Paper"));
    assert_eq!(state.document.version().value(), 0);
    assert!(temp.path().join("main.tex").is_file());
    assert!(temp.path().join("main.docx").is_file());
    assert!(temp.path().join(".tessera").join("project.json").is_file());

    let saved: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp.path().join(".tessera").join("project.json"))
            .expect("read project.json"),
    )
    .expect("parse project.json");
    assert_eq!(saved["schema_version"], 1);
    assert_eq!(saved["doc_version"], 0);
    assert_eq!(saved["last_edited_pane"], "latex");
}

#[test]
fn open_project_reconstructs_missing_project_json() {
    let temp = TempDir::new("reconstruct");
    fs::write(temp.path().join("main.tex"), source_with_heading()).expect("write main.tex");

    let mut session = ProjectSession::new();
    let state = session
        .open_project(&temp.path_string())
        .expect("open project");

    assert_eq!(state.document.metadata().title(), Some("Paper"));
    assert!(state
        .warnings
        .iter()
        .any(|warning| warning.message.contains("project.json was missing")));
    assert!(temp.path().join(".tessera").join("project.json").is_file());

    let saved: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp.path().join(".tessera").join("project.json"))
            .expect("read reconstructed project.json"),
    )
    .expect("parse reconstructed project.json");
    assert_eq!(saved["last_edited_pane"], "latex");

    let mut reopened = ProjectSession::new();
    let reopened_state = reopened.open_project(&temp.path_string()).expect("reopen");
    assert!(!reopened_state
        .warnings
        .iter()
        .any(|warning| warning.message.contains("project.json was missing")));
}

#[test]
fn open_project_without_main_tex_fails_with_a_clear_error() {
    let temp = TempDir::new("empty_project");
    fs::create_dir_all(temp.path().join("project")).expect("create empty project dir");

    let mut session = ProjectSession::new();
    let error = session
        .open_project(&temp.path().join("project").display().to_string())
        .expect_err("open_project without main.tex fails");

    match error {
        TesseraError::IoError { path, message } => {
            assert!(path.ends_with("main.tex"), "error path is main.tex: {path}");
            assert!(
                message.contains("main.tex was not found"),
                "message: {message}"
            );
            assert!(
                message.contains("Open File"),
                "message points at the standalone-file alternative: {message}"
            );
        }
        _ => panic!("missing main.tex is an IoError, got {error:?}"),
    }
}

#[test]
fn stale_versions_are_rejected_for_both_edit_paths() {
    let temp = TempDir::new("stale");
    let mut session = new_session(&temp);
    let result = parse(&mut session, 0, source_with_heading());
    assert_eq!(result.new_version.value(), 1);

    let stale_latex = session.parse_latex(&LatexEditRequest {
        base_version: tessera_lib::core::model::DocVersion::first(),
        full_source: source_with_heading().to_string(),
    });
    match stale_latex {
        Err(TesseraError::StaleVersion { requested, current }) => {
            assert_eq!(requested.value(), 0);
            assert_eq!(current.value(), 1);
        }
        other => panic!("expected StaleVersion, got {other:?}"),
    }

    let stale_docx = session.apply_docx_edit(&DocxEditRequest {
        base_version: tessera_lib::core::model::DocVersion::first(),
        patches: vec![DmPatch::Remove {
            path: Vec::new(),
            index: 0,
        }],
    });
    assert!(matches!(stale_docx, Err(TesseraError::StaleVersion { .. })));

    let future = session.parse_latex(&LatexEditRequest {
        base_version: session.document().expect("document").version().next(),
        full_source: source_with_heading().to_string(),
    });
    assert!(matches!(future, Err(TesseraError::ValidationError { .. })));
}

#[test]
fn alternating_pane_edits_keep_one_consistent_document() {
    let temp = TempDir::new("alternating");
    let mut session = new_session(&temp);

    let first = parse(&mut session, 0, source_with_heading());
    assert_eq!(first.new_version.value(), 1);
    assert!(matches!(first.document.body()[0], Block::Heading { .. }));

    let version = first.new_version;
    let second = session
        .apply_docx_edit(&DocxEditRequest {
            base_version: version,
            patches: vec![DmPatch::Replace {
                path: vec![1],
                block: Block::paragraph(vec![
                    Inline::text("Edited in the DOCX pane.".into()).expect("text")
                ]),
            }],
        })
        .expect("DOCX edit applies");
    assert_eq!(second.new_version.value(), 2);
    assert!(second.latex_render.contains("Edited in the DOCX pane."));

    let third = session
        .parse_latex(&LatexEditRequest {
            base_version: second.new_version,
            full_source: "\\documentclass{article}\n\
                         \\title{Paper}\n\
                         \\begin{document}\n\
                         \\section{Intro}\\label{sec:intro}\n\
                         Edited in the DOCX pane.\n\
                         \n\
                         Added in LaTeX.\n\
                         \\end{document}\n"
                .to_string(),
        })
        .expect("second LaTeX edit applies");
    assert_eq!(third.new_version.value(), 3);
    assert_eq!(third.document.body().len(), 3);

    let fourth = session
        .apply_docx_edit(&DocxEditRequest {
            base_version: third.new_version,
            patches: vec![DmPatch::Remove {
                path: Vec::new(),
                index: 2,
            }],
        })
        .expect("DOCX removal applies");
    assert_eq!(fourth.new_version.value(), 4);
    assert_eq!(fourth.document.body().len(), 2);
    assert!(!fourth.latex_render.contains("Added in LaTeX."));
}

#[test]
fn loss_prone_edits_produce_warnings_instead_of_silent_loss() {
    let temp = TempDir::new("warnings");
    let mut session = new_session(&temp);
    let parsed = parse(&mut session, 0, source_with_heading());

    let removed_label = session
        .apply_docx_edit(&DocxEditRequest {
            base_version: parsed.new_version,
            patches: vec![DmPatch::Replace {
                path: vec![0],
                block: Block::paragraph(
                    vec![Inline::text("Heading is gone".into()).expect("text")],
                ),
            }],
        })
        .expect("replacement applies");
    assert!(removed_label
        .warnings
        .iter()
        .any(|warning| warning.message.contains("cross-reference label")));

    let opaque = session
        .parse_latex(&LatexEditRequest {
            base_version: removed_label.new_version,
            full_source: "\\documentclass{article}\n\
                         \\begin{document}\n\
                         \\custommacro{still here}\n\
                         \\end{document}\n"
                .to_string(),
        })
        .expect("opaque source parses");
    assert!(matches!(opaque.document.body()[0], Block::Opaque(_)));
    assert!(opaque
        .warnings
        .iter()
        .any(|warning| warning.message.contains("opaque block")));
}

#[test]
fn save_writes_current_content_and_reopen_restores_the_version() {
    let temp = TempDir::new("save");
    let mut session = new_session(&temp);
    let edited = parse(&mut session, 0, source_with_heading());
    session.save_project().expect("save project");

    let saved_tex = fs::read_to_string(temp.path().join("main.tex")).expect("read main.tex");
    assert!(saved_tex.contains("First paragraph."));
    let docx = fs::read(temp.path().join("main.docx")).expect("read main.docx");
    assert_eq!(&docx[..2], b"PK");

    let saved: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp.path().join(".tessera").join("project.json"))
            .expect("read project.json"),
    )
    .expect("parse project.json");
    assert_eq!(saved["doc_version"], edited.new_version.value());
    assert_eq!(saved["last_edited_pane"], "latex");

    let mut reopened = ProjectSession::new();
    let state = reopened.open_project(&temp.path_string()).expect("reopen");
    assert_eq!(state.document.version(), edited.new_version);
    assert_eq!(state.document.body(), edited.document.body());
}

#[test]
fn nested_patch_paths_are_rejected_without_mutating_the_document() {
    let temp = TempDir::new("paths");
    let mut session = new_session(&temp);
    let parsed = parse(&mut session, 0, source_with_heading());
    let before = parsed.document.clone();

    let result = session.apply_docx_edit(&DocxEditRequest {
        base_version: parsed.new_version,
        patches: vec![DmPatch::Remove {
            path: vec![0, 1],
            index: 0,
        }],
    });
    assert!(matches!(result, Err(TesseraError::ValidationError { .. })));
    assert_eq!(session.document().expect("document").body(), before.body());
}
