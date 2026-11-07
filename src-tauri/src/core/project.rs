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

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::core::diff::{
    apply_dm_patches, check_base_version, document_labels, document_sync_warnings,
    initial_document_warnings, DocSyncResult, DocxEditRequest, LatexEditRequest,
};
use crate::core::docx::reader::read_docx_with_assets;
use crate::core::docx::writer::write_docx_file;
use crate::core::error::{Pane, TesseraError};
use crate::core::latex::highlight::{highlight, HighlightRequest, HighlightToken};
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
    /// `Some` when the session holds a standalone `.tex` or `.docx` file
    /// rather than a project directory (see `ProjectSession::open_document`).
    file: Option<OpenFile>,
}

impl Project {
    /// Directory that `Figure::asset_path` values are resolved against.
    /// Project mode uses `.tessera/assets`. A standalone `.tex` file
    /// resolves images next to the file. A standalone `.docx` file uses
    /// a temporary directory holding extracted media and staged relative
    /// images from the directory containing the file.
    fn assets_dir(&self) -> PathBuf {
        match &self.file {
            None => assets_dir(&self.root),
            Some(OpenFile {
                temp_assets: Some(temp),
                ..
            }) => temp.0.clone(),
            Some(_) => self.root.clone(),
        }
    }
}

/// Which format a standalone file uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileFormat {
    Tex,
    Docx,
}

/// A standalone file opened by `open_document`. Saving writes back to
/// `path` only; no project directory or `.tessera/` folder is created.
#[derive(Debug)]
struct OpenFile {
    path: PathBuf,
    format: FileFormat,
    temp_assets: Option<TempAssets>,
}

/// A temporary directory removed when the session drops it.
#[derive(Debug)]
struct TempAssets(PathBuf);

impl Drop for TempAssets {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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

