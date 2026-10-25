# Specifications

This document defines exactly what Tessera supports. Anything not listed
here as supported is either out of scope or lands as an `Opaque` block
(see `docs/contracts.md`). "Supported" means: parses from LaTeX into the
Document Model, round-trips back to identical LaTeX, and has a defined
DOCX mapping in both directions.

## 1. Supported LaTeX subset

### 1.1 Document classes

- `article`, `report`, `book` recognized and preserved in
  `DocMetadata.latex_class`.
- Class options (`[12pt]`, `[a4paper]`, etc.) are preserved verbatim as
  strings in `latex_class_options` but are not individually interpreted;
  they affect LaTeX-pane and built-in PDF typesetting but have no DOCX
  pane equivalent beyond being carried through on export back to LaTeX.

### 1.2 Preamble

- `\usepackage{...}` lines are preserved verbatim and re-emitted
  unchanged on export to LaTeX. They are not interpreted; package-specific
  commands used in the body that are not in this spec's command list are
  `Opaque`.
- `\title`, `\author`, `\date` map to `DocMetadata`.
- Custom macro definitions (`\newcommand`, `\def`) are preserved verbatim
  in the preamble but **uses** of custom macros in the body are `Opaque`,
  since Tessera does not expand macros.

### 1.3 Structure

| LaTeX | DM node |
|---|---|
| `\section{...}` | `Block::Heading { level: H1 }` |
| `\subsection{...}` | `Block::Heading { level: H2 }` |
| `\subsubsection{...}` | `Block::Heading { level: H3 }` |
| `\paragraph{...}` | `Block::Heading { level: H4 }` |
| Blank-line-separated text | `Block::Paragraph` |
| `\label{...}` following a sectioning command or inside a float | attached to that node's `label` field |
| `\ref{...}`, `\cref{...}`, `\autoref{...}` | `Inline::CrossRef` |

`\part` and `\chapter` are supported only under `report`/`book` classes
and map to `H1`, shifting `\section` down to `H2` accordingly for that
document; the DOCX pane reflects the same shift.

### 1.4 Text formatting

| LaTeX | DM node |
|---|---|
| `\textbf{...}`, `\bfseries` (scoped) | `Inline::Bold` |
| `\textit{...}`, `\emph{...}`, `\itshape` (scoped) | `Inline::Italic` |
| `\underline{...}` | `Inline::Underline` |
| `\texttt{...}`, `\verb|...|` | `Inline::Code` |
| `\footnote{...}` | `Inline::Footnote` |
| `\\` (line break) | `Inline::LineBreak` |

Nested formatting (bold inside italic, etc.) is supported to arbitrary
depth since `Inline` variants are recursive.

When multiple supported spellings map to one inline node, LaTeX output
uses `\\textbf`, `\\textit`, and `\\underline` for bold, italic, and
underline. Inline code uses `\\verb` when a safe delimiter is available
and `\\texttt` otherwise. These canonical spellings keep printer output
stable while the parser continues to accept the alternatives above.

### 1.5 Lists

| LaTeX | DM node |
|---|---|
| `itemize` environment | `Block::List { kind: Unordered }` |
| `enumerate` environment | `Block::List { kind: Ordered }` |
| Nested `itemize`/`enumerate` | nested `Block::List` inside a list item's `Vec<Block>` |

Custom list labels (`\item[label]`) are not in the initial supported set;
they land as `Opaque` at the item level. Tracked for a later version (see
"Deferred" below).

### 1.6 Tables

| LaTeX | DM node |
|---|---|
| `tabular` environment | `Table.rows` |
| `table` float wrapping a `tabular` | `Block::Table` with `caption`/`label` populated from `\caption`/`\label` inside the float |
| `\multicolumn{n}{...}{...}` | `TableCell.col_span` |
| `\multirow{n}{...}{...}` (requires `multirow` package) | `TableCell.row_span` |

Column alignment specifiers (`l`, `c`, `r`, `p{...}`) and rules (`\hline`,
`\toprule`/`\midrule`/`\bottomrule` from `booktabs`) are preserved on
export back to LaTeX via a per-table style record, but are not
independently editable from the DOCX pane in the initial version; editing
table borders from the DOCX pane is listed under "Deferred."

### 1.7 Figures

| LaTeX | DM node |
|---|---|
| `figure` environment with `\includegraphics` | `Block::Figure` |
| `\caption{...}` inside `figure` | `Figure.caption` |
| `\label{...}` inside `figure` | `Figure.label` |
| `width=` option of `\includegraphics` | `Figure.width_percent`, converted from whatever unit LaTeX used (relative to `\linewidth`/`\textwidth` or absolute, normalized to a percentage of text width) |

Referenced image files are copied into `.tessera/assets/` on project
open if not already there, and `asset_path` is always relative to that
directory, never an absolute filesystem path, so a project remains
portable.

### 1.8 Math

- Inline math (`$...$`, `\(...\)`) maps to `Inline::InlineMath`, storing
  the raw LaTeX math source unchanged. Tessera does not parse or validate
  math syntax; it is opaque *content* but a first-class DM *node*, meaning
  it round-trips and gets a defined (if visually approximate) DOCX
  rendering, which distinguishes it from `Opaque` blocks.
- Display math (`equation`, `equation*`, `\[...\]`) maps to
  `Block::Equation`, with `display_mode: true` and `false` matching
  whether the `*` (unnumbered) form was used.
- DOCX rendering of math uses Office Math Markup Language (OMML),
  generated from the stored LaTeX source via the conversion rules in
  `src-tauri/src/core/docx/writer.rs`. Only the math subset expressible
  in both LaTeX math and OMML converts with full fidelity; constructs
  outside that intersection are flagged with a `SyncWarning` (see
  `docs/contracts.md`) rather than silently approximated.
