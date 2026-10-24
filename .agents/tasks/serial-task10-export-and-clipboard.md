# serial-task10: Export and clipboard wiring

## Prerequisites

`serial-task09-editor-pane-integration` complete. The live document and
both editor panes must be fully working before wiring up export, since
export operates on whatever `Document` the store currently holds.

## Goal

Complete the real wiring behind `ExportMenu.tsx` (whose static structure
`parallel-task08-B` already built), connecting it to the `export`
command via `parallel-task08-A`'s IPC layer, including file-save dialogs
and clipboard behavior for all formats in `docs/specs.md` section 4.

## Read first

- `docs/specs.md` section 4 in full.
- `docs/contracts.md` section 2, `ExportRequest`/`ExportResult`.
- `.agents/skills/tessera-frontend-design.md` section 6, "Export menu"
  subsection.
- `.agents/skills/cross-platform-awareness.md`, "Clipboard and file
  dialogs," since this task is the primary place that applies.

## Files touched

```
src/components/ExportMenu.tsx (wiring, building on
  parallel-task08-B's static structure)
src/components/ExportMenu.test.tsx (new)
```

## Steps

1. Wire each export format option to call the `exportDocument` IPC
   wrapper from `parallel-task08-A`, using Tauri's native file-save
   dialog (via `@tauri-apps/api`, not a hand-rolled OS-specific
   implementation, per `.agents/skills/cross-platform-awareness.md`) for
   the `WrittenToFile` path.
2. Wire the "copy to clipboard" option (or per-format clipboard buttons,
   whichever `parallel-task08-B`'s static structure settled on) to use
   Tauri's clipboard API for the `ClipboardPayload` result, for every
   format that supports it per `docs/specs.md` section 4.
3. Implement the PDF fallback caveat display: when `exportDocument`'s
   result indicates the built-in typesetter path was used (per
   `parallel-task06-B`'s `ExportResult` extension), show "Basic PDF (no
   TeX engine found)" in the note slot `parallel-task08-B` built.
4. Implement error handling: a failed export (`TesseraError::
   ExportError`) surfaces as a clear, non-blocking message in the export
   menu itself, not a silent failure or a disruptive modal, consistent
   with the non-blocking principle applied elsewhere in
   `docs/specs.md` section 6.
5. Write tests covering each format's success path and the error path,
   mocking the IPC layer rather than requiring a real backend for these
   particular tests (full real-backend coverage is
   `serial-task11`'s e2e job).

## Definition of done

- [ ] All six export options (LaTeX, DOCX, PDF, Markdown, plain text,
      copy to clipboard, per `docs/specs.md` section 4, noting clipboard
      applies across multiple formats) are fully wired and functional.
- [ ] File-save dialogs use Tauri's native API, not a custom
      implementation.
- [ ] The PDF fallback caveat displays correctly when the built-in
      typesetter path is used.
- [ ] Export errors surface clearly without a blocking modal.
- [ ] `npm run typecheck`, `npm run lint`, and tests pass.
