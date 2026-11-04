// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { useCallback, useEffect, useRef, useState } from "react";
import {
  exportDocument,
  type ExportFormat,
  type TesseraError,
} from "../ipc/commands";
import { saveFileDialog, writeClipboardText } from "../ipc/native";

interface ExportMenuProps {
  isOpen: boolean;
  onClose: () => void;
  /** False when no project is open; export requires an open document. */
  hasDocument: boolean;
}

type ExportEntryId = ExportFormat | "clipboard";

interface ExportEntry {
  id: ExportEntryId;
  name: string;
  note: string;
}

interface ExportStatus {
  kind: "success" | "error";
  message: string;
}

const entries: ExportEntry[] = [
  { id: "Latex", name: "LaTeX", note: "Exact source" },
  { id: "Docx", name: "DOCX", note: "Word compatible" },
  { id: "Pdf", name: "PDF", note: "Basic PDF, no TeX engine found" },
  { id: "Markdown", name: "Markdown", note: "Pandoc compatible" },
  { id: "PlainText", name: "Plain text", note: "Flattened text" },
  { id: "clipboard", name: "Copy to clipboard", note: "Markdown format" },
];

const saveFilters: Record<
  Exclude<ExportEntryId, "clipboard">,
  { name: string; extensions: string[] }
> = {
  Latex: { name: "LaTeX document", extensions: ["tex"] },
  Docx: { name: "Word document", extensions: ["docx"] },
  Pdf: { name: "PDF document", extensions: ["pdf"] },
  Markdown: { name: "Markdown document", extensions: ["md"] },
  PlainText: { name: "Text document", extensions: ["txt"] },
};

const PDF_FALLBACK_NOTE = "Basic PDF (no TeX engine found)";

function exportErrorMessage(error: unknown): string {
  if (typeof error !== "object" || error === null || !("kind" in error)) {
    return "The export failed.";
  }
  const tesseraError = error as TesseraError;
  switch (tesseraError.kind) {
    case "ExportError":
      return `Export to ${tesseraError.format} failed: ${tesseraError.message}`;
    case "IoError":
      return `Could not write ${tesseraError.path}: ${tesseraError.message}`;
    case "ValidationError":
      return `Export failed: ${tesseraError.message}`;
    case "ParseError":
      return `Export failed: ${tesseraError.message}`;
    case "StaleVersion":
      return "The document changed during export. Try again.";
  }
}

export function ExportMenu({
  isOpen,
  onClose,
  hasDocument,
}: ExportMenuProps): JSX.Element | null {
  const menuRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<ExportStatus | null>(null);
  const [pdfNote, setPdfNote] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) {
      setStatus(null);
      return;
    }

    previousFocusRef.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    closeButtonRef.current?.focus();

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      previousFocusRef.current?.focus();
      previousFocusRef.current = null;
    };
  }, [isOpen, onClose]);

  const runExport = useCallback(async (id: ExportEntryId): Promise<void> => {
    try {
      if (id === "clipboard") {
        // docs/specs.md section 4: clipboard payloads exist for the text
        // formats; the menu's clipboard entry is the Markdown one.
        const result = await exportDocument({
          format: "Markdown",
          destination: null,
        });
        if (result.kind !== "ClipboardPayload") {
          setStatus({
            kind: "error",
            message:
              "The backend returned a file result for a clipboard export.",
          });
          return;
        }
        await writeClipboardText(result.content);
        setStatus({
          kind: "success",
          message: "Markdown copied to clipboard.",
        });
        return;
      }

      const destination = await saveFileDialog({ filters: [saveFilters[id]] });
      if (destination === null) return;
      const result = await exportDocument({ format: id, destination });
      if (result.kind !== "WrittenToFile") {
        setStatus({
          kind: "error",
          message: "The backend returned a clipboard result for a file export.",
        });
        return;
      }
      if (id === "Pdf") {
        // docs/specs.md section 4: the selected PDF tier comes back as
        // pdf_engine; surface the built-in fallback in the note slot.
        const engine = result.pdf_engine;
        if (!engine || engine.type === "BuiltIn") {
          setPdfNote(PDF_FALLBACK_NOTE);
        } else {
          setPdfNote(`System TeX: ${engine.program}`);
        }
      }
      setStatus({ kind: "success", message: `Saved to ${result.path}` });
    } catch (error) {
      setStatus({ kind: "error", message: exportErrorMessage(error) });
    }
  }, []);

  const handleExport = useCallback(
    (id: ExportEntryId) => {
      if (!hasDocument || busy) return;
      setBusy(true);
      setStatus(null);
      void runExport(id).finally(() => setBusy(false));
    },
    [hasDocument, busy, runExport],
  );

  if (!isOpen) return null;

  return (
    <div className="export-menu-overlay" onClick={onClose} role="presentation">
      <div
        className="export-menu"
        ref={menuRef}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-labelledby="export-menu-title"
        aria-modal="true"
      >
        <div className="export-menu-header">
          <h2 id="export-menu-title">Export</h2>
          <button
            ref={closeButtonRef}
            className="export-menu-close"
            type="button"
            onClick={onClose}
            aria-label="Close export menu"
          >
            Close
          </button>
        </div>
        <ul className="export-menu-list">
          {entries.map((fmt) => {
            const note =
              fmt.id === "Pdf" && pdfNote !== null ? pdfNote : fmt.note;
            return (
              <li key={fmt.id}>
                <button
                  className="export-menu-item"
                  type="button"
                  disabled={!hasDocument || busy}
                  onClick={() => handleExport(fmt.id)}
                >
                  <span className="export-menu-name">{fmt.name}</span>
                  {note && <span className="export-menu-note">{note}</span>}
                </button>
              </li>
            );
          })}
        </ul>
        {status && (
          <p
            className={`export-menu-status${status.kind === "error" ? " export-menu-status-error" : ""}`}
            role={status.kind === "error" ? "alert" : "status"}
          >
            {status.message}
          </p>
        )}
      </div>
    </div>
  );
}
