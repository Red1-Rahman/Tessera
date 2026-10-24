# serial-task11: End-to-end tests and cross-platform CI verification

## Prerequisites

`serial-task10-export-and-clipboard` complete. The full application
(sync, highlighting, export, clipboard) must be feature-complete for v1
before comprehensive e2e coverage and cross-platform verification are
meaningful.

## Goal

Build out full Playwright end-to-end coverage in `tests/e2e/` and verify
the application behaves identically across Windows, macOS, and Linux via
the three CI workflows in `.github/workflows/`, closing any
platform-specific gaps found.

## Read first

- `.agents/skills/cross-platform-awareness.md` in full, this is the
  checklist for everything this task should be actively hunting for.
- `docs/conventions.md`, Testing section.
- `.github/workflows/build-linux.yml`, `build-macos.yml`,
  `build-windows.yml`.

## Files touched

```
tests/e2e/*.spec.ts (expand beyond serial-task09's basic smoke test)
.github/workflows/*.yml (fix only if a genuine CI configuration bug is
  found; this task is not expected to redesign the workflows)
```

## Steps

1. Expand `tests/e2e/` to cover: full open-edit-sync-export flow for
   each export format, multi-file LaTeX project open/save
   (`docs/specs.md` section 3), `Opaque` block handling round trip (open
   a `.docx` with unsupported content, confirm it renders as `Opaque`,
   confirm re-export preserves it), and the undo/redo behavior from
   `parallel-task08-A` if that scope was implemented.
2. Run the full e2e suite on all three platforms via the existing CI
   workflows. Do not rely on only running it locally on one platform, per
   `.agents/skills/cross-platform-awareness.md`'s CI verification
   section.
3. Investigate and fix any platform-specific failure. Common categories
   to check first, per `cross-platform-awareness.md`: path separator
   assumptions, case-sensitivity assumptions, line-ending handling in
   round-trip tests, the TeX engine detection path from
   `parallel-task06-B`, and Tauri file-dialog/clipboard behavior
   differences.
4. Confirm the network-capable-dependency check in each workflow
   (`docs/conventions.md`, "Network ban") still passes with the final,
   complete dependency tree.
5. Confirm binary sizes and build times are reasonable on all three
   platforms; this is not a hard gate but worth noting if something has
   grown unexpectedly (for example, an export format implementation
   accidentally bundling a large asset).
6. Document any platform-specific behavior difference that turned out to
   be intentional and unavoidable (rare, but possible for things like
   native file dialogs looking different) in a short note in
   `docs/architecture.md` or `docs/conventions.md` as appropriate, so it
   is not mistaken for a bug by a future contributor.

## Definition of done

- [ ] Full e2e suite passes on all three platforms via CI, not just
      locally.
- [ ] Every platform-specific failure found has been fixed, not worked
      around with a platform-specific skip unless genuinely justified
      and documented.
- [ ] Network-capable-dependency checks pass with the final dependency
      tree on all three workflows.
- [ ] Any intentional, unavoidable platform difference is documented.
- [ ] `workflow_dispatch` manual trigger confirmed working on all three
      workflows (not just the path-triggered automatic runs).
