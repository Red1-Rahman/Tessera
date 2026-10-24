# Tessera

A local, offline desktop application that keeps a LaTeX document and a Word
(.docx) document synchronized through a shared intermediate model. Edit
either side and the other updates to match. No network access, no telemetry,
no cloud sync. Everything stays on disk, in formats you can open with any
other tool.

Built for people writing academic papers who want LaTeX's typesetting and
version-control friendliness together with a Word view for collaborators,
reviewers, and journals that require `.docx` submissions, without uploading
a manuscript to a third-party web converter.

## Why this exists

Online LaTeX-to-docx converters require uploading the manuscript to a server
you do not control. For unpublished research, that is not an acceptable
trade-off. Tessera runs entirely on the user's machine. The Rust backend
never opens a network socket for anything other than the OS file dialogs and
the local filesystem.

## Core features

- Split-pane editor: LaTeX source on one side, a structured DOCX-equivalent
  view on the other.
- Bidirectional sync. A change in either pane updates the shared document
  model, which re-renders both panes.
- Okabe-Ito colorblind-safe syntax highlighting for LaTeX.
- Export to LaTeX (`.tex`), Word (`.docx`), PDF, Markdown (`.md`), and plain
  text (`.txt`).
- Copy the current document to the clipboard in any of the above formats.
- All files live wherever the user saves them. A saved project is a plain
  directory, readable and editable outside the app.
- No internet access. No accounts. No analytics.

## Non-features (by design)

- No cloud sync, no accounts, no telemetry, no update check over the
  network.
- No LaTeX package manager. Tessera renders a defined, documented subset of
  LaTeX (see `docs/specs.md`). It is not a full TeX distribution
  replacement; for PDF export it shells out to a local, user-installed TeX
  engine if present, and falls back to its own typesetter otherwise (see
  `docs/architecture.md`).

## Status

Pre-1.0. See `docs/specs.md` for the supported LaTeX subset and
`docs/contracts.md` for the data contracts between the Rust core and the
frontend. This repository is being developed with AI coding agents in the
loop; see `AGENTS.md` for how agents are expected to work in this codebase.

## Tech stack

Rust (core, document model, file I/O) + Tauri (desktop shell) + TypeScript
and React (editor UI), compiled to static assets bundled into the app, with
no runtime network fetches. Full rationale in `docs/techstack.yml`.

## Building from source

Prerequisites: a stable Rust toolchain, Node.js LTS, and the platform
dependencies Tauri requires (see
[Tauri's prerequisites guide](https://tauri.app/start/prerequisites/) for
your OS).

```bash
git clone https://github.com/redwanrahman/tessera.git
cd tessera
npm install
npm run tauri dev      # development, hot reload
npm run tauri build    # release binaries in src-tauri/target/release/bundle
```

Platform-specific CI builds are defined in `.github/workflows/`:

- `build-linux.yml`
- `build-macos.yml`
- `build-windows.yml`

Each triggers on pushes and pull requests that touch `src/` or `tests/`,
and can also be run manually (`workflow_dispatch`).

## Project layout

```
tessera/
├── src/                  # TypeScript + React frontend
├── src-tauri/            # Rust backend, Tauri app shell
│   ├── src/
│   │   ├── core/         # document model, conversion engines
│   │   ├── commands/     # Tauri command handlers (IPC surface)
│   │   └── main.rs
│   └── Cargo.toml
├── tests/
├── docs/                 # architecture, specs, contracts, conventions
├── .agents/               # prompts and skills for AI coding agents
└── .github/workflows/
```

See `docs/architecture.md` for how the pieces fit together, `docs/specs.md`
for exactly what LaTeX and DOCX features are supported, and
`docs/conventions.md` for coding standards.

## License

GPL v3 or later. Copyright (c) 2026 Redwan Rahman. See `LICENSE` for the
full text. Tessera is free software: you may redistribute and modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option)
any later version.
