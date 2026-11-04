// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LatexEditor } from "./LatexEditor";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("LatexEditor", () => {
  it("mounts the CodeMirror source and applies highlight token decorations", async () => {
    render(
      <LatexEditor
        value="\\section{Intro}"
        tokens={[{ start: 0, end: 8, kind: "Command" }]}
        flash={null}
        warnings={[]}
        onChange={vi.fn()}
        onScroll={vi.fn()}
        scrollRatio={null}
        onNavigate={vi.fn()}
      />
    );

    expect(screen.getByTestId("latex-editor").textContent).toContain("\\section{Intro}");
    await waitFor(() => expect(document.querySelector(".cm-token-command")).not.toBeNull());
  });

  it("labels document warnings without inventing a block index", () => {
    render(
      <LatexEditor
        value="body"
        tokens={[]}
        flash={null}
        warnings={[{ message: "Document metadata was preserved", affected_path: [] }]}
        onChange={vi.fn()}
        onScroll={vi.fn()}
        scrollRatio={null}
        onNavigate={vi.fn()}
      />
    );

    expect(screen.getByText(/Document: Document metadata was preserved/)).toBeTruthy();
  });

  it("applies external source updates without reporting them as local edits", async () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <LatexEditor value="first" tokens={[]} flash={null} warnings={[]} onChange={onChange} onScroll={vi.fn()} scrollRatio={null} onNavigate={vi.fn()} />
    );
    rerender(
      <LatexEditor value="second" tokens={[]} flash={null} warnings={[]} onChange={onChange} onScroll={vi.fn()} scrollRatio={null} onNavigate={vi.fn()} />
    );

    await waitFor(() => expect(screen.getByTestId("latex-editor").textContent).toContain("second"));
    expect(onChange).not.toHaveBeenCalled();
  });
});
