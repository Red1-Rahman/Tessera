// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { redo, undo } from "prosemirror-history";
import type { Command, EditorState } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";
import {
  BLOCK_KINDS,
  activeContainer,
  currentBlockKind,
  markActive,
  setBlockKind,
  toggleList,
  toggleMarkCommand,
  toggleQuote,
  type BlockKind,
  type MarkName,
} from "./docxFormatting";
import "./DocxFormatToolbar.css";

interface DocxFormatToolbarProps {
  view: EditorView | null;
  state: EditorState | null;
}

interface ToolButton {
  id: string;
  label: string;
  title: string;
  command: Command;
  pressed?: (state: EditorState) => boolean;
  className?: string;
}

const markButton = (mark: MarkName, label: string, title: string, className: string): ToolButton => ({
  id: mark,
  label,
  title,
  command: toggleMarkCommand(mark),
  pressed: (state) => markActive(state, mark),
  className,
});

const historyGroup: ToolButton[] = [
  { id: "undo", label: "Undo", title: "Undo (Ctrl+Z)", command: undo },
  { id: "redo", label: "Redo", title: "Redo (Ctrl+Shift+Z)", command: redo },
];

const markGroup: ToolButton[] = [
  markButton("bold", "B", "Bold (Ctrl+B)", "docx-tool-bold"),
  markButton("italic", "I", "Italic (Ctrl+I)", "docx-tool-italic"),
  markButton("underline", "U", "Underline (Ctrl+U)", "docx-tool-underline"),
  markButton("code", "</>", "Inline code", "docx-tool-code"),
];

const blockGroup: ToolButton[] = [
  {
    id: "bullet_list",
    label: "Bullets",
    title: "Bulleted list (Ctrl+Shift+8)",
    command: toggleList("bullet_list"),
    pressed: (state) => activeContainer(state) === "bullet_list",
  },
  {
    id: "ordered_list",
    label: "Numbered",
    title: "Numbered list (Ctrl+Shift+7)",
    command: toggleList("ordered_list"),
    pressed: (state) => activeContainer(state) === "ordered_list",
  },
  {
    id: "blockquote",
    label: "Quote",
    title: "Block quote",
    command: toggleQuote,
    pressed: (state) => activeContainer(state) === "blockquote",
  },
];

/**
 * Word-style formatting bar for the DOCX pane. It offers only what the
 * Document Model can represent. Buttons run ProseMirror commands on the
 * editor view, so every change flows through the normal edit and sync path.
 */
export function DocxFormatToolbar({ view, state }: DocxFormatToolbarProps): JSX.Element {
  const run = (command: Command): void => {
    if (!view) return;
    command(view.state, (tr) => view.dispatch(tr));
    view.focus();
  };

  const renderButton = (button: ToolButton): JSX.Element => {
    const enabled = state !== null && button.command(state);
    const pressed = state !== null && button.pressed ? button.pressed(state) : undefined;
    return (
      <button
        key={button.id}
        type="button"
        className={`docx-tool-btn${button.className ? ` ${button.className}` : ""}${pressed ? " is-active" : ""}`}
        title={button.title}
        aria-label={button.title.replace(/ \(.*\)$/, "")}
        aria-pressed={pressed}
        disabled={!enabled}
        onMouseDown={(event) => event.preventDefault()}
        onClick={() => run(button.command)}
      >
        {button.label}
      </button>
    );
  };

  const kind: BlockKind = (state && currentBlockKind(state)) ?? "paragraph";

  return (
    <div className="docx-toolbar" role="toolbar" aria-label="Document formatting">
      <div className="docx-tool-group">{historyGroup.map(renderButton)}</div>
      <div className="docx-tool-group">
        <select
          className="docx-tool-select"
          aria-label="Block style"
          title="Block style (Ctrl+Alt+0 to 6)"
          value={kind}
          disabled={state === null}
          onChange={(event) => run(setBlockKind(event.target.value as BlockKind))}
        >
          {BLOCK_KINDS.map((option) => (
            <option
              key={option.value}
              value={option.value}
              disabled={state !== null && option.value !== kind && !setBlockKind(option.value)(state)}
            >
              {option.label}
            </option>
          ))}
        </select>
      </div>
      <div className="docx-tool-group">{markGroup.map(renderButton)}</div>
      <div className="docx-tool-group">{blockGroup.map(renderButton)}</div>
    </div>
  );
}