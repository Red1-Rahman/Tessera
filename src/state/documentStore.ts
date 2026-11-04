// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import type {
  Document,
  DocVersion,
  DocSyncResult,
  ProjectState,
  SyncWarning,
} from "../ipc/commands";

// Debounce constants defined exactly once per docs/conventions.md.
export const LATEX_REPARSE_DEBOUNCE_MS = 250;
export const DOCX_SYNC_DEBOUNCE_MS = 250;
export const MAX_UNDO_DEPTH = 100;

export interface ApplySyncResultOptions {
  restore?: boolean;
}

export interface DocumentStoreState {
  document: Document | null;
  version: DocVersion;
  undoStack: Document[];
  redoStack: Document[];
  warnings: SyncWarning[];
  projectLoadId: number;
}

let projectLoadSequence = 0;

let currentState: DocumentStoreState = {
  document: null,
  version: 0,
  undoStack: [],
  redoStack: [],
  warnings: [],
  projectLoadId: 0,
};

const listeners = new Set<() => void>();

function notifyListeners(): void {
  listeners.forEach((listener) => listener());
}

/**
 * Returns the current store snapshot. Used with React's useSyncExternalStore.
 */
export function getSnapshot(): DocumentStoreState {
  return currentState;
}

/**
 * Subscribes to store changes. Returns an unsubscribe function.
 */
export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Replaces the store state with a newly opened or created project.
 * Clears the undo/redo history.
 */
export function loadProject(project: ProjectState): void {
  currentState = {
    document: project.document,
    version: project.document.version,
    undoStack: [],
    redoStack: [],
    warnings: project.warnings,
    projectLoadId: ++projectLoadSequence,
  };
  notifyListeners();
}

/**
 * Applies a sync result from the backend.
 * Discards the result if its version is not strictly newer than the current
 * version, handling out-of-order async responses safely. Restore results
 * update the document without changing either history stack.
 * Returns true if the state was updated, false if discarded.
 */
export function applySyncResult(
  result: DocSyncResult,
  options: ApplySyncResultOptions = {}
): boolean {
  if (result.new_version <= currentState.version) {
    return false;
  }

  if (options.restore) {
    currentState = {
      ...currentState,
      document: result.document,
      version: result.new_version,
      warnings: result.warnings,
    };
    notifyListeners();
    return true;
  }

  const nextUndoStack = currentState.document
    ? [...currentState.undoStack, currentState.document]
    : currentState.undoStack;
  const boundedUndoStack =
    nextUndoStack.length > MAX_UNDO_DEPTH
      ? nextUndoStack.slice(-MAX_UNDO_DEPTH)
      : nextUndoStack;

  currentState = {
    document: result.document,
    version: result.new_version,
    undoStack: boundedUndoStack,
    redoStack: [], // Clear redo stack on new edit
    warnings: result.warnings,
    projectLoadId: currentState.projectLoadId,
  };
  notifyListeners();
  return true;
}

/**
 * Reverts to the previous document in the undo stack.
 * Returns the restored Document, or null if nothing to undo.
 * The caller (editor component) is responsible for pushing this restored
 * state to the backend via parse_latex or apply_docx_edit to sync the other pane.
 */
export function undo(): Document | null {
  if (currentState.undoStack.length === 0 || !currentState.document) {
    return null;
  }

  const previous = currentState.undoStack[currentState.undoStack.length - 1];
  currentState = {
    ...currentState,
    document: previous,
    version: currentState.version,
    undoStack: currentState.undoStack.slice(0, -1),
    redoStack: [...currentState.redoStack, currentState.document],
  };
  notifyListeners();
  return previous;
}

/**
 * Advances to the next document in the redo stack.
 * Returns the restored Document, or null if nothing to redo.
 */
export function redo(): Document | null {
  if (currentState.redoStack.length === 0) {
    return null;
  }

  const next = currentState.redoStack[currentState.redoStack.length - 1];
  const nextUndoStack = currentState.document
    ? [...currentState.undoStack, currentState.document]
    : currentState.undoStack;

  currentState = {
    ...currentState,
    document: next,
    version: currentState.version,
    undoStack: nextUndoStack,
    redoStack: currentState.redoStack.slice(0, -1),
  };
  notifyListeners();
  return next;
}

export function canUndo(): boolean {
  return currentState.undoStack.length > 0;
}

export function canRedo(): boolean {
  return currentState.redoStack.length > 0;
}
