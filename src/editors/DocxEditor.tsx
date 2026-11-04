// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { forwardRef, useEffect, useImperativeHandle, useMemo, useRef } from "react";
import { Schema, type Node as ProseMirrorNode } from "prosemirror-model";
import { EditorState, Plugin, PluginKey } from "prosemirror-state";
import { baseKeymap, toggleMark } from "prosemirror-commands";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { Decoration, DecorationSet, EditorView } from "prosemirror-view";
import "prosemirror-view/style/prosemirror.css";
import type { Block, Document, Inline, SyncWarning } from "../ipc/commands";

export interface DocxEditorHandle {
  scrollToBlock(index: number): void;
  scrollToRatio(ratio: number): void;
}

interface DocxEditorProps {
  document: Document;
  warnings: SyncWarning[];
  flashToken: number;
  flashIndices: number[];
  hasUnsubmittedChanges: boolean;
  onChange: (blocks: Block[]) => void;
  onScroll: (ratio: number) => void;
  scrollRatio: number | null;
  onNavigate: (index: number) => void;
}

type Payload = Record<string, unknown>;

const remoteUpdate = new PluginKey("tessera-remote-update");
const decorationsKey = new PluginKey<DecorationSet>("tessera-decorations");

const warningWidget = (position: number, warning: SyncWarning): Decoration =>
  Decoration.widget(
    position,
    () => {
      const marker = document.createElement("button");
      marker.type = "button";
      marker.className = "docx-warning-marker";
      marker.setAttribute("aria-label", "Show sync warning for this block");
      marker.textContent = "!";
      marker.title = warning.message;
      marker.addEventListener("click", () => {
        marker.classList.toggle("expanded");
        marker.textContent = marker.classList.contains("expanded") ? warning.message : "!";
      });
      return marker;
    },
    { side: -1, key: `warning-${warning.affected_path.join(".")}-${warning.message}` }
  );

function makeDecorations(doc: ProseMirrorNode, warnings: SyncWarning[], flashIndices: number[], token: number): DecorationSet {
  const next: Decoration[] = [];
  let position = 0;
  doc.forEach((node, _offset, index) => {
    for (const warning of warnings.filter((item) => item.affected_path[0] === index)) {
      next.push(warningWidget(position, warning));
    }
    if (flashIndices.includes(index)) {
      next.push(Decoration.node(position, position + node.nodeSize, {
        class: `docx-sync-flash docx-sync-flash-${blockSemanticKind(node)}`,
        "data-flash-token": String(token),
      }, { flash: true }));
    }
    position += node.nodeSize;
  });
  for (const warning of warnings.filter((item) => item.affected_path.length === 0)) {
    next.push(warningWidget(0, warning));
  }
  return DecorationSet.create(doc, next);
}

const decorationsPlugin = new Plugin<DecorationSet>({
  key: decorationsKey,
  state: {
    init: () => DecorationSet.empty,
    apply(transaction, previous) {
      const update = transaction.getMeta(decorationsKey) as
        | { warnings: SyncWarning[]; flashIndices: number[]; token: number }
        | undefined;
      if (update) return makeDecorations(transaction.doc, update.warnings, update.flashIndices, update.token);
      if (transaction.getMeta("tessera-clear-flash")) {
        const retained = previous.find().filter((decoration) => !decoration.spec.flash);
        return DecorationSet.create(transaction.doc, retained);
      }
      return previous.map(transaction.mapping, transaction.doc);
    },
  },
  props: {
    decorations(state) {
      return decorationsKey.getState(state) ?? DecorationSet.empty;
    },
  },
});

