// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Reader for the WordprocessingML part of a DOCX archive.
//!
//! The reader is deliberately conservative.  A paragraph whose style or
//! markup is not one of the supported Word constructs is retained as a
//! `Docx` opaque block rather than being guessed at or discarded.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use zip::ZipArchive;

use crate::core::error::{Pane, TesseraError};
use crate::core::model::{
    Block, DocMetadata, Document, Equation, Figure, HeadingLevel, Inline, OpaqueOrigin, Table,
    TableCell,
};

const MAX_DOCX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;

/// Result of reading a DOCX, including media bytes which the project storage
/// layer can copy into `.tessera/assets`.
#[derive(Debug)]
pub struct DocxReadResult {
    /// The semantic document tree.
    pub document: Document,
    /// Media keyed by the `Figure::asset_path` value, such as `media/image1.png`.
    pub assets: HashMap<String, Vec<u8>>,
}

/// Reads a DOCX byte stream into the Document Model.
pub fn read_docx(bytes: &[u8]) -> Result<Document, TesseraError> {
    Ok(read_docx_with_assets(bytes)?.document)
}

/// Reads a DOCX and returns both its Document Model and embedded media.
pub fn read_docx_with_assets(bytes: &[u8]) -> Result<DocxReadResult, TesseraError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| parse_error(format!("invalid DOCX zip archive: {e}")))?;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| parse_error(format!("cannot inspect DOCX entry: {e}")))?;
        if entry.size() > MAX_DOCX_ENTRY_BYTES {
            return Err(parse_error(format!(
                "DOCX entry {} exceeds the {} byte limit",
                entry.name(),
                MAX_DOCX_ENTRY_BYTES
            )));
        }
    }
    let document_xml = read_part(&mut archive, "word/document.xml")?;
    let styles_xml = read_optional_part(&mut archive, "word/styles.xml")?;
    let rels_xml = read_optional_part(&mut archive, "word/_rels/document.xml.rels")?;
    let styles = parse_styles(styles_xml.as_deref())?;
    let relationships = parse_relationships(rels_xml.as_deref())?;
    let (body, media_refs) = parse_document(&document_xml, &styles, &relationships)?;
    let mut assets = HashMap::new();
    for path in media_refs {
        if let Ok(data) = read_part(&mut archive, &format!("word/{path}")) {
            assets.insert(path, data);
        }
    }
    let metadata = DocMetadata::new(None, Vec::new(), None, None, Vec::new())?;
    Ok(DocxReadResult {
        document: Document::new(metadata, body),
        assets,
    })
}

fn parse_error(message: String) -> TesseraError {
    TesseraError::ParseError {
        pane: Pane::Docx,
        message,
        span: None,
    }
}

fn read_part<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, TesseraError> {
    let file = archive
        .by_name(name)
        .map_err(|e| parse_error(format!("missing DOCX part {name}: {e}")))?;
    read_limited(file, name)
}

fn read_optional_part<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Option<Vec<u8>>, TesseraError> {
    match archive.by_name(name) {
        Ok(file) => Ok(Some(read_limited(file, name)?)),
        Err(_) => Ok(None),
    }
}

fn read_limited(reader: impl Read, name: &str) -> Result<Vec<u8>, TesseraError> {
    read_limited_to(reader, name, MAX_DOCX_ENTRY_BYTES)
}

fn read_limited_to(reader: impl Read, name: &str, limit: u64) -> Result<Vec<u8>, TesseraError> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| parse_error(format!("cannot read {name}: {e}")))?;
    if bytes.len() as u64 > limit {
        return Err(parse_error(format!(
            "DOCX entry {name} exceeds the {limit} byte limit"
        )));
    }
    Ok(bytes)
}

fn attr<'a>(start: &'a BytesStart<'a>, name: &[u8]) -> Option<String> {
    start
        .attributes()
        .filter_map(Result::ok)
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| String::from_utf8(a.value.into_owned()).ok())
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|b| *b == b':').next().unwrap_or(name)
}

