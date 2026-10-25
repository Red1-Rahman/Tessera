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
