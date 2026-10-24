# parallel-task06-A: Okabe-Ito LaTeX highlighter

## Prerequisites

`serial-task05-diff-and-sync-engine` complete, and (specifically for this
task, unlike the other `parallel-task06-*` tasks) `parallel-task03-A`
(LaTeX parser) complete, since this task shares the parser's internals
per `docs/codebase.txt`'s note that `highlight.rs` "shares parser
internals."

## Can run alongside

`parallel-task06-B` (PDF export), `parallel-task06-C` (Markdown export),
`parallel-task06-D` (plain text export). None of these four touch each
other's files.

## Goal

Implement `src-tauri/src/core/latex/highlight.rs`: token classification
for the Okabe-Ito syntax highlighting described in `docs/specs.md`
section 5 and `docs/conventions.md`'s palette reference, exposed through
the `get_highlight_tokens` Tauri command from `docs/contracts.md` section
2.

## Read first

- `docs/specs.md` section 5 in full.
- `docs/conventions.md`, "Okabe-Ito palette reference," for the exact
  eight-color token mapping.
- `.agents/skills/tessera-frontend-design.md` section 2, "Color," for
  how this mapping needs to extend consistently into the DOCX pane's own
  highlight colors later (this task only produces the LaTeX-side tokens,
  but the token categories chosen here are what the frontend will mirror
  for DOCX pane highlighting in `serial-task09`, so name categories
  clearly).

## Files touched

```
src-tauri/src/core/latex/highlight.rs
src-tauri/src/commands/document.rs (add get_highlight_tokens handler,
  thin wrapper only)
src-tauri/tests/highlight_tests.rs (new)
```

## Steps

1. Reuse the parser's tokenization/lexing step from
   `parallel-task03-A` rather than re-implementing a second LaTeX
   tokenizer; "shares parser internals" per `docs/codebase.txt` is a
   hard requirement, not a suggestion, since a second independent
   tokenizer risks disagreeing with the parser about what a token is,
   which is exactly the failure mode `docs/specs.md` section 5 warns
   against ("highlighting and parsing never disagree about what a token
   is").
2. Classify each token into one of the eight Okabe-Ito categories per
   `docs/conventions.md`'s table: commands, braces/brackets, environment
   names, comments, math delimiters/content, errors/unresolved
   references, default text. Note the palette table has eight entries
   but one (`Opaque` block tint) is a block-level treatment, not a token
   classification; this task's token classifier produces seven token
   categories, not eight.
3. Implement `HighlightToken` output (per `docs/contracts.md` section 2's
   `get_highlight_tokens` row; check the exact return shape there, add
   it to `docs/contracts.md` if it is not already spelled out in full
   and this task is the first to need the complete shape, following
   `.agents/skills/contract-discipline.md`).
4. Handle "errors/unresolved references" (Vermillion) by checking
   against the current `Document`'s known `label` set (cross-references
   per `docs/contracts.md`'s invariants section) so an unresolved `\ref`
   highlights distinctly.
5. Write tests confirming token classification is correct across a range
   of fixtures, reusing `tests/fixtures/latex/` where possible.

## Definition of done

- [ ] Token classification reuses the LaTeX parser's own tokenizer, not
      a separately written one.
- [ ] All seven applicable Okabe-Ito token categories are correctly
      assigned per `docs/conventions.md`'s table.
- [ ] Unresolved cross-references highlight as errors (Vermillion).
- [ ] `get_highlight_tokens` command implemented and its response shape
      is documented in full in `docs/contracts.md` if it was not
      already.
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