fn parse_styles(xml: Option<&[u8]>) -> Result<HashMap<String, String>, TesseraError> {
    let mut result = HashMap::new();
    let Some(xml) = xml else { return Ok(result) };
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut id = None;
    let mut name = None;
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Start(e) if local_name(e.name().as_ref()) == b"style" => {
                id = attr(&e, b"w:styleId")
            }
            Event::Empty(e) if local_name(e.name().as_ref()) == b"name" => {
                name = attr(&e, b"w:val")
            }
            Event::End(e) if local_name(e.name().as_ref()) == b"style" => {
                if let (Some(id), Some(name)) = (id.take(), name.take()) {
                    result.insert(id, normalize_style_name(&name));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

fn normalize_style_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("heading"))
    {
        let suffix = trimmed.get(7..).unwrap_or_default().trim();
        if matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6") {
            return format!("Heading {suffix}");
        }
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "normal" => "Normal".into(),
        "body text" => "Body Text".into(),
        "caption" => "Caption".into(),
        "quote" => "Quote".into(),
        "intense quote" => "Intense Quote".into(),
        _ => trimmed.to_string(),
    }
}

fn parse_relationships(xml: Option<&[u8]>) -> Result<HashMap<String, String>, TesseraError> {
    let mut result = HashMap::new();
    let Some(xml) = xml else { return Ok(result) };
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Empty(e) | Event::Start(e)
                if local_name(e.name().as_ref()) == b"Relationship" =>
            {
                if let (Some(id), Some(target)) = (attr(&e, b"Id"), attr(&e, b"Target")) {
                    if target.starts_with("media/") {
                        result.insert(id, target);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

fn parse_document(
    xml: &[u8],
    styles: &HashMap<String, String>,
    relationships: &HashMap<String, String>,
) -> Result<(Vec<Block>, Vec<String>), TesseraError> {
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut body = Vec::new();
    let mut media = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Start(e) if local_name(e.name().as_ref()) == b"p" => {
                let (block, refs) = parse_paragraph(&mut reader, e, styles, relationships)?;
                body.push(block);
                media.extend(refs);
            }
            Event::Start(e) if local_name(e.name().as_ref()) == b"tbl" => {
                let (table, refs) = parse_table(&mut reader)?;
                body.push(Block::table(table));
                media.extend(refs);
            }
            Event::Start(e) if local_name(e.name().as_ref()) == b"oMathPara" => {
                let math = collect_math(&mut reader, b"oMathPara")?;
                body.push(Block::equation(Equation::new(math, true, None)?));
            }
            Event::Start(e) if local_name(e.name().as_ref()) == b"sectPr" => {
                skip_element(&mut reader, b"sectPr")?
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok((body, media))
}

fn parse_paragraph(
    reader: &mut Reader<&[u8]>,
    start: BytesStart<'_>,
    styles: &HashMap<String, String>,
    relationships: &HashMap<String, String>,
) -> Result<(Block, Vec<String>), TesseraError> {
    let mut xml_writer = quick_xml::Writer::new(Vec::new());
    xml_writer
        .write_event(Event::Start(start.into_owned()))
        .map_err(|e| parse_error(e.to_string()))?;
    let mut depth = 1usize;
    let mut buf = Vec::new();
    let mut style = None;
    let mut has_unsupported = false;
    let mut text = String::new();
    let mut refs = Vec::new();
    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?;
        let end_paragraph =
            matches!(&event, Event::End(e) if local_name(e.name().as_ref()) == b"p" && depth == 1);
        match &event {
            Event::Start(e) => {
                let qname = e.name();
                let n = local_name(qname.as_ref());
                if n == b"pStyle" {
                    style = attr(e, b"w:val");
                } else if matches!(
                    n,
                    b"ins"
                        | b"del"
                        | b"commentRangeStart"
                        | b"commentRangeEnd"
                        | b"object"
                        | b"fldSimple"
                ) {
                    has_unsupported = true;
                } else if n == b"blip" {
                    if let Some(id) = attr(e, b"r:embed") {
                        if let Some(path) = relationships.get(&id) {
                            refs.push(path.clone());
                        }
                    }
                }
                depth += 1;
            }
            Event::Empty(e) => {
                let qname = e.name();
                let n = local_name(qname.as_ref());
                if n == b"pStyle" {
                    style = attr(e, b"w:val");
                }
                if n == b"tab" {
                    text.push('\t');
                }
                if n == b"br" {
                    text.push('\n');
                }
                if n == b"blip" {
                    if let Some(id) = attr(e, b"r:embed") {
                        if let Some(path) = relationships.get(&id) {
                            refs.push(path.clone());
                        }
                    }
                }
            }
            Event::Text(e) => {
                let value = e.unescape().map_err(|e| parse_error(e.to_string()))?;
                text.push_str(&value);
            }
            Event::End(_) => {
                depth -= 1;
            }
            Event::Eof => return Err(parse_error("unexpected end of document paragraph".into())),
            _ => {}
        }
        xml_writer
            .write_event(event.into_owned())
            .map_err(|e| parse_error(e.to_string()))?;
        if end_paragraph {
            break;
        }
        buf.clear();
    }
    let style_name = match style {
        Some(id) => styles
            .get(&id)
            .cloned()
            .or_else(|| built_in_style(&id))
            .unwrap_or_else(|| format!("Unknown style {id}")),
        None => "Normal".into(),
    };
    if has_unsupported || !is_supported_style(&style_name) {
        let raw = String::from_utf8_lossy(&xml_writer.into_inner()).into_owned();
        return Ok((
            Block::opaque(crate::core::model::OpaqueBlock::new(
                OpaqueOrigin::Docx,
                raw,
                format!("Unsupported Word paragraph style or markup: {style_name}"),
            )?),
            refs,
        ));
    }
    if !refs.is_empty() {
        let path = refs[0].clone();
        let figure = Figure::new(
            path,
            if text.trim().is_empty() {
                None
            } else {
                Some(vec![Inline::text(text)?])
            },
            None,
            None,
        )?;
        return Ok((Block::figure(figure), refs));
    }
    let inline = if xml_writer.get_ref().windows(5).any(|w| w == b"oMath") {
        if text.is_empty() {
            Vec::new()
        } else {
            vec![Inline::inline_math(text)?]
        }
    } else if text.is_empty() {
        Vec::new()
    } else {
        vec![Inline::text(text)?]
    };
    let block = match style_name.as_str() {
        "Heading 1" => Block::heading(HeadingLevel::H1, inline, None)?,
        "Heading 2" => Block::heading(HeadingLevel::H2, inline, None)?,
        "Heading 3" => Block::heading(HeadingLevel::H3, inline, None)?,
        "Heading 4" => Block::heading(HeadingLevel::H4, inline, None)?,
        "Heading 5" => Block::heading(HeadingLevel::H5, inline, None)?,
        "Heading 6" => Block::heading(HeadingLevel::H6, inline, None)?,
        "Quote" | "Intense Quote" => Block::quote(vec![Block::paragraph(inline)]),
        "Normal" | "Body Text" | "Caption" => Block::paragraph(inline),
        _ => Block::paragraph(inline),
    };
    Ok((block, refs))
}

fn built_in_style(style_id: &str) -> Option<String> {
    match style_id {
        "Normal" => Some("Normal".into()),
        "Caption" => Some("Caption".into()),
        "Quote" => Some("Quote".into()),
        "Heading1" => Some("Heading 1".into()),
        "Heading2" => Some("Heading 2".into()),
        "Heading3" => Some("Heading 3".into()),
        "Heading4" => Some("Heading 4".into()),
        "Heading5" => Some("Heading 5".into()),
        "Heading6" => Some("Heading 6".into()),
        _ => None,
    }
}

fn is_supported_style(style: &str) -> bool {
    matches!(
        style,
        "Normal"
            | "Body Text"
            | "Caption"
            | "Quote"
            | "Intense Quote"
            | "Heading 1"
            | "Heading 2"
            | "Heading 3"
            | "Heading 4"
            | "Heading 5"
            | "Heading 6"
    )
}

fn parse_table(reader: &mut Reader<&[u8]>) -> Result<(Table, Vec<String>), TesseraError> {
    let mut rows: Vec<Vec<TableCell>> = Vec::new();
    let mut active_vertical: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut media = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Start(e) if local_name(e.name().as_ref()) == b"tr" => {
                let (parsed_row, refs) = parse_row(reader)?;
                let row_index = rows.len();
                let mut row = Vec::new();
                let mut next_vertical = HashMap::new();
                let mut updated_cells = std::collections::HashSet::new();
                let mut column = 0usize;
                for (cell, merge) in parsed_row {
                    if merge == VerticalMerge::Continue {
                        let span = cell.col_span() as usize;
                        for offset in 0..span {
                            let current_column = column + offset;
                            if let Some(&(old_row, old_cell)) = active_vertical.get(&current_column)
                            {
                                if updated_cells.insert((old_row, old_cell)) {
                                    let previous = &rows[old_row][old_cell];
                                    rows[old_row][old_cell] = TableCell::new(
                                        previous.blocks().to_vec(),
                                        previous.col_span(),
                                        previous.row_span().saturating_add(1),
                                    )?;
                                }
                                next_vertical.insert(current_column, (old_row, old_cell));
                            }
                        }
                        column += span;
                    } else {
                        let span = cell.col_span() as usize;
                        if merge == VerticalMerge::Restart {
                            for offset in 0..span {
                                next_vertical.insert(column + offset, (row_index, row.len()));
                            }
                        }
                        column += span;
                        row.push(cell);
                    }
                }
                rows.push(row);
                active_vertical = next_vertical;
                media.extend(refs);
            }
            Event::End(e) if local_name(e.name().as_ref()) == b"tbl" => break,
            Event::Eof => return Err(parse_error("unexpected end of table".into())),
            _ => {}
        }
        buf.clear();
    }
    Ok((Table::new(rows, None, None)?, media))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VerticalMerge {
    None,
    Restart,
    Continue,
}

type ParsedCell = (TableCell, VerticalMerge);

fn parse_row(reader: &mut Reader<&[u8]>) -> Result<(Vec<ParsedCell>, Vec<String>), TesseraError> {
    let mut cells = Vec::new();
    let mut media = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Start(e) if local_name(e.name().as_ref()) == b"tc" => {
                let (cell, merge, refs) = parse_cell(reader)?;
                cells.push((cell, merge));
                media.extend(refs);
            }
            Event::End(e) if local_name(e.name().as_ref()) == b"tr" => break,
            Event::Eof => return Err(parse_error("unexpected end of table row".into())),
            _ => {}
        }
        buf.clear();
    }
    Ok((cells, media))
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
) -> Result<(TableCell, VerticalMerge, Vec<String>), TesseraError> {
    let mut text = String::new();
    let mut col_span = 1;
    let row_span = 1;
    let mut merge = VerticalMerge::None;
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Start(e) => {
                let qname = e.name();
                let n = local_name(qname.as_ref());
                if n == b"gridSpan" {
                    col_span = attr(&e, b"w:val").and_then(|v| v.parse().ok()).unwrap_or(1);
                }
                if n == b"vMerge" {
                    merge = if attr(&e, b"w:val").as_deref() == Some("restart") {
                        VerticalMerge::Restart
                    } else {
                        VerticalMerge::Continue
                    };
                }
            }
            Event::Empty(e) => {
                let qname = e.name();
                let n = local_name(qname.as_ref());
                if n == b"gridSpan" {
                    col_span = attr(&e, b"w:val").and_then(|v| v.parse().ok()).unwrap_or(1);
                }
                if n == b"vMerge" {
                    merge = if attr(&e, b"w:val").as_deref() == Some("restart") {
                        VerticalMerge::Restart
                    } else {
                        VerticalMerge::Continue
                    };
                }
            }
            Event::Text(e) => text.push_str(&e.unescape().map_err(|x| parse_error(x.to_string()))?),
            Event::End(e) if local_name(e.name().as_ref()) == b"tc" => break,
            Event::Eof => return Err(parse_error("unexpected end of table cell".into())),
            _ => {}
        }
        buf.clear();
    }
    let blocks = if text.trim().is_empty() {
        Vec::new()
    } else {
        vec![Block::paragraph(vec![Inline::text(text)?])]
    };
    Ok((
        TableCell::new(blocks, col_span, row_span)?,
        merge,
        Vec::new(),
    ))
}

