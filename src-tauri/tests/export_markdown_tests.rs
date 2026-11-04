// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use tessera_lib::core::export::markdown::render_markdown;
use tessera_lib::core::model::{
    Author, Block, DocMetadata, Document, Equation, Figure, HeadingLevel, Inline, ListKind,
    OpaqueBlock, OpaqueOrigin, Table, TableCell,
};

fn sample_metadata(title: Option<&str>) -> DocMetadata {
    DocMetadata::new(
        title.map(String::from),
        vec![Author::new("Test Author".into(), None).unwrap()],
        Some("2026-11-04".into()),
        Some("article".into()),
        vec![],
    )
    .unwrap()
}

#[test]
fn test_markdown_export_basic_structure() {
    let metadata = sample_metadata(Some("My Document"));
    let blocks = vec![
        Block::heading(
            HeadingLevel::H1,
            vec![Inline::text("Introduction".into()).unwrap()],
            Some("sec:intro".into()),
        )
        .unwrap(),
        Block::paragraph(vec![
            Inline::text("This is the first paragraph with ".into()).unwrap(),
            Inline::bold(vec![Inline::text("bold text".into()).unwrap()]),
            Inline::text(" and ".into()).unwrap(),
            Inline::italic(vec![Inline::text("italic text".into()).unwrap()]),
            Inline::text(".".into()).unwrap(),
        ]),
        Block::heading(
            HeadingLevel::H2,
            vec![Inline::text("Subsection".into()).unwrap()],
            None,
        )
        .unwrap(),
        Block::paragraph(vec![
            Inline::text("A paragraph with a line break.".into()).unwrap(),
            Inline::line_break(),
            Inline::text("Second line here.".into()).unwrap(),
        ]),
    ];

    let doc = Document::new(metadata, blocks);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("# My Document"));
    assert!(md.contains("# Introduction {#sec:intro}"));
    assert!(md.contains("## Subsection"));
    assert!(md.contains("**bold text**"));
    assert!(md.contains("*italic text*"));
    assert!(md.contains("  \nSecond line here."));
}

#[test]
fn test_markdown_export_inlines() {
    let metadata = sample_metadata(None);
    let blocks = vec![Block::paragraph(vec![
        Inline::underline(vec![Inline::text("underlined".into()).unwrap()]),
        Inline::text(" and ".into()).unwrap(),
        Inline::code("let x = 42;".into()).unwrap(),
        Inline::text(" and citation ".into()).unwrap(),
        Inline::citation(vec!["smith2024".into(), "jones2025".into()]).unwrap(),
        Inline::text(" and ref ".into()).unwrap(),
        Inline::cross_ref("sec:intro".into()).unwrap(),
        Inline::text(" and math ".into()).unwrap(),
        Inline::inline_math("E = mc^2".into()).unwrap(),
    ])];

    let doc = Document::new(metadata, blocks);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("<u>underlined</u>"));
    assert!(md.contains("`let x = 42;`"));
    assert!(md.contains("[@smith2024; @jones2025]"));
    assert!(md.contains("[@sec:intro]"));
    assert!(md.contains("$E = mc^2$"));
}

#[test]
fn test_markdown_export_lists() {
    let metadata = sample_metadata(None);
    let unordered = Block::list(
        ListKind::Unordered,
        vec![
            vec![Block::paragraph(vec![Inline::text("Item 1".into()).unwrap()])],
            vec![Block::paragraph(vec![Inline::text("Item 2".into()).unwrap()])],
        ],
    );
    let ordered = Block::list(
        ListKind::Ordered,
        vec![
            vec![Block::paragraph(vec![Inline::text("First".into()).unwrap()])],
            vec![Block::paragraph(vec![Inline::text("Second".into()).unwrap()])],
        ],
    );

    let doc = Document::new(metadata, vec![unordered, ordered]);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("- Item 1"));
    assert!(md.contains("- Item 2"));
    assert!(md.contains("1. First"));
    assert!(md.contains("2. Second"));
}

