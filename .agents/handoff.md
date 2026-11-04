# Agent handoff log

Running session notes for whoever picks up work in this repository
next. Each entry is one session (or one unit of assigned work). The
newest entry goes at the top, directly under this introduction, under a
heading in the form `## YYYY-MM-DD: short title`.

Add an entry at the end of a session when something the next agent
needs is not already recorded in the code, `docs/`, or the commit
messages. Worth recording: machine or toolchain changes, decisions and
their rationale, anything not verified locally, and work deliberately
left for a human. Skip an entry when the session changed nothing the
commits do not already explain. Keep entries factual and specific, with
file paths and commands, and no em dashes (see `docs/conventions.md`,
"Documentation style").

Session state belongs here; durable rules do not. A lesson that
applies beyond this repository and this session goes to
`.agents/skills/lessons-learned.md` as a tip instead, so this log stays
history and that file stays rules.

---

## 2026-11-04: Task 08-A frontend IPC and document store

### Changes

- Completed Task 08-A in commit `60e6c12`
  (`frontend(task08B): add IPC commands and document store with sync functionality`).
- Added the typed Tauri command wrappers and frontend contract types in
  `src/ipc/commands.ts`. This is the only frontend file that imports
  `@tauri-apps/api/core`.
- Added `src/state/documentStore.ts` with project loading, subscriptions,
  stale sync-result rejection, undo and redo stacks, and the two 250ms
  debounce constants required by `docs/conventions.md`.
- Added a restore-sync option for undo/redo follow-up results. Restore
  results update the document and store version without adding to or clearing
  either history stack.
- Kept the store version monotonic across `undo()` and `redo()`, while
  preserving the restored document's own version.
- Added `MAX_UNDO_DEPTH = 100` and bounded normal sync history to that
  depth. The no-current-document path does not add an undo entry.
- Added unit tests for every command wrapper, stale and out-of-order sync
  responses, listener notifications, undo/redo behavior, restore syncs,
  stale responses after undo, redo clearing, and undo depth limits.

### Verification and remaining limitations

- `npm test -- --run` passed: 3 test files and 20 tests.
- `npm run typecheck` passed.
- `npm run lint -- --quiet` passed.
- `git diff --check` passed before the commit.
- `npm run format:check` still reports formatting warnings across the
  repository, including the newly added frontend files. No formatting-only
  repository-wide rewrite was made.
- Task 08-B frontend shell work remains separate and can proceed without
  changing `src/ipc/` or `src/state/`.

---

## 2026-11-04: Task 07 command layer completion

### Changes

- Registered `commands::export::export` and
  `commands::document::get_highlight_tokens` in `src-tauri/src/lib.rs`.
- Added session helpers shared by each document/export Tauri handler and
  `src-tauri/tests/commands_integration_tests.rs`. The tests cover
  new/open/edit/save/highlight, file export to all five formats, a
  Markdown clipboard payload, and export error paths.
- Clarified in `docs/specs.md` that only text formats return clipboard
  payloads. DOCX and PDF require a file destination.
- Added `ProjectSession::assets_dir()` for the export command.
- Reconciled `Cargo.lock` with the existing `printpdf = "=0.7.0"`
  dependency. No crate was added. Kept printpdf 0.7 because the PDF
  implementation uses that API, the cached 0.11.3 source requires Rust
  1.88, and the project declares Rust 1.77.2. The 0.11.1 source was not
  available locally, and network access is prohibited.

### Verification and remaining limitations

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` and
  `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --
  -D warnings` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed: 40 unit,
  2 command integration, 14 contract, 8 Markdown, 7 PDF, 4 plain text,
  4 highlighting, 26 model, and 7 sync tests, plus doc tests.
- `cargo tree --manifest-path src-tauri/Cargo.toml` passed. Searches of
  the resolved tree found no `reqwest`, `hyper`, `ureq`, `curl`, `isahc`,
  or `tokio-tungstenite` packages.
- `cargo-deny` is not installed. The direct `printpdf 0.7.0` manifest
  declares MIT. A local scan found license expressions for 311 of the 471
  locked registry packages. The other 160 package sources were not
  cached, so a complete transitive license audit remains pending. Offline
  `cargo metadata` could not load `android_system_properties`.
- The local environment has no TeX engine. The system TeX subprocess path
  was not run end to end. The fallback PDF path passed its tests.
- `src/ipc/commands.ts` does not exist yet. Frontend wrappers remain in
  Task 08 scope. `docs/techstack.yml` still lists `pulldown-cmark`, while
  the Markdown renderer is hand-rolled; this drift was left unchanged.

---

## 2026-11-04: Task 06-B PDF export follow-up

### Changes

- Fixed the system TeX temporary layout to copy project assets beside
  `main.tex`, preserving the paths emitted by the LaTeX printer.
- Added tests for relative asset placement and temporary build directory
  cleanup.

### Verification and remaining environment issue

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, Clippy, and
  the full Rust test suite passed after the lockfile was reconciled during
  Task 07. The PDF fallback tests cover both repository LaTeX fixtures.
- `cargo tree` passed with no matches for the network-capable crate names
  listed above. `cargo-deny` is not installed; a complete transitive
  license audit remains pending.
- `docs/techstack.yml` still lists `pulldown-cmark` although Markdown
  export is hand-rolled. This drift was left unchanged.

---

## 2026-11-04: Task 06-D Plain text export gap completion

### Changes

- Added `src-tauri/tests/export_plaintext_tests.rs` asserting no Markdown-like
  markup characters (`**`, `*`, `<u>`, `` ` ``, `[@key]`) exist in plain text output,
  and testing level 1 (`=`) and level 2 (`-`) heading underlines, list indentation (`(1) `, `  `),
  table formatting, equations, figures, and opaque block rendering.
