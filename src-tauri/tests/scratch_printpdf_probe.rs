// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Temporary probe for printpdf APIs. Deleted before commit.

use std::collections::BTreeMap;

use printpdf::*;

#[test]
fn probe_ops_path() {
    let mut doc = PdfDocument::new("probe");
    let ops = vec![
        Op::SaveGraphicsState,
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point::new(Mm(20.0), Mm(270.0)),
        },
        Op::SetFont {
            font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
            size: Pt(12.0),
        },
        Op::SetLineHeight { lh: Pt(14.0) },
        Op::ShowText {
            items: vec![TextItem::Text("Hello ops".to_string())],
        },
        Op::EndTextSection,
        Op::RestoreGraphicsState,
    ];
    let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
    let bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut Vec::new());
    assert!(bytes.starts_with(b"%PDF"), "not a pdf: {:?}", &bytes[..8]);
}

#[test]
fn probe_html_path() {
    let html = r#"
    <html><head><style>
      h1 { font-size: 24px; }
      p { font-size: 12px; }
    </style></head><body>
      <h1>Title</h1>
      <p>A paragraph of text that should wrap somewhere along the line.</p>
      <ul><li>one</li><li>two</li></ul>
    </body></html>"#;
    let images = BTreeMap::new();
    let fonts = BTreeMap::new();
    let options = GeneratePdfOptions::default();
    let mut warnings = Vec::new();
    let doc = PdfDocument::from_html(html, &images, &fonts, &options, &mut warnings)
        .unwrap_or_else(|error| panic!("from_html failed: {error:?}"));

    // from_html returns Ok with an empty document when rendering fails,
    // reporting the cause only through `warnings`.
    assert!(
        doc.page_count() > 0,
        "HTML rendering produced no pages; warnings: {warnings:#?}"
    );
    let bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());
    assert!(bytes.starts_with(b"%PDF"));
}

