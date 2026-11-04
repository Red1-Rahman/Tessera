// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use tessera_lib::core::export::plaintext::render_plaintext;
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
fn test_plaintext_export_headings_underline_levels() {
    let metadata = sample_metadata(Some("Plain Document Title"));
    let blocks = vec![
        Block::heading(
            HeadingLevel::H1,
            vec![Inline::text("First Level Heading".into()).unwrap()],
            None,
        )
        .unwrap(),
        Block::paragraph(vec![Inline::text("Paragraph under H1.".into()).unwrap()]),
        Block::heading(
            HeadingLevel::H2,
            vec![Inline::text("Second Level Heading".into()).unwrap()],
            None,
        )
        .unwrap(),
        Block::paragraph(vec![Inline::text("Paragraph under H2.".into()).unwrap()]),
        Block::heading(
            HeadingLevel::H3,
            vec![Inline::text("Third Level Heading".into()).unwrap()],
            None,
        )
        .unwrap(),
        Block::paragraph(vec![Inline::text("Paragraph under H3.".into()).unwrap()]),
    ];

    let doc = Document::new(metadata, blocks);
    let text = render_plaintext(&doc).expect("plain text export succeeds");

    // Level 1 heading should be underlined with '='
    assert!(text.contains("First Level Heading\n==================="));
    // Level 2 heading should be underlined with '-'
    assert!(text.contains("Second Level Heading\n--------------------"));
    // Level 3 heading should be indented without underline markup
    assert!(text.contains(" Third Level Heading"));
}

#[test]
fn test_plaintext_export_no_markdown_syntax() {
    let metadata = sample_metadata(None);
    let blocks = vec![Block::paragraph(vec![
        Inline::bold(vec![Inline::text("bold text".into()).unwrap()]),
        Inline::text(" and ".into()).unwrap(),
        Inline::italic(vec![Inline::text("italic text".into()).unwrap()]),
        Inline::text(" and ".into()).unwrap(),
        Inline::underline(vec![Inline::text("underlined".into()).unwrap()]),
        Inline::text(" and ".into()).unwrap(),
        Inline::code("let x = 42;".into()).unwrap(),
        Inline::text(" and ".into()).unwrap(),
        Inline::citation(vec!["smith2024".into()]).unwrap(),
        Inline::text(" and ".into()).unwrap(),
        Inline::cross_ref("sec:intro".into()).unwrap(),
    ])];

    let doc = Document::new(metadata, blocks);
    let text = render_plaintext(&doc).expect("plain text export succeeds");

    // Must not contain Markdown markup indicators
    assert!(!text.contains("**bold text**"));
    assert!(!text.contains("*italic text*"));
    assert!(!text.contains("<u>underlined</u>"));
    assert!(!text.contains("`let x = 42;`"));
    assert!(!text.contains("[@smith2024]"));
    assert!(!text.contains("[@sec:intro]"));

    // Should contain flattened text
    assert!(text.contains(
        "bold text and italic text and underlined and let x = 42; and [smith2024] and [sec:intro]"
    ));
}

#[test]
fn test_plaintext_export_lists_no_markdown_markers() {
    let metadata = sample_metadata(None);
    let unordered = Block::list(
        ListKind::Unordered,
        vec![
            vec![Block::paragraph(vec![
                Inline::text("Bullet One".into()).unwrap()
            ])],
            vec![Block::paragraph(vec![
                Inline::text("Bullet Two".into()).unwrap()
            ])],
        ],
    );
    let ordered = Block::list(
        ListKind::Ordered,
        vec![
            vec![Block::paragraph(vec![
                Inline::text("Step One".into()).unwrap()
            ])],
            vec![Block::paragraph(vec![
                Inline::text("Step Two".into()).unwrap()
            ])],
        ],
    );

    let doc = Document::new(metadata, vec![unordered, ordered]);
    let text = render_plaintext(&doc).expect("plain text export succeeds");

    // Unordered items use exact two-space indentation ("  Bullet One"), not Markdown "-" or "*"
    assert!(text.lines().any(|line| line == "  Bullet One"));
    assert!(text.lines().any(|line| line == "  Bullet Two"));
    assert!(!text.contains("- Bullet One"));
    assert!(!text.contains("* Bullet One"));

    // Ordered items use "(1) ", not Markdown "1. "
    assert!(text.lines().any(|line| line == "(1) Step One"));
    assert!(text.lines().any(|line| line == "(2) Step Two"));
    assert!(!text.contains("1. Step One"));
}

#[test]
fn test_plaintext_export_table_equation_figure_opaque() {
    let metadata = sample_metadata(None);
    let cell1 = TableCell::new(
        vec![Block::paragraph(vec![
            Inline::text("Header 1".into()).unwrap()
        ])],
        1,
        1,
    )
    .unwrap();
    let cell2 = TableCell::new(
        vec![Block::paragraph(vec![
            Inline::text("Header 2".into()).unwrap()
        ])],
        1,
        1,
    )
    .unwrap();

    let table = Block::table(
        Table::new(
            vec![vec![cell1, cell2]],
            Some(vec![Inline::text("Sample Table".into()).unwrap()]),
            None,
        )
        .unwrap(),
    );

    let eq = Block::equation(Equation::new("E = mc^2".into(), true, None).unwrap());
    let fig = Block::figure(
        Figure::new(
            "arch.png".into(),
            Some(vec![Inline::text("Architecture Diagram".into()).unwrap()]),
            None,
            None,
        )
        .unwrap(),
    );
    let opaque = Block::opaque(
        OpaqueBlock::new(OpaqueOrigin::Latex, "\\rawcmd".into(), "Raw TeX".into()).unwrap(),
    );

    let doc = Document::new(metadata, vec![table, eq, fig, opaque]);
    let text = render_plaintext(&doc).expect("plain text export succeeds");

    assert!(text.contains("Table: Sample Table"));
    assert!(text.contains("Header 1\tHeader 2"));
    assert!(text.contains("E = mc^2"));
    assert!(text.contains("Figure: Architecture Diagram (arch.png)"));
    assert!(text.contains("[LaTeX content not shown: Raw TeX]"));
}
