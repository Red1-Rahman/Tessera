# Architecture

## Goals that shape every decision here

1. Fully offline. No code path may open a network socket. This is checked
   in CI (see `docs/conventions.md`, "Network ban").
2. Bidirectional sync between a LaTeX view and a DOCX view must never lose
   information silently. If a construct cannot round-trip, the user is told,
   not left guessing.
3. Files on disk are plain, inspectable formats. No proprietary blobs, no
   hidden state required to open a project elsewhere.
4. Cross-platform: Windows, macOS, Linux, from one Rust codebase.

## High-level shape

```
┌─────────────────────────────────────────────────────────────────┐
│                         Tauri Shell (Rust)                       │
│                                                                   │
│   ┌───────────────┐      ┌──────────────────────────────────┐   │
│   │  Frontend      │ IPC  │  Core (src-tauri/src/core)        │  │
│   │  (webview)     │◄────►│                                    │  │
│   │  React + TS    │      │  ┌──────────────────────────────┐ │  │
│   └───────────────┘      │  │   Document Model (DOM-ish)    │ │  │
│                            │  │   intermediate representation │ │  │
│                            │  └──────────────────────────────┘ │  │
│                            │        ▲              ▲            │  │
│                            │        │              │            │  │
│                            │  ┌──────────┐   ┌──────────┐       │  │
│                            │  │ LaTeX    │   │ DOCX     │       │  │
│                            │  │ parser/  │   │ parser/  │       │  │
│                            │  │ printer  │   │ writer   │       │  │
│                            │  └──────────┘   └──────────┘       │  │
│                            │                                     │  │
│                            │  ┌──────────────────────────────┐  │  │
│                            │  │ Export: PDF, Markdown, TXT    │  │  │
│                            │  └──────────────────────────────┘  │  │
│                            └────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

The frontend never parses LaTeX or DOCX itself. It sends raw text or user
edits to the Rust core through Tauri commands and receives back a
structured document (or a diff) to render. All conversion logic lives in
Rust so there is exactly one implementation of the sync rules, not one per
language.

## The Document Model (intermediate representation)

The center of the system is a single in-memory tree, the **Document
Model** (DM), defined in `src-tauri/src/core/model.rs` as a set of
`serde`-friendly Rust structs (see `docs/contracts.md` for the exact
schema). Both the LaTeX side and the DOCX side are projections of this one
tree:

```
LaTeX text  --parse-->   DM   --print-->  LaTeX text
DOCX bytes  --parse-->   DM   --write-->  DOCX bytes
                           │
                           ├--render--> PDF
                           ├--render--> Markdown
                           └--render--> Plain text
```

The DM is intentionally a semantic document tree, not a stylesheet-based
layout tree. It models things like "this is a section heading," "this is a
numbered list," "this is a citation," rather than "this text is 14pt
bold." Font sizes, margins, and similar layout detail are captured as
metadata on DM nodes where DOCX needs them, but LaTeX's own class/package
settings remain the source of truth for typeset appearance in the LaTeX
pane. This is the only workable design because LaTeX and DOCX do not share
a layout model; they share a document structure model.

### Why not convert LaTeX text directly to DOCX XML and back?

A direct, pairwise converter (LaTeX to DOCX, DOCX to LaTeX, as two
unrelated code paths) is how most existing tools work, and it is why they
drift: round-tripping through two independent converters does not
guarantee `A -> B -> A == A`. Routing both formats through one shared DM
means the round-trip property is enforced by construction. If the DM
cannot represent something, neither side can silently invent it on the
way back.

## Sync flow

The two editor panes do not talk to each other directly. Every edit flows
through the DM:

1. User types in the LaTeX pane.
2. The frontend debounces keystrokes (see `docs/conventions.md` for the
   debounce window) and sends the full LaTeX source, or an incremental
   edit, to the `parse_latex` Tauri command.
3. The Rust core parses the changed region, produces an updated DM, and
   diffs it against the previous DM.
4. The core prints a fresh DOCX-pane view from the updated DM and returns
   both the new DM version id and the DOCX-pane render to the frontend.
5. The frontend updates the DOCX pane. The LaTeX pane is left as the user
   typed it (no reformatting mid-edit).
6. The reverse path (DOCX pane edit to LaTeX pane) is symmetric, using
   `apply_docx_edit` and the LaTeX printer.

Edits are versioned (`docs/contracts.md` has the `DocVersion` type) so a
slow parse that returns after a newer edit was already applied is
discarded rather than overwriting fresher state.

### Conflict and loss handling

Some LaTeX constructs have no DOCX equivalent (raw TeX macros, certain
package-specific commands) and some DOCX features have no clean LaTeX
equivalent (certain field codes, OLE-embedded objects). The DM has an
explicit `Opaque` node type for exactly this: content that round-trips
losslessly through the pane that produced it (byte-for-byte on export back
to its native format) but is shown as a visibly marked, non-editable block
in the other pane. The user is never shown a silently dropped or
silently guessed translation. `docs/specs.md` lists which constructs land
as first-class DM nodes versus `Opaque`.

## Crate and module boundaries (Rust side)

```
src-tauri/src/
├── main.rs                # Tauri app bootstrap, window setup
├── commands/               # thin IPC handlers only, no business logic
│   ├── mod.rs
│   ├── document.rs         # open/save/new/parse_latex/apply_docx_edit
│   └── export.rs           # export_pdf/export_markdown/export_txt/export_docx/export_latex
└── core/
    ├── mod.rs
    ├── model.rs             # the Document Model types
    ├── latex/
    │   ├── parser.rs
    │   ├── printer.rs
    │   └── highlight.rs     # Okabe-Ito token classification
    ├── docx/
    │   ├── reader.rs        # OOXML zip -> DM
    │   └── writer.rs        # DM -> OOXML zip
    ├── export/
    │   ├── pdf.rs
    │   ├── markdown.rs
    │   └── plaintext.rs
    └── diff.rs               # DM diffing for incremental pane updates
