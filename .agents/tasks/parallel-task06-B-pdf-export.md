# parallel-task06-B: PDF export

## Prerequisites

`serial-task05-diff-and-sync-engine` complete.

## Can run alongside

`parallel-task06-A` (highlighter), `parallel-task06-C` (Markdown
export), `parallel-task06-D` (plain text export).

## Goal

Implement `src-tauri/src/core/export/pdf.rs`: the two-tier PDF export
strategy described in `docs/architecture.md`, "PDF export strategy," and
`docs/specs.md` section 4's PDF row.

## Read first

- `docs/architecture.md`, "PDF export strategy" in full.
- `docs/specs.md` section 4.
- `.agents/skills/cross-platform-awareness.md`, "System TeX engine
  detection" and "Process execution," since this module is the primary
  place those concerns apply.
- `docs/techstack.yml`'s entry for `printpdf`, the built-in typesetter
  fallback crate.
- `.agents/prompts/add-export-format.md` for the general pattern, though
  this format already has its contract defined, so this task is mostly
  that prompt's steps 3 and 5.

## Files touched

```
src-tauri/src/core/export/pdf.rs
src-tauri/src/commands/export.rs (PDF arm of the export command)
src-tauri/tests/export_pdf_tests.rs (new)
```

## Steps

1. Implement system TeX engine detection: check for `pdflatex`,
   `xelatex`, or `lualatex` on `PATH` using a cross-platform resolution
   method (a crate like `which`, per
   `.agents/skills/cross-platform-awareness.md`), not a hardcoded list
   of per-platform install paths.
2. If found: print the current `Document` to LaTeX via
   `parallel-task03-B`'s printer, write to a temp directory (via
   `std::env::temp_dir()`, not a hardcoded path), shell out to the
   detected engine using `std::process` with an argument array (never a
   shell string), and return the resulting PDF.
3. If not found: implement the built-in typesetter path using
   `printpdf`, producing a PDF directly from the `Document` for the
   supported subset in `docs/specs.md`. This does not need to match TeX
   engine output quality; it needs to be correct for the documented
   subset and clearly distinguishable from the TeX-engine path's output
   in the UI layer (expose which path was used in the `ExportResult` or
   a related field so `serial-task10`'s export menu can show "Basic PDF
   (no TeX engine found)" per `docs/specs.md`).
4. Ensure temp files are cleaned up after export, on both success and
   failure paths.
5. Write tests for: engine detection logic (mock or stub `PATH`
   resolution if full subprocess testing is impractical in CI), the
   built-in typesetter path producing valid PDF bytes for a range of
   fixture documents, and confirming no code path in this module opens
   a network connection (this module is a natural place for someone to
   accidentally reach for a network-based typesetting service; it must
   not).

## Definition of done

- [ ] System TeX engine detection works cross-platform without
      hardcoded install paths.
- [ ] TeX engine subprocess invocation uses argument arrays, not shell
      strings.
- [ ] Built-in typesetter fallback produces valid PDF output for the
      documented subset when no engine is found.
- [ ] The export result communicates which path was used, for the UI to
      surface per `docs/specs.md`.
- [ ] Temp files are cleaned up in both success and failure paths.
- [ ] No network-capable crate appears in this module's dependencies
      (`cargo tree` check per `docs/conventions.md`).
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
