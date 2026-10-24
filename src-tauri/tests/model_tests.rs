// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Unit tests for the Document Model's validating constructors and its
//! serialized shapes. These types carry `docs/contracts.md`, section 1,
//! so both the accept and the reject path of every constructor is
//! pinned here (see `.agents/tasks/serial-task02-document-model.md`).

use serde_json::{from_value, json, to_value};

use tessera_lib::core::error::{Pane, SourceSpan, TesseraError};
use tessera_lib::core::model::*;

fn validation_message(error: TesseraError) -> String {
    match error {
        TesseraError::ValidationError { field, message } => format!("{field}: {message}"),
        other => panic!("expected a ValidationError, got {other:?}"),
    }
}

#[test]
fn author_accepts_valid_input_and_trims_it() {
    let author = Author::new(
        "  Ada Lovelace  ".to_string(),
        Some(" Analytical Engine ".to_string()),
    )
    .expect("valid author");
    assert_eq!(author.name(), "Ada Lovelace");
    assert_eq!(author.affiliation(), Some("Analytical Engine"));
}

#[test]
fn author_accepts_missing_affiliation() {
    let author = Author::new("Ada Lovelace".to_string(), None).expect("valid author");
    assert_eq!(author.affiliation(), None);
}

#[test]
fn author_rejects_empty_and_whitespace_names() {
    let empty = validation_message(Author::new(String::new(), None).unwrap_err());
    assert!(empty.contains("author.name"), "{empty}");

    let blank = validation_message(Author::new("   \t ".to_string(), None).unwrap_err());
    assert!(blank.contains("author.name"), "{blank}");
}

#[test]
fn author_rejects_blank_affiliation() {
    let error = validation_message(
        Author::new("Ada Lovelace".to_string(), Some("  ".to_string())).unwrap_err(),
    );
    assert!(error.contains("author.affiliation"), "{error}");
}

#[test]
fn figure_accepts_relative_asset_path_with_width() {
    let figure = Figure::new(
        "figure1.png".to_string(),
        None,
        Some("fig:pipeline".to_string()),
        Some(75.0),
    )
    .expect("valid figure");
    assert_eq!(figure.asset_path(), "figure1.png");
    assert_eq!(figure.label(), Some("fig:pipeline"));
    assert_eq!(figure.width_percent(), Some(75.0));

    let nested = Figure::new("figures/plot1.png".to_string(), None, None, None)
        .expect("nested relative path is valid");
    assert_eq!(nested.asset_path(), "figures/plot1.png");
}

#[test]
fn figure_rejects_absolute_and_traversing_paths() {
    for path in [
        "C:\\secrets\\img.png",
        "C:img.png",
        "\\\\server\\share\\img.png",
        "/etc/img.png",
        "../img.png",
        "a/../b.png",
        "..\\img.png",
    ] {
        let error =
            validation_message(Figure::new(path.to_string(), None, None, None).unwrap_err());
        assert!(
            error.contains("figure.asset_path"),
            "path {path:?} should be rejected: {error}"
        );
    }
}

#[test]
fn figure_rejects_out_of_range_and_non_finite_widths() {
    for width in [
        Some(0.0),
        Some(-5.0),
        Some(100.5),
        Some(f32::NAN),
        Some(f32::INFINITY),
    ] {
        let error = validation_message(
            Figure::new("figure1.png".to_string(), None, None, width).unwrap_err(),
        );
        assert!(
            error.contains("figure.width_percent"),
            "width {width:?} should be rejected: {error}"
        );
    }
    assert!(
        Figure::new("figure1.png".to_string(), None, None, Some(100.0)).is_ok(),
        "100 percent is a valid width"
    );
}

#[test]
fn figure_rejects_blank_asset_path() {
    let error = validation_message(Figure::new("   ".to_string(), None, None, None).unwrap_err());
    assert!(error.contains("figure.asset_path"), "{error}");
}

#[test]
fn figure_label_must_be_a_valid_cross_reference_key() {
    let error = validation_message(
        Figure::new(
            "figure1.png".to_string(),
            None,
            Some("fig 1".to_string()),
            None,
        )
        .unwrap_err(),
    );
    assert!(error.contains("figure.label"), "{error}");
}

