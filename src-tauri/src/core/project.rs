// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Project directory I/O and the in-memory editing session
//! (`serial-task05`).
//!
//! A Tessera project is a directory holding `main.tex`, `main.docx`, and
//! the derived `.tessera/project.json` bookkeeping file (see
//! `docs/architecture.md`, "Storage model", and `docs/contracts.md`,
//! section 3). The session lives here rather than in `commands/` so the
//! full open/edit/save flow is testable without Tauri or a webview.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::diff::{
    apply_dm_patches, check_base_version, document_sync_warnings, initial_document_warnings,
    DocSyncResult, DocxEditRequest, LatexEditRequest,
};
use crate::core::docx::writer::write_docx_file;
use crate::core::error::{Pane, TesseraError};
use crate::core::latex::{parse_latex, print_latex};
use crate::core::model::{Author, Block, DocMetadata, Document, SyncWarning};

/// The on-disk `project.json` schema version this module reads and
/// writes. Bump it with a migration note in `docs/contracts.md` when the
/// shape changes.
const PROJECT_SCHEMA_VERSION: u32 = 1;

/// Upper bound when restoring `doc_version` from a derived file. A
/// corrupt or hostile counter must not make opening a project spin.
const MAX_RESTORED_VERSION: u64 = 1_000_000;

/// What `open_project` and `new_project` return to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Absolute or user-supplied project directory.
    pub path: String,
    /// The document parsed from `main.tex`.
    pub document: Document,
    pub last_edited_pane: Pane,
    /// Asset paths relative to `.tessera/assets`.
    pub assets: Vec<String>,
    /// Non-fatal problems found while loading, such as a reconstructed
    /// `project.json` or opaque content in `main.tex`.
    pub warnings: Vec<SyncWarning>,
}

/// Input to `new_project`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProjectRequest {
    pub path: String,
    /// Optional initial document title. Defaults to `Untitled`.
    pub title: Option<String>,
}

/// The in-memory state owned by the Tauri command layer for one window.
#[derive(Debug, Default)]
pub struct ProjectSession {
    project: Option<Project>,
}

#[derive(Debug)]
struct Project {
    root: PathBuf,
    document: Document,
    last_edited_pane: Pane,
    assets: BTreeSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectJson {
    schema_version: u32,
    doc_version: u64,
    last_edited_pane: String,
    assets: Vec<String>,
}

impl ProjectSession {
    /// Creates an empty session with no project open.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current document, or a validation error when no project is
    /// open.
    pub fn document(&self) -> Result<&Document, TesseraError> {
        Ok(&self.project_ref()?.document)
    }

    /// The current project state, for callers that need the path or
    /// assets as well as the document.
    pub fn state(&self) -> Result<ProjectState, TesseraError> {
        let project = self.project_ref()?;
        Ok(project_state(project, Vec::new()))
    }

