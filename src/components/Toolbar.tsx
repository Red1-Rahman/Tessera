// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { useState } from "react";

export type SyncStatus = "editing" | "syncing" | "synced" | "error";

interface ToolbarProps {
  syncStatus: SyncStatus;
  onExportClick: () => void;
  onSaveClick: () => void;
  saveDisabled?: boolean;
}

export function Toolbar({
  syncStatus,
  onExportClick,
  onSaveClick,
  saveDisabled = false,
}: ToolbarProps): JSX.Element {
  const [syncScroll, setSyncScroll] = useState(true);

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
      </div>
      <div className="toolbar-center">
        <label className="toolbar-toggle">
          <input
            type="checkbox"
            checked={syncScroll}
            onChange={(e) => setSyncScroll(e.target.checked)}
          />
          <span>Sync Scroll</span>
        </label>
      </div>
      <div className="toolbar-right">
        <div
          className={`sync-status sync-status-${syncStatus}`}
          title={statusLabels[syncStatus]}
        >
          <span className="sync-status-dot" />
          <span className="sync-status-text">{statusLabels[syncStatus]}</span>
        </div>
        <button onClick={onSaveClick} className="toolbar-btn" disabled={saveDisabled}>
          Save
        </button>
        <button onClick={onExportClick} className="toolbar-btn">
          Export
        </button>
      </div>
    </header>
  );
}