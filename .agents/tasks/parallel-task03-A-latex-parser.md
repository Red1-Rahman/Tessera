# parallel-task03-A: LaTeX parser

## Prerequisites

`serial-task02-document-model` complete. The DM shape must be stable
before this task starts, since this task builds directly against it.

## Can run alongside

`parallel-task03-B` (LaTeX printer), `parallel-task03-C` (DOCX reader),
`parallel-task03-D` (DOCX writer). None of these four tasks touch each
other's files or depend on each other's output; they only depend on
`serial-task02`.

Note: this task and `parallel-task03-B` will eventually need each
other's output for round-trip testing, but that testing is
`serial-task04`'s job, not this task's. Write this task's own unit tests
against known-good DM fixtures you construct by hand, not against the
printer's output, so the two tasks genuinely do not block each other.

## Goal

Implement `src-tauri/src/core/latex/parser.rs`: LaTeX source text to
Document Model, for the full subset defined in `docs/specs.md` section
1.

## Read first

- `docs/specs.md` section 1 in full (1.1 through 1.11). This is the
  exact scope, not a starting point to extrapolate from. Anything listed
  under 1.11 ("Explicitly out of scope") must become an `Opaque` block,
  not be skipped or crashed on.
- `docs/contracts.md` section 1 for the exact DM shape to parse into.
- `.agents/prompts/add-latex-construct.md` for the per-construct
  implementation pattern, since this task is effectively "do that
  prompt's steps 2 to 3's parser half, for every construct in
  docs/specs.md at once."
- `docs/techstack.yml`'s entry for `nom`, the parser combinator crate
  this module is built with.

## Files touched

```
src-tauri/src/core/latex/parser.rs
src-tauri/src/core/latex/mod.rs (module wiring only, coordinate with
  parallel-task03-B if both tasks need to touch this file; prefer each
  task adding its own `pub mod` line without reformatting the rest)
tests/fixtures/latex/*.tex (new fixture files, parser-focused)
```

## Steps

1. Build the parser incrementally in the order `docs/specs.md` section 1
   lists subsections: document classes and preamble (1.1, 1.2) first,
   since everything else nests inside a parsed document; then structure
   (1.3), text formatting (1.4), lists (1.5), tables (1.6), figures
   (1.7), math (1.8), citations (1.9), code blocks (1.10).
2. For each construct, decide and implement per
   `add-latex-construct.md` step 2: does it map to an existing DM node,
   or does it need `Opaque`. Everything in 1.11 is `Opaque` by
   definition; do not attempt to parse `tikz` contents structurally, for
   example, just capture it as `OpaqueBlock { origin: Latex, raw:
   <verbatim source>, summary: "..." }`.
3. Preserve whitespace and comments faithfully in whatever internal form
   lets `parallel-task03-B`'s printer reproduce them later; this parser
   does not need to pass round-trip tests itself (that's
   `serial-task04`), but it must retain the information the printer will
   need, meaning: do not discard comments or collapse whitespace
   variation during parsing, even if the DM node itself does not have an
   obvious field for it yet. If the current DM shape genuinely cannot
   hold this information, flag it as a blocking question rather than
   dropping it silently, since that would be exactly the kind of data
   loss `AGENTS.md` prohibits.
4. Multi-file projects (`docs/specs.md` section 3, `\input`/`\include`):
   implement the resolution and provenance tracking described there.
5. Write unit tests per construct against hand-written fixture files in
   `tests/fixtures/latex/`, asserting the exact expected DM tree.
6. Follow `.agents/skills/rust-conventions.md` throughout: no
   `.unwrap()`, validating constructors from `serial-task02` used
   wherever applicable, parse errors returned as
   `TesseraError::ParseError` with an accurate `SourceSpan`.

## Definition of done

- [ ] Every construct listed as supported in `docs/specs.md` sections
      1.1 to 1.10 parses into the DM node that section specifies.
- [ ] Every construct listed in 1.11 parses into `Opaque` with
      `origin: Latex` and a meaningful `summary`.
- [ ] Multi-file project resolution (section 3) works with provenance
      tracking.
- [ ] Parse errors return `TesseraError::ParseError` with a correct
      `SourceSpan`, not a panic, for malformed input (test this with at
      least a few deliberately broken fixtures).
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
      pass.
- [ ] Unit tests exist for every construct and pass.
- [ ] No whitespace or comment information needed for later
      byte-identical printing was discarded; if a DM shape gap was found,
      it was flagged and resolved against `docs/contracts.md`, not
      silently worked around.
