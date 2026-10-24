# parallel-task03-D: DOCX writer

## Prerequisites

`serial-task02-document-model` complete.

## Can run alongside

`parallel-task03-A` (LaTeX parser), `parallel-task03-B` (LaTeX printer),
`parallel-task03-C` (DOCX reader). This task does not depend on
`parallel-task03-C`'s implementation; build and test by writing `.docx`
files and opening them in Word/LibreOffice to verify, not by feeding
output back into the reader. The two converge in `serial-task04`.

## Goal

Implement `src-tauri/src/core/docx/writer.rs`: Document Model to DOCX
(OOXML zip), per `docs/specs.md` section 2. The output must be a normal,
editable Word document using Word's own built-in styles and heading
navigation, not a document built entirely from manual character
formatting; this is explicitly called out in `docs/specs.md` section 2
as the whole point of this module, since it is what makes `main.docx`
genuinely useful as a submission file.

## Read first

- `docs/specs.md` section 2 in full.
- `docs/contracts.md` section 1.
- `docs/techstack.yml`'s entries for `quick-xml` and `zip`.
- `docs/architecture.md`'s storage model section, for how `Figure.
  asset_path` relates to `.tessera/assets/` and how that needs to land
  in the DOCX's `word/media/` folder on write.

## Files touched

```
src-tauri/src/core/docx/writer.rs
src-tauri/src/core/docx/mod.rs (module wiring only, coordinate with
  parallel-task03-C if both need to touch this file)
tests/fixtures/docx/*.docx (new fixture files, writer-focused: expected
  output docx files or XML snippets to assert against)
```

## Steps

1. Build the minimal valid DOCX zip structure: `[Content_Types].xml`,
   `_rels/.rels`, `word/document.xml`, `word/styles.xml`, and the
   relationship files media/images need.
2. Map each DM `Block`/`Inline` node type to its Word built-in style, per
   `docs/specs.md` section 2's forward mapping (this is the mapping
   `parallel-task03-C` implements the reverse of; keep a mental or
   written note of the exact style name used per node type so
   `serial-task04` can confirm both directions agree without needing to
   re-derive the mapping from scratch).
3. Headings use `Heading 1` through `Heading 6` (matching
   `docs/contracts.md`'s `HeadingLevel` variants). Tables use real Word
   table XML (`<w:tbl>`) with `gridSpan`/`vMerge` for `col_span`/
   `row_span`, not a manually drawn grid of bordered paragraphs. Figures
   embed into `word/media/` with a proper relationship and inline/anchor
   placement. Captions use the `Caption` style.
4. Math: convert `Equation`/`InlineMath` LaTeX source to OMML for the
   supported subset in `docs/specs.md` 1.8; outside that intersection,
   emit a `SyncWarning` per `docs/contracts.md` rather than a silently
   wrong OMML approximation.
5. `OpaqueBlock { origin: Latex, .. }` (content that originated in the
   LaTeX pane and has no DOCX equivalent): render as a clearly marked,
   non-editable-looking block in the output document, consistent with
   how `docs/specs.md` section 6 describes the UI-level visual treatment,
   translated into what's achievable in static DOCX output (for example
   a shaded paragraph with the `summary` text and a note that the
   original content is preserved in the paired `.tex` file).
   `OpaqueBlock { origin: Docx, .. }` content, if it round-trips through
   here at all in some edge case, should re-emit its own `raw` XML
   verbatim.
6. Write tests that produce `.docx` output and verify both the XML
   structure directly (via `quick-xml` parsing the output back, or XML
   snapshot comparison) and, for a representative sample, confirm the
   file opens correctly by converting to PDF with a local tool and
   visually checking layout (same verification pattern the `docx` skill
   uses: convert to PDF, rasterize, inspect).

## Definition of done

- [ ] Output is valid OOXML: a `.docx` produced by this module opens
      without repair warnings in Word and in LibreOffice.
- [ ] Every DM node type maps to its documented Word built-in style or
      structural equivalent; no block is represented purely through
      manual character formatting where a built-in style exists for it.
- [ ] Table merged cells produce correct `gridSpan`/`vMerge` XML, not a
      visual approximation using nested tables or empty cells.
- [ ] Math outside the OMML-compatible subset produces a `SyncWarning`,
      not a silently wrong rendering.
- [ ] `OpaqueBlock` content renders with a visibly distinct, non-editable
      treatment, never silently merged into surrounding normal text.
- [ ] At least one output file was manually opened and visually verified
      in Word or LibreOffice, not only validated structurally.
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
      pass.