#[test]
fn test_markdown_export_code_block_and_quote() {
    let metadata = sample_metadata(None);
    let code = Block::code_block(Some("rust".into()), "fn main() {\n    println!(\"Hello\");\n}".into()).unwrap();
    let quote = Block::quote(vec![Block::paragraph(vec![Inline::text("Quoted text line".into()).unwrap()])]);

    let doc = Document::new(metadata, vec![code, quote]);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("```rust\nfn main() {\n    println!(\"Hello\");\n}\n```"));
    assert!(md.contains("> Quoted text line"));
}

#[test]
fn test_markdown_export_equations_and_figures() {
    let metadata = sample_metadata(None);
    let eq = Block::equation(Equation::new("\\int_0^\\infty x^2 dx".into(), true, Some("eq:int".into())).unwrap());
    let fig = Block::figure(
        Figure::new(
            "figure1.png".into(),
            Some(vec![Inline::text("Sample Figure".into()).unwrap()]),
            Some("fig:sample".into()),
            Some(85.0),
        )
        .unwrap(),
    );

    let doc = Document::new(metadata, vec![eq, fig]);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("$$\\int_0^\\infty x^2 dx$$"));
    assert!(md.contains("![Sample Figure](figure1.png){width=85%}"));
}

#[test]
fn test_markdown_export_table_spans() {
    let metadata = sample_metadata(None);
    let cell1 = TableCell::new(
        vec![Block::paragraph(vec![Inline::text("Merged Col".into()).unwrap()])],
        2,
        1,
    )
    .unwrap();
    let cell2 = TableCell::new(
        vec![Block::paragraph(vec![Inline::text("Row 1 Cell".into()).unwrap()])],
        1,
        2,
    )
    .unwrap();
    let cell3 = TableCell::new(
        vec![Block::paragraph(vec![Inline::text("Normal Cell".into()).unwrap()])],
        1,
        1,
    )
    .unwrap();
    let cell4 = TableCell::new(
        vec![Block::paragraph(vec![Inline::text("Lower Cell".into()).unwrap()])],
        1,
        1,
    )
    .unwrap();

    let table = Table::new(
        vec![vec![cell1], vec![cell2, cell3], vec![cell4]],
        Some(vec![Inline::text("Table Caption".into()).unwrap()]),
        Some("tab:demo".into()),
    )
    .unwrap();

    let doc = Document::new(metadata, vec![Block::table(table)]);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("*Table Caption* {#tab:demo}"));
    assert!(md.contains("| Merged Col | Merged Col |"));
    assert!(md.contains("| --- | --- |"));
}

#[test]
fn test_markdown_export_opaque_block() {
    let metadata = sample_metadata(None);
    let opaque_latex = Block::opaque(
        OpaqueBlock::new(
            OpaqueOrigin::Latex,
            "\\custommacro{arg}".into(),
            "Raw TeX macro".into(),
        )
        .unwrap(),
    );
    let opaque_docx = Block::opaque(
        OpaqueBlock::new(
            OpaqueOrigin::Docx,
            "<w:customObj/>".into(),
            "OLE Object".into(),
        )
        .unwrap(),
    );

    let doc = Document::new(metadata, vec![opaque_latex, opaque_docx]);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("<!-- Opaque LaTeX block omitted from Markdown export: Raw TeX macro -->"));
    assert!(md.contains("<!-- Opaque DOCX block omitted from Markdown export: OLE Object -->"));
}

#[test]
fn test_markdown_export_footnotes() {
    let metadata = sample_metadata(None);
    let para = Block::paragraph(vec![
        Inline::text("Text with a footnote".into()).unwrap(),
        Inline::footnote(vec![Inline::text("Footnote details here".into()).unwrap()]),
        Inline::text(".".into()).unwrap(),
    ]);

    let doc = Document::new(metadata, vec![para]);
    let md = render_markdown(&doc).expect("markdown export succeeds");

    assert!(md.contains("Text with a footnote[^1]."));
    assert!(md.contains("[^1]: Footnote details here"));
}