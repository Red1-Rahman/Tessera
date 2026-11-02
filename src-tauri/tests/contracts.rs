// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Round-trip contract test suite (`serial-task04`).
//!
//! Sections: JSON shape snapshots, golden fixtures, Opaque byte-exact
//! round trips, and `proptest` round-trip properties.

use insta::assert_json_snapshot;
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;
use std::fs;
use std::path::PathBuf;

use tessera_lib::core::docx::writer::DocxOutput;
use tessera_lib::core::docx::{read_docx, write_docx};
use tessera_lib::core::latex::{parse_latex, print_latex};
use tessera_lib::core::model::{
    Author, Block, DocMetadata, DocVersion, Document, Equation, Figure, HeadingLevel, Inline,
    ListKind, OpaqueBlock, OpaqueOrigin, Table, TableCell,
};

/// Absolute path into the repository-level `tests/fixtures` directory,
/// independent of the working directory `cargo test` happens to use.
fn fixtures_dir(kind: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(kind)
}

/// Writes `doc` to DOCX. The generated documents contain no figures, so the
/// assets directory is never read; the system temp dir is a safe placeholder.
fn docx_write(doc: &Document) -> DocxOutput {
    write_docx(doc, std::env::temp_dir()).expect("DOCX writer failed on Document")
}

// =========================================================================
// 1. Contract Type JSON Shape Snapshot Tests (docs/contracts.md Section 1)
// =========================================================================

#[test]
fn snapshot_doc_version_shape() {
    let version = DocVersion::first();
    assert_json_snapshot!("doc_version_shape", serde_json::to_value(version).unwrap());
}

#[test]
fn snapshot_doc_metadata_shape() {
    let author = Author::new("Alice Smith".into(), Some("University".into())).unwrap();
    let metadata = DocMetadata::new(
        Some("Test Title".into()),
        vec![author],
        Some("2026-10-25".into()),
        Some("article".into()),
        vec!["12pt".into(), "a4paper".into()],
    )
    .unwrap();
    assert_json_snapshot!(
        "doc_metadata_shape",
        serde_json::to_value(&metadata).unwrap()
    );
}

#[test]
fn snapshot_block_variants_shape() {
    let heading = Block::heading(
        HeadingLevel::H1,
        vec![Inline::text("Heading Text".into()).unwrap()],
        Some("sec:intro".into()),
    )
    .unwrap();
    let paragraph = Block::paragraph(vec![Inline::text("Paragraph text.".into()).unwrap()]);
    let list = Block::list(
        ListKind::Unordered,
        vec![vec![Block::paragraph(vec![
            Inline::text("Item 1".into()).unwrap()
        ])]],
    );
    let code_block = Block::code_block(Some("rust".into()), "fn main() {}".into()).unwrap();
    let quote = Block::quote(vec![Block::paragraph(vec![Inline::text(
        "Quoted text".into(),
    )
    .unwrap()])]);
    let opaque_node = OpaqueBlock::new(
        OpaqueOrigin::Latex,
        "\\custommacro".into(),
        "Raw TeX macro".into(),
    )
    .unwrap();
    let opaque = Block::opaque(opaque_node);

    assert_json_snapshot!(
        "block_heading_shape",
        serde_json::to_value(&heading).unwrap()
    );
    assert_json_snapshot!(
        "block_paragraph_shape",
        serde_json::to_value(&paragraph).unwrap()
    );
    assert_json_snapshot!("block_list_shape", serde_json::to_value(&list).unwrap());
    assert_json_snapshot!(
        "block_code_block_shape",
        serde_json::to_value(&code_block).unwrap()
    );
    assert_json_snapshot!("block_quote_shape", serde_json::to_value(&quote).unwrap());
    assert_json_snapshot!("block_opaque_shape", serde_json::to_value(&opaque).unwrap());
}

