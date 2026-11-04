// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportMenu } from "./ExportMenu";
import { exportDocument } from "../ipc/commands";
import { saveFileDialog, writeClipboardText } from "../ipc/native";

vi.mock("../ipc/commands", () => ({
  exportDocument: vi.fn(),
}));

vi.mock("../ipc/native", () => ({
  saveFileDialog: vi.fn(),
  writeClipboardText: vi.fn(),
}));

const exportDocumentMock = vi.mocked(exportDocument);
const saveFileDialogMock = vi.mocked(saveFileDialog);
const writeClipboardTextMock = vi.mocked(writeClipboardText);

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ExportMenu", () => {
  it("renders all six export entries when open", () => {
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);
    for (const name of [
      "LaTeX",
      "DOCX",
      "PDF",
      "Markdown",
      "Plain text",
      "Copy to clipboard",
    ]) {
      expect(
        screen.getByRole("button", { name: new RegExp(`^${name}`) }),
      ).toBeTruthy();
    }
  });

  it("renders nothing when closed", () => {
    const { container } = render(
      <ExportMenu isOpen={false} onClose={() => undefined} hasDocument />,
    );
    expect(container.textContent).toBe("");
  });

  it("disables every entry when no document is open", () => {
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument={false} />);
    for (const button of screen.getAllByRole("button", {
      name: /LaTeX|DOCX|PDF|Markdown|Plain text|Copy to clipboard/,
    })) {
      expect((button as HTMLButtonElement).disabled).toBe(true);
    }
    expect(exportDocumentMock).not.toHaveBeenCalled();
  });

  it("saves LaTeX through the save dialog with a .tex filter", async () => {
    saveFileDialogMock.mockResolvedValue("D:\\out\\main.tex");
    exportDocumentMock.mockResolvedValue({
      kind: "WrittenToFile",
      path: "D:\\out\\main.tex",
    });
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^LaTeX/ }));

    await waitFor(() =>
      expect(screen.getByRole("status").textContent).toBe(
        "Saved to D:\\out\\main.tex",
      ),
    );
    expect(saveFileDialogMock).toHaveBeenCalledWith({
      filters: [{ name: "LaTeX document", extensions: ["tex"] }],
    });
    expect(exportDocumentMock).toHaveBeenCalledWith({
      format: "Latex",
      destination: "D:\\out\\main.tex",
    });
  });

  it("does not call the backend when the save dialog is cancelled", async () => {
    saveFileDialogMock.mockResolvedValue(null);
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^Markdown/ }));

    await waitFor(() => expect(saveFileDialogMock).toHaveBeenCalled());
    expect(exportDocumentMock).not.toHaveBeenCalled();
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("copies the Markdown clipboard payload to the clipboard", async () => {
    exportDocumentMock.mockResolvedValue({
      kind: "ClipboardPayload",
      format: "Markdown",
      content: "# Title",
    });
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^Copy to clipboard/ }));

    await waitFor(() =>
      expect(screen.getByRole("status").textContent).toBe(
        "Markdown copied to clipboard.",
      ),
    );
    expect(exportDocumentMock).toHaveBeenCalledWith({
      format: "Markdown",
      destination: null,
    });
    expect(writeClipboardTextMock).toHaveBeenCalledWith("# Title");
  });

  it("shows the built-in typesetter caveat in the PDF note slot", async () => {
    saveFileDialogMock.mockResolvedValue("D:\\out\\main.pdf");
    exportDocumentMock.mockResolvedValue({
      kind: "WrittenToFile",
      path: "D:\\out\\main.pdf",
      pdf_engine: { type: "BuiltIn" },
    });
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^PDF/ }));

    await waitFor(() =>
      expect(screen.getByText("Basic PDF (no TeX engine found)")).toBeTruthy(),
    );
  });

  it("shows the system TeX program in the PDF note slot", async () => {
    saveFileDialogMock.mockResolvedValue("D:\\out\\main.pdf");
    exportDocumentMock.mockResolvedValue({
      kind: "WrittenToFile",
      path: "D:\\out\\main.pdf",
      pdf_engine: { type: "SystemTeX", program: "pdflatex" },
    });
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^PDF/ }));

    await waitFor(() =>
      expect(screen.getByText("System TeX: pdflatex")).toBeTruthy(),
    );
  });

  it("surfaces an ExportError as a non-blocking alert and re-enables the entries", async () => {
    saveFileDialogMock.mockResolvedValue("D:\\out\\main.pdf");
    exportDocumentMock.mockRejectedValue({
      kind: "ExportError",
      format: "Pdf",
      message: "PDF export is not available for this document.",
    });
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^PDF/ }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe(
      "Export to Pdf failed: PDF export is not available for this document.",
    );
    await waitFor(() =>
      expect(
        (screen.getByRole("button", { name: /^DOCX/ }) as HTMLButtonElement)
          .disabled,
      ).toBe(false),
    );
  });

  it("disables every entry while an export is in flight", async () => {
    let resolveExport: (value: {
      kind: "WrittenToFile";
      path: string;
    }) => void = () => undefined;
    saveFileDialogMock.mockResolvedValue("D:\\out\\main.md");
    exportDocumentMock.mockReturnValue(
      new Promise((resolve) => {
        resolveExport = resolve;
      }),
    );
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^Markdown/ }));
    await waitFor(() =>
      expect(
        (screen.getByRole("button", { name: /^DOCX/ }) as HTMLButtonElement)
          .disabled,
      ).toBe(true),
    );

    resolveExport({ kind: "WrittenToFile", path: "D:\\out\\main.md" });
    await waitFor(() =>
      expect(
        (screen.getByRole("button", { name: /^DOCX/ }) as HTMLButtonElement)
          .disabled,
      ).toBe(false),
    );
  });

  it("shows a generic message for an unexpected error shape", async () => {
    exportDocumentMock.mockRejectedValue("network offline");
    render(<ExportMenu isOpen onClose={() => undefined} hasDocument />);

    fireEvent.click(screen.getByRole("button", { name: /^Copy to clipboard/ }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe("The export failed.");
  });
});
