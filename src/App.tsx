// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { SplitPane } from "./components/SplitPane";
import { Toolbar, type SyncStatus } from "./components/Toolbar";
import { ExportMenu } from "./components/ExportMenu";
import { DocxEditor, diffBlocks, type DocxEditorHandle } from "./editors/DocxEditor";
import { LatexEditor, type EditorFlash, type LatexEditorHandle } from "./editors/LatexEditor";
import {
  applyDocxEdit,
  exportDocument,
  getHighlightTokens,
  parseLatex,
  type Block,
  type DmPatch,
  type HighlightKind,
  type HighlightToken,
  type TesseraError,
} from "./ipc/commands";
import {
  applySyncResult,
  DOCX_SYNC_DEBOUNCE_MS,
  getSnapshot,
  LATEX_REPARSE_DEBOUNCE_MS,
  subscribe,
} from "./state/documentStore";
import "./styles.css";

type PendingEdit = { kind: "latex"; source: string } | { kind: "docx"; blocks: Block[]; revision: number };

function errorMessage(error: unknown): string {
  if (typeof error !== "object" || error === null || !("kind" in error)) return "The document could not be synchronized.";
  const tesseraError = error as TesseraError;
  switch (tesseraError.kind) {
    case "StaleVersion": return `This edit used version ${tesseraError.requested}, but the current version is ${tesseraError.current}.`;
    case "ParseError": return tesseraError.message;
    case "ValidationError": return tesseraError.message;
    case "IoError": return tesseraError.message;
    case "ExportError": return tesseraError.message;
  }
}

function findLabelIndex(blocks: Block[], label: string): number {
  const contains = (block: Block): boolean => {
    if ((block.type === "Heading" || block.type === "Figure" || block.type === "Equation") && block.label === label) return true;
    if (block.type === "Table" && block.label === label) return true;
    if (block.type === "Quote") return block.blocks.some(contains);
    if (block.type === "List") return block.items.some((item) => item.some(contains));
    if (block.type === "Table") return block.rows.some((row) => row.some((cell) => cell.blocks.some(contains)));
    return false;
  };
  return blocks.findIndex(contains);
}

function labelForBlock(block: Block): string | null {
  if (block.type === "Heading" || block.type === "Figure" || block.type === "Equation") return block.label;
  if (block.type === "Table" && block.label) return block.label;
  if (block.type === "Quote") {
    for (const child of block.blocks) {
      const label = labelForBlock(child);
      if (label) return label;
    }
  }
  if (block.type === "List") {
    for (const item of block.items) {
      for (const child of item) {
        const label = labelForBlock(child);
        if (label) return label;
      }
    }
  }
  if (block.type === "Table") {
    for (const cell of block.rows.flat()) {
      for (const child of cell.blocks) {
        const label = labelForBlock(child);
        if (label) return label;
      }
    }
  }
  return null;
}

function changedBlockIndices(previous: Block[], next: Block[]): number[] {
  const patches = diffBlocks(previous, next);
  return [...new Set(patches.map((patch) => patch.op === "Remove" ? patch.index ?? 0 : patch.op === "Insert" ? patch.index ?? 0 : patch.path[0] ?? 0))];
}

function textDifference(previous: string, next: string): { from: number; to: number } {
  let from = 0;
  while (from < previous.length && from < next.length && previous.charCodeAt(from) === next.charCodeAt(from)) from += 1;
  let previousEnd = previous.length;
  let nextEnd = next.length;
  while (previousEnd > from && nextEnd > from && previous.charCodeAt(previousEnd - 1) === next.charCodeAt(nextEnd - 1)) {
    previousEnd -= 1;
    nextEnd -= 1;
  }
  if (nextEnd === from && next.length > 0) return { from: Math.max(0, from - 1), to: Math.min(next.length, from + 1) };
  return { from, to: nextEnd };
}

function flashKind(tokens: HighlightToken[], from: number, to: number): HighlightKind {
  return tokens.find((token) => token.start < to && token.end > from)?.kind ?? "Text";
}

