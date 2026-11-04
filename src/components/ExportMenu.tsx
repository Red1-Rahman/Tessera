// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { useEffect, useRef } from "react";

interface ExportMenuProps {
  isOpen: boolean;
  onClose: () => void;
}

const formats = [
  { id: "latex", name: "LaTeX", note: "Exact source" },
  { id: "docx", name: "DOCX", note: "Word compatible" },
  { id: "pdf", name: "PDF", note: "Basic PDF, no TeX engine found" },
  { id: "markdown", name: "Markdown", note: "Pandoc compatible" },
  { id: "plaintext", name: "Plain text", note: "Flattened text" },
  { id: "clipboard", name: "Copy to clipboard", note: "Markdown format" },
];

export function ExportMenu({ isOpen, onClose }: ExportMenuProps): JSX.Element | null {
  const menuRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!isOpen) return;

    previousFocusRef.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
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
          {formats.map((fmt) => (
            <li key={fmt.id}>
              <button className="export-menu-item" type="button" disabled>
                <span className="export-menu-name">{fmt.name}</span>
                {fmt.note && <span className="export-menu-note">{fmt.note}</span>}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}