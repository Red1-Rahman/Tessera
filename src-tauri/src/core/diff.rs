// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Document Model diffing, patch application, and sync loss detection
//! (`serial-task05`).
//!
//! The frontend never receives an independently rendered LaTeX and DOCX
//! view. A pane edit updates one [`Document`] here, and the resulting
//! document plus a block-level diff describe what the other pane needs
//! to refresh. See `docs/architecture.md`, "Sync flow", and
//! `docs/contracts.md`, section 2.
//!
//! # Patch paths
//!
//! `DmPatch` paths address top-level blocks in this version:
//! `Replace` uses a one-element path, and `Insert`/`Remove` use an empty
//! path plus their `index` field. Nested list items, quotes, and table
//! cells are not patched directly yet. A non-empty `Insert`/`Remove`
//! path, or a `Replace` path of any other length, is rejected with
//! `TesseraError::ValidationError` rather than guessed at.

use serde::{Deserialize, Serialize};

use crate::core::error::TesseraError;
use crate::core::model::{
    Block, DocVersion, Document, Inline, OpaqueBlock, SyncWarning, Table, TableCell, TableStyle,
};

/// A LaTeX pane edit: the full source after the edit, checked against the
/// version the frontend last saw (`docs/contracts.md`, section 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatexEditRequest {
    pub base_version: DocVersion,
    pub full_source: String,
}

/// A DOCX pane edit, expressed as Document Model patches rather than a
/// full serialized document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxEditRequest {
    pub base_version: DocVersion,
    pub patches: Vec<DmPatch>,
}

/// One Document Model mutation from the DOCX pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum DmPatch {
    /// Replace the top-level block at `path[0]`.
    Replace { path: Vec<usize>, block: Block },
    /// Insert `block` at `index` in the document body. `path` must be
    /// empty in this version.
    Insert {
        path: Vec<usize>,
        index: usize,
        block: Block,
    },
    /// Remove the body block at `index`. `path` must be empty in this
    /// version.
    Remove { path: Vec<usize>, index: usize },
}

/// The successful result of either pane applying an edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSyncResult {
    pub new_version: DocVersion,
    pub document: Document,
    /// LaTeX source for the LaTeX pane. For a LaTeX-origin edit this is
    /// the source the user typed, not a reformatted print.
    pub latex_render: String,
    pub warnings: Vec<SyncWarning>,
}

/// What happened to one body slot between two documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockChangeKind {
    Added,
    Removed,
    Replaced,
}

/// One changed block slot in a [`DocumentDiff`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockChange {
    pub index: usize,
    pub kind: BlockChangeKind,
}

/// The minimal set of body slots a receiving pane needs to update.
///
/// This is internal to the core for now. `DocSyncResult` already carries
/// the complete document, and `docs/contracts.md` does not expose a diff
/// field, so adding one would be a contract change without a current
/// consumer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentDiff {
    pub metadata_changed: bool,
    pub trailing_trivia_changed: bool,
    pub blocks: Vec<BlockChange>,
}

impl DocumentDiff {
    /// True when metadata, trailing trivia, and every body slot match.
    pub fn is_empty(&self) -> bool {
        !self.metadata_changed && !self.trailing_trivia_changed && self.blocks.is_empty()
    }
}

/// Compares two documents slot by slot. Frontend edits in the supported
/// subset are localized, so index-aligned comparison identifies the
/// changed region without re-rendering the whole pane.
pub fn diff_documents(old: &Document, new: &Document) -> DocumentDiff {
    let mut diff = DocumentDiff {
        metadata_changed: old.metadata() != new.metadata(),
        trailing_trivia_changed: old.trailing_trivia() != new.trailing_trivia(),
        blocks: Vec::new(),
    };

    let shared = old.body().len().min(new.body().len());
    for index in 0..shared {
        if old.body()[index] != new.body()[index] {
            diff.blocks.push(BlockChange {
                index,
                kind: BlockChangeKind::Replaced,
            });
        }
    }
    for index in shared..new.body().len() {
        diff.blocks.push(BlockChange {
            index,
            kind: BlockChangeKind::Added,
        });
    }
    for index in shared..old.body().len() {
        diff.blocks.push(BlockChange {
            index,
            kind: BlockChangeKind::Removed,
        });
    }

    diff
}

