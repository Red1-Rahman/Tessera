// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { DOMParser as ProseMirrorDOMParser } from "prosemirror-model";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Block, Document, SyncWarning } from "../ipc/commands";
import { blockToNode, DocxEditor, diffBlocks, docxSchema, nodeToBlock, type DocxEditorHandle } from "./DocxEditor";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const makeDocument = (body: Block[]): Document => ({
  version: 1,
  metadata: { title: null, authors: [], date: null, latex_class: null, latex_class_options: [], preamble_raw: [] },
  body,
  trailing_trivia: "",
});

const paragraph = (text: string): Block => ({ type: "Paragraph", inline: [{ type: "Text", text }], trivia: { leading: "" } });

const renderEditor = (options: { body?: Block[]; warnings?: SyncWarning[]; flashIndices?: number[] } = {}) => render(
  <DocxEditor
    document={makeDocument(options.body ?? [paragraph("Editable text")])}
    warnings={options.warnings ?? []}
    flashToken={options.flashIndices?.length ? 1 : 0}
    flashIndices={options.flashIndices ?? []}
    hasUnsubmittedChanges={false}
    onChange={vi.fn()}
    onScroll={vi.fn()}
    scrollRatio={null}
    onNavigate={vi.fn()}
  />
);

describe("DocxEditor", () => {
  it("renders block and document warnings as working inline widgets", () => {
    const warnings: SyncWarning[] = [
      { message: "Block needs attention", affected_path: [0] },
      { message: "Document metadata warning", affected_path: [] },
    ];
    renderEditor({ warnings });

    const markers = screen.getAllByRole("button", { name: "Show sync warning for this block" });
    expect(markers).toHaveLength(2);
    fireEvent.click(markers[0]);
    expect(markers[0].textContent).toBe("Block needs attention");
  });

  it("clears the sync flash decoration after its short display interval", async () => {
    vi.useFakeTimers();
    renderEditor({ flashIndices: [0] });
    expect(document.querySelector(".docx-sync-flash")).not.toBeNull();

    await act(async () => { vi.advanceTimersByTime(231); });
    expect(document.querySelector(".docx-sync-flash")).toBeNull();
  });

  it("keeps opaque blocks inert and visually distinct", () => {
    renderEditor({ body: [{ type: "Opaque", origin: "Latex", raw: "\\custom", summary: "Unsupported command" }] });
    const opaque = screen.getByText("Unsupported command (Latex content)");
    expect(opaque.getAttribute("contenteditable")).toBe("false");
    expect(opaque.classList.contains("opaque-block")).toBe(true);
  });

  it("emits top-level patches while leaving identical prefix and suffix blocks alone", () => {
    const before = [paragraph("first"), paragraph("middle"), paragraph("last")];
    const after = [paragraph("first"), paragraph("changed"), paragraph("last")];
    expect(diffBlocks(before, after)).toEqual([
      { op: "Replace", path: [1], block: paragraph("changed") },
    ]);
  });

  it("round-trips code text wrapped in another semantic mark", () => {
    const block: Block = {
      type: "Paragraph",
      inline: [{ type: "Bold", inline: [{ type: "Code", text: "const value = 1" }] }],
      trivia: { leading: "" },
    };
    expect(nodeToBlock(blockToNode(block))).toEqual(block);
  });

  it("parses pasted headings and inline bold into the document schema", () => {
    const source = document.createElement("div");
    source.innerHTML = "<h2><strong>Heading</strong></h2><p>Body</p>";
    const parsed = ProseMirrorDOMParser.fromSchema(docxSchema).parse(source);
    expect(parsed.child(0).type.name).toBe("heading");
    expect(parsed.child(0).firstChild?.marks[0].type.name).toBe("bold");
    expect(parsed.child(1).textContent).toBe("Body");
  });

  it("defers remote document replacement while local edits are unsubmitted", () => {
    const { rerender } = renderEditor();
    const updated = makeDocument([paragraph("Confirmed from LaTeX")]);
    const props = { warnings: [], flashToken: 0, flashIndices: [], onChange: vi.fn(), onScroll: vi.fn(), scrollRatio: null, onNavigate: vi.fn() };
    rerender(<DocxEditor {...props} document={updated} hasUnsubmittedChanges />);
    expect(screen.getByText("Editable text")).toBeTruthy();
    rerender(<DocxEditor {...props} document={updated} hasUnsubmittedChanges={false} />);
    expect(screen.getByText("Confirmed from LaTeX")).toBeTruthy();
  });

  it("ignores out-of-range scroll requests", () => {
    const ref = createRef<DocxEditorHandle>();
    render(
      <DocxEditor ref={ref} document={makeDocument([paragraph("Only block")])} warnings={[]} flashToken={0} flashIndices={[]} hasUnsubmittedChanges={false} onChange={vi.fn()} onScroll={vi.fn()} scrollRatio={null} onNavigate={vi.fn()} />
    );
    expect(() => ref.current?.scrollToBlock(99)).not.toThrow();
  });
});
