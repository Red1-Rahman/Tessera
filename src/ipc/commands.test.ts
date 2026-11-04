// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { describe, it, expect, vi, beforeEach } from "vitest";
import * as commands from "./commands";
import type {
  ProjectState,
  DocSyncResult,
  ExportResult,
  HighlightToken,
} from "./commands";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

const mockProjectState: ProjectState = {
  path: "/test/project",
  document: {
    version: 1,
    metadata: {
      title: "Test",
      authors: [],
      date: null,
      latex_class: "article",
      latex_class_options: [],
      preamble_raw: [],
    },
    body: [],
    trailing_trivia: "",
  },
  last_edited_pane: "Latex",
  assets: [],
  warnings: [],
};

describe("IPC Commands", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("openProject invokes open_project with path", async () => {
    mockInvoke.mockResolvedValue(mockProjectState);
    const result = await commands.openProject("/test/project");
    expect(mockInvoke).toHaveBeenCalledWith("open_project", {
      path: "/test/project",
    });
    expect(result).toEqual(mockProjectState);
  });

  it("newProject invokes new_project with request", async () => {
    mockInvoke.mockResolvedValue(mockProjectState);
    const req = { path: "/new", title: "New Doc" };
    const result = await commands.newProject(req);
    expect(mockInvoke).toHaveBeenCalledWith("new_project", { request: req });
    expect(result).toEqual(mockProjectState);
  });

  it("parseLatex invokes parse_latex with request", async () => {
    const syncResult: DocSyncResult = {
      new_version: 2,
      document: mockProjectState.document,
      latex_render: "\\section{Hello}",
      warnings: [],
    };
    mockInvoke.mockResolvedValue(syncResult);
    const req = { base_version: 1, full_source: "\\section{Hello}" };
    const result = await commands.parseLatex(req);
    expect(mockInvoke).toHaveBeenCalledWith("parse_latex", { request: req });
    expect(result).toEqual(syncResult);
  });

  it("applyDocxEdit invokes apply_docx_edit with request", async () => {
    const syncResult: DocSyncResult = {
      new_version: 2,
      document: mockProjectState.document,
      latex_render: "",
      warnings: [],
    };
    mockInvoke.mockResolvedValue(syncResult);
    const req = { base_version: 1, patches: [] };
    const result = await commands.applyDocxEdit(req);
    expect(mockInvoke).toHaveBeenCalledWith("apply_docx_edit", { request: req });
    expect(result).toEqual(syncResult);
  });

  it("saveProject invokes save_project with no args", async () => {
    mockInvoke.mockResolvedValue(null);
    await commands.saveProject();
    expect(mockInvoke).toHaveBeenCalledWith("save_project");
  });

  it("exportDocument invokes export with request", async () => {
    const exportResult: ExportResult = {
      kind: "WrittenToFile",
      path: "/out.pdf",
    };
    mockInvoke.mockResolvedValue(exportResult);
    const req = { format: "Pdf" as const, destination: "/out.pdf" };
    const result = await commands.exportDocument(req);
    expect(mockInvoke).toHaveBeenCalledWith("export", { request: req });
    expect(result).toEqual(exportResult);
  });

  it("getHighlightTokens invokes get_highlight_tokens with request", async () => {
    const tokens: HighlightToken[] = [
      { start: 0, end: 5, kind: "Command" },
    ];
    mockInvoke.mockResolvedValue(tokens);
    const req = { source: "\\textbf" };
    const result = await commands.getHighlightTokens(req);
    expect(mockInvoke).toHaveBeenCalledWith("get_highlight_tokens", {
      request: req,
    });
    expect(result).toEqual(tokens);
  });

  it("propagates invoke rejections as TesseraError objects", async () => {
    const err = {
      kind: "ValidationError",
      field: "path",
      message: "missing",
    };
    mockInvoke.mockRejectedValue(err);
    await expect(commands.openProject("")).rejects.toEqual(err);
  });
});