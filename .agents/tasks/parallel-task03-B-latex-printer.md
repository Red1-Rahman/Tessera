# parallel-task03-B: LaTeX printer

## Prerequisites

`serial-task02-document-model` complete.

## Can run alongside

`parallel-task03-A` (LaTeX parser), `parallel-task03-C` (DOCX reader),
`parallel-task03-D` (DOCX writer). This task does not depend on the
parser's implementation, only on the DM shape from `serial-task02`.
Build and test this task against hand-constructed DM fixtures, not
against `parallel-task03-A`'s output; the two converge in
`serial-task04`.

## Goal

Implement `src-tauri/src/core/latex/printer.rs`: Document Model to LaTeX
source text, for the full subset defined in `docs/specs.md` section 1.
This is the module where round-trip correctness actually lives; treat
byte-exact output as the real requirement, not "looks right."

## Read first

- `docs/specs.md` section 1 in full, same scope as the parser task.
- `docs/contracts.md` section 1.
- `.agents/prompts/add-latex-construct.md`, the printer half of step 3.
- `.agents/prompts/fix-roundtrip-bug.md`, read in advance so you
  recognize round-trip risk areas (whitespace, comments, escaping) while
  writing this module the first time, rather than discovering them later
  as bugs.

## Files touched

```
src-tauri/src/core/latex/printer.rs
src-tauri/src/core/latex/mod.rs (module wiring only, coordinate with
  parallel-task03-A if both need to touch this file)
tests/fixtures/latex/*.tex (reuse the same fixtures parallel-task03-A
  creates if they already exist by the time you write a test against
  them; otherwise add your own and parallel-task03-A's later tests in
  serial-task04 will reuse them)
```

## Steps

1. For each DM node type, implement printing back to the exact LaTeX
   syntax `docs/specs.md` specifies. Where `docs/specs.md` documents a
   normalization (for example `\emph` vs `\textit` both parsing to
   `Inline::Italic`, see `add-latex-construct.md`'s "multiple spellings"
   note), pick the documented canonical spelling and print that
   consistently, after confirming `docs/specs.md` actually documents the
   choice; if it doesn't yet, add that documentation in this task as
   part of making the normalization decision explicit.
2. `OpaqueBlock` nodes with `origin: Latex` must print back to their
   `raw` field byte-for-byte, unmodified. This is the easiest part of
   the module to get right and the most damaging to get wrong; write a
   dedicated test for it early.
3. Whitespace and comment reproduction: if `parallel-task03-A` has
   landed by the time you write this, check what form it preserves this
   information in and match it; if not, make a reasonable assumption
   (trailing comments preserved adjacent to the node they follow,
   blank-line paragraph separation preserved) and flag it for
   `serial-task04` to verify against the actual parser's output.
4. Multi-file projects: print back into the correct originating file per
   the provenance tracking described in `docs/specs.md` section 3, not
   flattened into one file.
5. Write unit tests from hand-constructed DM trees (built using
   `serial-task02`'s validating constructors) to expected LaTeX string
   output, covering every construct in `docs/specs.md` 1.1 to 1.10, plus
   the `Opaque` byte-exact case from step 2.

## Definition of done

- [ ] Every DM node type from `docs/contracts.md` section 1 has a
      printer implementation.
- [ ] `OpaqueBlock { origin: Latex, .. }` prints back byte-for-byte
      identical to its `raw` field, tested explicitly.
- [ ] Any normalization decision (multiple LaTeX spellings mapping to
      one DM node) is documented in `docs/specs.md`, not just
      implemented silently.
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
      pass.
- [ ] Unit tests exist for every construct and pass.
- [ ] Multi-file output targets the correct source file per node
      provenance.
