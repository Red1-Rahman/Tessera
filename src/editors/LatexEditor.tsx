// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { EditorState, StateEffect, StateField, type Extension } from "@codemirror/state";
import {
  Decoration,
  DecorationSet,
  EditorView,
  keymap,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import type { HighlightKind, HighlightToken } from "../ipc/commands";
import { createScrollEchoSuppressor, type ScrollEchoSuppressor } from "./scrollEcho";

export interface EditorFlash {
  from: number;
  to: number;
  kind: HighlightKind;
  token: number;
}

interface LatexEditorProps {
  value: string;
  tokens: HighlightToken[];
  flash: EditorFlash | null;
  warnings: { message: string; affected_path: number[] }[];
  onChange: (value: string) => void;
  onScroll: (ratio: number) => void;
  scrollRatio: number | null;
  onNavigate: (label: string) => void;
}

export interface LatexEditorHandle {
  scrollToLabel(label: string): void;
}

const setTokens = StateEffect.define<HighlightToken[]>();
const setFlash = StateEffect.define<EditorFlash | null>();

const tokenClass: Record<HighlightKind, string> = {
  Command: "cm-token-command",
  Bracket: "cm-token-bracket",
  Environment: "cm-token-environment",
  Comment: "cm-token-comment",
  Math: "cm-token-math",
  Error: "cm-token-error",
  Text: "cm-token-text",
};

const tokenField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(decorations, transaction) {
    decorations = decorations.map(transaction.changes);
    for (const effect of transaction.effects) {
      if (effect.is(setTokens)) {
        const ranges = effect.value.flatMap((token) => {
          const from = Math.max(0, Math.min(token.start, transaction.newDoc.length));
          const to = Math.max(from, Math.min(token.end, transaction.newDoc.length));
          return to > from
            ? [Decoration.mark({ class: tokenClass[token.kind] }).range(from, to)]
            : [];
        });
        decorations = Decoration.set(ranges, true);
      }
    }
    return decorations;
  },
  provide: (field) => EditorView.decorations.from(field),
});

const flashField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(decorations, transaction) {
    decorations = decorations.map(transaction.changes);
    for (const effect of transaction.effects) {
      if (!effect.is(setFlash)) continue;
      const flash = effect.value;
      if (!flash) {
        decorations = Decoration.none;
        continue;
      }
      const from = Math.max(0, Math.min(flash.from, transaction.newDoc.length));
      const to = Math.max(from, Math.min(flash.to, transaction.newDoc.length));
      decorations =
        to > from
          ? Decoration.set([
              Decoration.mark({
                class: `cm-sync-flash cm-sync-flash-${flash.kind.toLowerCase()}`,
                attributes: { "data-flash-token": String(flash.token) },
              }).range(from, to),
            ])
          : Decoration.none;
    }
    return decorations;
  },
  provide: (field) => EditorView.decorations.from(field),
});

function createExtensions(onChange: (value: string) => void): Extension[] {
  return [
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    tokenField,
    flashField,
    EditorView.lineWrapping,
    EditorView.theme({
      "&": { height: "100%", backgroundColor: "transparent", color: "var(--ok-black)" },
      ".cm-scroller": { overflow: "auto", font: "var(--text-latex-body)" },
      ".cm-content": { minHeight: "100%", padding: "var(--space-lg)" },
      ".cm-gutters": { backgroundColor: "transparent", border: "none", color: "var(--chrome-text-muted)" },
      ".cm-cursor": { borderLeftColor: "var(--chrome-text)" },
      ".cm-token-command": { color: "var(--ok-orange)" },
      ".cm-token-bracket": { color: "var(--ok-sky)" },
      ".cm-token-environment": { color: "var(--ok-green)" },
      ".cm-token-comment": { color: "var(--ok-black)", backgroundColor: "color-mix(in srgb, var(--ok-yellow) 32%, transparent)" },
      ".cm-token-math": { color: "var(--ok-blue)" },
      ".cm-token-error": { color: "var(--ok-vermillion)", textDecoration: "underline wavy" },
      ".cm-token-text": { color: "var(--ok-black)" },
      ".cm-sync-flash": { animation: "tessera-sync-flash 200ms ease-out" },
      ".cm-sync-flash-command": { "--flash-color": "color-mix(in srgb, var(--ok-orange) 20%, transparent)" },
      ".cm-sync-flash-bracket": { "--flash-color": "color-mix(in srgb, var(--ok-sky) 25%, transparent)" },
      ".cm-sync-flash-environment": { "--flash-color": "color-mix(in srgb, var(--ok-green) 20%, transparent)" },
      ".cm-sync-flash-comment": { "--flash-color": "color-mix(in srgb, var(--ok-yellow) 35%, transparent)" },
      ".cm-sync-flash-math": { "--flash-color": "color-mix(in srgb, var(--ok-blue) 20%, transparent)" },
      ".cm-sync-flash-error": { "--flash-color": "color-mix(in srgb, var(--ok-vermillion) 20%, transparent)" },
      ".cm-sync-flash-text": { "--flash-color": "color-mix(in srgb, var(--ok-black) 12%, transparent)" },
    }),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) onChange(update.state.doc.toString());
    }),
  ];
}