- Updated `docs/specs.md` section 4 to document exact plain text heading underline conventions,
  list indentation formatting, and opaque block output format.
- Updated `docs/codebase.txt` to include `export_plaintext_tests.rs`.

### Verification

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed.

## 2026-11-04: Task 06-C Markdown export gap completion

### Changes

- Added `src-tauri/tests/export_markdown_tests.rs` covering all Document
  Model block and inline node types, Math mode conventions (`$...$` / `$$...$$`),
  `OpaqueBlock` comments, table cell span flattening, lists, code blocks,
  and footnotes.
- Updated `docs/specs.md` section 4 to record the table cell span flattening
  convention (col_span repeats content, row_span leaves empty continuation cells)
  and aligned the `OpaqueBlock` rendering description with HTML comment output.
- Updated `docs/codebase.txt` to include `export_markdown_tests.rs`.

### Verification

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed.

## 2026-11-02: Review fixes for parser and exports

### Changes

- Made inline `\verb` delimiter and closing-span arithmetic UTF-8 safe.
- Added strict and lenient shared tokenizer modes. Strict parsing still
  reports malformed input, while highlighting emits `Error` tokens and
  continues.
- Replaced highlighter UTF-16 offset storage with a byte-indexed vector
  that only returns offsets for valid character boundaries.
- Updated Markdown labels, cross-references, code fences, list
  continuation indentation, and escaping for dollar signs, at signs, and
  list markers.
- Renamed shadowing bindings in plain-text export and restored distinct
  level 1 and level 2 heading underlines.

### Verification

- `cargo fmt --manifest-path src-tauri/Cargo.toml` passed.
- `cargo check --manifest-path src-tauri/Cargo.toml --tests` passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --
  -D warnings` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed: 38 unit
  tests, 14 contract tests, 4 highlighter integration tests, 26 model
  tests, 7 sync tests, and doc tests.
- The highlighter work was committed as `2307b2e` and
  `385c0dd`. Markdown and plain-text exporters were committed as
  `ac8e993`, `a86c972`, and integration fix `f08a843`. No Copilot
  co-author trailer was added.

## 2026-11-02: Task 04 round-trip contract suite

### Task 04 completion

- Added `src-tauri/tests/contracts.rs` with JSON shape snapshots,
  LaTeX and DOCX golden fixture tests, Opaque byte-exact checks, and
  `proptest` round-trip properties.
- Added accepted snapshots under `src-tauri/tests/snapshots/` and the
  persisted property-test seed file
  `src-tauri/tests/contracts.proptest-regressions`.
- Fixed LaTeX canonical printing and parsing for trailing trivia,
  escaped text, inline math at paragraph start, line breaks, and verbatim
  or listing delimiters.
- Fixed DOCX read-back for leading Title and Subtitle metadata, line
  breaks, and `No Spacing` code paragraphs. DOCX property generators now
  cover only representations that preserve their DM semantics.
- Added the documented DOCX metadata mapping to `docs/specs.md`.

### Verification

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --
  -D warnings` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed: 27 unit
  tests, 26 model tests, 14 contract tests, and all doc tests.
- No `.snap.new` files remain. Manual Word or LibreOffice visual
  verification remains unavailable on this machine, as noted in the
  previous handoff entry.

## 2026-10-25: Task 03 recovery and verification

### Schema alignment

- The follow-up's claim that the source-fidelity fields were already in
  `docs/contracts.md` was not true on disk. The contract and
  `src-tauri/src/core/model.rs` now both include `Document.trailing_trivia`,
  `DocMetadata.preamble_raw`, per-block trivia, leading trivia for tables,
  figures, and equations, `TableStyle`, and `RuleStyle`.
- New optional fidelity fields default when deserializing older JSON.
  `src-tauri/tests/model_tests.rs` covers a serde round trip for the new
  data. Constructors preserve their earlier signatures; setters/accessors
  manage uninterpreted source strings.

