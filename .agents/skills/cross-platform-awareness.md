---
name: cross-platform-awareness
description: Apply whenever writing code that touches the filesystem, process execution (system TeX engine detection), paths, or anything that could behave differently on Windows, macOS, or Linux. Tessera ships CI builds for all three platforms and must behave identically on each.
---

# Cross-platform awareness

Tessera builds and ships on Windows, macOS, and Linux from one codebase
(see `.github/workflows/`, `docs/techstack.yml`). This skill is a
standing checklist for code that is prone to platform-specific bugs.

## Paths

- Never construct a path with a hardcoded `/` or `\` separator. Use
  `std::path::PathBuf`/`Path` and its join methods in Rust; in the
  frontend, do not construct filesystem paths at all, since the frontend
  should never touch the filesystem directly (all file I/O goes through
  Tauri commands, see `docs/architecture.md`).
- Project directory structure (`docs/architecture.md`, "Storage model")
  must round-trip identically regardless of platform. If you add a new
  file under `.tessera/`, confirm its name is valid on all three
  platforms (avoid characters Windows forbids in filenames, avoid
  reserved names like `CON`, `PRN` on Windows if the name is ever
  user-influenced).
- Case sensitivity differs: Linux filesystems are typically
  case-sensitive, Windows and (by default) macOS are not. Never rely on
  two paths differing only by case to refer to different files or
  assets.

## Line endings

- LaTeX source files should be written and read consistently regardless
  of platform. Decide and document (in `docs/specs.md` if user-visible,
  otherwise in the relevant module's doc comment) whether Tessera
  normalizes to `\n` internally and only converts at the OS boundary, or
  preserves the file's original line ending style. Whichever is chosen,
  the parser and printer must agree, or round-trip tests will pass on
  the development platform and fail in CI on another.

## System TeX engine detection (PDF export)

`docs/architecture.md`'s "PDF export strategy" describes detecting
`pdflatex`/`xelatex`/`lualatex` on `PATH`. This detection must:

- Use a method that works identically across platforms (checking `PATH`
  resolution via a crate like `which`, not a hardcoded list of
  per-platform install locations).
- Handle the executable extension difference on Windows (`.exe`)
  transparently; this is usually handled by `PATH`-resolution crates
  automatically, but verify rather than assume if writing this detection
  logic directly.
- Never assume a specific TeX distribution's layout (TeX Live on
  Linux/macOS vs. MiKTeX on Windows have different directory structures);
  detection should only care whether the binary runs, not where it lives.

## Process execution

- When shelling out to a system TeX engine, use Rust's `std::process`
  with argument arrays, never a shell string that gets interpreted
  differently by `cmd.exe` versus a POSIX shell.
- Temp directory handling for the LaTeX-to-PDF intermediate files should
  use `std::env::temp_dir()` or a crate wrapper, not a hardcoded `/tmp`.

## Clipboard and file dialogs

- These go through Tauri's own cross-platform APIs
  (`@tauri-apps/api`), never a platform-specific shell command or native
  binding written by hand. If a needed clipboard content type is not
  supported by Tauri's clipboard API, that is a sign to check for a
  Tauri plugin providing it rather than writing platform-specific code
  three times.

## CI verification

Every pull request affecting `src-tauri/` or anything path/process
related should be expected to pass on all three CI workflows in
`.github/workflows/` before merge. A change that only compiles and
passes tests on the developer's (or agent's) own platform is not
verified; do not treat local success on one platform as sufficient
confidence for a cross-platform concern. If CI cannot be run as part of
your workflow, flag platform-sensitive changes explicitly in the PR
description so a human reviewer knows to pay extra attention.