#[test]
fn snapshot_inline_variants_shape() {
    let text = Inline::text("Hello world".into()).unwrap();
    let bold = Inline::bold(vec![Inline::text("Bold text".into()).unwrap()]);
    let italic = Inline::italic(vec![Inline::text("Italic text".into()).unwrap()]);
    let underline = Inline::underline(vec![Inline::text("Underlined text".into()).unwrap()]);
    let code = Inline::code("println!()".into()).unwrap();
    let citation = Inline::citation(vec!["smith2024".into()]).unwrap();
    let cross_ref = Inline::cross_ref("sec:intro".into()).unwrap();
    let footnote = Inline::footnote(vec![Inline::text("Footnote content".into()).unwrap()]);
    let inline_math = Inline::inline_math("E = mc^2".into()).unwrap();
    let line_break = Inline::line_break();

    assert_json_snapshot!("inline_text_shape", serde_json::to_value(&text).unwrap());
    assert_json_snapshot!("inline_bold_shape", serde_json::to_value(&bold).unwrap());
    assert_json_snapshot!(
        "inline_italic_shape",
        serde_json::to_value(&italic).unwrap()
    );
    assert_json_snapshot!(
        "inline_underline_shape",
        serde_json::to_value(&underline).unwrap()
    );
    assert_json_snapshot!("inline_code_shape", serde_json::to_value(&code).unwrap());
    assert_json_snapshot!(
        "inline_citation_shape",
        serde_json::to_value(&citation).unwrap()
    );
    assert_json_snapshot!(
        "inline_cross_ref_shape",
        serde_json::to_value(&cross_ref).unwrap()
    );
    assert_json_snapshot!(
        "inline_footnote_shape",
        serde_json::to_value(&footnote).unwrap()
    );
    assert_json_snapshot!(
        "inline_inline_math_shape",
        serde_json::to_value(&inline_math).unwrap()
    );
    assert_json_snapshot!(
        "inline_line_break_shape",
        serde_json::to_value(&line_break).unwrap()
    );
}

#[test]
fn snapshot_table_and_cell_shape() {
    let cell = TableCell::new(
        vec![Block::paragraph(vec![
            Inline::text("Cell content".into()).unwrap()
        ])],
        1,
        1,
    )
    .unwrap();
    let table = Table::new(
        vec![vec![cell]],
        Some(vec![Inline::text("Table Caption".into()).unwrap()]),
        Some("tbl:test".into()),
    )
    .unwrap();
    let table_block = Block::table(table);

    assert_json_snapshot!(
        "table_block_shape",
        serde_json::to_value(&table_block).unwrap()
    );
}

#[test]
fn snapshot_figure_and_equation_shape() {
    let fig = Figure::new(
        "figure1.png".into(),
        Some(vec![Inline::text("Fig Caption".into()).unwrap()]),
        Some("fig:test".into()),
        Some(80.0),
    )
    .unwrap();
    let eq = Equation::new("a^2 + b^2 = c^2".into(), true, Some("eq:pythagoras".into())).unwrap();

    assert_json_snapshot!(
        "figure_block_shape",
        serde_json::to_value(Block::figure(fig)).unwrap()
    );
    assert_json_snapshot!(
        "equation_block_shape",
        serde_json::to_value(Block::equation(eq)).unwrap()
    );
}

// =========================================================================
// 2. Golden-File Fixture Tests
// =========================================================================
//
// Fixtures and snapshots must be checked out with LF line endings on every
// platform (see `.gitattributes`), otherwise `\r\n` leaks into the parsed
// trivia and the snapshots differ between Windows and Linux CI.

#[test]
fn test_golden_latex_fixtures() {
    let latex_dir = fixtures_dir("latex");
    assert!(
        latex_dir.exists(),
        "LaTeX fixture directory is missing: {}",
        latex_dir.display()
    );

    let mut paths: Vec<PathBuf> = fs::read_dir(&latex_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("tex"))
        .collect();
    paths.sort();

    assert!(
        !paths.is_empty(),
        "Expected at least one .tex fixture in {}, but found 0",
        latex_dir.display()
    );

    for path in paths {
        let content = fs::read_to_string(&path).unwrap();
        let parsed_doc = parse_latex(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e:?}", path.display()));
        let file_stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        assert_json_snapshot!(
            format!("golden_latex_{file_stem}"),
            serde_json::to_value(&parsed_doc).unwrap()
        );
    }
}