/// Rejects an edit that targets a version the core has already moved
/// past. A request newer than the current document is also rejected, as
/// validation failure rather than stale, since it cannot come from this
/// session.
pub fn check_base_version(current: DocVersion, requested: DocVersion) -> Result<(), TesseraError> {
    if requested < current {
        return Err(TesseraError::StaleVersion { requested, current });
    }
    if requested > current {
        return Err(TesseraError::validation(
            "base_version",
            format!("edit targets future document version {requested}, current is {current}"),
        ));
    }
    Ok(())
}

/// Re-builds an untrusted block through the Document Model constructors.
///
/// `Block` derives `Deserialize` for IPC, which bypasses validating
/// constructors by design. Every block arriving in a `DmPatch` goes
/// through this function before it can enter the document (`docs/
/// contracts.md`'s validation discipline).
pub fn validate_block(block: Block) -> Result<Block, TesseraError> {
    match block {
        Block::Heading {
            level,
            inline,
            label,
            trivia,
        } => {
            let mut rebuilt = Block::heading(level, validate_inlines(inline)?, label)?;
            set_trivia(&mut rebuilt, trivia);
            Ok(rebuilt)
        }
        Block::Paragraph { inline, trivia } => {
            let mut rebuilt = Block::paragraph(validate_inlines(inline)?);
            set_trivia(&mut rebuilt, trivia);
            Ok(rebuilt)
        }
        Block::List {
            kind,
            items,
            trivia,
        } => {
            let mut validated_items = Vec::with_capacity(items.len());
            for item in items {
                let mut blocks = Vec::with_capacity(item.len());
                for child in item {
                    blocks.push(validate_block(child)?);
                }
                validated_items.push(blocks);
            }
            let mut rebuilt = Block::list(kind, validated_items);
            set_trivia(&mut rebuilt, trivia);
            Ok(rebuilt)
        }
        Block::Table(table) => {
            let caption = table
                .caption()
                .map(|inline| validate_inlines(inline.to_vec()))
                .transpose()?;
            let mut rows = Vec::with_capacity(table.rows().len());
            for row in table.rows() {
                let mut cells = Vec::with_capacity(row.len());
                for cell in row {
                    let mut blocks = Vec::with_capacity(cell.blocks().len());
                    for child in cell.blocks() {
                        blocks.push(validate_block(child.clone())?);
                    }
                    cells.push(TableCell::new(blocks, cell.col_span(), cell.row_span())?);
                }
                rows.push(cells);
            }
            let mut rebuilt = Table::new(rows, caption, table.label().map(str::to_string))?;
            rebuilt.set_leading_trivia(table.leading_trivia().to_string());
            rebuilt.set_latex_style(table.latex_style().map(validate_table_style));
            Ok(Block::table(rebuilt))
        }
        Block::Figure(figure) => {
            let caption = figure
                .caption()
                .map(|inline| validate_inlines(inline.to_vec()))
                .transpose()?;
            let mut rebuilt = crate::core::model::Figure::new(
                figure.asset_path().to_string(),
                caption,
                figure.label().map(str::to_string),
                figure.width_percent(),
            )?;
            rebuilt.set_leading_trivia(figure.leading_trivia().to_string());
            Ok(Block::figure(rebuilt))
        }
        Block::Equation(equation) => {
            let mut rebuilt = crate::core::model::Equation::new(
                equation.latex_source().to_string(),
                equation.display_mode(),
                equation.label().map(str::to_string),
            )?;
            rebuilt.set_leading_trivia(equation.leading_trivia().to_string());
            Ok(Block::equation(rebuilt))
        }
        Block::CodeBlock {
            language,
            text,
            trivia,
        } => {
            let mut rebuilt = Block::code_block(language, text)?;
            set_trivia(&mut rebuilt, trivia);
            Ok(rebuilt)
        }
        Block::Quote { blocks, trivia } => {
            let mut validated = Vec::with_capacity(blocks.len());
            for child in blocks {
                validated.push(validate_block(child)?);
            }
            let mut rebuilt = Block::quote(validated);
            set_trivia(&mut rebuilt, trivia);
            Ok(rebuilt)
        }
        Block::Opaque(opaque) => Ok(Block::opaque(OpaqueBlock::new(
            opaque.origin(),
            opaque.raw().to_string(),
            opaque.summary().to_string(),
        )?)),
    }
}

