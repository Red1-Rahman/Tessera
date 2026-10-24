# serial-task01: Project scaffolding

## Prerequisites

None. This is the first task.

## Goal

Stand up the repository skeleton so every later task has a real place to
put code: a buildable (even if nearly empty) Tauri app, a working Rust
workspace, a working frontend build, CI that runs, and licensing in
place. Nothing in this task implements the Document Model or any
conversion logic.

## Read first

- `README.md` for build commands and project layout.
- `docs/techstack.yml` for exact dependency choices and versions.
- `docs/conventions.md` for the network ban and licensing header format.
- `docs/codebase.txt` for the full target file tree.

## Files touched

```
Cargo.toml (workspace root, if used) / src-tauri/Cargo.toml
src-tauri/tauri.conf.json
src-tauri/src/main.rs
package.json
tsconfig.json
src/main.tsx
src/App.tsx (placeholder only, real layout comes in serial-task08)
.github/workflows/*.yml (already provided, verify they run)
LICENSE
```

## Steps

1. Initialize the Tauri + React + TypeScript + Vite project structure
   matching `docs/codebase.txt` exactly. Use the dependency versions and
   rationale in `docs/techstack.yml`, do not substitute alternatives
   without flagging it.
2. Configure `tauri.conf.json` with no network/http capability scope.
   Confirm this explicitly; this file is the single most important
   artifact of this task given the network ban in `docs/conventions.md`.
3. Add the GPL v3-or-later `LICENSE` file (full text) and the per-file
   header block specified in `docs/conventions.md`, "Licensing headers,"
   to every source file created in this task.
4. Set up `rustfmt.toml` (if deviating from defaults, document why) and
   confirm `cargo fmt --check` and `cargo clippy --all-targets -- -D
   warnings` both run clean on the empty skeleton.
5. Set up ESLint + Prettier config for the frontend, matching
   `docs/conventions.md`'s TypeScript/React style section.
6. Confirm `npm run tauri dev` launches an empty window and `npm run
   tauri build` produces a binary, on whichever platform you are working
   on.
7. Verify the three CI workflows in `.github/workflows/` actually run
   on a trivial push (they already exist; this task is about making the
   repository they check against buildable, not writing new workflow
   files).
8. Add a minimal placeholder `App.tsx` (a single rendered string is
   enough) so the build has something to compile; the real split-pane
   layout is `parallel-task08-B`'s job, not this task's.

## Definition of done

- [ ] `cargo fmt --check` and `cargo clippy --all-targets -- -D
      warnings` pass on the empty Rust skeleton.
- [ ] `npm run lint` and `npm run typecheck` pass on the empty frontend
      skeleton.
- [ ] `npm run tauri dev` opens a window; `npm run tauri build` produces
      a binary.
- [ ] All three `.github/workflows/*.yml` jobs pass on a trivial commit.
- [ ] `tauri.conf.json` has no network/http capability scope. Checked by
      hand, not just by CI, since this is the first and most important
      line of defense for the network ban.
- [ ] `LICENSE` file present with full GPL v3-or-later text.
- [ ] Every created source file has the license header from
      `docs/conventions.md`.
- [ ] Directory layout matches `docs/codebase.txt` (empty directories
      with `mod.rs`/`index` placeholders where needed are fine; this
      task does not need real implementations in them).