export const docxSchema = new Schema({
  nodes: {
    doc: { content: "block*" },
    text: { group: "inline" },
    paragraph: {
      group: "block",
      content: "inline*",
      attrs: { payload: { default: { type: "Paragraph", inline: [], trivia: { leading: "" } } } },
      parseDOM: [{ tag: "p" }],
      toDOM: () => ["p", { class: "docx-paragraph" }, 0],
    },
    heading: {
      group: "block",
      content: "inline*",
      attrs: { payload: { default: { type: "Heading", level: "H1", inline: [], label: null, trivia: { leading: "" } } } },
      parseDOM: Array.from({ length: 6 }, (_unused, index) => ({
        tag: `h${index + 1}`,
        getAttrs: () => ({ payload: { type: "Heading", level: `H${index + 1}`, inline: [], label: null, trivia: { leading: "" } } }),
      })),
      toDOM: (node) => [`h${headingNumber(node.attrs.payload)}`, { class: "docx-heading", "data-label": (node.attrs.payload as Payload).label ?? "" }, 0],
    },
    bullet_list: {
      group: "block",
      content: "list_item*",
      attrs: { payload: { default: { type: "List", kind: "Unordered", items: [], trivia: { leading: "" } } } },
      parseDOM: [{ tag: "ul" }],
      toDOM: () => ["ul", { class: "docx-list" }, 0],
    },
    ordered_list: {
      group: "block",
      content: "list_item*",
      attrs: { payload: { default: { type: "List", kind: "Ordered", items: [], trivia: { leading: "" } } } },
      parseDOM: [{ tag: "ol" }],
      toDOM: () => ["ol", { class: "docx-list" }, 0],
    },
    list_item: { content: "block*", parseDOM: [{ tag: "li" }], toDOM: () => ["li", 0] },
    blockquote: {
      group: "block",
      content: "block*",
      attrs: { payload: { default: { type: "Quote", blocks: [], trivia: { leading: "" } } } },
      parseDOM: [{ tag: "blockquote" }],
      toDOM: () => ["blockquote", { class: "docx-quote" }, 0],
    },
    code_block: {
      group: "block",
      content: "text*",
      marks: "",
      code: true,
      attrs: { payload: { default: { type: "CodeBlock", language: null, text: "", trivia: { leading: "" } } } },
      parseDOM: [{ tag: "pre", preserveWhitespace: "full" }],
      toDOM: () => ["pre", { class: "docx-code-block" }, ["code", 0]],
    },
    table: {
      group: "block",
      content: "table_row*",
      attrs: { payload: { default: { type: "Table", rows: [], caption: null, label: null, leading_trivia: "", latex_style: null } } },
      parseDOM: [{ tag: "table" }],
      toDOM: () => ["table", { class: "docx-table" }, ["tbody", 0]],
    },
    table_row: { content: "table_cell*", parseDOM: [{ tag: "tr" }], toDOM: () => ["tr", 0] },
    table_cell: {
      content: "block*",
      attrs: { col_span: { default: 1 }, row_span: { default: 1 } },
      parseDOM: [{ tag: "td" }, { tag: "th" }],
      toDOM: (node) => ["td", { colspan: node.attrs.col_span, rowspan: node.attrs.row_span }, 0],
    },
    figure: {
      group: "block",
      atom: true,
      selectable: false,
      attrs: { payload: { default: { type: "Figure", asset_path: "", caption: null, label: null, width_percent: null, leading_trivia: "" } } },
      toDOM: (node) => ["figure", { class: "docx-figure", "data-label": (node.attrs.payload as Payload).label ?? "" }, `Figure: ${(node.attrs.payload as Payload).asset_path}`],
    },
    equation: {
      group: "block",
      atom: true,
      selectable: false,
      attrs: { payload: { default: { type: "Equation", latex_source: "", display_mode: true, label: null, leading_trivia: "" } } },
      toDOM: (node) => ["div", { class: "docx-equation", "data-label": (node.attrs.payload as Payload).label ?? "" }, `$$${(node.attrs.payload as Payload).latex_source}$$`],
    },
    opaque: {
      group: "block",
      atom: true,
      selectable: false,
      attrs: { payload: { default: { type: "Opaque", origin: "Latex", raw: "", summary: "Unsupported content" } } },
      toDOM: (node) => {
        const payload = node.attrs.payload as Payload;
        return ["aside", { class: "opaque-block", contenteditable: "false", title: String(payload.summary) }, `${String(payload.summary)} (${String(payload.origin)} content)`];
      },
    },
    hard_break: { inline: true, group: "inline", selectable: false, parseDOM: [{ tag: "br" }], toDOM: () => ["br"] },
    citation: {
      inline: true,
      group: "inline",
      atom: true,
      attrs: { keys: { default: [] } },
      toDOM: (node) => ["span", { class: "docx-citation", contenteditable: "false" }, `[${(node.attrs.keys as string[]).join(", ")}]`],
    },
    cross_ref: {
      inline: true,
      group: "inline",
      atom: true,
      attrs: { label: { default: "" } },
      toDOM: (node) => ["span", { class: "docx-cross-reference", "data-label": node.attrs.label, role: "link", tabindex: "0" }, `\u00a0${String(node.attrs.label)}\u00a0`],
    },
    inline_math: {
      inline: true,
      group: "inline",
      atom: true,
      attrs: { latex_source: { default: "" } },
      toDOM: (node) => ["span", { class: "docx-inline-math", contenteditable: "false" }, `$${String(node.attrs.latex_source)}$`],
    },
    footnote: { inline: true, group: "inline", content: "inline*", toDOM: () => ["sup", { class: "docx-footnote" }, 0] },
  },
  marks: {
    bold: { parseDOM: [{ tag: "strong" }, { tag: "b" }], toDOM: () => ["strong", { class: "docx-bold" }, 0] },
    italic: { parseDOM: [{ tag: "em" }, { tag: "i" }], toDOM: () => ["em", { class: "docx-italic" }, 0] },
    underline: { parseDOM: [{ tag: "u" }], toDOM: () => ["u", { class: "docx-underline" }, 0] },
    code: { parseDOM: [{ tag: "code" }], toDOM: () => ["code", { class: "docx-inline-code" }, 0] },
  },
});