#[test]
fn equation_accepts_valid_source_and_trims_it() {
    let equation = Equation::new(
        "  E = mc^2  ".to_string(),
        true,
        Some("eq:mass".to_string()),
    )
    .expect("valid equation");
    assert_eq!(equation.latex_source(), "E = mc^2");
    assert!(equation.display_mode());
    assert_eq!(equation.label(), Some("eq:mass"));
}

#[test]
fn equation_rejects_empty_source() {
    let error = validation_message(Equation::new("  \n ".to_string(), false, None).unwrap_err());
    assert!(error.contains("equation.latex_source"), "{error}");
}

#[test]
fn table_accepts_valid_cell_spans() {
    let cell = TableCell::new(vec![], 2, 1).expect("valid cell");
    assert_eq!(cell.col_span(), 2);
    assert_eq!(cell.row_span(), 1);
    let table =
        Table::new(vec![vec![cell]], None, Some("tab:results".to_string())).expect("valid table");
    assert_eq!(table.label(), Some("tab:results"));
    assert_eq!(table.rows().len(), 1);
}

#[test]
fn table_cell_rejects_zero_spans() {
    let col = validation_message(TableCell::new(vec![], 0, 1).unwrap_err());
    assert!(col.contains("table_cell.col_span"), "{col}");
    let row = validation_message(TableCell::new(vec![], 1, 0).unwrap_err());
    assert!(row.contains("table_cell.row_span"), "{row}");
}

#[test]
fn heading_validates_its_label() {
    let heading = Block::heading(HeadingLevel::H1, vec![], Some("sec:intro".to_string()))
        .expect("valid heading");
    assert_eq!(heading.label(), Some("sec:intro"));

    let error = validation_message(
        Block::heading(HeadingLevel::H1, vec![], Some("  ".to_string())).unwrap_err(),
    );
    assert!(error.contains("heading.label"), "{error}");
}

#[test]
fn opaque_block_requires_raw_content_and_a_summary() {
    let block = OpaqueBlock::new(
        OpaqueOrigin::Latex,
        "\\customcmd{value}".to_string(),
        "Raw TeX macro: \\customcmd".to_string(),
    )
    .expect("valid opaque block");
    assert_eq!(block.raw(), "\\customcmd{value}");
    assert_eq!(block.origin(), OpaqueOrigin::Latex);

    let raw = validation_message(
        OpaqueBlock::new(OpaqueOrigin::Latex, String::new(), "summary".to_string()).unwrap_err(),
    );
    assert!(raw.contains("opaque.raw"), "{raw}");

    let summary = validation_message(
        OpaqueBlock::new(OpaqueOrigin::Docx, "raw".to_string(), "   ".to_string()).unwrap_err(),
    );
    assert!(summary.contains("opaque.summary"), "{summary}");
}

#[test]
fn doc_metadata_rejects_blank_optional_fields_and_class_options() {
    let title = validation_message(
        DocMetadata::new(Some("  ".to_string()), vec![], None, None, vec![]).unwrap_err(),
    );
    assert!(title.contains("metadata.title"), "{title}");

    let option = validation_message(
        DocMetadata::new(
            None,
            vec![],
            None,
            None,
            vec!["12pt".to_string(), " ".to_string()],
        )
        .unwrap_err(),
    );
    assert!(
        option.contains("metadata.latex_class_options[1]"),
        "{option}"
    );
}

#[test]
fn doc_metadata_accepts_valid_preamble_values() {
    let author = Author::new("Ada Lovelace".to_string(), None).expect("valid author");
    let metadata = DocMetadata::new(
        Some("On Analytical Engines".to_string()),
        vec![author],
        Some("2026".to_string()),
        Some("article".to_string()),
        vec!["12pt".to_string(), "a4paper".to_string()],
    )
    .expect("valid metadata");
    assert_eq!(metadata.title(), Some("On Analytical Engines"));
    assert_eq!(metadata.authors().len(), 1);
    assert_eq!(metadata.latex_class(), Some("article"));
    assert_eq!(
        metadata.latex_class_options(),
        ["12pt".to_string(), "a4paper".to_string()]
    );
}

