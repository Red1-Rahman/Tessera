# parallel-task08-A: Frontend IPC layer and document store

## Prerequisites

`serial-task07-tauri-command-layer` complete. The full Rust command
surface must exist and be stable before writing typed wrappers against
it.

## Can run alongside

`parallel-task08-B` (frontend shell). This task does not touch
`src/components/` or `src/editors/`; `08-B` does not touch `src/ipc/` or
`src/state/`. They can be developed concurrently and will be joined in
`serial-task09`.

## Goal

Implement `src/ipc/commands.ts` (the only file allowed to call Tauri
`invoke()` directly, per `docs/conventions.md`) and
`src/state/documentStore.ts` (the single store mirroring the backend's
Document Model version).

## Read first

- `docs/contracts.md` section 2 in full, "Frontend-side types"
  subsection especially.
- `docs/conventions.md`, TypeScript/React style section, especially the
  rule that no component may import `@tauri-apps/api` directly.
- `docs/architecture.md`'s frontend architecture section.
- `.agents/skills/contract-discipline.md`, since this task is the
  primary place TypeScript/Rust type drift would be introduced if not
  careful.

## Files touched

```
src/ipc/commands.ts
src/state/documentStore.ts
src/ipc/commands.test.ts (new)
src/state/documentStore.test.ts (new)
```

## Steps

1. Write TypeScript types mirroring every type in `docs/contracts.md`
   section 2, field-for-field, matching casing exactly (no case
   conversion, per `docs/contracts.md`'s note on `serde` configuration).
2. Write one typed wrapper function per Tauri command in
   `docs/contracts.md` section 2's table: `openProject`, `newProject`,
   `parseLatex`, `applyDocxEdit`, `saveProject`, `exportDocument`,
   `getHighlightTokens`. Each wraps `invoke()` and returns a typed
   `Promise`, converting the tagged `TesseraError` shape into a
   TypeScript discriminated union the rest of the frontend can
   pattern-match on.
3. Implement `documentStore.ts`: holds the current `Document`,
   `DocVersion`, and the debounce constants
   (`LATEX_REPARSE_DEBOUNCE_MS`, `DOCX_SYNC_DEBOUNCE_MS`, both 250ms per
   `docs/conventions.md`, defined once here and imported everywhere
   else, never duplicated as a literal).
4. Implement the version-check logic on the frontend side matching
   `docs/architecture.md`'s "Sync flow": discard a response whose
   `new_version` is older than or equal to a version already applied, to
   handle the case where a slow backend response for an older edit
   arrives after a newer one already landed.
5. Implement an undo/redo stack on top of the `Document` version history
   if not already specified elsewhere; if this is genuinely new scope
   beyond what `docs/contracts.md` and `docs/architecture.md` describe,
   flag it rather than inventing backend support silently, since undo
   semantics across two synced panes may need a contract addition.
6. Write unit tests for the store's version-discarding logic and for
   each IPC wrapper's success and error-mapping paths (mock the
   underlying `invoke()` call, do not require a running Tauri backend
   for these tests, per `docs/conventions.md`'s "no test may require
   network access," and more generally keep these fast and
   backend-independent).

## Definition of done

- [ ] Every type in `docs/contracts.md` section 2 has a matching
      TypeScript type in `commands.ts`, field-for-field.
- [ ] Every command has exactly one typed wrapper function; no other
      file imports `@tauri-apps/api` directly (this becomes enforceable
      by `serial-task09` and later tasks, verify it is true at the end
      of this task for the files that exist so far).
- [ ] `documentStore.ts` correctly discards stale `DocVersion` responses,
      tested explicitly with an out-of-order-arrival simulation.
- [ ] Debounce constants are defined exactly once in this file and
      nowhere else.
- [ ] `npm run typecheck` and `npm run lint` pass.
- [ ] Unit tests pass without requiring a running backend.
