// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! The Document Model (DM), the one intermediate representation both the
//! LaTeX pane and the DOCX pane are views of (see
//! `docs/architecture.md`, "The Document Model", and
//! `docs/contracts.md`, section 1, which this module implements
//! field-for-field).
//!
//! # Validation discipline
//!
//! Every type here that can be built from untrusted input (parsed LaTeX,
//! parsed DOCX, an IPC payload from the frontend) has a validating
//! constructor returning `Result<T, TesseraError>`; its fields are
//! private so no code path can construct an invalid instance with a
//! struct literal. Deserialization still bypasses constructors by
//! design, so any `Deserialize` result coming from the frontend must be
//! re-validated at the deserialization site (see
//! `.agents/skills/rust-conventions.md`, "Boundary validation").
//!
//! # Round-trip invariant
//!
//! The LaTeX parser/printer pair and the DOCX reader/writer pair must
//! satisfy `parse(print(d)) == d` for every `d` built from first-class
//! nodes (excludes `OpaqueBlock` content, which is format-specific by
//! definition). See `docs/contracts.md`, "Contract testing".

use std::fmt;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::core::error::TesseraError;

/// Validates a cross-reference label: non-empty after trimming and
/// containing no embedded whitespace, since labels are shared
/// cross-reference keys between headings, floats, equations, and
/// `Inline::CrossRef` (see `docs/contracts.md`, "Invariants").
///
/// Returns the trimmed label, which is what callers should store.
pub fn validate_label(field: &str, label: &str) -> Result<String, TesseraError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(TesseraError::validation(
            field,
            "label must not be empty or whitespace-only",
        ));
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(TesseraError::validation(
            field,
            "label must not contain whitespace",
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates an optional human-facing string: `None` and whitespace-only
/// are both rejected as `Some`, and any present value is returned
/// trimmed.
fn validate_optional_text(
    field: &str,
    value: Option<String>,
) -> Result<Option<String>, TesseraError> {
    match value {
        None => Ok(None),
        Some(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(TesseraError::validation(
                    field,
                    "value must not be empty or whitespace-only when present",
                ));
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

/// A single Tessera document: metadata plus a body of blocks. The shared
/// intermediate representation both editor panes render.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    version: DocVersion,
    metadata: DocMetadata,
    body: Vec<Block>,
}

impl Document {
    /// Builds a document at [`DocVersion::first`]. Version increments are
    /// owned by the Document Model itself (see [`DocVersion::next`]);
    /// callers never pick a version.
    pub fn new(metadata: DocMetadata, body: Vec<Block>) -> Self {
        Document {
            version: DocVersion::first(),
            metadata,
            body,
        }
    }

    /// The version this document was last mutated to.
    pub fn version(&self) -> DocVersion {
        self.version
    }

    /// The document's metadata.
    pub fn metadata(&self) -> &DocMetadata {
        &self.metadata
    }

    /// The document body, block order as displayed.
    pub fn body(&self) -> &[Block] {
        &self.body
    }

    /// Mutable access to the body for the sync engine.
    ///
    /// Callers must only replace or reorder blocks with values built
    /// through the constructors in this module; there is no validation
    /// here to catch a struct literal built downstream.
    pub fn body_mut(&mut self) -> &mut Vec<Block> {
        &mut self.body
    }

    /// Replaces the body, keeping the current version. Used by the sync
    /// engine when a pane edit produces a whole new body.
    pub fn set_body(&mut self, body: Vec<Block>) {
        self.body = body;
    }

    /// Advances this document to the next [`DocVersion`]. Returns the new
    /// version for the caller to hand to the frontend.
    pub fn bump_version(&mut self) -> DocVersion {
        self.version = self.version.next();
        self.version
    }
}

/// Monotonically increasing per-session version counter, used to discard
/// stale async parse results (see `docs/architecture.md`, "Sync flow").
///
/// The inner value is private: the only way to obtain a `DocVersion` is
/// [`DocVersion::first`] followed by [`DocVersion::next`], so core code
/// can never issue a version older than one already handed out. JSON
/// representation is the bare number (for example `42`), matching
/// `docs/contracts.md`, section 2 and section 3.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct DocVersion(u64);

impl DocVersion {
    /// The version a fresh session starts at.
    pub const fn first() -> Self {
        DocVersion(0)
    }

    /// The version that follows this one. Saturates instead of wrapping
    /// so a pathological session cannot roll back to a stale number.
    pub const fn next(&self) -> Self {
        DocVersion(self.0.saturating_add(1))
    }

    /// The raw counter value, for display and logging only. Never feed
    /// this back into construction; there is no constructor from it.
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for DocVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Document-level metadata, mapped from the LaTeX preamble's `\title`,
/// `\author`, `\date`, and `\documentclass` (see `docs/specs.md`,
/// sections 1.1 and 1.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DocMetadata {
    title: Option<String>,
    authors: Vec<Author>,
    date: Option<String>,
    latex_class: Option<String>,
    latex_class_options: Vec<String>,
}

impl DocMetadata {
    /// Validates and builds document metadata.
    ///
    /// Optional strings must not be empty when present; class options
    /// must each be a non-empty token. Authors are validated by
    /// [`Author::new`] before they get here.
    pub fn new(
        title: Option<String>,
        authors: Vec<Author>,
        date: Option<String>,
        latex_class: Option<String>,
        latex_class_options: Vec<String>,
    ) -> Result<Self, TesseraError> {
        let title = validate_optional_text("metadata.title", title)?;
        let date = validate_optional_text("metadata.date", date)?;
        let latex_class = validate_optional_text("metadata.latex_class", latex_class)?;
        for (index, option) in latex_class_options.iter().enumerate() {
            if option.trim().is_empty() {
                return Err(TesseraError::validation(
                    format!("metadata.latex_class_options[{index}]"),
                    "class options must not be empty or whitespace-only",
                ));
            }
        }
        Ok(DocMetadata {
            title,
            authors,
            date,
            latex_class,
            latex_class_options,
        })
    }

    /// The document title, if the source declared one.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The document's authors, in declaration order.
    pub fn authors(&self) -> &[Author] {
        &self.authors
    }

    /// The document date, if declared.
    pub fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }

    /// The LaTeX document class (for example `"article"`), preserved so
    /// round-tripping to LaTeX never loses it.
    pub fn latex_class(&self) -> Option<&str> {
        self.latex_class.as_deref()
    }

    /// The document class options, preserved verbatim and uninterpreted
    /// (see `docs/specs.md`, section 1.1).
    pub fn latex_class_options(&self) -> &[String] {
        &self.latex_class_options
    }
}

/// One author, from a `\author` entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Author {
    name: String,
    affiliation: Option<String>,
}

impl Author {
    /// Validates and builds an author. The name must be non-empty after
    /// trimming; an affiliation, when present, must be too.
    pub fn new(name: String, affiliation: Option<String>) -> Result<Self, TesseraError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TesseraError::validation(
                "author.name",
                "author name must not be empty or whitespace-only",
            ));
        }
        let affiliation = validate_optional_text("author.affiliation", affiliation)?;
        Ok(Author {
            name: trimmed.to_string(),
            affiliation,
        })
    }

    /// The author's name, trimmed at construction.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The author's affiliation, if the source gave one.
    pub fn affiliation(&self) -> Option<&str> {
        self.affiliation.as_deref()
    }
}

