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

In the Rust code, the fields of the struct types in section 1 are
private and are read through accessor methods, so no code path can
bypass a validating constructor with a struct literal. The JSON shape
produced and consumed across IPC is exactly as shown in this document;
restricting field visibility changes only Rust-level constructibility,
never the wire format. Enum variant fields (the `Block` and `Inline`
variants) stay public because Rust enums cannot have private variant
fields; producers must use the constructors listed under "Constructors"
in section 1, and any value arriving from a frontend payload is
re-validated at the deserialization site in the command layer.

## 1. The Document Model (DM)

Defined in `src-tauri/src/core/model.rs`.

```rust
/// A single Tessera document, the shared intermediate representation
/// that both the LaTeX pane and the DOCX pane are views of.
/// Fields are private; use Document::new plus the accessors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    version: DocVersion,
    metadata: DocMetadata,
    body: Vec<Block>,
}

/// Monotonically increasing per-session version counter. Used to discard
/// stale async parse results (see architecture.md, "Sync flow").
/// The inner value is private; see "Constructors" below.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct DocVersion(u64);

/// Fields are private; use DocMetadata::new plus the accessors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DocMetadata {
    title: Option<String>,
    authors: Vec<Author>,
    date: Option<String>,
    /// LaTeX document class, e.g. "article", "report". Preserved even
    /// though the DOCX pane has no equivalent concept, so round-trip to
    /// LaTeX does not lose it.
    latex_class: Option<String>,
    latex_class_options: Vec<String>,
}

/// Fields are private; use Author::new plus name()/affiliation().
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Author {
    name: String,
    affiliation: Option<String>,
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

/// Fields are private; use Table::new plus the accessors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table {
    rows: Vec<Vec<TableCell>>,
    caption: Option<Vec<Inline>>,
    label: Option<String>,
}

/// Fields are private; use TableCell::new plus the accessors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    blocks: Vec<Block>,
    col_span: u32,
    row_span: u32,
}

/// Fields are private; use Figure::new plus the accessors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Figure {
    /// Path relative to the project's .tessera/assets directory.
    asset_path: String,
    caption: Option<Vec<Inline>>,
    label: Option<String>,
    width_percent: Option<f32>,
}

/// Fields are private; use Equation::new plus the accessors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Equation {
    /// Always stored as LaTeX math source, the DM's canonical math
    /// representation regardless of which pane produced it.
    latex_source: String,
    display_mode: bool,
    label: Option<String>,
}

/// Content that round-trips losslessly through its origin format but has
/// no first-class DM representation. Rendered as a clearly marked,
/// non-editable block in the pane that did not produce it.
/// Fields are private; use OpaqueBlock::new plus the accessors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpaqueBlock {
    origin: OpaqueOrigin,
    /// Raw bytes/text in the origin format, preserved verbatim.
    raw: String,
    /// Short human-readable description shown in the UI, e.g.
    /// "Raw TeX macro: \customcmd".
    summary: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpaqueOrigin { Latex, Docx }

/// An inline (span-level) node. All variants are struct variants: an
/// internally tagged enum cannot serialize a newtype variant holding a
/// string or sequence, so `Text(String)` would be unserializable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Inline {
    Text { text: String },
    Bold { inline: Vec<Inline> },
    Italic { inline: Vec<Inline> },
    Underline { inline: Vec<Inline> },
    Code { text: String },
    Citation { keys: Vec<String> },
    CrossRef { label: String },
    Footnote { inline: Vec<Inline> },
    InlineMath { latex_source: String },
    LineBreak,
}
```

### Constructors

Every section 1 type that can be built from untrusted input is
constructed only through these functions, all in
`src-tauri/src/core/model.rs`, each returning `Result<T,
TesseraError>` with a `ValidationError` naming the offending field. The
"rejects" column is what the validation enforces beyond the type shape.

| Type | Constructor | Rejects |
|---|---|---|
| `Document` | `Document::new(metadata, body)` | (no field-level rules; version is always `DocVersion::first()`) |
| `DocMetadata` | `DocMetadata::new(...)` | empty `title`/`date`/`latex_class` when present; empty class option |
| `Author` | `Author::new(name, affiliation)` | empty/whitespace `name`; empty `affiliation` when present |
| `Block` | `Block::heading`, `Block::paragraph`, `Block::list`, `Block::table`, `Block::figure`, `Block::equation`, `Block::code_block`, `Block::quote`, `Block::opaque` | invalid heading `label`; empty `language` when present |
| `Table` | `Table::new(rows, caption, label)` | invalid `label` |
| `TableCell` | `TableCell::new(blocks, col_span, row_span)` | `col_span` or `row_span` of 0 |
| `Figure` | `Figure::new(asset_path, caption, label, width_percent)` | empty `asset_path`; absolute path or `..` traversal; `width_percent` outside (0, 100] or non-finite; invalid `label` |
| `Equation` | `Equation::new(latex_source, display_mode, label)` | empty `latex_source`; invalid `label` |
| `OpaqueBlock` | `OpaqueBlock::new(origin, raw, summary)` | empty `raw`; empty `summary` |
| `Inline` | `Inline::text`, `Inline::bold`, `Inline::italic`, `Inline::underline`, `Inline::code`, `Inline::citation`, `Inline::cross_ref`, `Inline::footnote`, `Inline::inline_math`, `Inline::line_break` | empty text/code/math; empty citation key list or empty key; invalid `cross_ref` label |

The free function `validate_label(field, label)` implements the shared
label rule (non-empty after trimming, no embedded whitespace) used by
every `label` field and by `Inline::CrossRef`; the command layer also
uses it to re-validate labels arriving from the frontend.

`DocVersion` has no public constructor at all: it starts at
`DocVersion::first()` and advances only through `DocVersion::next()` (or
`Document::bump_version()`), so core code cannot hand the frontend a
version older than one already issued. `DocVersion::value()` exposes the
raw `u64` for display and logging only.

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

Implementation note (serial-task02): implementing section 1 found one
shape change. `Inline` was documented with `#[serde(tag = "type")]`
while six of its variants were newtypes over a string or a sequence
(`Text(String)`, `Bold(Vec<Inline>)`, `Italic`, `Underline`,
`Code(String)`, `Footnote(Vec<Inline>)`). An internally tagged enum
cannot serialize a newtype variant with that shape, so the documented
type produced a runtime error ("cannot serialize tagged newtype
variant") for any inline content. Those variants are now struct
variants (`Text { text }`, `Bold { inline }`, `Code { text }`,
`Footnote { inline }`, and the italic/underline equivalents), keeping
the `type` tag both documented and real. Variant names are unchanged,
so `docs/specs.md`'s LaTeX-to-node mappings still apply as written. All
other section 1 types implemented exactly as documented; the only
code-level restrictions, private fields plus validating constructors and
a non-constructible `DocVersion`, are described in section 1's intro
and "Constructors" subsection and do not change any JSON shape.

## 2. Tauri command surface (Rust to frontend)

Defined in `src-tauri/src/commands/`. Every command returns
`Result<T, TesseraError>`, serialized as a tagged JSON object so the
frontend can discriminate without string matching.

`TesseraError`, `Pane`, `SourceSpan`, and `ExportFormat` are defined in
`src-tauri/src/core/error.rs` and re-exported from
`src-tauri/src/commands/` for the IPC surface, because `core/` code
returns `TesseraError` directly and must not depend on `commands/` (see
`docs/architecture.md`, "Crate and module boundaries"). The wire shapes
below are unchanged by that placement.

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
