// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Binary entry point. All real setup lives in `tessera_lib::run` so the
/// application logic is reachable from integration tests without a
/// webview.
fn main() {
    tessera_lib::run();
}
