# parallel-task08-B: Frontend shell

## Prerequisites

`serial-task07-tauri-command-layer` complete (so the shape of what the
toolbar and panes will eventually need to call is stable), though this
task's own components do not call `invoke()` directly and can be built
and visually tested with mock/placeholder data before
`parallel-task08-A`'s real IPC layer exists.

## Can run alongside

`parallel-task08-A` (frontend IPC layer and document store). This task
owns `src/App.tsx` and `src/components/`; `08-A` owns `src/ipc/` and
`src/state/`. Build this task's components to accept data and callbacks
as props, so wiring them to the real store in `serial-task09` is a
matter of passing real props in, not restructuring the components.

## Goal

Implement the non-editor UI shell: `App.tsx`, `SplitPane.tsx`,
`Toolbar.tsx`, and the layout/visual foundation `ExportMenu.tsx` will
sit in (the menu's actual export wiring is `serial-task10`'s job; this
task can build its static structure and visual design).

## Read first

- `.agents/skills/tessera-frontend-design.md` in full. This is the
  primary reference for this task; sections 1 (Layout), 2 (Color), 3
  (Typography), and 6 (Interaction details) apply most directly.
- `docs/specs.md` section 6, "UI behavior."
- `docs/conventions.md`, TypeScript/React style section.

## Files touched

```
src/App.tsx
src/components/SplitPane.tsx
src/components/Toolbar.tsx
src/components/ExportMenu.tsx (static structure/styling only; real
  export wiring is serial-task10)
src/App.test.tsx (new)
src/components/SplitPane.test.tsx (new)
```

## Steps

1. Implement `SplitPane.tsx`: resizable two-pane layout per
   `tessera-frontend-design.md` section 1, default 50/50, a real
   grabbable divider (8 to 10px hit target, 1px visible line) that
   resizes live on drag, per that skill's direct-manipulation
   requirement. Accept the two pane contents as children/props so
   `serial-task09` can drop the real editors in without modifying this
   component.
2. Implement `Toolbar.tsx`: single thin strip above both panes, holding
   export trigger, save, and a sync-status indicator placeholder (the
   real sync status wiring happens once `parallel-task08-A`'s store is
   connected in `serial-task09`; build the indicator's visual states
   now: editing/syncing/synced/error, per
   `tessera-frontend-design.md` section 5).
3. Implement the synced-scroll toggle control described in
   `tessera-frontend-design.md` section 1 as a toolbar control (the
   actual scroll-syncing behavior needs the real editor instances from
   `serial-task09`; this task wires up the toggle's UI state only).
4. Implement `ExportMenu.tsx`'s static structure: flat list, one item
   per format from `docs/specs.md` section 4, each showing format name
   plus a one-line note slot (for the PDF fallback caveat, wired for
   real in `serial-task10`).
5. Apply the color system (CSS custom properties) from
   `tessera-frontend-design.md` section 2 at the app root, including
   dark mode via `prefers-color-scheme`, and the typography scale from
   section 3.
6. Implement `prefers-reduced-motion` and `prefers-contrast` handling at
   the shell level per `tessera-frontend-design.md` section 7, even
   though most of the motion this affects (sync flashes, cross-reference
   scroll) is implemented in later tasks; the CSS custom properties and
   media query structure should exist now so later tasks only need to
   use them, not introduce them.
7. Write component tests for `SplitPane`'s resize behavior and
   `Toolbar`'s status indicator state transitions, using mock props, not
   a real backend connection.

## Definition of done

- [ ] `SplitPane` resizes live on drag, respects `--pane-min-width`,
      and accepts pane content via props/children without any
      assumption about what those panes contain.
- [ ] `Toolbar` renders all four sync-status visual states correctly
      from a prop, and the synced-scroll toggle's UI state works
      (behavior wiring comes later).
- [ ] `ExportMenu`'s static structure lists all format options from
      `docs/specs.md` section 4 with a note slot.
- [ ] Color system CSS custom properties match
      `tessera-frontend-design.md` section 2 exactly, including dark
      mode.
- [ ] `prefers-reduced-motion` and `prefers-contrast` media query
      structure exists at the shell level.
- [ ] `npm run typecheck`, `npm run lint`, and component tests pass.
- [ ] No file in this task's scope imports `@tauri-apps/api` or calls
      `invoke()` directly.