#[test]
fn test_golden_docx_fixtures() {
    let docx_dir = fixtures_dir("docx");
    assert!(
        docx_dir.exists(),
        "DOCX fixture directory is missing: {}",
        docx_dir.display()
    );

    let mut paths: Vec<PathBuf> = fs::read_dir(&docx_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("docx"))
        .collect();
    paths.sort();

    assert!(
        !paths.is_empty(),
        "Expected at least one .docx fixture in {}, but found 0",
        docx_dir.display()
    );

    for path in paths {
        let bytes = fs::read(&path).unwrap();
        let parsed_doc = read_docx(&bytes)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e:?}", path.display()));
        let file_stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        assert_json_snapshot!(
            format!("golden_docx_{file_stem}"),
            serde_json::to_value(&parsed_doc).unwrap()
        );
    }
}

// =========================================================================
// 3. Opaque Byte-Exact Round-Trip Tests
// =========================================================================

/// Prints a document holding one LaTeX-origin Opaque block, checks the raw
/// string is copied verbatim, then parses the output and checks that exactly
/// one Opaque block with the same raw string comes back.
///
/// Each `raw` must be ONE LaTeX construct (a single command with its
/// arguments, or one environment): that is the granularity at which the
/// parser creates Opaque blocks, so it is the granularity at which the
/// byte-exact property can hold.
fn assert_latex_opaque_roundtrip(raw: &str) {
    let node =
        OpaqueBlock::new(OpaqueOrigin::Latex, raw.to_string(), "Raw TeX".to_string()).unwrap();
    let doc = Document::new(DocMetadata::default(), vec![Block::opaque(node)]);
    let printed = print_latex(&doc);

    assert!(
        printed.contains(raw),
        "Printed LaTeX must contain the exact raw string.\nExpected:\n{raw}\nGot:\n{printed}"
    );

    let parsed = parse_latex(&printed).expect("Failed to parse printed LaTeX with Opaque block");
    let opaques: Vec<&OpaqueBlock> = parsed
        .body()
        .iter()
        .filter_map(|b| match b {
            Block::Opaque(op) => Some(op),
            _ => None,
        })
        .collect();

    assert_eq!(
        parsed.body().len(),
        1,
        "Expected exactly one block after re-parsing, got {:?}",
        parsed.body()
    );
    assert_eq!(opaques.len(), 1, "Expected exactly one Opaque block");
    assert_eq!(opaques[0].origin(), OpaqueOrigin::Latex);
    assert_eq!(opaques[0].raw(), raw);
}

#[test]
fn test_opaque_latex_environment_byte_exact_roundtrip() {
    assert_latex_opaque_roundtrip("\\begin{customenv}\n  body text\n\\end{customenv}");
}

#[test]
fn test_opaque_latex_command_byte_exact_roundtrip() {
    assert_latex_opaque_roundtrip("\\customcmd{arg1}");
}

#[test]
fn test_opaque_docx_byte_exact_roundtrip() {
    // The reader maps paragraphs with an unrecognized style to Opaque and
    // stores them in its normalized serialization (attribute order, explicit
    // `xml:space="preserve"`). The raw string below is already in that form
    // and uses a style name no Tessera mapping knows, so it must survive
    // write -> read byte-for-byte.
    let raw_docx_part = concat!(
        "<w:p><w:pPr><w:pStyle w:val=\"TesseraUnknownStyle\"/></w:pPr>",
        "<w:r><w:t xml:space=\"preserve\">Custom OOXML</w:t></w:r></w:p>"
    );
    let opaque_node = OpaqueBlock::new(
        OpaqueOrigin::Docx,
        raw_docx_part.to_string(),
        "Raw Docx XML".to_string(),
    )
    .unwrap();
    let opaque_block = Block::opaque(opaque_node);

    let doc = Document::new(DocMetadata::default(), vec![opaque_block]);
    let output = docx_write(&doc);
    let read_back_doc =
        read_docx(&output.bytes).expect("Failed to read back DOCX with Opaque block");

    let opaque_found = read_back_doc.body().iter().any(|b| match b {
        Block::Opaque(op) => op.origin() == OpaqueOrigin::Docx && op.raw() == raw_docx_part,
        _ => false,
    });

    assert!(
        opaque_found,
        "Read-back DOCX must contain the exact raw string of the Opaque node; got {:?}",
        read_back_doc.body()
    );
}

