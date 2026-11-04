// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Document } from "../ipc/commands";
import { DocxEditor } from "./DocxEditor";

afterEach(cleanup);

const document_: Document = {
  version: 1,
  metadata: { title: null, authors: [], date: null, latex_class: null, latex_class_options: [], preamble_raw: [] },
  body: [{ type: "Paragraph", inline: [{ type: "Text", text: "Editable text" }], trivia: { leading: "" } }],
  trailing_trivia: "",
};

describe("DocxFormatToolbar", () => {
  it("shows the block style and sends list edits through onChange", () => {
    const onChange = vi.fn();
    render(
      <DocxEditor document={document_} warnings={[]} flashToken={0} flashIndices={[]} hasUnsubmittedChanges={false} onChange={onChange} onScroll={vi.fn()} scrollRatio={null} onNavigate={vi.fn()} />
    );
    expect((screen.getByLabelText("Block style") as HTMLSelectElement).value).toBe("paragraph");
    fireEvent.click(screen.getByRole("button", { name: "Bulleted list" }));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0][0]).toMatchObject({ type: "List", kind: "Unordered" });
  });
});