# Prompt: Fix a round-trip bug

Use this when a bug report or failing test shows that
`parse(print(document)) != document`, `print(parse(source)) != source`,
or the DOCX equivalent, for content that `docs/specs.md` says should be
fully supported (not `Opaque`). Round-trip correctness is the core
promise of this application; treat these bugs as high priority regardless
of how small the diff looks.

## Steps

1. **Reproduce with the smallest possible fixture.** Reduce the failing
   input to the minimal LaTeX or DOCX snippet that still fails. Add this
   minimal fixture to `tests/fixtures/` even before fixing the bug, as a
   regression test, following the naming pattern already used there.

2. **Identify which side of the round-trip is wrong.** Three
   possibilities, and they need different fixes:
   - The parser is dropping or misreading information (fix the parser).
   - The printer is not faithfully reproducing what the Document Model
     holds (fix the printer).
   - The Document Model itself cannot represent the distinction that was
     lost (this is a modeling gap; it needs a `docs/contracts.md` change,
     not just a parser or printer patch, see
     `.agents/prompts/add-latex-construct.md` if a new field or node
     type is required).

3. **Check whether `proptest` should have already caught this.** If the
   round-trip property test in `src-tauri/tests/contracts.rs` exists but
   did not generate the failing case, check whether the `proptest`
   generator (`Arbitrary` impl or strategy) for the relevant node type
   has a gap, for example never generating empty strings, deeply nested
   structures, or a specific character that breaks escaping. Fix the
   generator in the same change so this class of bug cannot recur
   silently.

4. **Fix the actual bug.** Keep the fix scoped to the parser, printer, or
   model, whichever step 2 identified. Resist the temptation to patch
   around the symptom somewhere else (for instance, special-casing the
   printer's caller instead of fixing the printer).

5. **Verify against the full fixture suite,** not just the new minimal
   case, since a parser/printer fix for one construct can regress another
   construct that shares code paths.

6. **Update documentation if the bug revealed a specs.md inaccuracy.** If
   `docs/specs.md` claimed a construct round-trips exactly but it
   actually involves a known, intentional normalization (see the
   "multiple spellings" note in `add-latex-construct.md`), document that
   exception rather than leaving the spec silently wrong.

7. Run the full checklist in `AGENTS.md`, "Before opening a pull
   request."

## Things to watch for

- Whitespace, comments, and escaping are the most common source of
  "close but not byte-identical" round-trip bugs in LaTeX. Check these
  first before assuming the structural logic is wrong.
- For DOCX round-trip bugs, check whether the issue is actually in
  `quick-xml` element ordering or namespace handling rather than in
  Tessera's own mapping logic; OOXML is sensitive to element order in
  ways that are easy to get subtly wrong.
- A round-trip bug that only appears after multiple edit cycles (not a
  single parse-print cycle) may be a `DocVersion`/staleness issue in the
  sync flow (`docs/architecture.md`, "Sync flow") rather than a pure
  parser/printer bug. Check `src/state/documentStore.ts` version handling
  before assuming the backend conversion logic is at fault.
