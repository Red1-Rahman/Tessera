// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Export renderers for formats other than LaTeX and DOCX (see
//! `docs/specs.md`, section 4).

/// Markdown export (`parallel-task06-C`).
pub mod markdown;
/// Plain text export (`parallel-task06-D`).
pub mod plaintext;