function headingNumber(payload: Payload): number {
  return Math.min(6, Math.max(1, Number(String(payload.level ?? "H1").slice(1)) || 1));
}

function payloadFrom(block: Block): Payload {
  return { ...block } as unknown as Payload;
}

function inlineToNodes(inline: Inline): ProseMirrorNode[] {
  switch (inline.type) {
    case "Text":
      return [docxSchema.text(inline.text)];
    case "Bold":
    case "Italic":
    case "Underline":
    case "Footnote": {
      const content = inline.inline.flatMap(inlineToNodes);
      const type = inline.type === "Footnote" ? "footnote" : inline.type.toLowerCase();
      const node = inline.type === "Footnote" ? docxSchema.nodes.footnote.create(null, content) : null;
      return node ? [node] : content.map((child) => child.mark([...child.marks, docxSchema.marks[type].create()]));
    }
    case "Code":
      return [docxSchema.text(inline.text, [docxSchema.marks.code.create()])];
    case "Citation":
      return [docxSchema.nodes.citation.create({ keys: inline.keys })];
    case "CrossRef":
      return [docxSchema.nodes.cross_ref.create({ label: inline.label })];
    case "InlineMath":
      return [docxSchema.nodes.inline_math.create({ latex_source: inline.latex_source })];
    case "LineBreak":
      return [docxSchema.nodes.hard_break.create()];
  }
}

export function blockToNode(block: Block): ProseMirrorNode {
  const payload = payloadFrom(block);
  switch (block.type) {
    case "Paragraph":
      return docxSchema.nodes.paragraph.create({ payload }, block.inline.flatMap(inlineToNodes));
    case "Heading":
      return docxSchema.nodes.heading.create({ payload }, block.inline.flatMap(inlineToNodes));
    case "List": {
      const items = block.items.map((item) => docxSchema.nodes.list_item.create(null, item.map(blockToNode)));
      return docxSchema.nodes[block.kind === "Ordered" ? "ordered_list" : "bullet_list"].create({ payload }, items);
    }
    case "Quote":
      return docxSchema.nodes.blockquote.create({ payload }, block.blocks.map(blockToNode));
    case "CodeBlock":
      return docxSchema.nodes.code_block.create({ payload }, block.text ? docxSchema.text(block.text) : null);
    case "Table":
      return docxSchema.nodes.table.create(
        { payload },
        block.rows.map((row) =>
          docxSchema.nodes.table_row.create(null, row.map((cell) =>
            docxSchema.nodes.table_cell.create(
              { col_span: cell.col_span, row_span: cell.row_span },
              cell.blocks.map(blockToNode)
            )
          ))
        )
      );
    case "Figure":
      return docxSchema.nodes.figure.create({ payload });
    case "Equation":
      return docxSchema.nodes.equation.create({ payload });
    case "Opaque":
      return docxSchema.nodes.opaque.create({ payload });
  }
}

