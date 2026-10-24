# parallel-task06-D: Plain text export

## Prerequisites

`serial-task05-diff-and-sync-engine` complete.

## Can run alongside

`parallel-task06-A` (highlighter), `parallel-task06-B` (PDF export),
`parallel-task06-C` (Markdown export).

## Goal

Implement `src-tauri/src/core/export/plaintext.rs`: Document Model to
plain text, per `docs/specs.md` section 4's plain text row. This is an
intentionally lossy export; the task is to flatten cleanly and legibly,
not to preserve structure.

## Read first

- `docs/specs.md` section 4, the plain text row specifically, including
  its note that this format "is lossy by nature and labeled as such in
  the export menu."
- `docs/contracts.md` section 1.
- `.agents/prompts/add-export-format.md` steps 3 and 5.

## Files touched

```
src-tauri/src/core/export/plaintext.rs
src-tauri/src/commands/export.rs (plain text arm of the export command)
src-tauri/tests/export_plaintext_tests.rs (new)
```

## Steps

1. Implement a pure function `render_plaintext(doc: &Document) ->
   Result<String, TesseraError>`.
2. Headings become underlined or numbered lines per `docs/specs.md`'s
   description; pick one convention and apply it consistently (for
   example, H1 underlined with `=`, H2 with `-`, H3 and below prefixed
   with a numeral or indentation), and record the chosen convention in
   `docs/specs.md` if it does not already spell out the exact rule.
3. All inline formatting (bold, italic, underline, code) flattens to
   plain text with no markup; do not invent ad hoc markers (asterisks,
   underscores) since that would make this format ambiguous with
   Markdown, which already exists as a separate, non-lossy-by-design
   option.
4. Math: render the LaTeX math source as-is inline (no markup wrapper),
   since there is no plain-text math notation to convert to; this is an
   accepted, documented loss of visual fidelity for this format, not a
   bug.
5. `OpaqueBlock` and tables: flatten to readable text (table rows as
   space- or tab-separated values, `OpaqueBlock` as its `summary` text
   in brackets); this format does not need a structured fallback since
   it is lossy throughout by design, unlike the Markdown export's more
   careful handling.
6. Write tests rendering representative fixtures and snapshot-comparing
   output, including at least one test confirming the output contains no
   stray markup characters that could be mistaken for the Markdown
   export's output.

## Definition of done

- [ ] Every DM node type renders to plain text without panicking.
- [ ] Heading convention is consistent and documented in
      `docs/specs.md` if not already.
- [ ] No ad hoc markup characters appear in the output.
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
