# serial-task09: Editor pane integration

## Prerequisites

Both `parallel-task08-A` (frontend IPC layer and document store) and
`parallel-task08-B` (frontend shell) complete. This is the convergence
point: the real editors get built and wired to both the shell and the
store at once.

## Goal

Implement `src/editors/LatexEditor.tsx` (CodeMirror 6) and
`src/editors/DocxEditor.tsx` (ProseMirror or Tiptap), wire both into
`parallel-task08-B`'s `SplitPane`, and connect both to
`parallel-task08-A`'s `documentStore` and IPC wrappers, completing the
live bidirectional sync loop described in `docs/architecture.md`.

## Read first

- `docs/architecture.md`, "Sync flow" in full, this is the behavior
  being wired up end to end for the first time in this task.
- `.agents/skills/tessera-frontend-design.md` sections 2 (Color,
  specifically the DOCX pane mirroring the LaTeX pane's token colors), 4
  (Motion, the sync flash), and 5 (Sync status and warnings).
- `docs/specs.md` sections 5 and 6.
- `docs/techstack.yml`'s entries for `codemirror` and
  `prosemirror`/`tiptap`.

## Files touched

```
src/editors/LatexEditor.tsx
src/editors/DocxEditor.tsx
src/App.tsx (wiring only: connect editors to SplitPane and the store)
src/editors/LatexEditor.test.tsx (new)
src/editors/DocxEditor.test.tsx (new)
tests/e2e/sync.spec.ts (new, basic Playwright smoke test; full e2e
  coverage is serial-task11's job, but a basic round-trip smoke test
  belongs here to confirm the wiring actually works before moving on)
```

## Steps

1. Implement `LatexEditor.tsx` as a CodeMirror 6 instance with an
   Okabe-Ito theme built from `parallel-task06-A`'s
   `get_highlight_tokens` command (via `parallel-task08-A`'s typed IPC
   wrapper), applying the token-to-color mapping from
   `docs/conventions.md`.
2. Implement `DocxEditor.tsx` as a ProseMirror (or Tiptap) instance
   rendering the structured `Document` from the store, with semantic
   highlight colors mirroring the LaTeX pane's token categories per
   `tessera-frontend-design.md` section 2's requirement that both panes
   use "the same eight colors for the same concepts."
3. Wire `LatexEditor`'s change events through the debounce constant from
   `documentStore.ts` to `parseLatex`, per `docs/architecture.md`'s sync
   flow steps 1 to 5: debounce, call the command, receive
   `DocSyncResult`, update the DOCX pane, leave the LaTeX pane as typed.
4. Wire `DocxEditor`'s structured edits (as `DmPatch` operations, per
   `docs/contracts.md`) through the equivalent debounce to
   `applyDocxEdit`, symmetric to step 3.
5. Implement the sync flash motion from
   `tessera-frontend-design.md` section 4 on the receiving pane after
   each successful sync, using the diff information from
   `serial-task05`'s `core/diff.rs` (surfaced through `DocSyncResult`).
6. Implement `OpaqueBlock` rendering in `DocxEditor.tsx` per
   `tessera-frontend-design.md` section 2's "Opaque blocks" subsection:
   tinted, bordered, `cursor: default`, non-editable.
7. Implement `SyncWarning` inline annotations in both panes per
   `tessera-frontend-design.md` section 5 and `docs/specs.md` section 6:
   anchored to the affected block, non-blocking, never a modal.
8. Implement cross-reference click behavior (`Inline::CrossRef`):
   scroll-into-view in both panes plus the same highlight flash language
   as a sync update, per `tessera-frontend-design.md` section 6.
9. Implement the synced-scroll toggle's actual behavior now that real
   editor instances exist (the toggle's UI was built in
   `parallel-task08-B`).
10. Write a basic Playwright smoke test: type in the LaTeX pane, confirm
    the DOCX pane updates; edit the DOCX pane, confirm the LaTeX pane
    updates. This does not need to be exhaustive, `serial-task11` owns
    full e2e coverage; this task's test exists to confirm the wiring
    itself works before moving on.

## Definition of done

- [ ] Editing the LaTeX pane updates the DOCX pane within the debounce
      window, and vice versa, confirmed by the smoke test.
- [ ] LaTeX pane highlighting uses the exact Okabe-Ito mapping from
      `docs/conventions.md`.
- [ ] DOCX pane semantic highlighting uses the same color-to-concept
      mapping as the LaTeX pane.
- [ ] Sync flash motion fires on the receiving pane after each
      successful sync and respects `prefers-reduced-motion`.
- [ ] `OpaqueBlock` renders as specified: tinted, bordered, inert.
- [ ] `SyncWarning` renders as a non-blocking inline annotation, never a
      modal, in both panes.
- [ ] Cross-reference clicks scroll both panes and flash consistently.
- [ ] Stale-version responses (from `parallel-task08-A`'s store logic)
      are correctly discarded during real editing, not just in unit
      tests with mocked timing.
- [ ] `npm run typecheck`, `npm run lint`, component tests, and the
      smoke test all pass.
