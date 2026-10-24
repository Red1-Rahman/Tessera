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
