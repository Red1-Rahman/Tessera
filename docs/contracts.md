# Contracts

This document is the single source of truth for every data shape that
crosses a boundary in Tessera: Rust core to Tauri command layer, Tauri
command layer to frontend, and the on-disk project format. If code and
this document disagree, this document wins until the code is fixed or
this document is updated in the same change.

All Rust types below are `serde::Serialize + serde::Deserialize`. Fields
are validated at construction, not trusted after deserialization; every
type that can be built from untrusted input (parsed LaTeX, parsed DOCX,
IPC payloads from the frontend) goes through a constructor function that
returns `Result<T, TesseraError>`, never a bare struct literal built
directly from untrusted fields. This is the Rust equivalent of a Pydantic
model: validation lives on the type's boundary, not scattered through
call sites.

## 1. The Document Model (DM)

Defined in `src-tauri/src/core/model.rs`.

```rust
/// A single Tessera document, the shared intermediate representation
/// that both the LaTeX pane and the DOCX pane are views of.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub version: DocVersion,
    pub metadata: DocMetadata,
    pub body: Vec<Block>,
}

/// Monotonically increasing per-session version counter. Used to discard
/// stale async parse results (see architecture.md, "Sync flow").
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocVersion(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DocMetadata {
    pub title: Option<String>,
    pub authors: Vec<Author>,
    pub date: Option<String>,
    /// LaTeX document class, e.g. "article", "report". Preserved even
    /// though the DOCX pane has no equivalent concept, so round-trip to
    /// LaTeX does not lose it.
    pub latex_class: Option<String>,
    pub latex_class_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Author {
    pub name: String,
    pub affiliation: Option<String>,
}

/// A block-level node. This is the backbone of the document tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Block {
    Heading { level: HeadingLevel, inline: Vec<Inline>, label: Option<String> },
    Paragraph { inline: Vec<Inline> },
    List { kind: ListKind, items: Vec<Vec<Block>> },
    Table(Table),
    Figure(Figure),
    Equation(Equation),
    CodeBlock { language: Option<String>, text: String },
    Quote { blocks: Vec<Block> },
    /// Content that only one pane can faithfully represent. See
    /// architecture.md, "Conflict and loss handling".
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HeadingLevel { H1, H2, H3, H4, H5, H6 }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ListKind { Ordered, Unordered }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table {
    pub rows: Vec<Vec<TableCell>>,
    pub caption: Option<Vec<Inline>>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    pub blocks: Vec<Block>,
    pub col_span: u32,
    pub row_span: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Figure {
    /// Path relative to the project's .tessera/assets directory.
    pub asset_path: String,
    pub caption: Option<Vec<Inline>>,
    pub label: Option<String>,
    pub width_percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Equation {
    /// Always stored as LaTeX math source, the DM's canonical math
    /// representation regardless of which pane produced it.
    pub latex_source: String,
    pub display_mode: bool,
    pub label: Option<String>,
}

/// Content that round-trips losslessly through its origin format but has
/// no first-class DM representation. Rendered as a clearly marked,
/// non-editable block in the pane that did not produce it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpaqueBlock {
    pub origin: OpaqueOrigin,
    /// Raw bytes/text in the origin format, preserved verbatim.
    pub raw: String,
    /// Short human-readable description shown in the UI, e.g.
    /// "Raw TeX macro: \customcmd".
    pub summary: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpaqueOrigin { Latex, Docx }

/// An inline (span-level) node.
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
```

### Invariants

- `DocVersion` only increases within a session. The core never issues a
  version older than one already handed to the frontend.
- `Block::Opaque` and inline content are never silently merged or
  reformatted by a pane that cannot interpret them. An `Opaque` block
  passed through export to its own origin format must produce
  byte-identical `raw` output.
- Every `label` field (headings, tables, figures, equations) is the
  shared key used for `Inline::CrossRef`, mirroring LaTeX's `\label`/
  `\ref` pairing so DOCX cross-references map onto it directly.

## 2. Tauri command surface (Rust to frontend)

