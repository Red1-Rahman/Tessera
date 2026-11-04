// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { expect, test } from "@playwright/test";

interface SyncTestState {
  parseCalls: number;
  deferNextParse: boolean;
  releaseParse: (() => void) | null;
}

type TestWindow = Window & { __TESSERA_TEST__?: SyncTestState };

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    let version = 1;
    let body = [{ type: "Paragraph", inline: [{ type: "Text", text: "Hello from DOCX" }], trivia: { leading: "" } }];
    const testState: SyncTestState = { parseCalls: 0, deferNextParse: false, releaseParse: null };
    (window as TestWindow).__TESSERA_TEST__ = testState;
    const documentModel = () => ({
      version,
      metadata: { title: null, authors: [], date: null, latex_class: "article", latex_class_options: [], preamble_raw: [] },
      body,
      trailing_trivia: "",
    });
    const renderLatex = () => body.map((block) => block.inline?.map((inline) => inline.text ?? inline.inline?.map((part) => part.text ?? "").join("")).join("") ?? "").join("\n");

    (window as Window & { __TAURI_INTERNALS__?: { invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown> } }).__TAURI_INTERNALS__ = {
      invoke: async (command, args = {}) => {
        if (command === "export") return { kind: "ClipboardPayload", format: "Latex", content: renderLatex() };
        if (command === "get_highlight_tokens") return [];
        if (command === "parse_latex") {
          const request = args.request as { base_version: number; full_source: string };
          testState.parseCalls += 1;
          const applyParse = () => {
            body = [{ type: "Paragraph", inline: [{ type: "Text", text: request.full_source }], trivia: { leading: "" } }];
            version = Math.max(version, request.base_version) + 1;
            return { new_version: version, document: documentModel(), latex_render: request.full_source, warnings: [] };
          };
          if (testState.deferNextParse) {
            testState.deferNextParse = false;
            return new Promise((resolve) => { testState.releaseParse = () => resolve(applyParse()); });
          }
          return applyParse();
        }
        if (command === "apply_docx_edit") {
          const request = args.request as { base_version: number; patches: { op: string; index?: number; path?: number[]; block?: typeof body[number] }[] };
          for (const patch of request.patches) {
            if (patch.op === "Replace" && patch.block) body[patch.path?.[0] ?? 0] = patch.block;
            else if (patch.op === "Insert" && patch.block) body.splice(patch.index ?? body.length, 0, patch.block);
            else if (patch.op === "Remove") body.splice(patch.index ?? 0, 1);
          }
          version = Math.max(version, request.base_version) + 1;
          return { new_version: version, document: documentModel(), latex_render: renderLatex(), warnings: [] };
        }
        throw new Error(`Unexpected Tauri command: ${command}`);
      },
    };
  });
  await page.goto("/");
  await page.evaluate(async () => {
    const store = await import("/src/state/documentStore.ts");
    store.loadProject({
      path: "/private/test",
      document: {
        version: 1,
        metadata: { title: null, authors: [], date: null, latex_class: "article", latex_class_options: [], preamble_raw: [] },
        body: [{ type: "Paragraph", inline: [{ type: "Text", text: "Hello from DOCX" }], trivia: { leading: "" } }],
        trailing_trivia: "",
      },
      last_edited_pane: "Latex",
      assets: [],
      warnings: [],
    });
  });
});

test("LaTeX and DOCX edits round-trip through the document model", async ({ page }) => {
  const latex = page.locator(".cm-content");
  await expect(latex).toContainText("Hello from DOCX");
  await latex.fill("Changed from LaTeX");
  await expect(page.locator(".docx-prosemirror")).toContainText("Changed from LaTeX", { timeout: 10_000 });

  await page.locator(".docx-prosemirror p").fill("Changed from DOCX");
  await expect(page.locator(".cm-content")).toContainText("Changed from DOCX", { timeout: 10_000 });
});

test("keeps newer LaTeX typing when an earlier parse returns", async ({ page }) => {
  await page.evaluate(() => { (window as TestWindow).__TESSERA_TEST__!.deferNextParse = true; });
  const latex = page.locator(".cm-content");
  await latex.fill("First snapshot");
  await page.waitForFunction(() => (window as TestWindow).__TESSERA_TEST__!.parseCalls === 1);
  await latex.fill("Newer keystrokes");

  await page.evaluate(() => { (window as TestWindow).__TESSERA_TEST__!.releaseParse?.(); });
  await expect(page.locator(".docx-prosemirror")).toContainText("First snapshot", { timeout: 10_000 });
  await expect(latex).toContainText("Newer keystrokes");
  await expect(page.locator(".docx-prosemirror")).toContainText("Newer keystrokes", { timeout: 10_000 });
});
