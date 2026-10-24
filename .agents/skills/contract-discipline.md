---
name: contract-discipline
description: Apply whenever a change touches a type or shape defined in docs/contracts.md, the Document Model, Tauri IPC types, or the on-disk project format. Ensures code and documentation never drift apart across the Rust/frontend boundary.
---

# Contract discipline

`docs/contracts.md` exists because Tessera has exactly one source of
truth for data shapes crossing the Rust-to-frontend boundary, and letting
that drift from the actual code is how sync bugs and silent data loss
creep in. This skill is the standing rule for any change that touches a
type documented there.

## The rule

**A code change to a type in `docs/contracts.md` and the corresponding
documentation update land in the same commit or pull request, never
split across two.** A PR that changes `Block` in `model.rs` without
touching `docs/contracts.md` section 1 is incomplete, not "documentation
debt to clean up later."

## Where drift tends to happen

1. **Rust struct/enum changed, TypeScript mirror not updated.**
   `src/ipc/commands.ts` hand-mirrors every type in `docs/contracts.md`
   section 2. Adding a field to `LatexEditRequest` in Rust and forgetting
   the TypeScript side produces a runtime shape mismatch that
   `serde`/JSON will not catch at compile time on either side. Check both
   whenever either changes.

2. **A new `Block` or `Inline` variant added without updating the DOCX
   writer/reader.** `docs/contracts.md` section 1 is the schema; every
   variant needs a defined mapping in both
   `src-tauri/src/core/docx/writer.rs` and `...reader.rs`, documented in
   `docs/specs.md` section 1 or 2. A variant that parses from LaTeX but
   has no DOCX writer mapping will panic or silently drop content at
   export time; neither is acceptable (see `AGENTS.md`, "No silent data
   loss").

3. **`TesseraError` variant added without updating the frontend's error
   handling.** If the frontend pattern-matches on `TesseraError.kind`
   (per `docs/contracts.md` section 2's tagged enum), a new variant needs
   a corresponding case there, even if it is just a generic fallback
   message; silently falling through to an "unknown error" is acceptable
   only if that is a deliberate, documented default case, not an
   oversight.

4. **On-disk format (`docs/contracts.md` section 3) changed without a
   `schema_version` bump.** `.tessera/project.json`'s `schema_version`
   field exists precisely so old projects can be migrated or at least
   detected as outdated. A shape change to this file without bumping the
   version breaks that detection for every existing user's saved
   projects.

## Verification steps

Before considering a contract-touching change complete:

- [ ] `docs/contracts.md` reflects the final shape, not an intermediate
      one from earlier in the session.
- [ ] `src/ipc/commands.ts` TypeScript types match, field for field,
      including optionality (`Option<T>` maps to `T | null` or `T |
      undefined`, be precise about which and consistent with the rest of
      the file).
- [ ] If a `Block`/`Inline` variant changed: both DOCX writer and reader
      handle it, and `docs/specs.md` documents the mapping.
- [ ] If this is a genuinely new type (not a field addition to an
      existing one): the contract tests described in `docs/contracts.md`,
      "Contract testing," have a corresponding entry (golden fixture,
      proptest generator case, or insta snapshot, as appropriate).
- [ ] If the on-disk format changed: `schema_version` was bumped and a
      migration note (even if "no migration needed, additive field with
      a default") is added to `docs/contracts.md` section 3.

## When a contract change is large

If a task requires a substantial reshaping of the Document Model (not
just adding one field or variant), propose the new shape in
`docs/contracts.md` as its own, reviewable change before writing the
implementation against it. This mirrors how `.agents/prompts/
add-latex-construct.md` step 2 asks for the same sequencing: shape first,
reviewed, then implementation. It is much cheaper to revise a type
definition in a markdown file than to revise it after parser, printer,
writer, reader, and frontend code all already depend on the old shape.