    /// Returns the current project's asset directory, or an error when no
    /// project is open.
    pub fn assets_dir(&self) -> Result<PathBuf, TesseraError> {
        Ok(self.project_ref()?.assets_dir())
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
    ///
    /// A directory without a readable `main.tex` fails with an
    /// `IoError` whose message names both the missing file and the user
    /// action to take instead of the bare operating system error text.
    pub fn open_project(&mut self, path: &str) -> Result<ProjectState, TesseraError> {
        let root = project_root(path)?;
        let main_tex = root.join("main.tex");
        let source = match fs::read_to_string(&main_tex) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(TesseraError::IoError {
                    path: main_tex.display().to_string(),
                    message: format!(
                        "main.tex was not found in {}. Tessera opens a project from a folder that contains main.tex; use File > Open File... for a standalone .tex or .docx file",
                        root.display()
                    ),
                });
            }
            Err(error) => return Err(io_error(&main_tex, error)),
        };
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
            file: None,
        };

        if reconstructed {
            write_project_json(&project)?;
        }

        let state = project_state(&project, warnings);
        self.project = Some(project);
        Ok(state)
    }

    /// Opens a standalone `.tex` or `.docx` file as a single-file session.
    ///
    /// The file's content becomes the document. `save_project` then writes
    /// back to that same file only: a `.tex` file receives the printed
    /// LaTeX and a `.docx` file receives the written DOCX. No `main.tex`,
    /// `main.docx`, or `.tessera/` directory is created. The returned
    /// `ProjectState.path` is the file path. Opening replaces any active
    /// session, like `open_project`.
    ///
    /// Fails with `ValidationError` on field `path` for a blank path or an
    /// extension other than `tex` or `docx` (case-insensitive), and with
    /// `IoError` or `ParseError` when the file cannot be read or parsed.
    pub fn open_document(&mut self, path: &str) -> Result<ProjectState, TesseraError> {
        if path.trim().is_empty() {
            return Err(TesseraError::validation(
                "path",
                "file path must not be empty or whitespace-only",
            ));
        }
        let file_path = PathBuf::from(path);
        let extension = file_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        let format = match extension.as_deref() {
            Some("tex") => FileFormat::Tex,
            Some("docx") => FileFormat::Docx,
            _ => {
                return Err(TesseraError::validation(
                    "path",
                    "only .tex and .docx files can be opened",
                ))
            }
        };
        let root = file_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let mut warnings = Vec::new();
        let (document, last_edited_pane, temp_assets) = match format {
            FileFormat::Tex => {
                let source =
                    fs::read_to_string(&file_path).map_err(|error| io_error(&file_path, error))?;
                (parse_latex(&source)?, Pane::Latex, None)
            }
            FileFormat::Docx => {
                let bytes = fs::read(&file_path).map_err(|error| io_error(&file_path, error))?;
                let result = read_docx_with_assets(&bytes)?;
                let temp = extract_docx_assets(&result.assets)?;
                (result.document, Pane::Docx, temp)
            }
        };
        warnings.extend(initial_document_warnings(&document));

        let project = Project {
            root,
            assets: document_assets(&document),
            document,
            last_edited_pane,
            file: Some(OpenFile {
                path: file_path,
                format,
                temp_assets,
            }),
        };
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
            file: None,
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
        if let Some(file) = &project.file {
            match file.format {
                FileFormat::Tex => {
                    write_file_replacing(&file.path, print_latex(&document).as_bytes())?
                }
                FileFormat::Docx => {
                    let assets_dir = project.assets_dir();
                    stage_relative_docx_assets(project, &document, &assets_dir)?;
                    let _ = write_docx_file(&document, &file.path, assets_dir)?;
                }
            }
            project.assets = document_assets(&document);
            return Ok(());
        }
        let main_tex = project.root.join("main.tex");
        fs::write(&main_tex, print_latex(&document)).map_err(|error| io_error(&main_tex, error))?;
        let _ = write_docx_file(
            &document,
            project.root.join("main.docx"),
            project.assets_dir(),
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

    /// Classifies Okabe-Ito tokens for a LaTeX pane range, using the
    /// current document's labels to resolve cross-references.
    pub fn get_highlight_tokens(
        &self,
        request: &HighlightRequest,
    ) -> Result<Vec<HighlightToken>, TesseraError> {
        let document = self.document()?;
        highlight(&request.source, &document_labels(document))
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
    let path = match &project.file {
        Some(file) => file.path.display().to_string(),
        None => project.root.display().to_string(),
    };
    ProjectState {
        path,
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

/// Writes media extracted from a DOCX into a fresh temporary directory so
/// the DOCX writer can re-embed it on save. Only relative paths made of
/// plain components are written; anything else is skipped with a warning.
fn extract_docx_assets(
    assets: &HashMap<String, Vec<u8>>,
) -> Result<Option<TempAssets>, TesseraError> {
    if assets.is_empty() {
        return Ok(None);
    }
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "tessera_docx_assets_{}_{}",
        std::process::id(),
        unique
    ));
    if let Err(error) = fs::create_dir_all(&root) {
        return Err(io_error(&root, error));
    }
    let temp = TempAssets(root);
    for (name, bytes) in assets {
        let relative = Path::new(name);
        let is_plain = relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
        if !is_plain {
            return Err(TesseraError::validation(
                "docx.media",
                format!("DOCX image `{name}` is not a plain relative path"),
            ));
        }
        let destination = temp.0.join(relative);
        let written = destination
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&destination, bytes));
        if let Err(error) = written {
            return Err(io_error(&destination, error));
        }
    }
    Ok(Some(temp))
}

fn stage_relative_docx_assets(
    project: &Project,
    document: &Document,
    destination_root: &Path,
) -> Result<(), TesseraError> {
    for asset in document_assets(document) {
        let destination = destination_root.join(&asset);
        if destination.exists() {
            continue;
        }
        let source = project.root.join(&asset);
        if !source.is_file() {
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
        }
        fs::copy(&source, &destination).map_err(|error| io_error(&destination, error))?;
    }
    Ok(())
}

fn write_file_replacing(path: &Path, contents: &[u8]) -> Result<(), TesseraError> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let temporary = path.with_extension(format!(
        "{}.tessera-tmp-{}-{}",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("tmp"),
        std::process::id(),
        unique
    ));
    let write_result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| file.write_all(contents));
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(path, error));
    }
    if fs::rename(&temporary, path).is_err() {
        let replacement = fs::remove_file(path).and_then(|()| fs::rename(&temporary, path));
        if let Err(replacement_error) = replacement {
            let _ = fs::remove_file(&temporary);
            return Err(io_error(path, replacement_error));
        }
    }
    Ok(())
}

fn io_error(path: &Path, error: io::Error) -> TesseraError {
    TesseraError::IoError {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}