function nodeToInlines(node: ProseMirrorNode): Inline[] {
  return node.content.content.flatMap((child): Inline[] => {
    if (child.type.name === "text") {
      const text = child.text ?? "";
      let inline: Inline = child.marks.some((mark) => mark.type.name === "code")
        ? { type: "Code", text }
        : { type: "Text", text };
      const variants: Record<string, "Bold" | "Italic" | "Underline"> = {
        bold: "Bold", italic: "Italic", underline: "Underline",
      };
      for (const mark of child.marks) {
        const variant = variants[mark.type.name];
        if (variant) inline = { type: variant, inline: [inline] };
      }
      return [inline];
    }
    switch (child.type.name) {
      case "hard_break": return [{ type: "LineBreak" }];
      case "citation": return [{ type: "Citation", keys: child.attrs.keys as string[] }];
      case "cross_ref": return [{ type: "CrossRef", label: String(child.attrs.label) }];
      case "inline_math": return [{ type: "InlineMath", latex_source: String(child.attrs.latex_source) }];
      case "footnote": return [{ type: "Footnote", inline: nodeToInlines(child) }];
      default: return [];
    }
  });
}

export function nodeToBlock(node: ProseMirrorNode): Block {
  const payload = node.attrs.payload as Payload;
  switch (node.type.name) {
    case "paragraph": return { ...payload, inline: nodeToInlines(node) } as unknown as Block;
    case "heading": return { ...payload, inline: nodeToInlines(node) } as unknown as Block;
    case "bullet_list":
    case "ordered_list":
      return {
        ...payload,
        type: "List",
        kind: node.type.name === "ordered_list" ? "Ordered" : "Unordered",
        items: node.content.content.map((item) => item.content.content.map(nodeToBlock)),
      } as unknown as Block;
    case "blockquote": return { ...payload, blocks: node.content.content.map(nodeToBlock) } as unknown as Block;
    case "code_block": return { ...payload, text: node.textContent } as unknown as Block;
    case "table":
      return {
        ...payload,
        rows: node.content.content.map((row) => row.content.content.map((cell) => ({
          blocks: cell.content.content.map(nodeToBlock),
          col_span: cell.attrs.col_span as number,
          row_span: cell.attrs.row_span as number,
        }))),
      } as unknown as Block;
    case "figure":
    case "equation":
    case "opaque": return payload as unknown as Block;
    default: throw new Error(`Unsupported ProseMirror block ${node.type.name}`);
  }
}

function blockSemanticKind(node: ProseMirrorNode): string {
  if (node.type.name === "equation") return "math";
  if (node.type.name === "opaque") return "opaque";
  return "text";
}

function labelsInBlock(block: Block, index: number, label: string): number | null {
  if (
    (block.type === "Heading" || block.type === "Table" || block.type === "Figure" || block.type === "Equation") &&
    block.label === label
  ) return index;
  const nested = block.type === "Quote" ? block.blocks : block.type === "List" ? block.items.flat() : [];
  for (const child of nested) {
    const found = labelsInBlock(child, index, label);
    if (found !== null) return found;
  }
  if (block.type === "Table") {
    for (const cell of block.rows.flat()) {
      for (const child of cell.blocks) {
        const found = labelsInBlock(child, index, label);
        if (found !== null) return found;
      }
    }
  }
  return null;
}

function schemaPlugins(getDocument: () => Document, onNavigate: (index: number) => void): Plugin[] {
  return [
    new Plugin({
      key: remoteUpdate,
      filterTransaction(transaction, state) {
        if (transaction.getMeta(remoteUpdate)) return true;
        if (!transaction.docChanged) return true;
        const protectedNodes: string[] = [];
        state.doc.descendants((node) => {
          if (node.type.name === "opaque") protectedNodes.push(JSON.stringify(node.toJSON()));
        });
        const nextProtectedNodes: string[] = [];
        transaction.doc.descendants((node) => {
          if (node.type.name === "opaque") nextProtectedNodes.push(JSON.stringify(node.toJSON()));
        });
        return JSON.stringify(protectedNodes.sort()) === JSON.stringify(nextProtectedNodes.sort());
      },
      props: {
        handleClick(_view, _pos, event) {
          const target = event.target;
          if (!(target instanceof Element)) return false;
          const reference = target.closest<HTMLElement>(".docx-cross-reference");
          if (!reference) return false;
          const label = reference.dataset.label;
          if (!label) return false;
          const index = getDocument().body.findIndex((block, bodyIndex) => labelsInBlock(block, bodyIndex, label) !== null);
          if (index >= 0) onNavigate(index);
          return index >= 0;
        },
        handleDOMEvents: {
          keydown(_view, event) {
            if (event.key !== "Enter" && event.key !== " ") return false;
            const target = event.target;
            if (!(target instanceof Element)) return false;
            const reference = target.closest<HTMLElement>(".docx-cross-reference");
            const label = reference?.dataset.label;
            if (!label) return false;
            const index = getDocument().body.findIndex((block, bodyIndex) => labelsInBlock(block, bodyIndex, label) !== null);
            if (index < 0) return false;
            event.preventDefault();
            onNavigate(index);
            return true;
          },
        },
      },
    }),
  ];
}

