// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { toggleMark } from "prosemirror-commands";
import type { Node as ProseMirrorNode } from "prosemirror-model";
import { TextSelection, type Command, type EditorState, type Transaction } from "prosemirror-state";

// ProseMirror commands behind the DOCX pane's formatting toolbar. They
// only express what the Document Model can represent (inline marks,
// paragraph/heading/code blocks, lists, quotes), so nothing a toolbar
// button produces can be lost on sync. Each command takes the usual
// (state, dispatch?) pair and returns whether it applies, which lets the
// toolbar use the same function to decide whether a button is enabled.

type Payload = Record<string, unknown>;

export type MarkName = "bold" | "italic" | "underline" | "code";
export type ListName = "bullet_list" | "ordered_list";
export type ContainerName = ListName | "blockquote";

const HEADING_LEVELS = ["H1", "H2", "H3", "H4", "H5", "H6"] as const;
export type BlockKind = "paragraph" | (typeof HEADING_LEVELS)[number] | "code_block";

export const BLOCK_KINDS: { value: BlockKind; label: string }[] = [
  { value: "paragraph", label: "Paragraph" },
  ...HEADING_LEVELS.map((level) => ({ value: level, label: `Heading ${level.slice(1)}` })),
  { value: "code_block", label: "Code block" },
];

function payloadOf(node: ProseMirrorNode): Payload {
  return (node.attrs.payload ?? {}) as Payload;
}

/** Source text stored before a block (comments, blank lines). */
function triviaOf(node: ProseMirrorNode): string {
  const payload = payloadOf(node);
  const trivia = payload.trivia as { leading?: unknown } | undefined;
  if (trivia && typeof trivia.leading === "string") return trivia.leading;
  return typeof payload.leading_trivia === "string" ? payload.leading_trivia : "";
}

function withTrivia(node: ProseMirrorNode, leading: string): ProseMirrorNode | null {
  const payload = payloadOf(node);
  let next: Payload;
  if ("trivia" in payload) next = { ...payload, trivia: { leading } };
  else if ("leading_trivia" in payload) next = { ...payload, leading_trivia: leading };
  else return null;
  return node.type.create({ ...node.attrs, payload: next }, node.content, node.marks);
}

function kindOfNode(node: ProseMirrorNode): BlockKind | null {
  switch (node.type.name) {
    case "paragraph":
      return "paragraph";
    case "heading": {
      const level = String(payloadOf(node).level ?? "H1");
      return (HEADING_LEVELS as readonly string[]).includes(level) ? (level as BlockKind) : "H1";
    }
    case "code_block":
      return "code_block";
    default:
      return null;
  }
}

// ---------------------------------------------------------------------------
// Inline marks
// ---------------------------------------------------------------------------

export function toggleMarkCommand(name: MarkName): Command {
  return (state, dispatch) => toggleMark(state.schema.marks[name])(state, dispatch);
}

export function markActive(state: EditorState, name: MarkName): boolean {
  const type = state.schema.marks[name];
  const { from, to, empty, $from } = state.selection;
  if (empty) return Boolean(type.isInSet(state.storedMarks ?? $from.marks()));
  return state.doc.rangeHasMark(from, to, type);
}

// ---------------------------------------------------------------------------
// Block style (paragraph, headings, code block)
// ---------------------------------------------------------------------------

/** The block style at the cursor, or null when it is not one of the choices. */
export function currentBlockKind(state: EditorState): BlockKind | null {
  return kindOfNode(state.selection.$from.parent);
}

function attrsFor(kind: BlockKind, old: ProseMirrorNode): Record<string, unknown> {
  const oldKind = kindOfNode(old);
  if (oldKind === kind) return { ...old.attrs };
  const trivia = { leading: triviaOf(old) };
  if (kind === "paragraph") return { payload: { type: "Paragraph", inline: [], trivia } };
  if (kind === "code_block") {
    return { payload: { type: "CodeBlock", language: null, text: "", trivia } };
  }
  if (old.type.name === "heading") return { payload: { ...payloadOf(old), level: kind } };
  return { payload: { type: "Heading", level: kind, inline: [], label: null, trivia } };
}

/**
 * Converts every textblock in the selection to the given style. A heading
 * that carries a label is never converted to a non-heading, because the
 * label may be the target of a cross-reference and would be dropped.
 */
export function setBlockKind(kind: BlockKind): Command {
  return (state, dispatch) => {
    const { from, to } = state.selection;
    let needsChange = false;
    let blocked = false;
    state.doc.nodesBetween(from, to, (node) => {
      if (!node.isTextblock) return true;
      if (kindOfNode(node) !== kind) {
        needsChange = true;
        if (node.content.content.some((child) => !child.isText)) {
          blocked = true;
          return false;
        }
        const isHeading = node.type.name === "heading";
        if (isHeading && payloadOf(node).label != null && !kind.startsWith("H")) blocked = true;
      }
      return false;
    });
    if (!needsChange || blocked) return false;
    if (dispatch) {
      const { nodes } = state.schema;
      const type = kind === "paragraph" ? nodes.paragraph : kind === "code_block" ? nodes.code_block : nodes.heading;
      const tr = state.tr.setBlockType(from, to, type, (node) => attrsFor(kind, node));
      dispatch(tr.scrollIntoView());
    }
    return true;
  };
}

// ---------------------------------------------------------------------------
// Lists and quotes (top-level containers)
// ---------------------------------------------------------------------------

