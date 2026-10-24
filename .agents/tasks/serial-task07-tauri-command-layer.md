# serial-task07: Tauri command layer completion

## Prerequisites

All four of `parallel-task06-A` (highlighter), `parallel-task06-B` (PDF
export), `parallel-task06-C` (Markdown export), and `parallel-task06-D`
(plain text export) complete. `export`-related commands need all export
format renderers available to wire up.

## Goal

Finish `src-tauri/src/commands/export.rs` and confirm the complete
Tauri IPC surface from `docs/contracts.md` section 2 is implemented,
registered with the Tauri app, and consistent end to end. This task is
primarily integration and wiring, not new conversion logic; all the
conversion logic already exists from prior tasks.

## Read first

- `docs/contracts.md` section 2 in full: the complete command table.
- `docs/architecture.md`'s note that `commands/` is "thin IPC handlers
  only, no business logic."
- `docs/specs.md` section 4, the full export format and clipboard table.

## Files touched

```
src-tauri/src/commands/export.rs
src-tauri/src/commands/document.rs (final review/wiring pass only;
  the handlers themselves were implemented in serial-task05 and
  parallel-task06-A)
src-tauri/src/main.rs (register all commands with
  tauri::generate_handler!)
src-tauri/tests/commands_integration_tests.rs (new)
```

## Steps

1. Implement the `export` command's full dispatch across all five
   `ExportFormat` variants (`Latex`, `Docx`, `Pdf`, `Markdown`,
   `PlainText`), calling into the respective renderer from
   `parallel-task03-B`, `parallel-task03-D`, `parallel-task06-B`,
   `parallel-task06-C`, `parallel-task06-D`.
2. Implement both `ExportResult` variants: `WrittenToFile` (write to the
   `destination` path) and `ClipboardPayload` (return content for the
   frontend to hand to Tauri's clipboard API; this command itself does
   not touch the clipboard, per `docs/contracts.md`'s shape, where
   `content` is returned as data).
3. Confirm every command in `docs/contracts.md` section 2's table is
   registered in `main.rs`'s `tauri::generate_handler!` call; this is an
   easy thing to forget for a handler implemented in an earlier task and
   never actually wired up.
4. Write integration tests that exercise each command end to end (not
   through the webview, directly calling the command functions, per
   `docs/conventions.md`'s testing policy), including at least one
   full-cycle test: open a project, parse a LaTeX edit, export to each
   of the five formats, confirm all succeed.
5. Confirm error paths: an export of an unsupported or malformed state
   returns `TesseraError::ExportError` with a useful message, not a
   panic.
6. Run the full `cargo tree` network-capable-crate check across the
   entire `src-tauri` dependency tree now that every module is wired
   together, since this is the first point where the complete dependency
   set is assembled.

## Definition of done

- [ ] Every command in `docs/contracts.md` section 2's table is
      implemented, registered, and reachable from the frontend's typed
      `invoke` surface (even though the frontend itself is
      `parallel-task08-A`'s job, confirm the Rust side is ready for it).
- [ ] `export` dispatches correctly across all five formats and both
      `ExportResult` variants.
- [ ] A full open-edit-export integration test passes end to end for
      all five export formats.
- [ ] `cargo tree` across the full assembled dependency tree shows no
      network-capable crate.
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass
      for the full `src-tauri` crate, not just this task's own files.