```

`commands/` is deliberately thin: it validates input, calls into `core/`,
and maps results to the `Result<T, TesseraError>` shape in
`docs/contracts.md`. This keeps all real logic unit-testable without
spinning up Tauri or a webview, which matters for CI speed across three
platforms.

## PDF export strategy

Two tiers, chosen automatically, with the choice visible to the user:

1. **System TeX engine**, if the core detects a working `pdflatex`,
   `xelatex`, or `lualatex` on `PATH` at export time. The core prints DM to
   LaTeX, writes it to a temp directory, and shells out locally. This is
   not a network call; it is a local subprocess the user already has
   installed; it remains fully offline.
2. **Built-in typesetter** (`core/export/pdf.rs`), used when no TeX engine
   is found. Produces a PDF directly from the DM for the supported subset
   in `docs/specs.md`. Lower typographic fidelity than real TeX, clearly
   labeled in the UI as "Basic PDF (no TeX engine found)."

Neither path downloads a TeX distribution. Tessera does not install or
manage TeX; that remains the user's responsibility, documented in the
README.

## Frontend architecture

React + TypeScript, no server-side rendering, no data fetching layer
beyond Tauri's `invoke`. State is held in a single document store
(`src/state/documentStore.ts`) that mirrors the DM version the backend
last confirmed. UI state (cursor position, scroll offset, pane focus) is
local component state and never sent to the backend unless needed for a
feature (for example, "scroll the other pane to the matching section").

Key frontend modules:

```
src/
├── main.tsx
├── App.tsx
├── editors/
│   ├── LatexEditor.tsx       # CodeMirror 6 instance, Okabe-Ito theme
│   └── DocxEditor.tsx        # ProseMirror or Tiptap instance
├── state/
│   └── documentStore.ts      # DM mirror, version tracking, undo stack
├── ipc/
│   └── commands.ts           # typed wrappers over Tauri invoke()
└── components/
    ├── SplitPane.tsx
    ├── ExportMenu.tsx
    └── Toolbar.tsx
```

`src/ipc/commands.ts` is the only file allowed to call `invoke()`
directly; every Tauri command has one typed wrapper function there, with
types generated from (or hand-kept in sync with, until codegen lands) the
Rust `#[tauri::command]` signatures. See `docs/contracts.md`.

## Why Tauri over Electron

Tauri ships the OS's native webview instead of bundling Chromium, which
keeps binaries an order of magnitude smaller and keeps the trusted compute
base in Rust rather than a Node.js backend. Tauri's permission model also
makes "no network access" an enforceable, auditable configuration
(`tauri.conf.json` capabilities) rather than a convention someone has to
remember not to violate.

## Storage model

A Tessera project is a directory, not a single opaque file:

```
my-paper/
├── main.tex          # LaTeX source, always kept in sync
├── main.docx          # DOCX view, always kept in sync
├── .tessera/
│   ├── project.json   # project metadata, DM version, pane sync state
│   └── assets/         # images and other embedded resources
```

Both `main.tex` and `main.docx` are real, standalone files openable by any
other LaTeX tool or Word-compatible application at any time, not just from
inside Tessera. `.tessera/project.json` is bookkeeping Tessera uses to
resume a session (last DM version, which pane was last edited) and is
safe to delete; Tessera reconstructs it from `main.tex` on next open if
missing.

## Testing strategy

See `docs/conventions.md` for the full policy. In short: the DM, the
LaTeX parser/printer, and the DOCX reader/writer each have property-based
round-trip tests (`parse(print(dm)) == dm` and the DOCX equivalent) in
addition to example-based tests, because round-tripping is the feature
this application exists to get right.
