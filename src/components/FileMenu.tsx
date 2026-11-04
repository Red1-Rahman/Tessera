// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { useEffect, useRef, useState } from "react";
import "./FileMenu.css";

interface FileMenuProps {
  onOpenProject: () => void;
  onOpenFile: () => void;
}

/**
 * Toolbar dropdown offering "Open Project" (a directory) and "Open File"
 * (a standalone .tex or .docx). It only reports the choice; the dialogs
 * and backend calls are handled by the caller.
 */
export function FileMenu({ onOpenProject, onOpenFile }: FileMenuProps): JSX.Element {
  const [isOpen, setIsOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;
    const handlePointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setIsOpen(false);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setIsOpen(false);
    };
    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  const choose = (action: () => void) => {
    setIsOpen(false);
    action();
  };

  return (
    <div className="file-menu" ref={rootRef}>
      <button
        type="button"
        className="toolbar-btn"
        aria-haspopup="menu"
        aria-expanded={isOpen}
        onClick={() => setIsOpen((open) => !open)}
      >
        File
      </button>
      {isOpen && (
        <ul className="file-menu-list" role="menu">
          <li role="none">
            <button type="button" role="menuitem" className="file-menu-item" onClick={() => choose(onOpenProject)}>
              <span className="file-menu-name">Open Project...</span>
              <span className="file-menu-note">Choose a folder with main.tex</span>
            </button>
          </li>
          <li role="none">
            <button type="button" role="menuitem" className="file-menu-item" onClick={() => choose(onOpenFile)}>
              <span className="file-menu-name">Open File...</span>
              <span className="file-menu-note">A single .tex or .docx file</span>
            </button>
          </li>
        </ul>
      )}
    </div>
  );
}