fn validate_inlines(inlines: Vec<Inline>) -> Result<Vec<Inline>, TesseraError> {
    let mut validated = Vec::with_capacity(inlines.len());
    for inline in inlines {
        validated.push(match inline {
            Inline::Text { text } => Inline::text(text)?,
            Inline::Bold { inline } => Inline::bold(validate_inlines(inline)?),
            Inline::Italic { inline } => Inline::italic(validate_inlines(inline)?),
            Inline::Underline { inline } => Inline::underline(validate_inlines(inline)?),
            Inline::Code { text } => Inline::code(text)?,
            Inline::Citation { keys } => Inline::citation(keys)?,
            Inline::CrossRef { label } => Inline::cross_ref(label)?,
            Inline::Footnote { inline } => Inline::footnote(validate_inlines(inline)?),
            Inline::InlineMath { latex_source } => Inline::inline_math(latex_source)?,
            Inline::LineBreak => Inline::line_break(),
        });
    }
    Ok(validated)
}

fn validate_table_style(style: &TableStyle) -> TableStyle {
    TableStyle::new(
        style.column_specs().to_vec(),
        style.rule_style(),
        style.hline_before_rows().to_vec(),
    )
}

fn set_trivia(block: &mut Block, trivia: crate::core::model::BlockTrivia) {
    if let Some(target) = block.trivia_mut() {
        *target = trivia;
    }
}

/// Applies a DOCX pane edit atomically.
///
/// Every block is validated before any mutation is visible. If one patch
/// fails, the document is left exactly as it was. Returns non-fatal
/// warnings for content that the edit would lose or leave unresolved;
/// those are surfaced to both panes (see `docs/specs.md`, section 6).
pub fn apply_dm_patches(
    document: &mut Document,
    patches: &[DmPatch],
) -> Result<Vec<SyncWarning>, TesseraError> {
    let before = document.clone();
    let mut candidate = document.clone();
    let mut warnings = Vec::new();

    for patch in patches {
        match patch {
            DmPatch::Replace { path, block } => {
                let index = replace_index(path)?;
                let current_len = candidate.body().len();
                if index >= current_len {
                    return Err(TesseraError::validation(
                        "patch.replace.path",
                        format!(
                            "block index {index} is out of range for a body with {current_len} blocks"
                        ),
                    ));
                }
                let validated = validate_block(block.clone())?;
                let old = candidate.body()[index].clone();
                warnings.extend(block_loss_warnings(&old, Some(&validated), vec![index]));
                candidate.body_mut()[index] = validated;
            }
            DmPatch::Insert { path, index, block } => {
                ensure_container_path(path, "patch.insert.path")?;
                let current_len = candidate.body().len();
                if *index > current_len {
                    return Err(TesseraError::validation(
                        "patch.insert.index",
                        format!(
                            "insert index {index} is past the end of a {current_len}-block body"
                        ),
                    ));
                }
                let validated = validate_block(block.clone())?;
                candidate.body_mut().insert(*index, validated);
            }
            DmPatch::Remove { path, index } => {
                ensure_container_path(path, "patch.remove.path")?;
                let current_len = candidate.body().len();
                if *index >= current_len {
                    return Err(TesseraError::validation(
                        "patch.remove.index",
                        format!(
                            "remove index {index} is out of range for a body with {current_len} blocks"
                        ),
                    ));
                }
                let removed = candidate.body_mut().remove(*index);
                warnings.extend(block_loss_warnings(&removed, None, vec![*index]));
            }
        }
    }

    warnings.extend(diagnostic_warnings(Some(&before), &candidate));
    *document = candidate;
    Ok(warnings)
}

fn replace_index(path: &[usize]) -> Result<usize, TesseraError> {
    match path {
        [index] => Ok(*index),
        _ => Err(TesseraError::validation(
            "patch.replace.path",
            "replace path must contain exactly one top-level block index",
        )),
    }
}

fn ensure_container_path(path: &[usize], field: &str) -> Result<(), TesseraError> {
    if path.is_empty() {
        Ok(())
    } else {
        Err(TesseraError::validation(
            field,
            "nested container paths are not supported in this version; use an empty path",
        ))
    }
}