// =========================================================================
// 4. Proptest Round-Trip Property Tests
// =========================================================================
//
// Generators only produce values the Document Model accepts and that are in
// canonical form: no blank text, no leading/trailing space inside a text run
// at block edges, single spaces in metadata, and no two adjacent `Text`
// inlines (they are one logical run). A failure here is therefore a real
// parser/printer/reader/writer disagreement, not a generator gap.

/// Merges adjacent `Text` inlines (recursively) into canonical form.
fn merge_adjacent_text(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        let inline = match inline {
            Inline::Bold { inline } => Inline::bold(merge_adjacent_text(inline)),
            Inline::Italic { inline } => Inline::italic(merge_adjacent_text(inline)),
            Inline::Underline { inline } => Inline::underline(merge_adjacent_text(inline)),
            Inline::Footnote { inline } => Inline::footnote(merge_adjacent_text(inline)),
            other => other,
        };
        match (out.last_mut(), &inline) {
            (Some(Inline::Text { text: previous }), Inline::Text { text }) => {
                previous.push_str(text);
            }
            _ => out.push(inline),
        }
    }
    out
}

/// Text that is never blank and never starts or ends with a space.
const TEXT_RE: &str = "[a-zA-Z0-9_-]([a-zA-Z0-9 _-]{0,28}[a-zA-Z0-9_-])?";

fn arb_inline() -> impl Strategy<Value = Inline> {
    let leaf = prop_oneof![
        TEXT_RE.prop_map(|s| Inline::text(s).unwrap()),
        "[a-zA-Z0-9_]{1,20}".prop_map(|s| Inline::code(s).unwrap()),
        "[a-zA-Z0-9_]{1,15}".prop_map(|s| Inline::citation(vec![s]).unwrap()),
        "[a-zA-Z][a-zA-Z0-9_-]{1,15}".prop_map(|s| Inline::cross_ref(s).unwrap()),
        "[a-zA-Z0-9+=-][a-zA-Z0-9 +=-]{0,18}".prop_map(|s| Inline::inline_math(s).unwrap()),
        Just(Inline::line_break()),
    ];

    leaf.prop_recursive(3, 16, 3, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 1..4).prop_map(Inline::bold),
            prop::collection::vec(inner.clone(), 1..4).prop_map(Inline::italic),
            prop::collection::vec(inner.clone(), 1..4).prop_map(Inline::underline),
            prop::collection::vec(inner, 1..3).prop_map(Inline::footnote),
        ]
    })
}

fn arb_inlines(max: usize) -> impl Strategy<Value = Vec<Inline>> {
    prop::collection::vec(arb_inline(), 1..max).prop_map(merge_adjacent_text)
}

fn arb_docx_inlines(max: usize) -> impl Strategy<Value = Vec<Inline>> {
    prop::collection::vec(
        prop_oneof![
            TEXT_RE.prop_map(|s| Inline::text(s).unwrap()),
            Just(Inline::line_break()),
        ],
        1..max,
    )
    .prop_map(merge_adjacent_text)
}

fn arb_heading(labels: bool) -> impl Strategy<Value = Block> {
    let heading_level = prop_oneof![
        Just(HeadingLevel::H1),
        Just(HeadingLevel::H2),
        Just(HeadingLevel::H3),
        Just(HeadingLevel::H4),
    ];
    let label = if labels {
        prop::option::of("[a-zA-Z][a-zA-Z0-9_-]{1,10}").boxed()
    } else {
        Just(None::<String>).boxed()
    };

    (heading_level, arb_inlines(4), label)
        .prop_map(|(level, inlines, label)| Block::heading(level, inlines, label).unwrap())
}