#[test]
fn inline_constructors_reject_content_that_cannot_round_trip() {
    let text = validation_message(Inline::text("   ".to_string()).unwrap_err());
    assert!(text.contains("inline.text"), "{text}");

    let math = validation_message(Inline::inline_math(String::new()).unwrap_err());
    assert!(math.contains("inline.math.latex_source"), "{math}");

    let keys = validation_message(Inline::citation(vec![]).unwrap_err());
    assert!(keys.contains("inline.citation.keys"), "{keys}");

    let key = validation_message(
        Inline::citation(vec!["knuth1984".to_string(), " ".to_string()]).unwrap_err(),
    );
    assert!(key.contains("inline.citation.keys[1]"), "{key}");

    let cross_ref = validation_message(Inline::cross_ref("has space".to_string()).unwrap_err());
    assert!(cross_ref.contains("inline.cross_ref.label"), "{cross_ref}");
}

#[test]
fn inline_constructors_accept_valid_content() {
    assert_eq!(
        Inline::citation(vec!["knuth1984".to_string()]).expect("valid citation"),
        Inline::Citation {
            keys: vec!["knuth1984".to_string()]
        }
    );
    assert_eq!(
        Inline::cross_ref("sec:intro".to_string()).expect("valid cross ref"),
        Inline::CrossRef {
            label: "sec:intro".to_string()
        }
    );
    assert_eq!(Inline::line_break(), Inline::LineBreak);
}

#[test]
fn document_starts_at_the_first_version_and_bumps() {
    let metadata = DocMetadata::default();
    let mut document = Document::new(metadata, vec![Block::paragraph(vec![])]);
    assert_eq!(document.version(), DocVersion::first());
    assert_eq!(document.version().value(), 0);
    assert_eq!(document.bump_version(), DocVersion::first().next());
    assert_eq!(document.version().value(), 1);
    assert_eq!(document.bump_version().value(), 2);
    assert_eq!(document.version().value(), 2);
}

#[test]
fn doc_version_orders_and_serializes_as_a_bare_number() {
    assert!(DocVersion::first() < DocVersion::first().next());
    assert_eq!(
        to_value(DocVersion::first().next()).expect("serialize"),
        json!(1)
    );
}

#[test]
fn block_heading_serializes_with_a_type_tag() {
    let heading = Block::heading(
        HeadingLevel::H2,
        vec![Inline::text("Methods".to_string()).expect("valid text")],
        Some("sec:methods".to_string()),
    )
    .expect("valid heading");
    assert_eq!(
        to_value(heading).expect("serialize"),
        json!({
            "type": "Heading",
            "level": "H2",
            "inline": [{ "type": "Text", "text": "Methods" }],
            "label": "sec:methods"
        })
    );
}

#[test]
fn inline_round_trips_through_serde() {
    let inline = Inline::bold(vec![
        Inline::text("important".to_string()).expect("valid text"),
        Inline::inline_math("x^2".to_string()).expect("valid math"),
    ]);
    let value = to_value(&inline).expect("serialize");
    assert_eq!(
        value,
        json!({
            "type": "Bold",
            "inline": [
                { "type": "Text", "text": "important" },
                { "type": "InlineMath", "latex_source": "x^2" }
            ]
        })
    );
    let back: Inline = from_value(value).expect("deserialize");
    assert_eq!(back, inline);
}

#[test]
fn opaque_block_serializes_with_its_origin() {
    let block = Block::opaque(
        OpaqueBlock::new(
            OpaqueOrigin::Latex,
            "\\mycmd{}".to_string(),
            "Raw TeX macro".to_string(),
        )
        .expect("valid opaque block"),
    );
    assert_eq!(
        to_value(block).expect("serialize"),
        json!({
            "type": "Opaque",
            "origin": "Latex",
            "raw": "\\mycmd{}",
            "summary": "Raw TeX macro"
        })
    );
}

#[test]
fn tessera_error_serializes_with_a_kind_tag() {
    let error = TesseraError::ParseError {
        pane: Pane::Latex,
        message: "unexpected token".to_string(),
        span: Some(SourceSpan {
            start_line: 3,
            start_col: 5,
            end_line: 3,
            end_col: 9,
        }),
    };
    assert_eq!(
        to_value(error).expect("serialize"),
        json!({
            "kind": "ParseError",
            "pane": "Latex",
            "message": "unexpected token",
            "span": {
                "start_line": 3,
                "start_col": 5,
                "end_line": 3,
                "end_col": 9
            }
        })
    );
}