/// A block-level node: the backbone of the document tree. JSON shape is
/// `{"type": "<variant>", ...fields}`, per `docs/contracts.md`, section
/// 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Block {
    Heading {
        level: HeadingLevel,
        inline: Vec<Inline>,
        label: Option<String>,
    },
    Paragraph {
        inline: Vec<Inline>,
    },
    List {
        kind: ListKind,
        items: Vec<Vec<Block>>,
    },
    Table(Table),
    Figure(Figure),
    Equation(Equation),
    CodeBlock {
        language: Option<String>,
        text: String,
    },
    Quote {
        blocks: Vec<Block>,
    },
    /// Content that only one pane can faithfully represent. See
    /// `docs/architecture.md`, "Conflict and loss handling".
    Opaque(OpaqueBlock),
}

impl Block {
    /// Builds a heading, validating its optional cross-reference label.
    pub fn heading(
        level: HeadingLevel,
        inline: Vec<Inline>,
        label: Option<String>,
    ) -> Result<Self, TesseraError> {
        let label = match label {
            Some(value) => Some(validate_label("heading.label", &value)?),
            None => None,
        };
        Ok(Block::Heading {
            level,
            inline,
            label,
        })
    }

    /// Builds a paragraph.
    pub fn paragraph(inline: Vec<Inline>) -> Self {
        Block::Paragraph { inline }
    }

