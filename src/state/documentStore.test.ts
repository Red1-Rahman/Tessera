// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { describe, it, expect, beforeEach, vi } from "vitest";
import * as store from "./documentStore";
import type { DocSyncResult, ProjectState, Document } from "../ipc/commands";

const makeDoc = (version: number): Document => ({
  version,
  metadata: {
    title: null,
    authors: [],
    date: null,
    latex_class: null,
    latex_class_options: [],
    preamble_raw: [],
  },
  body: [],
  trailing_trivia: "",
});

const makeProject = (version: number): ProjectState => ({
  path: "/test",
  document: makeDoc(version),
  last_edited_pane: "Latex",
  assets: [],
  warnings: [],
});

describe("Document Store", () => {
  beforeEach(() => {
    store.loadProject(makeProject(1));
  });

  it("exports debounce constants with correct values", () => {
    expect(store.LATEX_REPARSE_DEBOUNCE_MS).toBe(250);
    expect(store.DOCX_SYNC_DEBOUNCE_MS).toBe(250);
    expect(store.MAX_UNDO_DEPTH).toBe(100);
  });

  it("notifies listeners on state changes", () => {
    const listener = vi.fn();
    const unsub = store.subscribe(listener);
    
    store.applySyncResult({
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    });
    
    expect(listener).toHaveBeenCalledTimes(1);
    unsub();
    
    store.applySyncResult({
      new_version: 3,
      document: makeDoc(3),
      latex_render: "",
      warnings: [],
    });
    expect(listener).toHaveBeenCalledTimes(1); // No longer notified
  });

  it("discards stale sync results (version <= current)", () => {
    const staleResult: DocSyncResult = {
      new_version: 1,
      document: makeDoc(1),
      latex_render: "",
      warnings: [],
    };
    
    const applied = store.applySyncResult(staleResult);
    expect(applied).toBe(false);
    expect(store.getSnapshot().version).toBe(1);
  });

  it("applies newer sync results and updates version", () => {
    const newResult: DocSyncResult = {
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    };
    
    const applied = store.applySyncResult(newResult);
    expect(applied).toBe(true);
    expect(store.getSnapshot().version).toBe(2);
    expect(store.getSnapshot().document?.version).toBe(2);
  });

  it("handles out-of-order arrivals correctly", () => {
    store.applySyncResult({
      new_version: 3,
      document: makeDoc(3),
      latex_render: "",
      warnings: [],
    });
    
    const applied = store.applySyncResult({
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    });
    
    expect(applied).toBe(false);
    expect(store.getSnapshot().version).toBe(3);
  });

  it("manages undo and redo stacks", () => {
    store.applySyncResult({
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    });
    store.applySyncResult({
      new_version: 3,
      document: makeDoc(3),
      latex_render: "",
      warnings: [],
    });
    
    expect(store.canUndo()).toBe(true);
    
    const undone = store.undo();
    expect(undone?.version).toBe(2);
    expect(store.getSnapshot().version).toBe(3);
    expect(store.canRedo()).toBe(true);
    
    const redone = store.redo();
    expect(redone?.version).toBe(3);
    expect(store.getSnapshot().version).toBe(3);
  });

  it("applies restore sync results without changing history", () => {
    store.applySyncResult({
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    });
    store.applySyncResult({
      new_version: 3,
      document: makeDoc(3),
      latex_render: "",
      warnings: [],
    });
    store.undo();

    const applied = store.applySyncResult(
      {
        new_version: 4,
        document: makeDoc(4),
        latex_render: "",
        warnings: [],
      },
      { restore: true }
    );

    expect(applied).toBe(true);
    expect(store.getSnapshot().version).toBe(4);
    expect(store.getSnapshot().document?.version).toBe(4);
    expect(store.getSnapshot().undoStack.map((doc) => doc.version)).toEqual([1]);
    expect(store.getSnapshot().redoStack.map((doc) => doc.version)).toEqual([3]);
  });

  it("discards stale sync results after undo", () => {
    store.applySyncResult({
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    });
    store.applySyncResult({
      new_version: 3,
      document: makeDoc(3),
      latex_render: "",
      warnings: [],
    });
    store.undo();

    const applied = store.applySyncResult({
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    });

    expect(applied).toBe(false);
    expect(store.getSnapshot().version).toBe(3);
    expect(store.getSnapshot().document?.version).toBe(2);
    expect(store.getSnapshot().undoStack.map((doc) => doc.version)).toEqual([1]);
    expect(store.getSnapshot().redoStack.map((doc) => doc.version)).toEqual([3]);
  });

  it("limits undo history depth", () => {
    for (let version = 2; version <= store.MAX_UNDO_DEPTH + 2; version += 1) {
      store.applySyncResult({
        new_version: version,
        document: makeDoc(version),
        latex_render: "",
        warnings: [],
      });
    }

    const snapshot = store.getSnapshot();
    expect(snapshot.undoStack).toHaveLength(store.MAX_UNDO_DEPTH);
    expect(snapshot.undoStack[0].version).toBe(2);
  });

  it("clears redo stack on new edit", () => {
    store.applySyncResult({
      new_version: 2,
      document: makeDoc(2),
      latex_render: "",
      warnings: [],
    });
    store.undo();
    expect(store.canRedo()).toBe(true);
    
    store.applySyncResult({
      new_version: 3,
      document: makeDoc(3),
      latex_render: "",
      warnings: [],
    });
    expect(store.canRedo()).toBe(false);
  });

  it("returns null for undo/redo when stacks are empty", () => {
    expect(store.undo()).toBeNull();
    expect(store.redo()).toBeNull();
    expect(store.canUndo()).toBe(false);
    expect(store.canRedo()).toBe(false);
  });
});