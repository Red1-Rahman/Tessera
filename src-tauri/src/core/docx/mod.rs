// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! DOCX (OOXML) side of the conversion engine: `reader.rs` (zip archive
//! to Document Model) and `writer.rs` (Document Model to zip archive),
//! per `docs/specs.md`, section 2.

/// DOCX archive to Document Model (`parallel-task03-C`).
pub mod reader;
/// Document Model to DOCX archive (`parallel-task03-D`).
pub mod writer;