- `align`, `align*`, `gather` environments are supported as a sequence of
  `Block::Equation` nodes sharing a group marker, documented further in
  `src-tauri/src/core/latex/parser.rs` doc comments.

### 1.9 Citations and bibliography

- `\cite{key1,key2}` and natbib/biblatex variants (`\citep`, `\citet`)
  map to `Inline::Citation { keys }`.
- `.bib` file content is not parsed into structured DM nodes in the
  initial version. The bibliography itself is tracked as project-level
  metadata (a referenced `.bib` path) rather than inlined into the DM;
  the rendered bibliography section in each pane is generated at export
  time from that file. Editing bibliography entries from the DOCX pane is
  listed under "Deferred."

### 1.10 Code blocks

- `verbatim` and `lstlisting` environments map to `Block::CodeBlock`.
  `language` is populated from the `language=` option of `lstlisting`
  when present, `None` otherwise.
- `minted` is recognized (`language` taken from its mandatory argument)
  but its execution/highlighting behavior is not reproduced; Tessera only
  extracts the source text.

### 1.11 Explicitly out of scope for v1 (always `Opaque`)

- Raw TeX macros without a mapping above (`\def`, custom commands in the
  body, conditional (`\ifx`) logic).
- `tikz`/`pgfplots` figures. These render as `Opaque` with a summary
  noting the environment name; they are not rasterized automatically.
- Multi-file projects using `\input`/`\include` are supported at the
  project level (see section 3) but cross-file `\ref`/`\label` resolution
  across more than one active pane session is deferred.
- Custom bibliography styles beyond what the chosen style (see
  `docs/techstack.yml`) renders natively.

## 2. DOCX mapping notes

The DOCX writer (`src-tauri/src/core/docx/writer.rs`) targets Word's
built-in style names (`Heading 1`, `Heading 2`, `Normal`, `Caption`, …) so
that a reader opening the exported `.docx` in Word, LibreOffice, or Google
Docs sees a normal, editable Word document using that application's own
heading navigation, not a document built entirely from manual character
formatting. This is also what makes `main.docx` genuinely useful as a
submission file, not just a visual approximation.

The DOCX reader (`src-tauri/src/core/docx/reader.rs`) maps built-in
styles back onto the DM table in section 1 in reverse. A `.docx` opened
in Tessera that was not originally produced by Tessera is read on a
best-effort basis: recognized built-in styles map to their DM
equivalents; anything else (custom styles, embedded objects, tracked
changes, comments) becomes `Opaque` with `origin: Docx`, exactly as it
would for an unsupported LaTeX construct.

## 3. Multi-file LaTeX projects

A project's `main.tex` may use `\input{chapter1}` / `\include{chapter1}`.
On project open, the core resolves these into a single DM by inlining
referenced files, tracking provenance (which DM nodes came from which
source file) so that saving writes changes back to the correct file
rather than flattening everything into `main.tex`. This provenance
tracking is internal to the core; it is not part of the public contract
in `docs/contracts.md` because the frontend does not need to be aware of
it; it only ever sees one `Document`.

## 4. Export formats

| Format | Mechanism |
|---|---|
| LaTeX (`.tex`) | DM printed via `core/latex/printer.rs`. Always available, always exact. |
| DOCX (`.docx`) | DM written via `core/docx/writer.rs`. Always available. |
| PDF | System TeX engine if present, else built-in typesetter. See `docs/architecture.md`, "PDF export strategy." |
| Markdown (`.md`) | DM rendered via `core/export/markdown.rs`. Math renders as raw LaTeX math delimited by `$...$`/`$$...$$` (Pandoc-compatible convention); `Opaque` blocks render as a fenced comment noting what was dropped. |
| Plain text (`.txt`) | DM rendered via `core/export/plaintext.rs`. All formatting, math, and structure is flattened to text; headings become underlined or numbered lines; this is lossy by nature and labeled as such in the export menu. |
| Clipboard | Any of the above formats, written to the system clipboard instead of a file, via Tauri's clipboard API. |

## 5. Syntax highlighting

LaTeX pane highlighting uses the eight-color Okabe-Ito palette (chosen
for colorblind accessibility, see `docs/conventions.md` for exact hex
values and the token-to-color mapping). Highlighting is computed by the
Rust core (`core/latex/highlight.rs`) from the same parser used for the
DM, so highlighting and parsing never disagree about what a token is.

## 6. UI behavior

- Split view is the default and only layout in v1: LaTeX pane left or
  top (configurable), DOCX pane right or bottom. No tabbed single-pane
  mode in v1.
- Edits debounce before triggering a re-parse (see
  `docs/conventions.md` for the exact interval) to avoid re-parsing on
  every keystroke, while still feeling live.
- A `SyncWarning` (from `docs/contracts.md`) surfaces as a non-blocking
  inline annotation at the affected location in both panes, never a
  modal dialog, so the user is not interrupted mid-edit.
- `Opaque` blocks render with a distinct visual treatment (background
  tint using one of the Okabe-Ito palette colors reserved for this
  purpose, see `docs/conventions.md`) and are not directly editable from
  the pane that did not originate them; a tooltip/summary explains why.

## 7. Deferred (explicitly not v1, tracked for later)

- Editing table borders and bibliography entries from the DOCX pane.
- Custom list item labels.
- `tikz` rasterization.
- Tracked changes / comments synchronized between panes (DOCX comments
  currently land as `Opaque`).
- Real-time collaborative editing. Tessera is single-user, single-machine
  by design; this is not planned, not merely postponed.