/// `labels` controls whether headings may carry a `\label`; pass `false`
/// for properties where the target format has no label representation.
fn arb_non_opaque_block(labels: bool) -> impl Strategy<Value = Block> {
    let paragraph = arb_inlines(5).prop_map(Block::paragraph);

    let code_block = (
        prop::option::of("rust|latex|python"),
        "[a-zA-Z0-9][a-zA-Z0-9 _\n]{0,49}",
    )
        .prop_map(|(lang, text)| Block::code_block(lang, text).unwrap());

    prop_oneof![arb_heading(labels), paragraph, code_block]
}

fn arb_blocks(labels: bool) -> impl Strategy<Value = Vec<Block>> {
    prop::collection::vec(arb_non_opaque_block(labels), 1..5)
}

/// Metadata with a title and one author. `latex_class` is LaTeX-only data:
/// pass `false` for properties that go through DOCX, which has no place to
/// store it.
fn arb_metadata(latex_class: bool) -> impl Strategy<Value = DocMetadata> {
    // Words separated by single spaces: metadata is whitespace-collapsed by
    // the LaTeX parser, so doubled spaces are not canonical.
    let title = prop::option::of("[a-zA-Z0-9]+( [a-zA-Z0-9]+){0,3}");
    let author_name = "[a-zA-Z]+( [a-zA-Z]+){0,2}";
    (title, author_name).prop_map(move |(t, a)| {
        DocMetadata::new(
            t,
            vec![Author::new(a, None).unwrap()],
            None,
            if latex_class {
                Some("article".into())
            } else {
                None
            },
            vec![],
        )
        .unwrap()
    })
}

fn arb_latex_document() -> impl Strategy<Value = Document> {
    (arb_metadata(true), arb_blocks(true)).prop_map(|(meta, blocks)| Document::new(meta, blocks))
}

/// DOCX, body only: default metadata, no heading labels. Isolates the
/// block/inline mapping from the metadata and label questions.
fn arb_docx_body_document() -> impl Strategy<Value = Document> {
    prop::collection::vec(
        prop_oneof![
            (Just(HeadingLevel::H1), arb_docx_inlines(4),)
                .prop_map(|(level, inline)| Block::heading(level, inline, None).unwrap()),
            arb_docx_inlines(5).prop_map(Block::paragraph),
        ],
        1..5,
    )
    .prop_map(|blocks| Document::new(DocMetadata::default(), blocks))
}

/// DOCX, full: title and author (name only) metadata. No heading labels:
/// bookmark support is deferred (`docs/specs.md`, section 7), so labels are
/// outside the DOCX round-trip contract for now.
fn arb_docx_full_document() -> impl Strategy<Value = Document> {
    (
        arb_metadata(false),
        prop::collection::vec(
            prop_oneof![
                (Just(HeadingLevel::H1), arb_docx_inlines(4),)
                    .prop_map(|(level, inline)| Block::heading(level, inline, None).unwrap()),
                arb_docx_inlines(5).prop_map(Block::paragraph),
            ],
            1..5,
        ),
    )
        .prop_map(|(meta, blocks)| Document::new(meta, blocks))
}

proptest! {
    // Integration tests have no lib.rs/main.rs next to them, so the default
    // persistence location cannot be resolved. Pin it explicitly so failing
    // cases are saved and replayed (commit the file).
    #![proptest_config(ProptestConfig {
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(
            "tests/contracts.proptest-regressions"
        ))),
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_latex_roundtrip(doc in arb_latex_document()) {
        let printed = print_latex(&doc);
        let parsed = parse_latex(&printed).expect("LaTeX parser failed on printed Document");
        prop_assert_eq!(parsed, doc);
    }

    #[test]
    fn prop_docx_body_roundtrip(doc in arb_docx_body_document()) {
        let output = docx_write(&doc);
        prop_assert_eq!(output.warnings.len(), 0, "writer reported lossy constructs");
        let read_back = read_docx(&output.bytes).expect("DOCX reader failed on written bytes");
        prop_assert_eq!(read_back, doc);
    }

    #[test]
    fn prop_docx_roundtrip(doc in arb_docx_full_document()) {
        let output = docx_write(&doc);
        prop_assert_eq!(output.warnings.len(), 0, "writer reported lossy constructs");
        let read_back = read_docx(&output.bytes).expect("DOCX reader failed on written bytes");
        prop_assert_eq!(read_back, doc);
    }
}