    /// Loads a project directory. `main.tex` is the source of truth; a
    /// missing `project.json` is reconstructed from it and written back
    /// (see `docs/architecture.md`, "Storage model").
    pub fn open_project(&mut self, path: &str) -> Result<ProjectState, TesseraError> {
        let root = project_root(path)?;
        let main_tex = root.join("main.tex");
        let source = fs::read_to_string(&main_tex).map_err(|error| io_error(&main_tex, error))?;
        let mut document = parse_latex(&source)?;
        let mut warnings = initial_document_warnings(&document);

        let mut last_edited_pane = Pane::Latex;
        let mut assets = document_assets(&document);
        let mut reconstructed = true;

        let project_json_path = root.join(".tessera").join("project.json");
        match fs::read_to_string(&project_json_path) {
            Ok(text) => match serde_json::from_str::<ProjectJson>(&text) {
                Ok(saved) if saved.schema_version == PROJECT_SCHEMA_VERSION => {
                    reconstructed = false;
                    last_edited_pane = pane_from_project_json(&saved.last_edited_pane);
                    restore_document_version(&mut document, saved.doc_version, &mut warnings);
                    assets.extend(saved.assets);
                }
                Ok(saved) => {
                    warnings.push(project_warning(format!(
                        "project.json schema_version {} is not supported; reconstructed it from main.tex",
                        saved.schema_version
                    )));
                }
                Err(error) => {
                    warnings.push(project_warning(format!(
                        "project.json could not be read ({error}); reconstructed it from main.tex"
                    )));
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                warnings.push(project_warning(
                    "project.json was missing and has been reconstructed from main.tex",
                ));
            }
            Err(error) => return Err(io_error(&project_json_path, error)),
        }

        copy_assets(&root, &document, &mut warnings);

        let project = Project {
            root,
            document,
            last_edited_pane,
            assets,
        };

        if reconstructed {
            write_project_json(&project)?;
        }

        let state = project_state(&project, warnings);
        self.project = Some(project);
        Ok(state)
    }

    /// Creates a new project directory with a starter `main.tex`, an
    /// initial `main.docx`, and `.tessera/project.json`. Refuses to
    /// overwrite an existing `main.tex`.
    pub fn new_project(
        &mut self,
        request: &NewProjectRequest,
    ) -> Result<ProjectState, TesseraError> {
        let root = project_root(&request.path)?;
        let main_tex = root.join("main.tex");
        if main_tex.exists() {
            return Err(TesseraError::validation(
                "path",
                format!("a project already exists at {}", root.display()),
            ));
        }
        if root.exists() && !root.is_dir() {
            return Err(TesseraError::IoError {
                path: root.display().to_string(),
                message: "project path exists and is not a directory".to_string(),
            });
        }

        let title = request
            .title
            .clone()
            .unwrap_or_else(|| "Untitled".to_string());
        let metadata = DocMetadata::new(
            Some(title),
            Vec::<Author>::new(),
            None,
            Some("article".into()),
            vec![],
        )?;
        let document = Document::new(metadata, Vec::new());
        let source = print_latex(&document);

        fs::create_dir_all(assets_dir(&root)).map_err(|error| io_error(&root, error))?;
        fs::write(&main_tex, source).map_err(|error| io_error(&main_tex, error))?;
        let _ = write_docx_file(&document, root.join("main.docx"), assets_dir(&root))?;

        let project = Project {
            root,
            document,
            last_edited_pane: Pane::Latex,
            assets: BTreeSet::new(),
        };
        write_project_json(&project)?;

        let state = project_state(&project, Vec::new());
        self.project = Some(project);
        Ok(state)
    }

    /// Writes `main.tex`, `main.docx`, and `.tessera/project.json` from
    /// the current document.
    pub fn save_project(&mut self) -> Result<(), TesseraError> {
        let project = self.project_mut()?;
        let document = project.document.clone();
        let main_tex = project.root.join("main.tex");
        fs::write(&main_tex, print_latex(&document)).map_err(|error| io_error(&main_tex, error))?;
        let _ = write_docx_file(
            &document,
            project.root.join("main.docx"),
            assets_dir(&project.root),
        )?;
        project.assets = document_assets(&document);
        write_project_json(project)?;
        Ok(())
    }

    /// Applies a LaTeX pane edit to the current document and returns the
    /// new version plus the DOCX-pane-relevant document state.
    pub fn parse_latex(
        &mut self,
        request: &LatexEditRequest,
    ) -> Result<DocSyncResult, TesseraError> {
        let project = self.project_mut()?;
        check_base_version(project.document.version(), request.base_version)?;
        let parsed = parse_latex(&request.full_source)?;
        let before = project.document.clone();

        project.document.set_metadata(parsed.metadata().clone());
        project.document.set_body(parsed.body().to_vec());
        project
            .document
            .set_trailing_trivia(parsed.trailing_trivia().to_string());
        let new_version = project.document.bump_version();
        project.last_edited_pane = Pane::Latex;

        Ok(DocSyncResult {
            new_version,
            document: project.document.clone(),
            latex_render: request.full_source.clone(),
            warnings: document_sync_warnings(&before, &project.document),
        })
    }

    /// Applies a DOCX pane edit as validated patches and returns the new
    /// version plus the freshly printed LaTeX view.
    pub fn apply_docx_edit(
        &mut self,
        request: &DocxEditRequest,
    ) -> Result<DocSyncResult, TesseraError> {
        let project = self.project_mut()?;
        check_base_version(project.document.version(), request.base_version)?;
        let warnings = apply_dm_patches(&mut project.document, &request.patches)?;
        let new_version = project.document.bump_version();
        project.last_edited_pane = Pane::Docx;

        Ok(DocSyncResult {
            new_version,
            document: project.document.clone(),
            latex_render: print_latex(&project.document),
            warnings,
        })
    }

    fn project_ref(&self) -> Result<&Project, TesseraError> {
        self.project.as_ref().ok_or_else(no_project_error)
    }

    fn project_mut(&mut self) -> Result<&mut Project, TesseraError> {
        self.project.as_mut().ok_or_else(no_project_error)
    }
}

fn no_project_error() -> TesseraError {
    TesseraError::validation("project", "no project is open")
}

fn project_root(path: &str) -> Result<PathBuf, TesseraError> {
    if path.trim().is_empty() {
        return Err(TesseraError::validation(
            "path",
            "project path must not be empty or whitespace-only",
        ));
    }
    Ok(PathBuf::from(path))
}

fn assets_dir(root: &Path) -> PathBuf {
    root.join(".tessera").join("assets")
}

fn project_state(project: &Project, warnings: Vec<SyncWarning>) -> ProjectState {
    ProjectState {
        path: project.root.display().to_string(),
        document: project.document.clone(),
        last_edited_pane: project.last_edited_pane,
        assets: project.assets.iter().cloned().collect(),
        warnings,
    }
}

fn project_warning(message: impl Into<String>) -> SyncWarning {
    SyncWarning {
        message: message.into(),
        affected_path: Vec::new(),
    }
}

fn pane_from_project_json(value: &str) -> Pane {
    match value {
        "docx" => Pane::Docx,
        _ => Pane::Latex,
    }
}

fn pane_to_project_json(pane: Pane) -> &'static str {
    match pane {
        Pane::Latex => "latex",
        Pane::Docx => "docx",
    }
}

fn restore_document_version(
    document: &mut Document,
    saved_version: u64,
    warnings: &mut Vec<SyncWarning>,
) {
    if saved_version > MAX_RESTORED_VERSION {
        warnings.push(project_warning(format!(
            "project.json doc_version {saved_version} is implausibly large; starting a fresh version sequence"
        )));
        return;
    }
    while document.version().value() < saved_version {
        document.bump_version();
    }
}

fn write_project_json(project: &Project) -> Result<(), TesseraError> {
    let path = project.root.join(".tessera").join("project.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    }
    let saved = ProjectJson {
        schema_version: PROJECT_SCHEMA_VERSION,
        doc_version: project.document.version().value(),
        last_edited_pane: pane_to_project_json(project.last_edited_pane).to_string(),
        assets: project.assets.iter().cloned().collect(),
    };
    let text = serde_json::to_string_pretty(&saved).map_err(|error| TesseraError::IoError {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    fs::write(&path, text + "\n").map_err(|error| io_error(&path, error))
}

fn document_assets(document: &Document) -> BTreeSet<String> {
    let mut assets = BTreeSet::new();
    collect_assets(document.body(), &mut assets);
    assets
}

fn collect_assets(blocks: &[Block], assets: &mut BTreeSet<String>) {
    for block in blocks {
        match block {
            Block::Figure(figure) => {
                assets.insert(figure.asset_path().to_string());
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_assets(item, assets);
                }
            }
            Block::Quote { blocks, .. } => collect_assets(blocks, assets),
            Block::Table(table) => {
                for row in table.rows() {
                    for cell in row {
                        collect_assets(cell.blocks(), assets);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Copies images referenced by `main.tex` into `.tessera/assets` when
/// they are still in the project root, per `docs/specs.md`, section 1.7.
/// Missing images are warnings, not failures: the LaTeX source remains
/// openable and the user can restore the file later.
fn copy_assets(root: &Path, document: &Document, warnings: &mut Vec<SyncWarning>) {
    let destination_root = assets_dir(root);
    for asset in document_assets(document) {
        let source = root.join(&asset);
        let destination = destination_root.join(&asset);
        if destination.exists() || source == destination {
            continue;
        }
        if source.is_file() {
            if let Some(parent) = destination.parent() {
                if let Err(error) = fs::create_dir_all(parent) {
                    warnings.push(project_warning(format!(
                        "could not create asset directory for `{asset}`: {error}"
                    )));
                    continue;
                }
            }
            if let Err(error) = fs::copy(&source, &destination) {
                warnings.push(project_warning(format!(
                    "could not copy image asset `{asset}`: {error}"
                )));
            }
        } else {
            warnings.push(project_warning(format!(
                "image asset `{asset}` was not found under the project directory"
            )));
        }
    }
}

fn io_error(path: &Path, error: io::Error) -> TesseraError {
    TesseraError::IoError {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}
