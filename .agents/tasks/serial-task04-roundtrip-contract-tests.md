# serial-task04: Round-trip contract test suite

## Prerequisites

All four of `parallel-task03-A` (LaTeX parser), `parallel-task03-B`
(LaTeX printer), `parallel-task03-C` (DOCX reader), and
`parallel-task03-D` (DOCX writer) complete. This task is the
convergence point for that parallel group: it is the first place the
four independently-built modules are tested against each other.

## Goal

Build the contract test suite described in `docs/contracts.md`, "Contract
testing," and in doing so, find and resolve any mismatch between how the
parallel tasks independently interpreted shared mapping tables (LaTeX
construct to DM node, DM node to Word style and back). This task owns
reconciling those mismatches; it is expected, not a sign something went
wrong in planning, since `parallel-task03-A`/`03-B` and `03-C`/`03-D`
were deliberately built without depending on each other's output.

## Read first

- `docs/contracts.md`, "Contract testing" section in full.
- `.agents/prompts/fix-roundtrip-bug.md`, since this task will likely
  surface several of exactly the bug class that prompt describes.
- `.agents/skills/contract-discipline.md`.

## Files touched

```
src-tauri/tests/contracts.rs
tests/fixtures/ (consolidate and deduplicate fixtures from
  parallel-task03-A through 03-D, which may have created overlapping or
  inconsistent fixtures independently)
```

## Steps

1. Consolidate `tests/fixtures/latex/` and `tests/fixtures/docx/` from
   the four parallel tasks. Where two tasks created similar fixtures
   independently (likely, since they could not coordinate), pick the
   clearer one and remove the duplicate, or merge them if both cover
   genuinely distinct cases.
2. Implement golden-file tests: fixed `.tex`/`.docx` fixtures paired with
   expected `Document` JSON (via `insta` snapshots), per
   `docs/contracts.md`.
3. Implement `proptest`-based round-trip property tests:
   `parse_latex(print_latex(d)) == d` and
   `read_docx(write_docx(d)) == d`, for randomly generated `Document`
   trees, excluding `Opaque` blocks by construction from the
   byte-exact-reproduction property (they have a different, narrower
   property, see step 5).
4. Run the full property test suite and fix every failure found. Expect
   failures here; this is precisely why the round-trip property is
   tested at this convergence point rather than trusted from each
   module's own unit tests. Follow `.agents/prompts/fix-roundtrip-bug.md`
   for each one: identify whether the parser, the printer, or the DM
   itself is at fault, and whether the `proptest` generator itself has a
   gap that let the bug through undetected until now.
5. Add the narrower `Opaque` round-trip property: a `Document` containing
   only `OpaqueBlock { origin: Latex }` nodes, printed to LaTeX, should
   reproduce the original `raw` field byte-for-byte in the output (same
   for `origin: Docx` through the DOCX writer).
6. Add snapshot tests asserting the serialized JSON shape of every
   `docs/contracts.md` section 1 type, so an unintentional shape change
   later is caught immediately (per `docs/contracts.md`'s
   `schema_version` discipline).
7. Where this task's testing reveals that `parallel-task03-C` and
   `parallel-task03-D` used different Word style names for the same DM
   node type (a likely mismatch given they were built independently),
   fix the mismatch in whichever module deviated from
   `docs/specs.md` section 2's intent, and update `docs/specs.md` if the
   document itself was ambiguous enough to cause the split.

## Definition of done

- [ ] Golden-file tests pass for every fixture.
- [ ] `proptest` round-trip tests pass for LaTeX (`parse . print ==
      id`) and DOCX (`read . write == id`), excluding `Opaque` content.
- [ ] `Opaque` byte-exact round-trip property passes for both origins.
- [ ] Snapshot tests exist for every `docs/contracts.md` section 1 type.
- [ ] Any mismatch found between `parallel-task03-C` and
      `parallel-task03-D`'s independent style-mapping assumptions is
      resolved, and `docs/specs.md` section 2 is updated if the mismatch
      traced back to ambiguity in that document.
- [ ] `cargo test` passes in full, including this new suite.
- [ ] Fixture directories are deduplicated and organized, not left as
      four parallel tasks' independent, overlapping fixture sets.
