# Conventions

Coding standards for Tessera. These apply to both human and AI-assisted
contributions; `AGENTS.md` and `.agents/skills/` reference this file rather
than repeating it.

## Network ban

No code in `src-tauri/` may open a network connection, under any
circumstance. This includes:

- No HTTP client crates in `Cargo.toml` (`reqwest`, `hyper` as a client,
  `ureq`, etc.). See `docs/techstack.yml`, "explicitly excluded."
- No `tauri.conf.json` capability granting network/http scope.
- No DNS resolution, no `TcpStream::connect`, no crate that transitively
  pulls one in for a feature Tessera does not use (check with
  `cargo tree` when adding a dependency).

CI runs a dependency audit step (see `.github/workflows/`) that fails the
build if a network-capable crate is introduced. If a legitimate local-only
feature appears to need one of these primitives, treat that as a sign the
feature needs rethinking, not a reason to request an exception.

## Rust style

- Format with `rustfmt` using the repository's default settings
  (`rustfmt.toml` if present; otherwise stable defaults). CI runs
  `cargo fmt --check`.
- Lint with `clippy`. CI runs `cargo clippy --all-targets -- -D warnings`.
  Warnings are build failures, not suggestions.
- No `.unwrap()` or `.expect()` outside of tests and `main.rs` bootstrap
  code. Library code (`core/`, `commands/`) returns `Result<T, TesseraError>`
  and propagates with `?`.
- Public types that cross a serialization boundary (see
  `docs/contracts.md`) are constructed only through a validating
  constructor or builder, never a bare struct literal built from
  untrusted input. This is the project's equivalent of Pydantic-style
  "validate at the boundary," applied in Rust: the type system enforces
  shape, the constructor enforces semantic validity (non-empty required
  strings, well-formed labels, in-range spans).
- Prefer `&str` over `String` in function signatures unless ownership is
  genuinely needed.
- Module files mirror the structure in `docs/architecture.md`. New core
  logic belongs under `core/`; `commands/` stays thin (IPC marshaling
  only).
- Document round-trip invariants with a doc comment on the parser/printer
  pair that must satisfy them (see `docs/contracts.md`, "Contract
  testing").

## TypeScript / React style

- Strict mode on (`"strict": true` in `tsconfig.json`). No `any` without
  a `// reason:` comment explaining why a precise type was not feasible.
- Components are function components with hooks. No class components.
- All Tauri `invoke()` calls go through `src/ipc/commands.ts`. A component
  importing `@tauri-apps/api` directly is a review rejection; it means a
  typed wrapper is missing and should be added instead.
- State shape mirrors the Rust contract types in `docs/contracts.md`
  field-for-field. If you need to reshape data for a component, do it in
  a selector/derived-state function, not by inventing a parallel type
  that drifts from the contract.
- Format with Prettier, lint with ESLint; CI runs both.

## Documentation style

- No em dashes anywhere in documentation (this file, `README.md`,
  `docs/*.md`, doc comments, commit messages). Use a period, a comma, a
  semicolon, or parentheses instead, whichever best fits the sentence.
  This applies to AI-assisted writing as much as human-written text.
- Prefer concrete file paths and type names over vague references ("the
  command handler" should be "`commands/document.rs`'s `parse_latex`
  handler").
- Rust doc comments (`///`) on every public type and function in `core/`
  and `commands/`. A doc comment states behavior and invariants, not just
  a restatement of the name.
- When a document in `docs/` is updated in a way that changes a contract,
  update `docs/contracts.md` in the same change. Documentation drift
  between `docs/architecture.md`, `docs/specs.md`, and `docs/contracts.md`
  is treated as a bug.

## Testing

- Every parser/printer/reader/writer pair in `core/` has:
  - Example-based tests against fixtures in `tests/fixtures/`.
  - A `proptest`-based round-trip property test (see
    `docs/contracts.md`, "Contract testing").
- Every Tauri command has at least one test in `src-tauri/tests/` that
  calls its underlying core function directly (not through the webview),
  asserting both the success and at least one `TesseraError` path.
- Frontend: component tests with Vitest for anything with non-trivial
  logic (not pure presentational components); end-to-end tests with
  Playwright for the full open-edit-sync-export flow, run against the
  built Tauri binary in CI on all three platforms.
- No test may require network access. A test that needs "the internet" to
  pass is testing the wrong thing in this codebase.

## Commit and PR conventions

- Conventional Commits style prefixes (`feat:`, `fix:`, `docs:`,
  `test:`, `refactor:`, `chore:`) for commit subject lines.
- A PR that changes a type in `docs/contracts.md` must update that file
  in the same PR.
- A PR that changes supported LaTeX/DOCX behavior must update
  `docs/specs.md` in the same PR.
- Keep commits scoped: one logical change per commit, not "various
  fixes."

## Debounce and timing constants

- LaTeX pane keystroke-to-reparse debounce: 250ms, defined as a named
  constant (`LATEX_REPARSE_DEBOUNCE_MS`) in
  `src/state/documentStore.ts`, not a magic number inline.
- DOCX pane edit-to-sync debounce: 250ms, same rationale, constant
  `DOCX_SYNC_DEBOUNCE_MS`.
- These values live in exactly one place each; do not duplicate the
  literal elsewhere.

## Okabe-Ito palette reference

The eight colors, used consistently across LaTeX syntax highlighting and
the `Opaque` block treatment described in `docs/specs.md`:

| Name | Hex | Typical use |
|---|---|---|
| Black | `#000000` | Default text |
| Orange | `#E69F00` | Commands (`\section`, `\textbf`, …) |
| Sky Blue | `#56B4E9` | Braces and brackets |
| Bluish Green | `#009E73` | Environment names (`itemize`, `table`, …) |
| Yellow | `#F0E442` | Comments |
| Blue | `#0072B2` | Math mode delimiters and content |
| Vermillion | `#D55E00` | Errors, unresolved references |
| Reddish Purple | `#CC79A7` | `Opaque` block background tint (at reduced opacity) |

Exact token-to-color assignment logic lives in
`src-tauri/src/core/latex/highlight.rs`; this table is the canonical
palette reference, not the token classification rules themselves (those
are in `docs/specs.md`, section 5, and the module's doc comments).

## Licensing headers

Every source file (`.rs`, `.ts`, `.tsx`) begins with:

```
// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
```

CI checks for this header's presence in changed files.
