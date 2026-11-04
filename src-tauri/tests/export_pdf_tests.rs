// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::fs;
use std::path::Path;

use tessera_lib::core::error::ExportFormat;
use tessera_lib::core::export::pdf::{detect_tex_engine, export_pdf_with_path_override};
use tessera_lib::core::export::{export_document, ExportRequest, ExportResult, PdfEngine};
use tessera_lib::core::latex::parse_latex;
use tessera_lib::core::model::{Author, Block, DocMetadata, Document, HeadingLevel, Inline};

fn sample_document() -> Document {
    let metadata = DocMetadata::new(
        Some("PDF Test Paper".into()),
        vec![Author::new("Test Author".into(), None).unwrap()],
        Some("2026-11-04".into()),
        Some("article".into()),
        vec![],
    )
    .unwrap();
    let blocks = vec![
        Block::heading(
            HeadingLevel::H1,
            vec![Inline::text("Introduction".into()).unwrap()],
            None,
        )
        .unwrap(),
        Block::paragraph(vec![
            Inline::text("Testing PDF generation output.".into()).unwrap()
        ]),
    ];
    Document::new(metadata, blocks)
}

#[test]
fn test_builtin_pdf_export_creates_valid_pdf_file() {
    let temp_dir = std::env::temp_dir().join(format!("tessera_test_pdf_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let pdf_path = temp_dir.join("output.pdf");

    let doc = sample_document();
    let assets = temp_dir.join("assets");

    // Pass empty fake PATH to force built-in fallback typesetter
    let engine =
        export_pdf_with_path_override(&doc, &assets, &pdf_path.to_string_lossy(), Some(""))
            .expect("builtin PDF export succeeds");

    assert_eq!(engine, PdfEngine::BuiltIn);
    assert!(pdf_path.exists());

    let bytes = fs::read(&pdf_path).expect("read pdf file");
    assert!(bytes.len() > 100);
    // PDF files start with %PDF- header magic bytes
    assert_eq!(&bytes[0..5], b"%PDF-");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_builtin_pdf_export_handles_latex_fixtures() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("latex");
    let temp_dir =
        std::env::temp_dir().join(format!("tessera_test_pdf_fixtures_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("create fixture PDF output directory");

    for fixture in ["parser-basic.tex", "printer-canonical.tex"] {
        let source = fs::read_to_string(fixture_dir.join(fixture)).expect("read LaTeX fixture");
        let document = parse_latex(&source).expect("parse LaTeX fixture");
        let output = temp_dir.join(fixture.replace(".tex", ".pdf"));
        let engine = export_pdf_with_path_override(
            &document,
            &temp_dir.join("assets"),
            &output.to_string_lossy(),
            Some(""),
        )
        .expect("render fixture PDF");

        assert_eq!(engine, PdfEngine::BuiltIn);
        let bytes = fs::read(output).expect("read fixture PDF");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_fake_path_tex_engine_detection() {
    let temp_dir =
        std::env::temp_dir().join(format!("tessera_test_fake_path_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    #[cfg(target_os = "windows")]
    let fake_executable = temp_dir.join("pdflatex.exe");
    #[cfg(not(target_os = "windows"))]
    let fake_executable = temp_dir.join("pdflatex");

    fs::write(&fake_executable, b"fake binary").expect("write fake binary");

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&fake_executable, fs::Permissions::from_mode(0o755));
    }

    let detected = detect_tex_engine(Some(&temp_dir.to_string_lossy()));
    assert!(detected.is_some());
    let (program_name, _path) = detected.unwrap();
    assert_eq!(program_name, "pdflatex");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_empty_path_does_not_detect_tex_engine() {
    assert!(detect_tex_engine(Some("")).is_none());
}

#[test]
fn test_export_request_validation_for_pdf() {
    let invalid_clipboard_req = ExportRequest {
        format: ExportFormat::Pdf,
        destination: None,
    };
    assert!(invalid_clipboard_req.validate().is_err());

    let invalid_empty_dest_req = ExportRequest {
        format: ExportFormat::Pdf,
        destination: Some("   ".into()),
    };
    assert!(invalid_empty_dest_req.validate().is_err());
}

#[test]
fn test_export_dispatcher_for_pdf() {
    let temp_dir =
        std::env::temp_dir().join(format!("tessera_test_pdf_disp_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let pdf_path = temp_dir.join("disp.pdf");

    let doc = sample_document();
    let request = ExportRequest {
        format: ExportFormat::Pdf,
        destination: Some(pdf_path.to_string_lossy().to_string()),
    };

    let result =
        export_document(&doc, &request, &temp_dir).expect("export dispatcher PDF succeeds");
    if let ExportResult::WrittenToFile { path, pdf_engine } = result {
        assert_eq!(path, pdf_path.to_string_lossy());
        assert!(pdf_engine.is_some());
    } else {
        panic!("expected WrittenToFile result");
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_pdf_module_contains_no_network_primitives() {
    let pdf_source = include_str!("../src/core/export/pdf.rs");
    assert!(!pdf_source.contains("reqwest"));
    assert!(!pdf_source.contains("hyper"));
    assert!(!pdf_source.contains("TcpStream"));
    assert!(!pdf_source.contains("http://"));
    assert!(!pdf_source.contains("https://"));
}