    /// Builds a list. Items are full block lists so nesting can be
    /// expressed with another `Block::List` inside an item.
    pub fn list(kind: ListKind, items: Vec<Vec<Block>>) -> Self {
        Block::List { kind, items }
    }

    /// Builds a table block from an already-validated [`Table`].
    pub fn table(table: Table) -> Self {
        Block::Table(table)
    }

    /// Builds a figure block from an already-validated [`Figure`].
    pub fn figure(figure: Figure) -> Self {
        Block::Figure(figure)
    }

    /// Builds an equation block from an already-validated [`Equation`].
    pub fn equation(equation: Equation) -> Self {
        Block::Equation(equation)
    }

    /// Builds a fenced code block. A present language tag must be
    /// non-empty after trimming.
    pub fn code_block(language: Option<String>, text: String) -> Result<Self, TesseraError> {
        let language = validate_optional_text("code_block.language", language)?;
        Ok(Block::CodeBlock { language, text })
    }

    /// Builds a block quote wrapping arbitrary block content.
    pub fn quote(blocks: Vec<Block>) -> Self {
        Block::Quote { blocks }
    }

    /// Wraps one-pane-only content in an [`OpaqueBlock`], the DM's
    /// no-silent-loss guarantee (see `docs/contracts.md`, "Invariants").
    pub fn opaque(block: OpaqueBlock) -> Self {
        Block::Opaque(block)
    }
}

/// Heading depth, from the sectioning command found in the source (see
/// `docs/specs.md`, section 1.3).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HeadingLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl HeadingLevel {
    /// The 1-based depth of this heading (H1 is 1, H6 is 6).
    pub const fn depth(self) -> u8 {
        match self {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        }
    }
}

/// Whether a list is numbered or bulleted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ListKind {
    Ordered,
    Unordered,
}

/// A table: rows of cells plus an optional caption and cross-reference
/// label (see `docs/specs.md`, section 1.6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table {
    rows: Vec<Vec<TableCell>>,
    caption: Option<Vec<Inline>>,
    label: Option<String>,
}

impl Table {
    /// Validates and builds a table. The label, when present, must be a
    /// well-formed cross-reference key; every cell is validated through
    /// [`TableCell::new`]'s span rules.
    pub fn new(
        rows: Vec<Vec<TableCell>>,
        caption: Option<Vec<Inline>>,
        label: Option<String>,
    ) -> Result<Self, TesseraError> {
        let label = match label {
            Some(value) => Some(validate_label("table.label", &value)?),
            None => None,
        };
        Ok(Table {
            rows,
            caption,
            label,
        })
    }

    /// The table's rows, each a list of cells.
    pub fn rows(&self) -> &[Vec<TableCell>] {
        &self.rows
    }

    /// The table's caption inlines, if the source gave one.
    pub fn caption(&self) -> Option<&[Inline]> {
        self.caption.as_deref()
    }

    /// The table's cross-reference label, if any.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

/// A single table cell: block content plus span counts (from
/// `\multicolumn` and `\multirow`, see `docs/specs.md`, section 1.6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    blocks: Vec<Block>,
    col_span: u32,
    row_span: u32,
}

impl TableCell {
    /// Validates and builds a cell. Span counts are 1-based: zero would
    /// make the cell disappear from the rendered table, so it is
    /// rejected rather than silently coerced.
    pub fn new(blocks: Vec<Block>, col_span: u32, row_span: u32) -> Result<Self, TesseraError> {
        if col_span == 0 {
            return Err(TesseraError::validation(
                "table_cell.col_span",
                "column span must be at least 1",
            ));
        }
        if row_span == 0 {
            return Err(TesseraError::validation(
                "table_cell.row_span",
                "row span must be at least 1",
            ));
        }
        Ok(TableCell {
            blocks,
            col_span,
            row_span,
        })
    }

    /// The cell's block content.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// How many columns this cell spans.
    pub const fn col_span(&self) -> u32 {
        self.col_span
    }

    /// How many rows this cell spans.
    pub const fn row_span(&self) -> u32 {
        self.row_span
    }
}

/// A figure: an image from the project's assets directory plus an
/// optional caption, label, and width.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Figure {
    asset_path: String,
    caption: Option<Vec<Inline>>,
    label: Option<String>,
    width_percent: Option<f32>,
}