### Task 03 status and verification

- Fixed `src-tauri/src/core/docx/writer.rs`: `io_error` had accidentally
  been nested inside `zip_error` after the latter's return expression,
  leaving the closing brace misplaced. Moved `io_error` to module scope.
- Fixed compile/lint defects in DOCX reader and LaTeX parser code. Added a
  DOCX package structure test and a LaTeX printer test for `TableStyle`.
- `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo fmt
  --manifest-path src-tauri/Cargo.toml --check`, `cargo clippy
  --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and
  `cargo test --manifest-path src-tauri/Cargo.toml` passed at the final run.
  The full Rust suite contains six unit tests and 26 model tests; coverage
  is much narrower than the four task definitions require.
- `cargo tree --manifest-path src-tauri/Cargo.toml` had no matches for
  `reqwest`, `hyper`, `ureq`, `tokio-tungstenite`, `isahc`, or `curl`.
- Neither LibreOffice nor Word was found on PATH. No real-world DOCX was
  available for visual validation, so Task C and Task D's manual-open
  checklist items remain unverified.
- Task A still lacks multi-file provenance and broad construct/error-span
  tests. Trivia fields now exist, but parser preservation is incomplete for
  per-block comments and whitespace. Task B uses stored table column specs
  and rule style, and its LaTeX-origin Opaque test is byte-oriented, but
  full byte-identical round trips are not established. Task C's vertical
  merge handling is incomplete and DOCX inline formatting/style coverage
  needs review. Task D's OOXML has structural test coverage only; unsupported
  math warnings exist, but OMML conversion is minimal and not visually
  validated.
- `docs/contracts.md` was updated to match the explicit schema supplied in
  the follow-up. `docs/specs.md` now states canonical LaTeX spellings for
  printer normalization. No files were staged or committed.

## 2026-10-24: serial-task01 (scaffolding) and serial-task02 (Document Model) complete

### Environment and dependencies

- Installed Rust 1.99.1 via `rustup` (was missing entirely on this
  machine; `cargo` is at `%USERPROFILE%\.cargo\bin`, not on the default
  `PATH`).
- Installed Visual Studio 2022 Build Tools with the C++ workload via
  `winget`. The first install attempt died mid-download; a forced
  reinstall (`winget install --force`) succeeded. `winget` now lists
  BuildTools as installed.
- Installed frontend dependencies with `npm install`
  (`package-lock.json` is committed, so `npm ci` works from here on).

### Schema change: `Inline` variants

- In `src-tauri/src/core/model.rs`, the `Inline` enum's six newtype
  variants (`Text(String)`, `Bold(Vec<Inline>)`, `Italic`, `Underline`,
  `Code(String)`, `Footnote(Vec<Inline>)`) became struct variants
  (`Text { text }`, `Bold { inline }`, `Code { text }`,
  `Footnote { inline }`, and the italic/underline equivalents).
- Rationale: serde's internal tagging (`#[serde(tag = "type")]`) fails
  at runtime on a newtype variant over a string or sequence, so the
  shape previously documented in `docs/contracts.md` could never
  serialize. Struct variants fix that while keeping the `type` tag the
  frontend discriminates on.
- Variant names are unchanged, so `docs/specs.md` needed no edits. The
  change and its rationale are recorded in `docs/contracts.md`,
  section 1's implementation note.

### Architecture decisions

- `TesseraError`, `Pane`, `SourceSpan`, and `ExportFormat` live in
  `src-tauri/src/core/error.rs` and are re-exported from
  `src-tauri/src/commands/mod.rs`, rather than being defined in
  `commands/` as section 2 originally read. `core/` returns
  `TesseraError` from its constructors and must not depend on the IPC
  layer; `docs/contracts.md` section 2 was updated in the same commit.
- Document Model structs (`Document`, `DocMetadata`, `Author`,
  `Table`, `TableCell`, `Figure`, `Equation`, `OpaqueBlock`) have
  private fields with public getters and validating constructors, so
  no struct literal can bypass validation. The JSON wire format is
  unchanged; `docs/contracts.md` section 1 documents the rule.

### Not verified / left for a human

- The three `.github/workflows/` CI workflows have never run; they
  trigger on push, and nothing was pushed.
- `npm run tauri dev` was not run interactively. The equivalent
  startup path is verified: the debug binary from
  `npm run tauri build -- --debug` launches and shows a window, and
  MSI plus NSIS bundles are produced.

### Odds and ends

- `npx tauri icon` generated `src-tauri/icons/android` and
  `src-tauri/icons/ios` subfolders alongside the desktop icons. They
  are unused (this is a desktop app) and safe to delete if they bother
  anyone.
- `src-tauri/Cargo.lock` is committed for reproducible builds, as
  `docs/codebase.txt` describes.