Defined in `src-tauri/src/commands/`. Every command returns
`Result<T, TesseraError>`, serialized as a tagged JSON object so the
frontend can discriminate without string matching.

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TesseraError {
    ParseError { pane: Pane, message: String, span: Option<SourceSpan> },
    ValidationError { field: String, message: String },
    IoError { path: String, message: String },
    ExportError { format: ExportFormat, message: String },
    StaleVersion { requested: DocVersion, current: DocVersion },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Pane { Latex, Docx }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}
```

| Command | Input | Output | Purpose |
|---|---|---|---|
| `open_project` | `path: String` | `ProjectState` | Load a project directory, parse `main.tex`, build the initial DM. |
| `new_project` | `NewProjectRequest` | `ProjectState` | Create a project directory with starter `main.tex`. |
| `parse_latex` | `LatexEditRequest` | `DocSyncResult` | Apply a LaTeX pane edit, return the updated DM and DOCX-pane render. |
| `apply_docx_edit` | `DocxEditRequest` | `DocSyncResult` | Apply a DOCX pane edit, return the updated DM and LaTeX-pane render. |
| `save_project` | `()` | `()` | Write `main.tex` and `main.docx` to disk from the current DM. |
| `export` | `ExportRequest` | `ExportResult` | Render the current DM to the requested format and return a file path or clipboard-ready payload. |
| `get_highlight_tokens` | `HighlightRequest` | `Vec<HighlightToken>` | Okabe-Ito token classification for a LaTeX source range. |

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct LatexEditRequest {
    pub base_version: DocVersion,
    pub full_source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocxEditRequest {
    pub base_version: DocVersion,
    /// The DM subtree(s) the DOCX pane edited, expressed as patches
    /// rather than a full document, since the DOCX pane edits structured
    /// content rather than raw text.
    pub patches: Vec<DmPatch>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum DmPatch {
    Replace { path: Vec<usize>, block: Block },
    Insert { path: Vec<usize>, index: usize, block: Block },
    Remove { path: Vec<usize>, index: usize },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocSyncResult {
    pub new_version: DocVersion,
    pub document: Document,
    /// Rendered LaTeX source for the LaTeX pane, always present so the
    /// pane that did not originate the edit can refresh.
    pub latex_render: String,
    pub warnings: Vec<SyncWarning>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncWarning {
    pub message: String,
    pub affected_path: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExportFormat { Latex, Docx, Pdf, Markdown, PlainText }

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    /// None means "return bytes for clipboard", Some means "write to
    /// this path".
    pub destination: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ExportResult {
    WrittenToFile { path: String },
    ClipboardPayload { format: ExportFormat, content: String },
}
```

### Frontend-side types

`src/ipc/commands.ts` keeps hand-written TypeScript types that mirror the
Rust types above field-for-field. Field names and casing are identical
(`serde` is configured with no case conversion; see
`docs/conventions.md`). Any divergence between the TS types and the Rust
types is a bug, caught by the contract tests described below.

## 3. On-disk project format

`.tessera/project.json`:

```json
{
  "schema_version": 1,
  "doc_version": 42,
  "last_edited_pane": "latex",
  "assets": ["figure1.png", "diagram2.svg"]
}
```

This file is derived, cheap to regenerate, and safe to delete. It exists
only to let Tessera resume exactly where a session left off; it is never
the source of truth for document content, `main.tex` is.

## 4. Contract testing

`src-tauri/tests/contracts.rs` contains:

- Golden-file tests: a fixed set of `.tex` and `.docx` fixtures with
  expected `Document` JSON, checked into `tests/fixtures/`.
- Round-trip property tests (via `proptest`) generating random `Document`
  trees and asserting `parse_latex(print_latex(d)) == d` and the DOCX
  equivalent, excluding `Opaque` blocks by construction (those are
  format-specific by definition).
- Serialization shape tests asserting the JSON Schema-equivalent shape of
  every type in this document has not changed without a corresponding
  `schema_version` bump, using `insta` snapshot tests on
  `serde_json::to_value`.

Any change to a type in this document requires an accompanying update to
these tests in the same pull request. See `AGENTS.md` for how this is
enforced for AI-assisted changes.
