# parallel-task03-C: DOCX reader

## Prerequisites

`serial-task02-document-model` complete.

## Can run alongside

`parallel-task03-A` (LaTeX parser), `parallel-task03-B` (LaTeX printer),
`parallel-task03-D` (DOCX writer). This task does not depend on
`parallel-task03-D`'s implementation; build and test against
hand-crafted or real-world `.docx` fixtures, not against
`parallel-task03-D`'s output. The two converge in `serial-task04`.

## Goal

Implement `src-tauri/src/core/docx/reader.rs`: DOCX (an OOXML zip
archive) to Document Model, per `docs/specs.md` section 2.

## Read first

- `docs/specs.md` section 2 in full, "DOCX mapping notes."
- `docs/contracts.md` section 1.
- `docs/techstack.yml`'s entries for `quick-xml` and `zip`, the crates
  this module is built with.
- The `docx` skill's background on OOXML structure (`word/document.xml`
  inside a zip archive) is useful general context for anyone unfamiliar
  with the format, even though that skill targets a different workflow
  than this task.

## Files touched

```
src-tauri/src/core/docx/reader.rs
src-tauri/src/core/docx/mod.rs (module wiring only, coordinate with
  parallel-task03-D if both need to touch this file)
tests/fixtures/docx/*.docx (new fixture files, reader-focused; include
  both Tessera-style fixtures and at least one real-world .docx produced
  by Word or LibreOffice, to test the "not originally produced by
  Tessera" path described in docs/specs.md section 2)
```

## Steps

1. Unzip and parse `word/document.xml` (and `word/styles.xml` for style
   name resolution) using `quick-xml`.
2. Map Word built-in style names (`Heading 1`, `Heading 2`, `Normal`,
   `Caption`, etc.) back onto DM node types, per the reverse-mapping
   table implied by `docs/specs.md` section 2 (the writer's forward
   mapping; this task is the inverse of `parallel-task03-D`'s work, so
   confirm the mapping table is identical in both directions even though
   the two tasks are built independently right now, reconciling any
   mismatch in `serial-task04`).
3. Anything not covered by a recognized built-in style (custom styles,
   embedded objects, tracked changes, comments) becomes `OpaqueBlock {
   origin: Docx, .. }` per `docs/specs.md` section 2's explicit
   best-effort policy. Do not attempt to interpret custom styles
   heuristically; if it's not a recognized built-in style, it's opaque.
4. Tables: map Word table XML (`<w:tbl>`) including merged cells
   (`gridSpan`, `vMerge`) to `Table`/`TableCell` with correct
   `col_span`/`row_span`.
5. Images: extract embedded media from the DOCX zip's `word/media/`
   folder into `Figure.asset_path` references; actual copying into
   `.tessera/assets/` is a project-level concern handled elsewhere (see
   `docs/architecture.md`'s storage model), this task just needs to
   produce the correct relative reference and make the raw image bytes
   available to whatever calls this reader.
6. Math: parse OMML (Office Math Markup Language) back to LaTeX math
   source for `Equation`/`InlineMath` nodes, per `docs/specs.md` section
   1.8's math subset.
7. Write unit tests against both hand-crafted minimal `.docx` fixtures
   and at least one real-world Word/LibreOffice-produced file, asserting
   the resulting DM tree, with particular attention to the "not
   originally produced by Tessera" fallback behavior.

## Definition of done

- [ ] Every built-in Word style listed implicitly by `docs/specs.md`
      section 2 maps to its documented DM node type.
- [ ] Unrecognized DOCX content (custom styles, embedded objects,
      tracked changes, comments) becomes `OpaqueBlock { origin: Docx,
      .. }`, never silently dropped or misinterpreted.
- [ ] Table merged cells map correctly to `col_span`/`row_span`.
- [ ] Images extract to `Figure` nodes with usable asset references.
- [ ] OMML math converts to LaTeX math source for the supported subset,
      with unsupported math outside that subset handled per
      `docs/specs.md` 1.8 (flagged, not silently approximated).
- [ ] Tested against at least one real-world, non-Tessera-produced
      `.docx` file, not only self-produced fixtures.
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
      pass.
