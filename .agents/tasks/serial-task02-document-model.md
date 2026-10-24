# serial-task02: Document Model

## Prerequisites

`serial-task01-scaffolding` complete (buildable Rust workspace exists).

## Goal

Implement the Document Model (DM) exactly as specified in
`docs/contracts.md` section 1. This is the single most important task in
the whole project: every later task, on both the Rust and frontend side,
depends on these types being correct and stable. Do not proceed past
this task with a DM shape you are not confident in; revisit
`docs/contracts.md` and get it reviewed before `parallel-task03-*`
starts, since four tasks will start building against this shape at once.

## Read first

- `docs/contracts.md` section 1 in full. This is not a summary to work
  from, it is the literal type definitions to implement.
- `docs/contracts.md`'s intro paragraph on boundary validation.
- `.agents/skills/rust-conventions.md`, especially "Boundary validation
  (the Pydantic in Rust pattern)."
- `docs/specs.md` section 1, to understand what each DM node type needs
  to eventually represent, even though parsing is not this task's job.

## Files touched

```
src-tauri/src/core/model.rs
src-tauri/src/core/mod.rs (module wiring only)
src-tauri/tests/model_tests.rs (new)
```

## Steps

1. Implement every type in `docs/contracts.md` section 1: `Document`,
   `DocVersion`, `DocMetadata`, `Author`, `Block` (all variants),
   `HeadingLevel`, `ListKind`, `Table`, `TableCell`, `Figure`,
   `Equation`, `OpaqueBlock`, `OpaqueOrigin`, `Inline` (all variants).
   Field names, types, and `serde` tagging must match exactly.
2. For every type that can be built from untrusted input (per the
   boundary validation pattern), implement a validating constructor, not
   just a public struct. Decide now which types need one; this list
   should cover at minimum `Author`, `Figure` (`width_percent` range
   checking), `Equation` (non-empty `latex_source`), and any `label`
   field (non-empty, no embedded whitespace, since labels are shared
   cross-reference keys per `docs/contracts.md`'s invariants section).
3. Implement `DocVersion` as a simple incrementing wrapper type with a
   `next()` method; do not allow external code to construct an arbitrary
   `DocVersion` value outside of the versioning logic that
   `serial-task05` will build on top of this.
4. Write unit tests directly against these types: constructing valid and
   invalid instances, confirming validation rejects what it should.
5. Do not implement `cargo tree` network checks, this task has no new
   dependencies beyond `serde`/`serde_json`, already present from
   `serial-task01`.
6. Leave `core/latex/`, `core/docx/`, `core/export/`, `core/diff.rs` as
   empty stub modules (or do not create them yet at all if the
   scaffolding task left them absent); those are not this task's
   responsibility.

## Definition of done

- [ ] Every type in `docs/contracts.md` section 1 is implemented with
      matching field names, types, and `serde` attributes.
- [ ] Every type identified in step 2 has a validating constructor
      returning `Result<T, TesseraError>` and is unit tested for both
      the accept and reject path.
- [ ] `cargo test` passes for `model_tests.rs`.
- [ ] `cargo clippy --all-targets -- -D warnings` passes.
- [ ] No type in this module is publicly constructible with invalid data
      through a struct literal where a validating constructor was called
      for in step 2 (check: are the relevant fields `pub` in a way that
      bypasses validation? If so, reconsider field visibility).
- [ ] A short note added to `docs/contracts.md`'s "Invariants"
      subsection confirming nothing was found to need a shape change
      during implementation; if something did need to change, that
      change is already reflected in `docs/contracts.md` per
      `.agents/skills/contract-discipline.md`, not left as a
      discrepancy for later.