fn collect_math(reader: &mut Reader<&[u8]>, end: &[u8]) -> Result<String, TesseraError> {
    let mut result = String::new();
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Text(e) => {
                result.push_str(&e.unescape().map_err(|x| parse_error(x.to_string()))?)
            }
            Event::End(e) if local_name(e.name().as_ref()) == end => break,
            Event::Eof => return Err(parse_error("unexpected end of OMML".into())),
            _ => {}
        }
        buf.clear();
    }
    if result.trim().is_empty() {
        Err(parse_error("empty OMML equation".into()))
    } else {
        Ok(result)
    }
}

fn skip_element(reader: &mut Reader<&[u8]>, end: &[u8]) -> Result<(), TesseraError> {
    let mut depth = 1;
    let mut buf = Vec::new();
    while depth > 0 {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| parse_error(e.to_string()))?
        {
            Event::Start(_) => depth += 1,
            Event::End(e) if local_name(e.name().as_ref()) == end => depth -= 1,
            Event::Eof => return Err(parse_error("unexpected end of XML".into())),
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_handcrafted_reader_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fixtures/docx/minimal-reader.docx"
        );
        let bytes = std::fs::read(path).expect("reader fixture");
        let result = read_docx_with_assets(&bytes).expect("valid fixture");
        assert!(!result.document.body().is_empty());
    }

    #[test]
    fn opaque_paragraph_preserves_outer_xml_tags() {
        let xml = br#"<w:document xmlns:w="urn:test"><w:body><w:p w:rsidR="1"><w:pPr><w:pStyle w:val="CustomStyle"/></w:pPr><w:r><w:t>keep</w:t></w:r></w:p></w:body></w:document>"#;
        let (blocks, _) = parse_document(xml, &HashMap::new(), &HashMap::new()).expect("parse");
        let Block::Opaque(value) = &blocks[0] else {
            panic!("opaque block")
        };
        assert!(value.raw().starts_with("<w:p w:rsidR=\"1\">"));
        assert!(value.raw().ends_with("</w:p>"));
    }

    #[test]
    fn vertical_merge_continuations_advance_the_grid_column() {
        let xml = br#"<w:tbl><w:tr><w:tc><w:tcPr><w:vMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc><w:tc><w:p><w:r><w:t>C</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#;
        let mut reader = Reader::from_reader(xml.as_slice());
        let mut buffer = Vec::new();
        let _ = reader.read_event_into(&mut buffer).expect("table start");
        let (table, _) = parse_table(&mut reader).expect("table");
        assert_eq!(table.rows()[0][0].row_span(), 2);
        assert_eq!(table.rows()[1][0].blocks().len(), 1);
        assert_eq!(
            table.rows()[1][0].blocks()[0],
            Block::paragraph(vec![Inline::text("C".into()).expect("text")])
        );
    }

    #[test]
    fn archive_entry_reads_stop_at_the_configured_limit() {
        assert!(read_limited_to(Cursor::new(b"1234"), "test", 3).is_err());
        assert_eq!(
            read_limited_to(Cursor::new(b"123"), "test", 3).expect("bounded"),
            b"123"
        );
    }

    #[test]
    fn non_ascii_heading_style_names_do_not_panic() {
        assert_eq!(normalize_style_name("標題"), "標題");
    }
}
