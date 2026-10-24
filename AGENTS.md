# AGENTS.md

This file is the entry point for any AI coding agent (Claude Code,
Cursor, Copilot Workspace, or similar) working in this repository. Read
this file first, then the specific documents it points to, before making
changes.

## Read in this order

1. `README.md` for the project's purpose and non-features.
2. `docs/architecture.md` for how the system fits together.
3. `docs/contracts.md` for exact data shapes. Treat this as load-bearing;
   do not infer a type's shape from a single call site when this document
   defines it.
4. `docs/specs.md` for what LaTeX/DOCX behavior is actually in scope.
   Do not implement support for a construct this document lists as
   out of scope or deferred without first updating `docs/specs.md`
   in the same change and flagging it to a human reviewer.
5. `docs/conventions.md` for coding standards, including the network ban
   and documentation style rules below.
6. `docs/codebase.txt` for a quick path-to-purpose lookup when you need to
   find where something lives without reading all of `docs/architecture.md`
   again.

Task-specific instructions live in `.agents/prompts/`. Reusable,
cross-task standards live in `.agents/skills/`. Load the ones relevant to
your current task; you do not need to read every file in both directories
for every change.

## Hard constraints (non-negotiable)

These are not style preferences. A change that violates any of these is
wrong regardless of whether it otherwise works.

- **No network access, ever.** No HTTP client crate, no network
  capability in `tauri.conf.json`, no "just for this one feature"
  exception. See `docs/conventions.md`, "Network ban." If a task seems to
  require network access, stop and ask a human; do not implement a
  workaround.
- **License is GPL v2 or later.** Every new source file gets the header
  specified in `docs/conventions.md`, "Licensing headers." Do not add a
  dependency whose license is incompatible with GPL-2.0-or-later without
  flagging it explicitly; when unsure, ask rather than assume
  compatibility.
- **The Document Model is the only sync path.** Never write code that
  updates the LaTeX pane and DOCX pane independently for the same logical
  edit. All edits flow through `core/model.rs` types as described in
  `docs/architecture.md`, "Sync flow." If a feature seems to require a
  pane-to-pane shortcut, that is a sign the Document Model needs a new
  node type, not a sign to bypass it.
- **No silent data loss.** A LaTeX or DOCX construct that cannot be
  represented in the Document Model must become an `Opaque` block (see
  `docs/contracts.md`), not be dropped, approximated without a warning,
  or guessed at.
- **Contracts and specs are updated in the same change as the code that
  needs them updated.** If you change a type in `src-tauri/src/core/
  model.rs`, update `docs/contracts.md` in the same patch. If you add
  support for a new LaTeX construct, update `docs/specs.md` in the same
  patch. A code change that makes a doc wrong is an incomplete change.

## Writing style for anything this agent produces

- No em dashes, in code comments, commit messages, generated
  documentation, or PR descriptions. See `docs/conventions.md`,
  "Documentation style."
- Use Pydantic-style validation discipline where it is relevant: in this
  codebase that means Rust types crossing a serialization boundary are
  validated at construction (see `docs/conventions.md`, Rust style, and
  `docs/contracts.md`'s intro paragraph). When writing or reviewing a new
  `serde`-derived type that takes untrusted input, give it a validating
  constructor rather than relying on `serde`'s default derive alone.

## Before opening a pull request

Run through this checklist. `.agents/prompts/pr-checklist.md` has the
expanded version with exact commands.

- [ ] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
      pass.
- [ ] `cargo test` passes, including the `proptest` round-trip suite in
      `src-tauri/tests/contracts.rs`.
- [ ] Frontend lints and `vitest` pass.
- [ ] No new dependency introduces network capability (check
      `cargo tree` and `package.json` diffs).
- [ ] `docs/contracts.md` and `docs/specs.md` updated if this change
      touches either.
- [ ] No em dashes in anything written for this change.
- [ ] New source files carry the GPL header.

## When you are unsure

Prefer asking over guessing for anything touching: the network ban, the
Document Model schema, license compatibility of a new dependency, or
whether a LaTeX/DOCX construct should be first-class or `Opaque`. Getting
these wrong is expensive to unwind later; getting a UI detail wrong is
not.

## Directory guide for agents

- `.agents/prompts/` holds instructions for specific recurring tasks
  (adding a LaTeX construct, adding an export format, fixing a round-trip
  bug, release prep). Start here if your task matches one of these.
- `.agents/skills/` holds standing skills an agent should apply whenever
  relevant, regardless of the specific task (Rust style enforcement,
  contract-change discipline, cross-platform build awareness).