function App(): JSX.Element {
  const documentState = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  const document = documentState.document;
  const hasDocument = document !== null;
  const warnings = documentState.warnings;
  const [syncStatus, setSyncStatus] = useState<SyncStatus>("editing");
  const [isExportOpen, setIsExportOpen] = useState(false);
  const [syncScroll, setSyncScroll] = useState(true);
  const [latexSource, setLatexSource] = useState("");
  const [tokens, setTokens] = useState<HighlightToken[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [syncConflict, setSyncConflict] = useState<string | null>(null);
  const [docxDirty, setDocxDirty] = useState(false);
  const [docxFlash, setDocxFlash] = useState({ token: 0, indices: [] as number[] });
  const [latexFlash, setLatexFlash] = useState<EditorFlash | null>(null);
  const [latexScrollRatio, setLatexScrollRatio] = useState<number | null>(null);
  const [docxScrollRatio, setDocxScrollRatio] = useState<number | null>(null);
  const latexRef = useRef<LatexEditorHandle>(null);
  const docxRef = useRef<DocxEditorHandle>(null);
  const pendingRef = useRef<PendingEdit[]>([]);
  const retriedEditsRef = useRef(new Set<PendingEdit>());
  const drainingRef = useRef(false);
  const latexTimerRef = useRef<number | null>(null);
  const docxTimerRef = useRef<number | null>(null);
  const latexDirtyRef = useRef(false);
  const docxDirtyRef = useRef(false);
  const docxRevisionRef = useRef(0);
  const syncConflictRef = useRef(false);
  const docxConflictRenderRef = useRef<string | null>(null);
  const sourceRef = useRef(latexSource);
  const flashTokenRef = useRef(0);
  const initializedProjectLoadRef = useRef(0);
  const observedProjectLoadRef = useRef(0);

  sourceRef.current = latexSource;

  useEffect(() => {
    const projectLoadId = documentState.projectLoadId;
    if (!hasDocument) return;
    if (observedProjectLoadRef.current !== projectLoadId) {
      observedProjectLoadRef.current = projectLoadId;
      pendingRef.current = [];
      if (latexTimerRef.current !== null) window.clearTimeout(latexTimerRef.current);
      if (docxTimerRef.current !== null) window.clearTimeout(docxTimerRef.current);
      latexDirtyRef.current = false;
      docxDirtyRef.current = false;
      docxRevisionRef.current = 0;
      syncConflictRef.current = false;
      docxConflictRenderRef.current = null;
      sourceRef.current = "";
      setLatexSource("");
      setTokens([]);
      setDocxDirty(false);
      setSyncConflict(null);
      setError(null);
      setSyncStatus("editing");
      setLatexScrollRatio(null);
      setDocxScrollRatio(null);
    }
    if (initializedProjectLoadRef.current === projectLoadId || latexDirtyRef.current) return;
    const previousProjectLoadId = initializedProjectLoadRef.current;
    initializedProjectLoadRef.current = projectLoadId;
    let cancelled = false;
    let completed = false;
    void exportDocument({ format: "Latex", destination: null }).then((result) => {
      if (!cancelled && !latexDirtyRef.current && result.kind === "ClipboardPayload") {
        setLatexSource(result.content);
        sourceRef.current = result.content;
        void getHighlightTokens({ source: result.content }).then((nextTokens) => {
          if (!cancelled) setTokens(nextTokens);
        }).catch(() => undefined);
      }
      completed = true;
    }).catch(() => { completed = true; });
    return () => {
      cancelled = true;
      if (!completed && initializedProjectLoadRef.current === projectLoadId) {
        initializedProjectLoadRef.current = previousProjectLoadId;
      }
    };
  }, [hasDocument, documentState.projectLoadId]);

  const drainQueue = useCallback(async (): Promise<void> => {
    if (drainingRef.current) return;
    drainingRef.current = true;
    while (pendingRef.current.length > 0 && !syncConflictRef.current) {
      const edit = pendingRef.current.shift();
      const current = getSnapshot();
      if (!edit || !current.document) continue;
      setSyncStatus("syncing");
      try {
        let result;
        if (edit.kind === "latex") {
          result = await parseLatex({ base_version: current.version, full_source: edit.source });
        } else {
          const patches = diffBlocks(current.document.body, edit.blocks).map((patch): DmPatch => {
            if (patch.op === "Replace") return { op: "Replace", path: patch.path, block: patch.block as Block };
            if (patch.op === "Insert") return { op: "Insert", path: [], index: patch.index as number, block: patch.block as Block };
            return { op: "Remove", path: [], index: patch.index as number };
          });
          if (patches.length === 0) {
            retriedEditsRef.current.delete(edit);
            setError(null);
            if (docxRevisionRef.current === edit.revision) {
              docxDirtyRef.current = false;
              setDocxDirty(false);
            }
            setSyncStatus("synced");
            continue;
          }
          result = await applyDocxEdit({ base_version: current.version, patches });
        }

        if (getSnapshot().projectLoadId !== current.projectLoadId) continue;

        if (!applySyncResult(result)) {
          if (!retriedEditsRef.current.has(edit)) {
            retriedEditsRef.current.add(edit);
            pendingRef.current.unshift(edit);
          } else {
            setError("A newer document version arrived during sync. The edit is still in the editor and can be resubmitted.");
            setSyncStatus("error");
          }
          if (pendingRef.current[0] === edit) continue;
          continue;
        }
        retriedEditsRef.current.delete(edit);
        setError(null);
        const indices = changedBlockIndices(current.document.body, result.document.body);
        const token = ++flashTokenRef.current;
        if (edit.kind === "latex") {
          setDocxFlash({ token, indices });
          latexDirtyRef.current = sourceRef.current !== edit.source;
        } else {
          if (latexDirtyRef.current) {
            syncConflictRef.current = true;
            docxConflictRenderRef.current = result.latex_render;
            setSyncConflict("Both panes changed during sync. Your LaTeX text and the DOCX update are preserved. Choose which pane to apply to the shared document.");
          } else {
            const difference = textDifference(sourceRef.current, result.latex_render);
            const nextTokens = await getHighlightTokens({ source: result.latex_render }).catch(() => []);
            setTokens(nextTokens);
            setLatexSource(result.latex_render);
            sourceRef.current = result.latex_render;
            setLatexFlash({ ...difference, kind: flashKind(nextTokens, difference.from, difference.to), token });
          }
          docxDirtyRef.current = docxRevisionRef.current !== edit.revision;
          setDocxDirty(docxDirtyRef.current);
        }
        if (edit.kind === "latex") {
          if (sourceRef.current === edit.source) {
            const nextTokens = await getHighlightTokens({ source: edit.source }).catch(() => []);
            if (sourceRef.current === edit.source) setTokens(nextTokens);
          }
        }
        setSyncStatus(syncConflictRef.current ? "error" : latexDirtyRef.current || docxDirtyRef.current ? "editing" : "synced");
      } catch (caught) {
        setError(errorMessage(caught));
        setSyncStatus("error");
        if (edit.kind === "docx" && !retriedEditsRef.current.has(edit)) {
          retriedEditsRef.current.add(edit);
          pendingRef.current.unshift(edit);
        }
      }
    }
    drainingRef.current = false;
  }, []);

  const queueEdit = useCallback((edit: PendingEdit) => {
    const existing = pendingRef.current.findIndex((pending) => pending.kind === edit.kind);
    if (existing >= 0) pendingRef.current[existing] = edit;
    else pendingRef.current.push(edit);
    void drainQueue();
  }, [drainQueue]);

  const handleLatexChange = useCallback((source: string) => {
    setLatexSource(source);
    sourceRef.current = source;
    latexDirtyRef.current = true;
    setSyncStatus("editing");
    if (latexTimerRef.current !== null) window.clearTimeout(latexTimerRef.current);
    latexTimerRef.current = window.setTimeout(
      () => queueEdit({ kind: "latex", source: sourceRef.current }),
      LATEX_REPARSE_DEBOUNCE_MS
    );
  }, [queueEdit]);

  const handleDocxChange = useCallback((blocks: Block[]) => {
    docxRevisionRef.current += 1;
    docxDirtyRef.current = true;
    setDocxDirty(true);
    const revision = docxRevisionRef.current;
    setSyncStatus("editing");
    if (docxTimerRef.current !== null) window.clearTimeout(docxTimerRef.current);
    docxTimerRef.current = window.setTimeout(
      () => queueEdit({ kind: "docx", blocks, revision }),
      DOCX_SYNC_DEBOUNCE_MS
    );
  }, [queueEdit]);

  const keepLatexConflict = useCallback(() => {
    syncConflictRef.current = false;
    setSyncConflict(null);
    setError(null);
    latexDirtyRef.current = true;
    queueEdit({ kind: "latex", source: sourceRef.current });
  }, [queueEdit]);

  const keepDocxConflict = useCallback(() => {
    const source = docxConflictRenderRef.current;
    if (source === null) return;
    syncConflictRef.current = false;
    setSyncConflict(null);
    setError(null);
    if (latexTimerRef.current !== null) window.clearTimeout(latexTimerRef.current);
    pendingRef.current = pendingRef.current.filter((edit) => edit.kind !== "latex");
    latexDirtyRef.current = false;
    sourceRef.current = source;
    setLatexSource(source);
    void getHighlightTokens({ source }).then(setTokens).catch(() => setTokens([]));
    setSyncStatus(docxDirtyRef.current ? "editing" : "synced");
  }, []);

  useEffect(() => () => {
    if (latexTimerRef.current !== null) window.clearTimeout(latexTimerRef.current);
    if (docxTimerRef.current !== null) window.clearTimeout(docxTimerRef.current);
  }, []);

  const navigateToLabel = useCallback((label: string) => {
    const blocks = getSnapshot().document?.body ?? [];
    const index = findLabelIndex(blocks, label);
    if (index < 0) return;
    latexRef.current?.scrollToLabel(label);
    docxRef.current?.scrollToBlock(index);
    setDocxFlash({ token: ++flashTokenRef.current, indices: [index] });
  }, []);

  const navigateToBlock = useCallback((index: number) => {
    const block = getSnapshot().document?.body[index];
    if (!block) return;
    docxRef.current?.scrollToBlock(index);
    const label = labelForBlock(block);
    if (label) latexRef.current?.scrollToLabel(label);
    setDocxFlash({ token: ++flashTokenRef.current, indices: [index] });
  }, []);

  const left = document ? (
    <LatexEditor
      ref={latexRef}
      value={latexSource}
      tokens={tokens}
      flash={latexFlash}
      warnings={warnings}
      onChange={handleLatexChange}
      onNavigate={navigateToLabel}
      onScroll={(ratio) => {
        if (syncScroll) setDocxScrollRatio(ratio);
      }}
      scrollRatio={syncScroll ? latexScrollRatio : null}
    />
  ) : (
    <div className="pane-empty-state" aria-label="LaTeX source editor">
      <h2>LaTeX</h2>
      <p>Open a project to start editing.</p>
    </div>
  );

  const right = document ? (
    <DocxEditor
      ref={docxRef}
      document={document}
      warnings={warnings}
      flashToken={docxFlash.token}
      flashIndices={docxFlash.indices}
      hasUnsubmittedChanges={docxDirty}
      onChange={handleDocxChange}
      onNavigate={navigateToBlock}
      onScroll={(ratio) => {
        if (syncScroll) setLatexScrollRatio(ratio);
      }}
      scrollRatio={syncScroll ? docxScrollRatio : null}
    />
  ) : (
    <div className="pane-empty-state" aria-label="DOCX document editor">
      <h2>DOCX</h2>
      <p>Open a project to see the structured document.</p>
    </div>
  );

  return (
    <main className="app-root" data-testid="app-root">
      <Toolbar
        syncStatus={syncStatus}
        syncScroll={syncScroll}
        onSyncScrollChange={setSyncScroll}
        onExportClick={() => setIsExportOpen(true)}
        onSaveClick={() => undefined}
        saveDisabled
      />
      {error && <div className="sync-error" role="alert">{error}</div>}
      {syncConflict && (
        <div className="sync-error" role="alert">
          {syncConflict}
          <button type="button" onClick={keepLatexConflict}>Keep LaTeX</button>
          <button type="button" onClick={keepDocxConflict}>Keep DOCX</button>
        </div>
      )}
      <div className="app-content">
        <SplitPane left={left} right={right} />
      </div>
      <ExportMenu isOpen={isExportOpen} onClose={() => setIsExportOpen(false)} hasDocument={hasDocument} />
    </main>
  );
}

export default App;
