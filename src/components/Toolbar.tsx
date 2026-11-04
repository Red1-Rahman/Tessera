// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { FileMenu } from "./FileMenu";

export type SyncStatus = "editing" | "syncing" | "synced" | "error";

interface ToolbarProps {
  syncStatus: SyncStatus;
  onOpenProject: () => void;
  onOpenFile: () => void;
  onExportClick: () => void;
  onSaveClick: () => void;
  syncScroll: boolean;
  onSyncScrollChange: (enabled: boolean) => void;
  saveDisabled?: boolean;
}

export function Toolbar({
  syncStatus,
  onOpenProject,
  onOpenFile,
  onExportClick,
  onSaveClick,
  syncScroll,
  onSyncScrollChange,
  saveDisabled = false,
}: ToolbarProps): JSX.Element {

  const statusLabels: Record<SyncStatus, string> = {
    editing: "Editing",
    syncing: "Syncing...",
    synced: "Synced",
    error: "Sync Error",
  };

  return (
    <header className="toolbar">
      <div className="toolbar-left">
        <h1 className="toolbar-title">Tessera</h1>
        <FileMenu onOpenProject={onOpenProject} onOpenFile={onOpenFile} />
      </div>
      <div className="toolbar-center">
        <label className="toolbar-toggle">
          <input
            type="checkbox"
            checked={syncScroll}
            onChange={(e) => onSyncScrollChange(e.target.checked)}
          />
          <span>Sync Scroll</span>
        </label>
      </div>
      <div className="toolbar-right">
        <div
          className={`sync-status sync-status-${syncStatus}`}
          title={statusLabels[syncStatus]}
          aria-live="polite"
        >
          <span className="sync-status-dot" />
          <span className="sync-status-text">{statusLabels[syncStatus]}</span>
        </div>
        <button type="button" onClick={onSaveClick} className="toolbar-btn" disabled={saveDisabled}>
          Save
        </button>
        <button type="button" onClick={onExportClick} className="toolbar-btn">
          Export
        </button>
      </div>
    </header>
  );
}