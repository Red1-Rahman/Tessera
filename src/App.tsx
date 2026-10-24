// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

/**
 * Application root. Placeholder only: serial-task08-B replaces this with
 * the real split-pane layout (LaTeX pane, DOCX pane, shared toolbar).
 * It renders no Tauri IPC calls, so it also works in a plain browser for
 * e2e smoke tests.
 */
function App(): JSX.Element {
  return (
    <main data-testid="app-root">
      <h1>Tessera</h1>
      <p>
        Placeholder shell. The split-pane LaTeX/DOCX editor arrives with the
        frontend shell task.
      </p>
    </main>
  );
}

export default App;
