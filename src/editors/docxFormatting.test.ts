// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { EditorState, TextSelection, type Command } from "prosemirror-state";
import { describe, expect, it } from "vitest";
import type { Block } from "../ipc/commands";
import { blockToNode, docxSchema, nodeToBlock } from "./DocxEditor";
import { setBlockKind, toggleList, toggleMarkCommand, toggleQuote } from "./docxFormatting";

const paragraph = (text: string): Block => ({ type: "Paragraph", inline: [{ type: "Text", text }], trivia: { leading: "" } });

// "one" occupies positions 1..4 and "two" occupies 6..9 in a two-paragraph doc.
function run(body: Block[], from: number, command: Command, to: number = from) {
  const doc = docxSchema.node("doc", null, body.map(blockToNode));
  let state = EditorState.create({ doc, selection: TextSelection.create(doc, from, to) });
  const applies = command(state);
  const ran = command(state, (tr) => {
    state = state.apply(tr);
  });
  return { applies, ran, blocks: state.doc.content.content.map(nodeToBlock) };
}

describe("docx formatting commands", () => {
  it("turns a paragraph into a heading and keeps its text", () => {
    const { blocks } = run([paragraph("one")], 2, setBlockKind("H2"));
    expect(blocks).toEqual([
      { type: "Heading", level: "H2", inline: [{ type: "Text", text: "one" }], label: null, trivia: { leading: "" } },
    ]);
  });

  it("refuses to turn a labeled heading into a paragraph", () => {
    const heading: Block = { type: "Heading", level: "H1", inline: [{ type: "Text", text: "Intro" }], label: "sec:intro", trivia: { leading: "" } };
    expect(run([heading], 2, setBlockKind("paragraph")).applies).toBe(false);
    const { blocks } = run([heading], 2, setBlockKind("H3"));
    expect(blocks[0]).toMatchObject({ type: "Heading", level: "H3", label: "sec:intro" });
  });

  it("refuses to convert a paragraph containing a citation", () => {
    const citation: Block = {
      type: "Paragraph",
      inline: [{ type: "Citation", keys: ["key"] }],
      trivia: { leading: "" },
    };
    expect(run([citation], 1, setBlockKind("code_block")).applies).toBe(false);
  });

  it("wraps selected paragraphs in a list, then unwraps them", () => {
    const wrapped = run([paragraph("one"), paragraph("two")], 2, toggleList("bullet_list"), 7);
    expect(wrapped.blocks).toEqual([
      { type: "List", kind: "Unordered", items: [[paragraph("one")], [paragraph("two")]], trivia: { leading: "" } },
    ]);
    const unwrapped = run(wrapped.blocks, 4, toggleList("bullet_list"));
    expect(unwrapped.blocks).toEqual([paragraph("one"), paragraph("two")]);
  });

  it("merges container and first-block trivia when unwrapping", () => {
    const list: Block = {
      type: "List",
      kind: "Unordered",
      items: [[{ ...paragraph("one"), trivia: { leading: "% inner\n" } }]],
      trivia: { leading: "% container\n" },
    };
    const { blocks } = run([list], 4, toggleList("bullet_list"));
    expect(blocks).toEqual([{ ...paragraph("one"), trivia: { leading: "% container\n% inner\n" } }]);
  });

  it("switches a bulleted list to a numbered one", () => {
    const list: Block = { type: "List", kind: "Unordered", items: [[paragraph("one")]], trivia: { leading: "" } };
    expect(run([list], 4, toggleList("ordered_list")).blocks[0]).toMatchObject({ type: "List", kind: "Ordered" });
  });

  it("wraps a paragraph in a quote", () => {
    expect(run([paragraph("one")], 2, toggleQuote).blocks).toEqual([
      { type: "Quote", blocks: [paragraph("one")], trivia: { leading: "" } },
    ]);
  });

  it("never wraps opaque blocks", () => {
    const opaque: Block = { type: "Opaque", origin: "Latex", raw: "\\custom", summary: "Unsupported" };
    expect(run([paragraph("one"), opaque, paragraph("two")], 2, toggleQuote, 8).applies).toBe(false);
  });

  it("applies bold to the selected text", () => {
    expect(run([paragraph("one")], 1, toggleMarkCommand("bold"), 4).blocks[0]).toEqual({
      type: "Paragraph",
      inline: [{ type: "Bold", inline: [{ type: "Text", text: "one" }] }],
      trivia: { leading: "" },
    });
  });
});