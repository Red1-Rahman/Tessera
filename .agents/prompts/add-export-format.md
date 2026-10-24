# Prompt: Add a new export format

Use this when the task is adding support for exporting the Document
Model to a format not currently in `docs/specs.md` section 4 (LaTeX,
DOCX, PDF, Markdown, plain text, and clipboard variants of each are
already supported; this prompt is for something beyond that list, for
example HTML or EPUB).

## Steps

1. **Confirm scope with `docs/specs.md` and a human first.** Adding an
   export format is a user-facing feature decision, not a pure
   implementation task. Do not start writing code before this is agreed,
   since it changes `ExportFormat` in `docs/contracts.md`, a type the
   frontend also depends on.

2. **Extend the contract.**
   - Add the new variant to `ExportFormat` in `docs/contracts.md` section
     2, and to the matching Rust enum in
     `src-tauri/src/commands/export.rs` (or wherever `ExportFormat` is
     defined per the current codebase layout; check
     `docs/codebase.txt`).
   - Update `src/ipc/commands.ts`'s TypeScript mirror of `ExportFormat`
     in the same change. These two must never drift (see
     `docs/contracts.md`, "Frontend-side types").

3. **Implement the renderer.**
   - New module under `src-tauri/src/core/export/`, following the
     pattern of `markdown.rs` and `plaintext.rs`: a pure function taking
     `&Document` and returning `Result<String, TesseraError>` (or bytes,
     if the format is binary).
   - Decide and document how each Document Model node type renders in
     the new format, especially `Opaque` blocks (never silently drop
     them; render some visible marker, matching the convention in
     `markdown.rs` of a fenced comment noting what was dropped).

4. **Wire into the export command.**
   - `src-tauri/src/commands/export.rs`: add the match arm for the new
     `ExportFormat` variant.
   - Frontend `ExportMenu.tsx`: add the new option.

5. **Tests:**
   - Example-based tests rendering a representative fixture document
     (ideally reuse `tests/fixtures/`) to the new format and snapshot
     the output.
   - A test confirming `Opaque` blocks produce a visible marker, not
     silent omission.

6. **Documentation, in the same change:**
   - `docs/specs.md` section 4: add a row to the export formats table.
   - `docs/contracts.md`: confirm the `ExportFormat` enum documentation
     matches the final implementation.
   - `README.md`: update the "Core features" export format list if this
     is a user-facing addition likely to be mentioned there.

7. Run the full checklist in `AGENTS.md`, "Before opening a pull
   request."

## Things to watch for

- Binary formats need a different `ExportResult` handling path than text
  formats; check whether `ClipboardPayload` in `docs/contracts.md` needs
  a variant for binary content, or whether clipboard export should simply
  be unavailable for this format (document that choice explicitly in
  `docs/specs.md` rather than leaving it to be discovered at runtime).
- Keep the renderer itself free of any Tauri-specific types; it should be
  testable as a plain Rust function on a `Document`, matching the
  thin-commands-layer rule in `docs/conventions.md`.
