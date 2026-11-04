// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Section 1: Document Model
// ---------------------------------------------------------------------------

export type DocVersion = number;

export interface BlockTrivia {
  leading: string;
}

export type HeadingLevel = "H1" | "H2" | "H3" | "H4" | "H5" | "H6";
export type ListKind = "Ordered" | "Unordered";
export type RuleStyle = "Hline" | "Booktabs" | "None";
export type OpaqueOrigin = "Latex" | "Docx";

export interface TableStyle {
  column_specs: string[];
  rule_style: RuleStyle;
  hline_before_rows: number[];
}

export interface TableCell {
  blocks: Block[];
  col_span: number;
  row_span: number;
}

export interface Table {
  rows: TableCell[][];
  caption: Inline[] | null;
  label: string | null;
  leading_trivia: string;
  latex_style: TableStyle | null;
}

export interface Figure {
  asset_path: string;
  caption: Inline[] | null;
  label: string | null;
  width_percent: number | null;
  leading_trivia: string;
}

export interface Equation {
  latex_source: string;
  display_mode: boolean;
  label: string | null;
  leading_trivia: string;
}

export interface OpaqueBlock {
  origin: OpaqueOrigin;
  raw: string;
  summary: string;
}

export type Inline =
  | { type: "Text"; text: string }
  | { type: "Bold"; inline: Inline[] }
  | { type: "Italic"; inline: Inline[] }
  | { type: "Underline"; inline: Inline[] }
  | { type: "Code"; text: string }
  | { type: "Citation"; keys: string[] }
  | { type: "CrossRef"; label: string }
  | { type: "Footnote"; inline: Inline[] }
  | { type: "InlineMath"; latex_source: string }
  | { type: "LineBreak" };

export type Block =
  | { type: "Heading"; level: HeadingLevel; inline: Inline[]; label: string | null; trivia: BlockTrivia }
  | { type: "Paragraph"; inline: Inline[]; trivia: BlockTrivia }
  | { type: "List"; kind: ListKind; items: Block[][]; trivia: BlockTrivia }
  | ({ type: "Table" } & Table)
  | ({ type: "Figure" } & Figure)
  | ({ type: "Equation" } & Equation)
  | { type: "CodeBlock"; language: string | null; text: string; trivia: BlockTrivia }
  | { type: "Quote"; blocks: Block[]; trivia: BlockTrivia }
  | ({ type: "Opaque" } & OpaqueBlock);

export interface Author {
  name: string;
  affiliation: string | null;
}

export interface DocMetadata {
  title: string | null;
  authors: Author[];
  date: string | null;
  latex_class: string | null;
  latex_class_options: string[];
  preamble_raw: string[];
}

export interface Document {
  version: DocVersion;
  metadata: DocMetadata;
  body: Block[];
  trailing_trivia: string;
}

// ---------------------------------------------------------------------------
// Section 2: Tauri Command Surface
// ---------------------------------------------------------------------------

export type Pane = "Latex" | "Docx";

export interface SourceSpan {
  start_line: number;
  start_col: number;
  end_line: number;
  end_col: number;
}

export type TesseraError =
  | { kind: "ParseError"; pane: Pane; message: string; span: SourceSpan | null }
  | { kind: "ValidationError"; field: string; message: string }
  | { kind: "IoError"; path: string; message: string }
  | { kind: "ExportError"; format: ExportFormat; message: string }
  | { kind: "StaleVersion"; requested: DocVersion; current: DocVersion };

export interface SyncWarning {
  message: string;
  affected_path: number[];
}

export interface ProjectState {
  path: string;
  document: Document;
  last_edited_pane: Pane;
  assets: string[];
  warnings: SyncWarning[];
}

export interface NewProjectRequest {
  path: string;
  title: string | null;
}

export interface LatexEditRequest {
  base_version: DocVersion;
  full_source: string;
}

export type DmPatch =
  | { op: "Replace"; path: number[]; block: Block }
  | { op: "Insert"; path: number[]; index: number; block: Block }
  | { op: "Remove"; path: number[]; index: number };

export interface DocxEditRequest {
  base_version: DocVersion;
  patches: DmPatch[];
}

export interface DocSyncResult {
  new_version: DocVersion;
  document: Document;
  latex_render: string;
  warnings: SyncWarning[];
}

export interface HighlightRequest {
  source: string;
}

export type HighlightKind =
  | "Command"
  | "Bracket"
  | "Environment"
  | "Comment"
  | "Math"
  | "Error"
  | "Text";

export interface HighlightToken {
  start: number;
  end: number;
  kind: HighlightKind;
}

export type ExportFormat = "Latex" | "Docx" | "Pdf" | "Markdown" | "PlainText";

export interface ExportRequest {
  format: ExportFormat;
  destination: string | null;
}

export type PdfEngine =
  | { type: "SystemTeX"; program: string }
  | { type: "BuiltIn" };

export type ExportResult =
  | { kind: "WrittenToFile"; path: string; pdf_engine?: PdfEngine }
  | { kind: "ClipboardPayload"; format: ExportFormat; content: string };

// ---------------------------------------------------------------------------
// Command Wrappers
// ---------------------------------------------------------------------------

export async function openProject(path: string): Promise<ProjectState> {
  return invoke<ProjectState>("open_project", { path });
}

export async function newProject(request: NewProjectRequest): Promise<ProjectState> {
  return invoke<ProjectState>("new_project", { request });
}

export async function parseLatex(request: LatexEditRequest): Promise<DocSyncResult> {
  return invoke<DocSyncResult>("parse_latex", { request });
}

export async function applyDocxEdit(request: DocxEditRequest): Promise<DocSyncResult> {
  return invoke<DocSyncResult>("apply_docx_edit", { request });
}

export async function saveProject(): Promise<void> {
  await invoke<null>("save_project");
}

export async function exportDocument(request: ExportRequest): Promise<ExportResult> {
  return invoke<ExportResult>("export", { request });
}

export async function getHighlightTokens(
  request: HighlightRequest
): Promise<HighlightToken[]> {
  return invoke<HighlightToken[]>("get_highlight_tokens", { request });
}