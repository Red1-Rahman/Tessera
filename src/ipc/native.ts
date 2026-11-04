// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

// Frontend-side native capabilities that are not registered Tauri
// commands: the native save and open dialogs and the operating system clipboard.
// They live here so that no component imports a Tauri plugin directly,
// matching the boundary rule for `src/ipc/commands.ts` in
// docs/conventions.md. Both plugins are local-only; neither grants any
// network capability.

export interface FileDialogFilter {
  name: string;
  extensions: string[];
}

export interface SaveFileDialogOptions {
  filters: FileDialogFilter[];
}

/**
 * Opens the platform save dialog. Resolves with the chosen path, or
 * null when the user cancelled the dialog.
 */
export function saveFileDialog(
  options: SaveFileDialogOptions,
): Promise<string | null> {
  return saveDialog(options);
}

/**
 * Writes plain text to the operating system clipboard. The backend
 * never touches the clipboard itself (see docs/api.md, "Export and
 * clipboard handoff"); this is the only clipboard write path.
 */
export function writeClipboardText(text: string): Promise<void> {
  return writeText(text);
}

export interface OpenFileDialogOptions {
  filters: FileDialogFilter[];
}

/**
 * Opens the platform directory picker. Resolves with the chosen
 * directory, or null when the user cancelled the dialog.
 */
export async function openDirectoryDialog(): Promise<string | null> {
  const selected = await openDialog({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

/**
 * Opens the platform file picker for a single file. Resolves with the
 * chosen path, or null when the user cancelled the dialog.
 */
export async function openFileDialog(
  options: OpenFileDialogOptions,
): Promise<string | null> {
  const selected = await openDialog({
    directory: false,
    multiple: false,
    filters: options.filters,
  });
  return typeof selected === "string" ? selected : null;
}