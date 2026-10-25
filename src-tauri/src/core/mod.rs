// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Framework-agnostic core: the Document Model and, over the course of
//! the build plan, the LaTeX parser/printer, DOCX reader/writer, export
//! renderers, and diff engine. Nothing in this module may depend on
//! Tauri, a webview, or the network.

/// DOCX (OOXML) reader and writer (`docs/specs.md`, section 2).
pub mod docx;
/// Tessera's error surface (`TesseraError` and friends), as shaped in
/// `docs/contracts.md`, section 2. Defined here rather than in
/// `commands/` so `core/` can return it without depending on the IPC
/// layer; `commands/` re-exports it.
pub mod error;
/// LaTeX parser and printer (`docs/specs.md`, section 1).
pub mod latex;
/// The Document Model types (`docs/contracts.md`, section 1).
pub mod model;
