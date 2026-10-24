---
name: rust-conventions
description: Apply Tessera's Rust coding standards whenever writing or reviewing Rust code in this repository. Covers error handling, boundary validation, module placement, and the network ban. Use for any change touching src-tauri/.
---

# Rust conventions for Tessera

This skill summarizes and operationalizes `docs/conventions.md`'s Rust
section and `docs/contracts.md`'s validation discipline. Read those two
files for full detail; this is the checklist form to apply while writing
code, not a replacement for them.

## Error handling

- Library code (`core/`, `commands/`) returns `Result<T, TesseraError>`.
  No `.unwrap()`, no `.expect()` outside `main.rs` bootstrap and test
  code.
- Use `?` to propagate. If converting an external crate's error type,
  implement `From<ExternalError> for TesseraError` rather than mapping
  inline at every call site.
- `anyhow` is for internal context in binary/top-level code only. If you
  reach for `anyhow::Error` inside `core/`, stop and use a typed
  `TesseraError` variant instead, adding one if needed (update
  `docs/contracts.md` section 2 if so).

## Boundary validation (the "Pydantic in Rust" pattern)

Every type in `docs/contracts.md` that can be built from untrusted input
(parsed LaTeX, parsed DOCX, a Tauri IPC payload from the frontend) needs
a validating constructor, not just a `serde` derive. Pattern:

```rust
impl Author {
    /// Validates that `name` is non-empty after trimming. Returns
    /// TesseraError::ValidationError on failure.
    pub fn new(name: String, affiliation: Option<String>) -> Result<Self, TesseraError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TesseraError::ValidationError {
                field: "name".to_string(),
                message: "author name must not be empty".to_string(),
            });
        }
        Ok(Self { name: trimmed.to_string(), affiliation })
    }
}
```

Do not construct `Author { name, affiliation }` directly from
deserialized IPC input anywhere outside this constructor. If `serde`
deserializes a type and the caller needs to re-validate it before use,
that re-validation call should be close to the deserialization site, not
scattered deep in business logic where it is easy to forget on a new call
path.

This matters most for:
- `DocMetadata` fields coming from parsed LaTeX preambles.
- `Block`/`Inline` nodes coming from either parser.
- Anything in `LatexEditRequest`, `DocxEditRequest`, `ExportRequest` (see
  `docs/contracts.md` section 2), since these come directly from the
  frontend and must not be trusted blindly even though the frontend is
  "our own" code; a stale frontend build or a bug there should not be
  able to produce an inconsistent `Document`.

## Module placement

- `commands/*.rs` files only marshal input/output and call into `core/`.
  If you find yourself writing more than a few lines of actual logic
  (branching on document content, doing a transformation) in a
  `commands/` file, move it to `core/` and call it from there. This keeps
  logic unit-testable without Tauri or a webview (see
  `docs/conventions.md`, Testing).
- New conversion logic for a format goes in that format's module
  (`core/latex/`, `core/docx/`, `core/export/`), not inline in `model.rs`
  or scattered across `commands/`.

## The network ban, operationally

Before adding any new crate to `Cargo.toml`:

```bash
cargo tree | grep -iE "reqwest|hyper|ureq|tokio-tungstenite"
```

If the crate you are adding is flagged, or pulls in something flagged
transitively, stop and reconsider; see `docs/conventions.md`, "Network
ban," and `AGENTS.md`, "Hard constraints." This is not a lint to silence,
it is a design constraint.

## Doc comments

Every public type and function in `core/` and `commands/` gets a `///`
doc comment describing behavior and invariants, not just restating the
name. For parser/printer pairs specifically, state the round-trip
invariant the pair is expected to satisfy, referencing
`docs/contracts.md`'s "Contract testing" section.