fn block_loss_warnings(old: &Block, new: Option<&Block>, path: Vec<usize>) -> Vec<SyncWarning> {
    let mut warnings = Vec::new();

    if let Some(label) = old.label() {
        let retained = new.and_then(Block::label) == Some(label);
        if !retained {
            warnings.push(SyncWarning {
                message: format!(
                    "edit removes cross-reference label `{label}`; references to it will be unresolved"
                ),
                affected_path: path.clone(),
            });
        }
    }

    if matches!(old, Block::Opaque(_)) {
        warnings.push(SyncWarning {
            message: "edit removes content that only the originating pane could represent"
                .to_string(),
            affected_path: path.clone(),
        });
    }

    if let Block::Table(table) = old {
        if table.latex_style().is_some() && !matches!(new, Some(Block::Table(_))) {
            warnings.push(SyncWarning {
                message: "edit removes LaTeX table column or rule information".to_string(),
                affected_path: path.clone(),
            });
        }
    }

    if let Block::Figure(figure) = old {
        let retained = matches!(new, Some(Block::Figure(updated)) if updated.asset_path() == figure.asset_path());
        if !retained {
            warnings.push(SyncWarning {
                message: format!(
                    "edit no longer references image asset `{}`",
                    figure.asset_path()
                ),
                affected_path: path,
            });
        }
    }

    warnings
}

/// Warnings that follow from the document state rather than one patch:
/// newly opaque content and newly unresolved cross-references.
pub fn document_sync_warnings(before: &Document, after: &Document) -> Vec<SyncWarning> {
    diagnostic_warnings(Some(before), after)
}

/// Warnings for a freshly loaded document: every opaque block and every
/// unresolved cross-reference is reported, since there is no earlier
/// version to compare against.
pub fn initial_document_warnings(document: &Document) -> Vec<SyncWarning> {
    diagnostic_warnings(None, document)
}

fn diagnostic_warnings(before: Option<&Document>, after: &Document) -> Vec<SyncWarning> {
    let mut warnings = Vec::new();
    let before_opaque = before.map(opaque_raws).unwrap_or_default();
    for (path, raw, summary) in opaque_entries(after) {
        if !before_opaque.contains(&raw) {
            warnings.push(SyncWarning {
                message: format!("content is preserved as an opaque block: {summary}"),
                affected_path: path,
            });
        }
    }

    let before_refs = before.map(unresolved_refs).unwrap_or_default();
    for (label, path) in unresolved_refs(after) {
        if !before_refs.iter().any(|(seen, _)| seen == &label) {
            warnings.push(SyncWarning {
                message: format!("unresolved cross-reference `{label}`"),
                affected_path: path,
            });
        }
    }

    warnings
}

fn opaque_raws(document: &Document) -> Vec<String> {
    opaque_entries(document)
        .into_iter()
        .map(|(_, raw, _)| raw)
        .collect()
}

fn opaque_entries(document: &Document) -> Vec<(Vec<usize>, String, String)> {
    let mut entries = Vec::new();
    walk_blocks(document.body(), &mut Vec::new(), &mut |path, block| {
        if let Block::Opaque(opaque) = block {
            entries.push((
                path.to_vec(),
                opaque.raw().to_string(),
                opaque.summary().to_string(),
            ));
        }
    });
    entries
}

fn unresolved_refs(document: &Document) -> Vec<(String, Vec<usize>)> {
    let labels = document_labels(document);
    let mut unresolved = Vec::new();
    walk_blocks(document.body(), &mut Vec::new(), &mut |path, block| {
        visit_block_inlines(block, &mut |inline| {
            if let Inline::CrossRef { label } = inline {
                if !labels.contains(label) {
                    unresolved.push((label.clone(), path.to_vec()));
                }
            }
        });
    });
    unresolved
}

/// Every cross-reference label defined by a heading, table, figure, or
/// equation in the document, including nested list, quote, and table
/// content. Used by the highlighter to distinguish resolved and
/// unresolved `\ref` targets.
pub fn document_labels(document: &Document) -> std::collections::HashSet<String> {
    let mut labels = std::collections::HashSet::new();
    walk_blocks(document.body(), &mut Vec::new(), &mut |_, block| {
        if let Some(label) = block.label() {
            labels.insert(label.to_string());
        }
    });
    labels
}

fn visit_block_inlines(block: &Block, visit: &mut impl FnMut(&Inline)) {
    match block {
        Block::Heading { inline, .. } | Block::Paragraph { inline, .. } => {
            visit_inlines(inline, visit);
        }
        Block::Table(table) => {
            if let Some(caption) = table.caption() {
                visit_inlines(caption, visit);
            }
            for row in table.rows() {
                for cell in row {
                    for nested in cell.blocks() {
                        visit_block_inlines(nested, visit);
                    }
                }
            }
        }
        Block::Figure(figure) => {
            if let Some(caption) = figure.caption() {
                visit_inlines(caption, visit);
            }
        }
        _ => {}
    }
}