impl Figure {
    /// Validates and builds a figure.
    ///
    /// `asset_path` is relative to the project's `.tessera/assets`
    /// directory, so absolute paths and `..` traversal are rejected:
    /// this string may come from an IPC payload and must not be usable
    /// to point the file loader outside the project. `width_percent`,
    /// when present, must be a finite value in (0, 100].
    pub fn new(
        asset_path: String,
        caption: Option<Vec<Inline>>,
        label: Option<String>,
        width_percent: Option<f32>,
    ) -> Result<Self, TesseraError> {
        let trimmed_path = asset_path.trim();
        if trimmed_path.is_empty() {
            return Err(TesseraError::validation(
                "figure.asset_path",
                "asset path must not be empty or whitespace-only",
            ));
        }
        let path = Path::new(trimmed_path);
        let traverses = path.components().any(|component| {
            matches!(
                component,
                Component::RootDir | Component::Prefix(_) | Component::ParentDir
            )
        });
        if path.is_absolute() || traverses {
            return Err(TesseraError::validation(
                "figure.asset_path",
                "asset path must be relative to .tessera/assets and must not contain '..'",
            ));
        }
        if let Some(width) = width_percent {
            if !width.is_finite() || width <= 0.0 || width > 100.0 {
                return Err(TesseraError::validation(
                    "figure.width_percent",
                    "width must be a finite percentage greater than 0 and at most 100",
                ));
            }
        }
        let label = match label {
            Some(value) => Some(validate_label("figure.label", &value)?),
            None => None,
        };
        Ok(Figure {
            asset_path: trimmed_path.to_string(),
            caption,
            label,
            width_percent,
        })
    }

    /// The image path, relative to `.tessera/assets`.
    pub fn asset_path(&self) -> &str {
        &self.asset_path
    }

    /// The figure caption inlines, if any.
    pub fn caption(&self) -> Option<&[Inline]> {
        self.caption.as_deref()
    }

    /// The figure's cross-reference label, if any.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The requested display width as a percentage of the text column,
    /// if the source specified one.
    pub const fn width_percent(&self) -> Option<f32> {
        self.width_percent
    }
}

/// A display or inline equation. Math is always stored as LaTeX source,
/// the DM's canonical math representation regardless of which pane
/// produced it (see `docs/specs.md`, section 1.7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Equation {
    latex_source: String,
    display_mode: bool,
    label: Option<String>,
}

impl Equation {
    /// Validates and builds an equation. The math source must be
    /// non-empty after trimming; an empty equation would render as
    /// nothing in one pane and could not round-trip.
    pub fn new(
        latex_source: String,
        display_mode: bool,
        label: Option<String>,
    ) -> Result<Self, TesseraError> {
        let trimmed = latex_source.trim();
        if trimmed.is_empty() {
            return Err(TesseraError::validation(
                "equation.latex_source",
                "equation source must not be empty or whitespace-only",
            ));
        }
        let label = match label {
            Some(value) => Some(validate_label("equation.label", &value)?),
            None => None,
        };
        Ok(Equation {
            latex_source: trimmed.to_string(),
            display_mode,
            label,
        })
    }

    /// The equation's LaTeX math source.
    pub fn latex_source(&self) -> &str {
        &self.latex_source
    }

    /// True for display equations (`\[...\]`, `equation` environment),
    /// false for inline math (`$...$`).
    pub const fn display_mode(&self) -> bool {
        self.display_mode
    }

    /// The equation's cross-reference label, if any.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

/// Content that round-trips losslessly through its origin format but has
/// no first-class DM representation. Rendered as a clearly marked,
/// non-editable block in the pane that did not produce it (see
/// `docs/contracts.md`, "Invariants", and `docs/specs.md`'s `Opaque`
/// rules).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpaqueBlock {
    origin: OpaqueOrigin,
    raw: String,
    summary: String,
}

impl OpaqueBlock {
    /// Validates and builds an opaque block. `raw` is preserved
    /// verbatim, so it must be non-empty (an empty opaque block has
    /// nothing to preserve) and `summary` must be non-empty because the
    /// UI shows it to explain what was fenced off.
    pub fn new(origin: OpaqueOrigin, raw: String, summary: String) -> Result<Self, TesseraError> {
        if raw.is_empty() {
            return Err(TesseraError::validation(
                "opaque.raw",
                "opaque content must not be empty",
            ));
        }
        let trimmed_summary = summary.trim();
        if trimmed_summary.is_empty() {
            return Err(TesseraError::validation(
                "opaque.summary",
                "opaque summary must not be empty or whitespace-only",
            ));
        }
        Ok(OpaqueBlock {
            origin,
            raw,
            summary: trimmed_summary.to_string(),
        })
    }

