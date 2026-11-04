// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SplitPane } from "./SplitPane";

afterEach(cleanup);

describe("SplitPane", () => {
  it("renders left and right children", () => {
    render(
      <SplitPane
        left={<div data-testid="left-child">Left</div>}
        right={<div data-testid="right-child">Right</div>}
      />
    );
    expect(screen.getByTestId("left-child")).toBeTruthy();
    expect(screen.getByTestId("right-child")).toBeTruthy();
  });

  it("renders a draggable divider", () => {
    render(<SplitPane left={<div />} right={<div />} />);
    const divider = screen.getByRole("separator");
    expect(divider).toBeTruthy();
  });

  it("updates flex-basis on divider drag", () => {
    render(<SplitPane left={<div />} right={<div />} />);
    const divider = screen.getByRole("separator");

    // Mock container rect
    const container = divider.parentElement as HTMLElement;
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
      left: 0,
      width: 1000,
      right: 1000,
      top: 0,
      bottom: 1000,
      height: 1000,
      x: 0,
      y: 0,
      toJSON: () => {},
    });

    fireEvent.pointerDown(divider);
    const pointerMove = new Event("pointermove", { bubbles: true });
    Object.defineProperty(pointerMove, "clientX", { value: 700 });
    fireEvent(divider, pointerMove);

    const leftPane = container.firstChild as HTMLElement;
    const splitMatch = leftPane.style.flexBasis.match(/calc\(([\d.]+)%/);
    expect(splitMatch).not.toBeNull();
    expect(Number(splitMatch?.[1])).toBeGreaterThan(60);

    fireEvent.pointerUp(window);
  });
});