fn visit_inlines(inlines: &[Inline], visit: &mut impl FnMut(&Inline)) {
    for inline in inlines {
        visit(inline);
        match inline {
            Inline::Bold { inline }
            | Inline::Italic { inline }
            | Inline::Underline { inline }
            | Inline::Footnote { inline } => visit_inlines(inline, visit),
            _ => {}
        }
    }
}

fn walk_blocks(
    blocks: &[Block],
    prefix: &mut Vec<usize>,
    visit: &mut impl FnMut(&[usize], &Block),
) {
    for (index, block) in blocks.iter().enumerate() {
        prefix.push(index);
        visit(prefix, block);
        match block {
            Block::List { items, .. } => {
                for (item_index, item) in items.iter().enumerate() {
                    prefix.push(item_index);
                    walk_blocks(item, prefix, visit);
                    prefix.pop();
                }
            }
            Block::Quote { blocks, .. } => walk_blocks(blocks, prefix, visit),
            Block::Table(table) => {
                for (row_index, row) in table.rows().iter().enumerate() {
                    for (cell_index, cell) in row.iter().enumerate() {
                        prefix.push(row_index);
                        prefix.push(cell_index);
                        walk_blocks(cell.blocks(), prefix, visit);
                        prefix.pop();
                        prefix.pop();
                    }
                }
            }
            _ => {}
        }
        prefix.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{
        Author, Block, DocMetadata, HeadingLevel, Inline, OpaqueBlock, OpaqueOrigin,
    };

    fn document(blocks: Vec<Block>) -> Document {
        let metadata = DocMetadata::new(
            None,
            vec![Author::new("A".into(), None).unwrap()],
            None,
            None,
            vec![],
        )
        .unwrap();
        Document::new(metadata, blocks)
    }

    fn paragraph(text: &str) -> Block {
        Block::paragraph(vec![Inline::text(text.into()).unwrap()])
    }

    #[test]
    fn diff_reports_added_removed_and_replaced_slots() {
        let old = document(vec![paragraph("one"), paragraph("two"), paragraph("three")]);
        let new = document(vec![paragraph("one"), paragraph("changed")]);
        let diff = diff_documents(&old, &new);

        assert_eq!(
            diff.blocks,
            vec![
                BlockChange {
                    index: 1,
                    kind: BlockChangeKind::Replaced,
                },
                BlockChange {
                    index: 2,
                    kind: BlockChangeKind::Removed,
                },
            ]
        );
    }

    #[test]
    fn diff_reports_metadata_and_trivia_changes() {
        let old = document(vec![paragraph("one")]);
        let mut new = old.clone();
        new.set_trailing_trivia("\n".into());
        let diff = diff_documents(&old, &new);
        assert!(diff.trailing_trivia_changed);
    }

    #[test]
    fn patches_are_applied_atomically_and_validate_blocks() {
        let mut doc = document(vec![paragraph("one"), paragraph("two")]);
        let patches = vec![
            DmPatch::Replace {
                path: vec![0],
                block: paragraph("changed"),
            },
            DmPatch::Replace {
                path: vec![9],
                block: paragraph("out of range"),
            },
        ];
        assert!(apply_dm_patches(&mut doc, &patches).is_err());
        assert_eq!(doc.body(), &[paragraph("one"), paragraph("two")]);
    }

    #[test]
    fn removing_a_labelled_block_produces_a_warning() {
        let heading = Block::heading(
            HeadingLevel::H1,
            vec![Inline::text("Intro".into()).unwrap()],
            Some("sec:intro".into()),
        )
        .unwrap();
        let mut doc = document(vec![heading, paragraph("body")]);
        let warnings = apply_dm_patches(
            &mut doc,
            &[DmPatch::Remove {
                path: vec![],
                index: 0,
            }],
        )
        .expect("patch applies");

        assert!(warnings
            .iter()
            .any(|warning| warning.message.contains("cross-reference label")));
    }

    #[test]
    fn introducing_opaque_content_produces_a_warning() {
        let mut doc = document(vec![paragraph("body")]);
        let opaque = Block::opaque(
            OpaqueBlock::new(OpaqueOrigin::Docx, "<w:p/>".into(), "DOCX comment".into()).unwrap(),
        );
        let warnings = apply_dm_patches(
            &mut doc,
            &[DmPatch::Insert {
                path: vec![],
                index: 1,
                block: opaque,
            }],
        )
        .expect("patch applies");

        assert!(warnings
            .iter()
            .any(|warning| warning.message.contains("opaque block")));
    }
}
