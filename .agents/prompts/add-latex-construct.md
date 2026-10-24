# Prompt: Add a new LaTeX construct

Use this when the task is "support `\command`/`environment` from LaTeX,"
for example adding support for a new list type, a new sectioning command,
or a new table feature currently listed as `Opaque` or absent entirely in
`docs/specs.md`.

## Steps

1. **Check `docs/specs.md` first.** Confirm the construct is not already
   listed as supported (in which case the task is a bug fix, not a new
   feature) and not explicitly listed under "Deferred" (in which case
   confirm with a human before proceeding, since deferred items were
   postponed for a reason that may still apply).

2. **Decide the Document Model representation.**
   - Does this map onto an existing `Block` or `Inline` variant in
     `docs/contracts.md` section 1? If so, use it; do not add a new
     variant for something that already fits.
   - If it genuinely needs a new node type, propose the type shape in
     `docs/contracts.md` section 1 first, as a standalone edit, before
     writing parser code against it. Follow the same `serde` tagging
     convention (`#[serde(tag = "type")]`) already in use.
   - If the construct cannot be represented at all (visually complex,
     package-specific rendering with no DOCX equivalent), it should
     become `Opaque`, not forced into an ill-fitting existing variant.
     Document this decision in `docs/specs.md` under the relevant
     section or under "Explicitly out of scope."

3. **Implement in this order:**
   - `src-tauri/src/core/latex/parser.rs`: parse the construct into the
     Document Model node.
   - `src-tauri/src/core/latex/printer.rs`: print the Document Model
     node back to the exact same LaTeX source. This is where round-trip
     correctness lives; write the printer with the parser's test
     fixtures in mind.
   - `src-tauri/src/core/docx/writer.rs`: map the new node to a DOCX
     representation using Word built-in styles where possible (see
     `docs/specs.md` section 2).
   - `src-tauri/src/core/docx/reader.rs`: map the DOCX representation
     back to the Document Model node, so a `.docx` using the equivalent
     Word feature round-trips too.
   - `src-tauri/src/core/latex/highlight.rs`: add token classification
     if the construct introduces new syntax that needs Okabe-Ito
     highlighting (see `docs/conventions.md` for the palette).

4. **Tests, in the same change:**
   - Add a fixture to `tests/fixtures/` exercising the new construct.
   - Add an example-based parser test and an example-based printer test.
   - Extend or confirm the `proptest` generator in
     `src-tauri/tests/contracts.rs` can produce this node type, so the
     round-trip property test actually exercises it.
   - Add a DOCX round-trip test (write then read, assert equality).

5. **Update documentation in the same change:**
   - `docs/specs.md`: add a row to the relevant table (section 1) with
     the LaTeX syntax and the Document Model node it maps to.
   - `docs/contracts.md`: if a new type was added, it should already be
     documented there from step 2; double check the final shape matches
     what was implemented.
   - `docs/codebase.txt`: only if this introduces a new file; most
     additions to this prompt's scope will not.

6. **Before finishing:** run the full checklist in `AGENTS.md`, "Before
   opening a pull request."

## Things to watch for

- A construct that has multiple LaTeX spellings for the same semantic
  thing (for example `\emph` and `\textit` both meaning italic) should
  parse to the same Document Model node, but the printer needs to know
  which spelling to use on export. Store the preferred spelling as part
  of the node if round-tripping the original spelling matters, or
  document in `docs/specs.md` that Tessera normalizes to one spelling
  and accept that as an intentional, documented exception to strict
  round-tripping. Do not silently normalize without documenting it.
- Whitespace and comment preservation: LaTeX comments (`%`) inside a
  construct's arguments are easy to drop accidentally during parsing.
  Check the printer test asserts byte-identical output on a fixture that
  includes a comment, not just one that does not.
