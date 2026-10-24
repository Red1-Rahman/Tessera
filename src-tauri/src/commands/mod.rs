// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! The IPC surface: thin Tauri command handlers that validate input,
//! call into `core/`, and map results to `Result<T, TesseraError>` (see
//! `docs/contracts.md`, section 2). Handlers stay free of business logic;
//! anything more than a few lines belongs in `core/`.
