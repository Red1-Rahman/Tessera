// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Tessera's error surface, exactly as shaped in `docs/contracts.md`,
//! section 2.
//!
//! The type lives in `core/` rather than in `commands/` because every
//! layer returns it: `core/` constructors raise
//! `TesseraError::ValidationError`, and the `commands/` layer re-exports
//! it for the IPC boundary. `commands/` must not be a dependency of
//! `core/`; that would invert the layering described in
//! `docs/architecture.md`, "Crate and module boundaries".

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::core::model::DocVersion;

/// Which editor pane a `TesseraError::ParseError` originated from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Pane {
    Latex,
    Docx,
}

/// A half-open region of a source document, in 1-based line/column
/// coordinates, used to anchor a parse error in the pane's own text.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Every format the `export` command can render (see `docs/contracts.md`,
/// section 2).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExportFormat {
    Latex,
    Docx,
    Pdf,
    Markdown,
    PlainText,
}

/// The single error type crossing every Tessera boundary, tagged with
/// `"kind"` in JSON so the frontend can discriminate without string
/// matching (see `docs/contracts.md`, section 2).
///
/// Variants are constructed through their named fields only; there is no
/// catch-all variant, so a message shown in the UI always corresponds to
/// a specific, real failure kind.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind")]
pub enum TesseraError {
    /// A source document could not be parsed into the Document Model.
    #[error("parse error in the {:?} pane: {}", .pane, .message)]
    ParseError {
        pane: Pane,
        message: String,
        span: Option<SourceSpan>,
    },
    /// A value failed validation at a boundary (constructor, IPC
    /// payload, or file read).
    #[error("validation error in field `{field}`: {message}")]
    ValidationError { field: String, message: String },
    /// A filesystem operation failed.
    #[error("I/O error on {path}: {message}")]
    IoError { path: String, message: String },
    /// Rendering the document to an export format failed.
    #[error("export to {format:?} failed: {message}")]
    ExportError {
        format: ExportFormat,
        message: String,
    },
    /// An edit arrived against a document version that has already been
    /// superseded; it must be discarded, not applied.
    #[error("stale document version: the edit targeted {requested}, current is {current}")]
    StaleVersion {
        requested: DocVersion,
        current: DocVersion,
    },
}

impl TesseraError {
    /// Convenience constructor for the validation failures raised by the
    /// Document Model's own validating constructors.
    ///
    /// `field` names the offending field (for example `"author.name"`),
    /// never the full payload, so the frontend can anchor the message to
    /// the right place.
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        TesseraError::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}-{}:{}",
            self.start_line, self.start_col, self.end_line, self.end_col
        )
    }
}
