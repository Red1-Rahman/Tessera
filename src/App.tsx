// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { useState } from "react";
import { SplitPane } from "./components/SplitPane";
import { Toolbar, SyncStatus } from "./components/Toolbar";
import { ExportMenu } from "./components/ExportMenu";
import "./styles.css";

function App(): JSX.Element {
  const syncStatus: SyncStatus = "editing";
  const [isExportOpen, setIsExportOpen] = useState(false);

  // Placeholder panes for now, will be replaced in task 09
  const latexPane = (
    <div className="pane-placeholder latex-placeholder">
      <h2>LaTeX Pane</h2>
      <p>CodeMirror editor will be integrated here in Task 09.</p>
    </div>
  );

  const docxPane = (
    <div className="pane-placeholder docx-placeholder">
      <h2>DOCX Pane</h2>
      <p>ProseMirror/Tiptap editor will be integrated here in Task 09.</p>
    </div>
  );

  return (
    <main className="app-root" data-testid="app-root">
      <Toolbar
        syncStatus={syncStatus}
        onExportClick={() => setIsExportOpen(true)}
        onSaveClick={() => undefined}
        saveDisabled
      />
      <div className="app-content">
        <SplitPane left={latexPane} right={docxPane} />
      </div>
      <ExportMenu isOpen={isExportOpen} onClose={() => setIsExportOpen(false)} />
    </main>
  );
}

export default App;