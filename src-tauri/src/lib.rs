// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Tessera's Rust library crate.
//!
//! The binary (`main.rs`) only calls [`run`]; everything else lives in
//! [`core`] (framework-agnostic document model and conversion logic) and
//! `commands` (the thin Tauri IPC surface). Keeping the logic in a lib
//! crate means `src-tauri/tests/` can exercise it without spinning up
//! Tauri or a webview.

/// Thin Tauri command handlers (IPC marshaling only, no business logic).
pub mod commands;
/// The Document Model and all conversion engines.
pub mod core;

/// Builds and runs the Tauri application.
///
/// Panics only in the way the Tauri bootstrap template does: a failure
/// here means the app cannot start at all, before any Tessera code runs.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the Tessera application");
}
