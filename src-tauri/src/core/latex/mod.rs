// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! LaTeX side of the conversion engine: `parser.rs` (source to Document
//! Model) and `printer.rs` (Document Model back to source), per
//! `docs/specs.md`, section 1.

/// LaTeX source to Document Model (`parallel-task03-A`).
pub mod parser;
/// Document Model to LaTeX source (`parallel-task03-B`).
pub mod printer;