export const DocxEditor = forwardRef<DocxEditorHandle, DocxEditorProps>(function DocxEditor(
  { document, warnings, flashToken, flashIndices, hasUnsubmittedChanges, onChange, onScroll, scrollRatio, onNavigate },
  forwardedRef
): JSX.Element {
  const mountRef = useRef<HTMLDivElement>(null);
  const paneRef = useRef<HTMLElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const callbacksRef = useRef({ onChange, onScroll, onNavigate });
  const applyingRemoteRef = useRef(false);
  const appliedScrollRef = useRef<number | null>(null);
  const documentRef = useRef(document);
  callbacksRef.current = { onChange, onScroll, onNavigate };
  documentRef.current = document;

  const currentDoc = useMemo(
    () => docxSchema.node("doc", null, document.body.map(blockToNode)),
    [document]
  );

  useImperativeHandle(forwardedRef, () => ({
    scrollToBlock(index) {
      const view = viewRef.current;
      if (!view || index < 0 || index >= view.state.doc.childCount) return;
      let position = 0;
      for (let current = 0; current < index && current < view.state.doc.childCount; current += 1) {
        position += view.state.doc.child(current).nodeSize;
      }
      const dom = view.nodeDOM(position);
      if (dom instanceof HTMLElement) {
        dom.scrollIntoView({ behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth", block: "center" });
        dom.classList.remove("docx-crossref-target");
        requestAnimationFrame(() => dom.classList.add("docx-crossref-target"));
        window.setTimeout(() => dom.classList.remove("docx-crossref-target"), 260);
      }
    },
    scrollToRatio(ratio) {
      const element = paneRef.current;
      if (!element) return;
      element.scrollTop = Math.max(0, element.scrollHeight - element.clientHeight) * ratio;
      appliedScrollRef.current = ratio;
    },
  }), []);

  useEffect(() => {
    const mount = mountRef.current;
    if (!mount) return;
    const state = EditorState.create({ doc: currentDoc, plugins: [
      decorationsPlugin,
      keymap({
        ...baseKeymap,
        "Mod-z": undo,
        "Mod-y": redo,
        "Mod-Shift-z": redo,
        "Mod-b": toggleMark(docxSchema.marks.bold),
        "Mod-i": toggleMark(docxSchema.marks.italic),
        "Mod-u": toggleMark(docxSchema.marks.underline),
      }),
      history(),
      ...schemaPlugins(() => documentRef.current, (index) => callbacksRef.current.onNavigate(index)),
    ] });
    const view = new EditorView(mount, {
      state,
      dispatchTransaction(transaction) {
        const previousState = view.state;
        const next = previousState.apply(transaction);
        const changed = next !== previousState;
        view.updateState(next);
        if (changed && transaction.docChanged && !transaction.getMeta(remoteUpdate) && !applyingRemoteRef.current) {
          callbacksRef.current.onChange(next.doc.content.content.map(nodeToBlock));
        }
      },
      attributes: { class: "docx-prosemirror" },
    });
    viewRef.current = view;
    const handleScroll = () => {
      const element = paneRef.current;
      if (!element) return;
      const range = element.scrollHeight - element.clientHeight;
      if (range > 0) callbacksRef.current.onScroll(element.scrollTop / range);
    };
    paneRef.current?.addEventListener("scroll", handleScroll, { passive: true });
    return () => {
      paneRef.current?.removeEventListener("scroll", handleScroll);
      view.destroy();
      viewRef.current = null;
    };
    // The ProseMirror instance is created once. External state is applied below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    if (!hasUnsubmittedChanges && !view.state.doc.eq(currentDoc)) {
      applyingRemoteRef.current = true;
      const transaction = view.state.tr.replaceWith(0, view.state.doc.content.size, currentDoc.content.content)
        .setMeta(remoteUpdate, true)
        .setMeta("addToHistory", false);
      view.dispatch(transaction);
      applyingRemoteRef.current = false;
    }
  }, [currentDoc, hasUnsubmittedChanges]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch(view.state.tr.setMeta(decorationsKey, { warnings, flashIndices, token: flashToken }));
    if (flashIndices.length === 0 || flashToken === 0) return;
    const timeout = window.setTimeout(() => {
      if (viewRef.current === view) view.dispatch(view.state.tr.setMeta("tessera-clear-flash", true));
    }, 230);
    return () => window.clearTimeout(timeout);
  }, [warnings, flashToken, flashIndices]);

  useEffect(() => {
    const element = paneRef.current;
    if (!element || scrollRatio === null || (appliedScrollRef.current !== null && Math.abs(scrollRatio - appliedScrollRef.current) < 1e-4)) return;
    element.scrollTop = Math.max(0, element.scrollHeight - element.clientHeight) * scrollRatio;
    appliedScrollRef.current = scrollRatio;
  }, [scrollRatio]);

  return (
    <section ref={paneRef} className="editor-pane docx-editor" aria-label="DOCX document editor">
      <div ref={mountRef} data-testid="docx-editor" />
    </section>
  );
});

