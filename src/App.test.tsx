// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";

afterEach(cleanup);

describe("App", () => {
  it("renders the application title in the toolbar", () => {
    render(<App />);
    expect(screen.getByText("Tessera")).toBeTruthy();
  });

  it("prompts for a project before mounting the editors", () => {
    render(<App />);
    expect(screen.getByText("LaTeX")).toBeTruthy();
    expect(screen.getByText("DOCX")).toBeTruthy();
    expect(screen.getByText("Open a project to start editing.")).toBeTruthy();
  });
});
