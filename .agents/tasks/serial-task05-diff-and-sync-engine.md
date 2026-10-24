# serial-task05: Diff and sync engine

## Prerequisites

`serial-task04-roundtrip-contract-tests` complete. The parser, printer,
reader, and writer must all be correct and mutually consistent before
building the sync layer on top of them; a sync bug on top of an
unverified conversion bug is much harder to diagnose than either in
isolation.

## Goal

Implement `src-tauri/src/core/diff.rs` and the document-editing parts of
`src-tauri/src/commands/document.rs`: the pieces that turn a pane edit
into an updated Document Model, a diff for the receiving pane, and a
version-checked response, per `docs/architecture.md`'s "Sync flow" and
`docs/contracts.md` section 2.

## Read first

- `docs/architecture.md`, "Sync flow" and "Conflict and loss handling"
  in full.
- `docs/contracts.md` section 2, especially `LatexEditRequest`,
  `DocxEditRequest`, `DmPatch`, `DocSyncResult`, `SyncWarning`,
  `StaleVersion`.
- `.agents/skills/rust-conventions.md` for module placement (this logic
  belongs in `core/`, with `commands/document.rs` staying a thin
  wrapper).

## Files touched

```
src-tauri/src/core/diff.rs
src-tauri/src/commands/document.rs (open_project, new_project,
  parse_latex, apply_docx_edit, save_project handlers)
src-tauri/src/commands/mod.rs (wiring only)
src-tauri/tests/sync_tests.rs (new)
```

## Steps

1. Implement `DocVersion` comparison logic: a request with
   `base_version` older than the core's current version is rejected with
   `TesseraError::StaleVersion`, not silently applied. This is the
   mechanism `docs/architecture.md` describes for discarding a slow
   parse that returns after a newer edit was already applied.
2. Implement `core/diff.rs`: given an old `Document` and a new
   `Document`, produce whatever internal diff representation the
   receiving pane needs to update only the changed region (not a full
   re-render), feeding into the frontend motion behavior described in
   `.agents/skills/tessera-frontend-design.md`'s sync flash. The diff
   does not need to be exposed as its own contract type if
   `DocSyncResult` already carries enough information for the frontend
   to compute what changed; check `docs/contracts.md` before adding a
   new field, and follow `.agents/skills/contract-discipline.md` if you
   do add one.
3. Implement `parse_latex` command handler: take a
   `LatexEditRequest`, call `parallel-task03-A`'s parser, version-check,
   produce a new `Document`, call `parallel-task03-B`'s printer only if
   needed for round-trip verification (not for the response, since the
   LaTeX pane keeps what the user typed per `docs/architecture.md`), and
   return a `DocSyncResult` including the DOCX-pane-relevant diff.
4. Implement `apply_docx_edit` command handler: take a `DocxEditRequest`
   (a list of `DmPatch` operations, not full text), apply them to the
   current `Document`, version-check, call
   `parallel-task03-B`'s printer to produce `latex_render` for
   `DocSyncResult`.
5. Implement construct-level loss detection: when a `DmPatch` or parsed
   edit would require information the DM cannot hold without becoming
   `Opaque`, or when a sync introduces unresolvable ambiguity (see
   `docs/architecture.md`, "Conflict and loss handling"), produce a
   `SyncWarning` rather than silently resolving it one way.
6. Implement `open_project`, `new_project`, `save_project` per
   `docs/contracts.md` section 2's table, including the
   `.tessera/project.json` read/write and `main.tex`/`main.docx` file
   I/O described in `docs/architecture.md`'s storage model.
7. Write integration tests simulating a sequence of alternating LaTeX
   and DOCX pane edits, confirming the DM stays consistent, stale
   versions are rejected, and a deliberately loss-prone edit produces a
   `SyncWarning` rather than silent data loss.

## Definition of done

- [ ] Stale `base_version` requests are rejected with
      `TesseraError::StaleVersion`, tested explicitly.
- [ ] `parse_latex` and `apply_docx_edit` both return correct
      `DocSyncResult` values, tested against multi-step edit sequences,
      not just single isolated edits.
- [ ] A loss-prone edit produces a `SyncWarning`, never silent data
      loss, tested explicitly with a case designed to trigger one.
- [ ] `open_project`/`new_project`/`save_project` correctly read/write
      `main.tex`, `main.docx`, and `.tessera/project.json`, including
      the "missing project.json is reconstructed from main.tex"
      behavior described in `docs/architecture.md`.
- [ ] `commands/document.rs` handlers remain thin; diffing and edit
      application logic lives in `core/diff.rs` and is independently
      unit-testable without Tauri.
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
