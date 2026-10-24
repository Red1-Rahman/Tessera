# parallel-task06-C: Markdown export

## Prerequisites

`serial-task05-diff-and-sync-engine` complete.

## Can run alongside

`parallel-task06-A` (highlighter), `parallel-task06-B` (PDF export),
`parallel-task06-D` (plain text export).

## Goal

Implement `src-tauri/src/core/export/markdown.rs`: Document Model to
Markdown, per `docs/specs.md` section 4's Markdown row.

## Read first

- `docs/specs.md` section 4, the Markdown row specifically.
- `docs/contracts.md` section 1, for the full set of node types this
  renderer must handle.
- `docs/techstack.yml`'s entry for `pulldown-cmark`.
- `.agents/prompts/add-export-format.md` steps 3 and 5 for the general
  renderer pattern this task follows.

## Files touched

```
src-tauri/src/core/export/markdown.rs
src-tauri/src/commands/export.rs (Markdown arm of the export command)
src-tauri/tests/export_markdown_tests.rs (new)
```

## Steps

1. Implement a pure function `render_markdown(doc: &Document) ->
   Result<String, TesseraError>` covering every `Block`/`Inline` variant
   in `docs/contracts.md` section 1.
2. Math renders as raw LaTeX math delimited by `$...$` (inline) and
   `$$...$$` (display), the Pandoc-compatible convention named
   explicitly in `docs/specs.md`.
3. `OpaqueBlock` nodes render as a fenced comment noting what was
   dropped (`docs/specs.md`'s exact wording), never silently omitted and
   never rendered as if it were real Markdown content.
4. Tables render as standard Markdown table syntax; cells with
   `col_span`/`row_span` greater than 1 cannot be represented in plain
   Markdown tables, so flatten with a documented, visible convention
   (for example repeating the cell content or inserting an empty cell
   with a note) rather than silently losing the span information;
   document the chosen convention in `docs/specs.md` if it is not
   already specified there.
5. Write tests rendering representative fixtures (reuse
   `tests/fixtures/`) and snapshot-comparing output.

## Definition of done

- [ ] Every DM node type renders to Markdown without panicking.
- [ ] Math uses the documented `$...$`/`$$...$$` convention.
- [ ] `OpaqueBlock` renders as a visible fenced comment, never silently
      dropped.
- [ ] The merged/spanned table cell convention is implemented and
      documented in `docs/specs.md` if not already present.
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
