// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! The IPC surface. Handlers validate input, call into `core/`, and marshal
//! results to the frontend.

pub use crate::core::error::{ExportFormat, Pane, SourceSpan, TesseraError};

pub mod document;
pub mod export;