export function diffBlocks(previous: Block[], next: Block[]): { op: "Replace" | "Insert" | "Remove"; path: number[]; index?: number; block?: Block }[] {
  const previousJson = previous.map((block) => JSON.stringify(block));
  const nextJson = next.map((block) => JSON.stringify(block));
  let prefix = 0;
  while (prefix < previous.length && prefix < next.length && previousJson[prefix] === nextJson[prefix]) prefix += 1;
  let suffix = 0;
  while (
    suffix < previous.length - prefix && suffix < next.length - prefix &&
    previousJson[previous.length - suffix - 1] === nextJson[next.length - suffix - 1]
  ) suffix += 1;
  const oldEnd = previous.length - suffix;
  const newEnd = next.length - suffix;
  const oldMiddle = previousJson.slice(prefix, oldEnd);
  const newMiddle = nextJson.slice(prefix, newEnd);
  const rows = oldMiddle.length + 1;
  const cols = newMiddle.length + 1;
  const lcs = Array.from({ length: rows }, () => Array<number>(cols).fill(0));
  for (let i = oldMiddle.length - 1; i >= 0; i -= 1) {
    for (let j = newMiddle.length - 1; j >= 0; j -= 1) {
      lcs[i][j] = oldMiddle[i] === newMiddle[j]
        ? 1 + lcs[i + 1][j + 1]
        : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
    }
  }

  const operations: { op: "Replace" | "Insert" | "Remove"; path: number[]; index?: number; block?: Block }[] = [];
  let oldIndex = 0;
  let newIndex = 0;
  let currentIndex = prefix;
  while (oldIndex < oldMiddle.length || newIndex < newMiddle.length) {
    if (oldIndex < oldMiddle.length && newIndex < newMiddle.length && oldMiddle[oldIndex] === newMiddle[newIndex]) {
      oldIndex += 1;
      newIndex += 1;
      currentIndex += 1;
      continue;
    }
    const oldStart = oldIndex;
    const newStart = newIndex;
    while (oldIndex < oldMiddle.length || newIndex < newMiddle.length) {
      if (oldIndex < oldMiddle.length && newIndex < newMiddle.length && oldMiddle[oldIndex] === newMiddle[newIndex]) break;
      if (newIndex < newMiddle.length && (oldIndex === oldMiddle.length || lcs[oldIndex][newIndex + 1] >= lcs[oldIndex + 1][newIndex])) newIndex += 1;
      else oldIndex += 1;
    }
    const oldCount = oldIndex - oldStart;
    const newCount = newIndex - newStart;
    const paired = Math.min(oldCount, newCount);
    for (let offset = 0; offset < paired; offset += 1) {
      operations.push({ op: "Replace", path: [currentIndex + offset], block: next[prefix + newStart + offset] });
    }
    for (let offset = oldCount - 1; offset >= paired; offset -= 1) {
      operations.push({ op: "Remove", path: [], index: currentIndex + offset });
    }
    for (let offset = paired; offset < newCount; offset += 1) {
      operations.push({ op: "Insert", path: [], index: currentIndex + offset, block: next[prefix + newStart + offset] });
    }
    currentIndex += newCount;
  }
  return operations;
}