    /// Which pane's format this content came from, and therefore which
    /// pane can still edit it.
    pub const fn origin(&self) -> OpaqueOrigin {
        self.origin
    }

    /// The raw content in its origin format, byte-for-byte.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The one-line description shown in the other pane's view of this
    /// block.
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

/// Which pane produced an [`OpaqueBlock`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpaqueOrigin {
    Latex,
    Docx,
}

/// An inline (span-level) node. JSON shape is
/// `{"type": "<variant>", ...fields}`, per `docs/contracts.md`, section
/// 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Code(String),
    Citation { keys: Vec<String> },
    CrossRef { label: String },
    Footnote(Vec<Inline>),
    InlineMath { latex_source: String },
    LineBreak,
}

impl Inline {
    /// Builds a text run. Empty text is rejected: it renders as nothing
    /// and cannot round-trip back to any source that produced it.
    pub fn text(text: String) -> Result<Self, TesseraError> {
        if text.trim().is_empty() {
            return Err(TesseraError::validation(
                "inline.text",
                "text run must not be empty or whitespace-only",
            ));
        }
        Ok(Inline::Text(text))
    }

    /// Builds a bold span (from `\textbf`, see `docs/specs.md`,
    /// section 1.4).
    pub fn bold(inner: Vec<Inline>) -> Self {
        Inline::Bold(inner)
    }

    /// Builds an italic span (from `\textit`/`\emph`).
    pub fn italic(inner: Vec<Inline>) -> Self {
        Inline::Italic(inner)
    }

    /// Builds an underlined span (from `\underline`).
    pub fn underline(inner: Vec<Inline>) -> Self {
        Inline::Underline(inner)
    }

    /// Builds an inline code run (from `\texttt`/`\verb`). Must be
    /// non-empty, for the same reason as [`Inline::text`].
    pub fn code(text: String) -> Result<Self, TesseraError> {
        if text.is_empty() {
            return Err(TesseraError::validation(
                "inline.code",
                "code run must not be empty",
            ));
        }
        Ok(Inline::Code(text))
    }

    /// Builds a citation group (from `\cite{key1,key2}`). Every key must
    /// be non-empty and whitespace-free; keys are citation database
    /// identifiers, not free text.
    pub fn citation(keys: Vec<String>) -> Result<Self, TesseraError> {
        if keys.is_empty() {
            return Err(TesseraError::validation(
                "inline.citation.keys",
                "citation must have at least one key",
            ));
        }
        let mut validated = Vec::with_capacity(keys.len());
        for (index, key) in keys.iter().enumerate() {
            let trimmed = key.trim();
            if trimmed.is_empty() {
                return Err(TesseraError::validation(
                    format!("inline.citation.keys[{index}]"),
                    "citation key must not be empty or whitespace-only",
                ));
            }
            validated.push(trimmed.to_string());
        }
        Ok(Inline::Citation { keys: validated })
    }

    /// Builds a cross-reference (from `\ref`/`\cref`/`\autoref`), whose
    /// label must match the label of some heading, table, figure, or
    /// equation (see `docs/contracts.md`, "Invariants").
    pub fn cross_ref(label: String) -> Result<Self, TesseraError> {
        let label = validate_label("inline.cross_ref.label", &label)?;
        Ok(Inline::CrossRef { label })
    }

    /// Builds a footnote span (from `\footnote`).
    pub fn footnote(inner: Vec<Inline>) -> Self {
        Inline::Footnote(inner)
    }

    /// Builds inline math (from `$...$`). The math source must be
    /// non-empty after trimming.
    pub fn inline_math(latex_source: String) -> Result<Self, TesseraError> {
        let trimmed = latex_source.trim();
        if trimmed.is_empty() {
            return Err(TesseraError::validation(
                "inline.math.latex_source",
                "inline math source must not be empty or whitespace-only",
            ));
        }
        Ok(Inline::InlineMath {
            latex_source: trimmed.to_string(),
        })
    }

    /// Builds a line break (from `\\`).
    pub fn line_break() -> Self {
        Inline::LineBreak
    }
}