const WRAPPABLE = new Set(["paragraph", "heading"]);

/** The list or quote that contains the cursor, if it is a top-level block. */
export function activeContainer(state: EditorState): ContainerName | null {
  const { $from } = state.selection;
  if ($from.depth === 0) return null;
  const name = $from.node(1).type.name;
  return name === "bullet_list" || name === "ordered_list" || name === "blockquote" ? name : null;
}

interface Span {
  from: number;
  to: number;
  first: number;
  last: number;
}

function topLevelSpan(state: EditorState): Span | null {
  const { $from, $to } = state.selection;
  if ($from.depth === 0 || $to.depth === 0) return null;
  return { from: $from.before(1), to: $to.after(1), first: $from.index(0), last: $to.index(0) };
}

function spanBlocks(state: EditorState, span: Span): ProseMirrorNode[] {
  const blocks: ProseMirrorNode[] = [];
  for (let index = span.first; index <= span.last; index += 1) blocks.push(state.doc.child(index));
  return blocks;
}

/**
 * Only plain paragraphs and headings are wrapped. Opaque blocks, tables,
 * and figures must never be moved into a container, and comments stored
 * as leading trivia on later blocks would be dropped, so those cases are
 * refused rather than approximated.
 */
function canWrap(blocks: ProseMirrorNode[]): boolean {
  return blocks.every(
    (block, index) =>
      WRAPPABLE.has(block.type.name) && (index === 0 || triviaOf(block).trim() === ""),
  );
}

function replaceSpan(state: EditorState, span: Span, nodes: ProseMirrorNode[]): Transaction {
  const tr = state.tr.replaceWith(span.from, span.to, nodes);
  return tr.setSelection(TextSelection.near(tr.doc.resolve(Math.min(span.from + 1, tr.doc.content.size))));
}

function unwrapContainer(state: EditorState, span: Span, container: ProseMirrorNode, dispatch?: (tr: Transaction) => void): boolean {
  const inner: ProseMirrorNode[] = [];
  if (container.type.name === "blockquote") container.forEach((child) => inner.push(child));
  else container.forEach((item) => item.forEach((child) => inner.push(child)));
  if (inner.length === 0) return false;
  if (dispatch) {
    const mergedLeading = triviaOf(container) + triviaOf(inner[0]);
    const first = withTrivia(inner[0], mergedLeading);
    if (mergedLeading.trim() !== "" && first === null) return false;
    const blocks = inner.map((block, index) => (index === 0 ? first ?? block : block));
    dispatch(replaceSpan(state, span, blocks).scrollIntoView());
  }
  return true;
}

export function toggleList(kind: ListName): Command {
  return (state, dispatch) => {
    const span = topLevelSpan(state);
    if (!span) return false;
    const { nodes } = state.schema;
    const blocks = spanBlocks(state, span);
    const isList = (node: ProseMirrorNode) => node.type === nodes.bullet_list || node.type === nodes.ordered_list;

    if (blocks.some(isList)) {
      if (blocks.length !== 1) return false;
      const list = blocks[0];
      if (list.type.name === kind) return unwrapContainer(state, span, list, dispatch);
      if (dispatch) {
        const payload = { ...payloadOf(list), kind: kind === "ordered_list" ? "Ordered" : "Unordered" };
        dispatch(state.tr.setNodeMarkup(span.from, nodes[kind], { payload }).scrollIntoView());
      }
      return true;
    }

    if (!canWrap(blocks)) return false;
    if (dispatch) {
      const items = blocks.map((block) => nodes.list_item.create(null, withTrivia(block, "") ?? block));
      const payload = {
        type: "List",
        kind: kind === "ordered_list" ? "Ordered" : "Unordered",
        items: [],
        trivia: { leading: triviaOf(blocks[0]) },
      };
      dispatch(replaceSpan(state, span, [nodes[kind].create({ payload }, items)]).scrollIntoView());
    }
    return true;
  };
}

export const toggleQuote: Command = (state, dispatch) => {
  const span = topLevelSpan(state);
  if (!span) return false;
  const { nodes } = state.schema;
  const blocks = spanBlocks(state, span);
  if (blocks.some((block) => block.type === nodes.blockquote)) {
    return blocks.length === 1 ? unwrapContainer(state, span, blocks[0], dispatch) : false;
  }
  if (!canWrap(blocks)) return false;
  if (dispatch) {
    const inner = blocks.map((block) => withTrivia(block, "") ?? block);
    const payload = { type: "Quote", blocks: [], trivia: { leading: triviaOf(blocks[0]) } };
    dispatch(replaceSpan(state, span, [nodes.blockquote.create({ payload }, inner)]).scrollIntoView());
  }
  return true;
};

/** Extra shortcuts, merged into the editor keymap. Mark and undo keys stay in DocxEditor. */
export const formattingKeymap: Record<string, Command> = {
  "Mod-Alt-0": setBlockKind("paragraph"),
  "Mod-Alt-1": setBlockKind("H1"),
  "Mod-Alt-2": setBlockKind("H2"),
  "Mod-Alt-3": setBlockKind("H3"),
  "Mod-Alt-4": setBlockKind("H4"),
  "Mod-Alt-5": setBlockKind("H5"),
  "Mod-Alt-6": setBlockKind("H6"),
  "Mod-Shift-8": toggleList("bullet_list"),
  "Mod-Shift-7": toggleList("ordered_list"),
};