/// A realistic Tessera-style document: headings, nested lists, a table,
/// a code block, a quote, unicode, and enough prose to paginate.
#[test]
fn probe_html_realistic() {
    let body: String = (1..=25)
        .map(|i| {
            format!(
                "<p>Paragraph {i}. Tessera keeps a LaTeX document and a DOCX \
                 document in sync, with unicode content such as caf&eacute; \
                 and &Omega; plus math-like text x^2 + y^2 = z^2.</p>"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"/><style>
  body {{ margin: 0; font-size: 12px; }}
  h1 {{ font-size: 24px; margin: 0 0 8px 0; }}
  h2 {{ font-size: 18px; margin: 12px 0 6px 0; }}
  p {{ margin: 0 0 8px 0; }}
  ul, ol {{ margin: 0 0 8px 0; }}
  pre {{ font-family: monospace; background: #f4f4f4; padding: 6px; margin: 0 0 8px 0; }}
  blockquote {{ margin: 0 0 8px 0; padding-left: 12px; border-left: 2px solid #cccccc; }}
  table {{ border-collapse: collapse; margin: 0 0 8px 0; }}
  td, th {{ border: 1px solid #999999; padding: 4px 8px; }}
</style></head><body>
  <h1>Chapter One</h1>
  <p>Intro paragraph that introduces the chapter and wraps across lines.</p>
  <h2>Section A</h2>
  <ul><li>First bullet</li><li>Second bullet with a longer piece of text that should wrap</li></ul>
  <ol><li>Numbered one</li><li>Numbered two</li></ol>
  <table>
    <tr><th>Format</th><th>Mechanism</th></tr>
    <tr><td>PDF</td><td>System TeX engine if present, else built-in typesetter</td></tr>
    <tr><td>Markdown</td><td>Rendered from the Document Model</td></tr>
  </table>
  <pre><code>\documentclass{{article}}
\begin{{document}}
Hello
\end{{document}}</code></pre>
  <blockquote>A quoted passage that should be indented.</blockquote>
  <pre>Inline math rendered as raw source: $E = mc^2$</pre>
  {body}
</body></html>"#
    );

    let images = BTreeMap::new();
    let fonts = BTreeMap::new();
    let mut options = GeneratePdfOptions::default();
    options.margin_top = Some(20.0);
    options.margin_bottom = Some(20.0);
    options.margin_left = Some(20.0);
    options.margin_right = Some(20.0);
    let mut warnings = Vec::new();
    let doc = PdfDocument::from_html(&html, &images, &fonts, &options, &mut warnings)
        .unwrap_or_else(|error| panic!("from_html failed: {error:?}"));

    let pages = doc.page_count();
    let error_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| format!("{w:?}").contains("Error"))
        .collect();
    assert!(
        pages > 0 && error_warnings.is_empty(),
        "pages={pages} error_warnings={error_warnings:#?} all={warnings:#?}"
    );
    assert!(pages >= 2, "expected pagination, got {pages} pages");
    let uncompressed = PdfSaveOptions {
        optimize: false,
        ..PdfSaveOptions::default()
    };
    let bytes = doc.save(&uncompressed, &mut Vec::new());
    assert!(bytes.starts_with(b"%PDF"));
    let text = String::from_utf8_lossy(&bytes);
    let has_show_op = text.contains("Tj") || text.contains("TJ");
    println!(
        "pages={pages} bytes={} show_ops={has_show_op} to_unicode={}",
        bytes.len(),
        text.contains("ToUnicode")
    );
    assert!(has_show_op, "no text-showing operators in content streams");
}

/// Empty font pool: does the HTML path survive when system font scanning
/// returns nothing usable?
#[test]
fn probe_html_empty_font_cache() {
    let html = "<html><head><style>body{font-size:12px}</style></head><body><p>Hello</p></body></html>";
    let images = BTreeMap::new();
    let fonts = BTreeMap::new();
    let options = GeneratePdfOptions::default();
    let mut warnings = Vec::new();

    let empty_cache = rust_fontconfig_empty_cache();
    let pool = SharedFontPool {
        fc_cache: std::sync::Arc::new(empty_cache),
        parsed_fonts: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
    };
    let doc =
        PdfDocument::from_html_with_cache(html, &images, &fonts, &options, &mut warnings, Some(pool))
            .unwrap_or_else(|error| panic!("from_html failed: {error:?}"));
    println!(
        "empty-font-cache pages={} warnings={:#?}",
        doc.page_count(),
        warnings
    );
}

/// Does `<img src="...">` resolve against the supplied `images` map and
/// land as a PDF Image XObject, without an explicit CSS height?
#[test]
fn probe_html_image_embedding() {
    let logo = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/icons/32x32.png"
    ))
    .expect("fixture png");

    let mut images = BTreeMap::new();
    images.insert("logo.png".to_string(), Base64OrRaw::Raw(logo));
    let fonts = BTreeMap::new();

    let html = r#"<!DOCTYPE html>
<html><head><style>
  body { margin: 0; font-size: 12px; }
  img { width: 64px; }
</style></head><body>
  <p>Figure below.</p>
  <img src="logo.png" alt="logo"/>
  <p>Caption text.</p>
</body></html>"#;

    let mut options = GeneratePdfOptions::default();
    options.margin_top = Some(20.0);
    options.margin_bottom = Some(20.0);
    options.margin_left = Some(20.0);
    options.margin_right = Some(20.0);

    let mut warnings = Vec::new();
    let doc = PdfDocument::from_html(html, &images, &fonts, &options, &mut warnings)
        .unwrap_or_else(|error| panic!("from_html failed: {error:?}"));
    assert!(
        doc.page_count() > 0,
        "no pages; warnings: {warnings:#?}"
    );

    let uncompressed = PdfSaveOptions {
        optimize: false,
        ..PdfSaveOptions::default()
    };
    let bytes = doc.save(&uncompressed, &mut Vec::new());
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("/Subtype /Image") || text.contains("/Subtype/Image"),
        "image was not embedded; warnings: {warnings:#?}"
    );
    println!(
        "image embedded, bytes={} warnings={:#?}",
        bytes.len(),
        warnings
    );
}

fn rust_fontconfig_empty_cache() -> printpdf::rust_fontconfig::FcFontCache {
    let pool = build_font_pool(&BTreeMap::new(), Some(&[]));
    (*pool.fc_cache).clone()
}