export const LatexEditor = forwardRef<LatexEditorHandle, LatexEditorProps>(function LatexEditor({
  value,
  tokens,
  flash,
  warnings,
  onChange,
  onScroll,
  scrollRatio,
  onNavigate,
}: LatexEditorProps, forwardedRef): JSX.Element {
  const mountRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const suppressChangeRef = useRef(false);
  const appliedScrollRef = useRef<number | null>(null);
  const scrollEchoRef = useRef<ScrollEchoSuppressor | null>(null);
  if (scrollEchoRef.current === null) scrollEchoRef.current = createScrollEchoSuppressor();
  const onNavigateRef = useRef(onNavigate);
  const onScrollRef = useRef(onScroll);

  onChangeRef.current = onChange;
  onNavigateRef.current = onNavigate;
  onScrollRef.current = onScroll;

  useImperativeHandle(forwardedRef, () => ({
    scrollToLabel(label) {
      const view = viewRef.current;
      if (!view) return;
      const source = view.state.doc.toString();
      const escapedLabel = label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      const match = new RegExp(`\\\\label\\s*\\{\\s*${escapedLabel}\\s*\\}`).exec(source);
      if (!match) return;
      const from = match.index;
      const to = from + match[0].length;
      view.dispatch({ effects: EditorView.scrollIntoView(from, { y: "center" }) });
      view.dispatch({ effects: setFlash.of({ from, to, kind: "Command", token: Date.now() }) });
      window.setTimeout(() => {
        if (viewRef.current === view) view.dispatch({ effects: setFlash.of(null) });
      }, 230);
    },
  }), []);

  useEffect(() => {
    const mount = mountRef.current;
    if (!mount) return;

    const view = new EditorView({
      state: EditorState.create({
        doc: value,
        extensions: createExtensions((source) => {
          if (!suppressChangeRef.current) onChangeRef.current(source);
        }),
      }),
      parent: mount,
    });
    viewRef.current = view;
    view.dom.classList.add("latex-codemirror");

    const handleScroll = () => {
      if (scrollEchoRef.current?.isSuppressed()) return;
      const scroller = view.scrollDOM;
      const range = scroller.scrollHeight - scroller.clientHeight;
      if (range > 0) onScrollRef.current(scroller.scrollTop / range);
    };
    const handleClick = (event: MouseEvent) => {
      const position = view.posAtCoords({ x: event.clientX, y: event.clientY });
      if (position === null) return;
      const line = view.state.doc.lineAt(position);
      const pattern = /\\(?:ref|cref|autoref)\s*\{([^{}]+)\}/g;
      for (const match of line.text.matchAll(pattern)) {
        const from = line.from + (match.index ?? -1);
        const to = from + match[0].length;
        if (from <= position && position <= to && match[1]) {
          onNavigateRef.current(match[1].trim());
          return;
        }
      }
    };
    const handleKeydown = (event: KeyboardEvent) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      const selection = view.state.selection.main;
      const line = view.state.doc.lineAt(selection.head);
      const pattern = /\\(?:ref|cref|autoref)\s*\{([^{}]+)\}/g;
      for (const match of line.text.matchAll(pattern)) {
        const from = line.from + (match.index ?? -1);
        const to = from + match[0].length;
        if (from <= selection.head && selection.head <= to && match[1]) {
          event.preventDefault();
          onNavigateRef.current(match[1].trim());
          return;
        }
      }
    };
    view.scrollDOM.addEventListener("scroll", handleScroll, { passive: true });
    view.dom.addEventListener("click", handleClick);
    view.dom.addEventListener("keydown", handleKeydown);

    return () => {
      view.scrollDOM.removeEventListener("scroll", handleScroll);
      view.dom.removeEventListener("click", handleClick);
      view.dom.removeEventListener("keydown", handleKeydown);
      scrollEchoRef.current?.dispose();
      view.destroy();
      viewRef.current = null;
    };
    // CodeMirror is created once. External values are applied in the next effect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    if (view.state.doc.toString() !== value) {
      suppressChangeRef.current = true;
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
      suppressChangeRef.current = false;
    }
  }, [value]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({ effects: setTokens.of(tokens) });
  }, [tokens]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view || !flash) return;
    view.dispatch({ effects: setFlash.of(flash) });
    const timeout = window.setTimeout(() => {
      if (viewRef.current === view) view.dispatch({ effects: setFlash.of(null) });
    }, 230);
    return () => window.clearTimeout(timeout);
  }, [flash]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view || scrollRatio === null || (appliedScrollRef.current !== null && Math.abs(scrollRatio - appliedScrollRef.current) < 1e-4)) return;
    const scroller = view.scrollDOM;
    const target = Math.max(0, scroller.scrollHeight - scroller.clientHeight) * scrollRatio;
    if (target !== scroller.scrollTop) {
      scrollEchoRef.current?.begin();
      scroller.scrollTop = target;
    }
    appliedScrollRef.current = scrollRatio;
  }, [scrollRatio]);

  return (
    <section className="editor-pane latex-editor" aria-label="LaTeX source editor">
      {warnings.length > 0 && (
        <ul className="editor-warnings" aria-label="LaTeX sync warnings">
          {warnings.map((warning, index) => (
            <li key={`${warning.affected_path.join(".")}-${index}`} title={warning.message}>
              <span aria-hidden="true">!</span> {warning.affected_path.length === 0 ? "Document" : `Block ${warning.affected_path[0] + 1}`}: {warning.message}
            </li>
          ))}
        </ul>
      )}
      <div className="latex-editor-mount" ref={mountRef} data-testid="latex-editor" />
    </section>
  );
});
