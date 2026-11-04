// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import React, { useState, useRef, useCallback, useEffect } from "react";

interface SplitPaneProps {
  left: React.ReactNode;
  right: React.ReactNode;
}

const DIVIDER_WIDTH_PX = 10;
const PANE_MIN_WIDTH_PX = 320;

export function SplitPane({ left, right }: SplitPaneProps): JSX.Element {
  const [splitPercent, setSplitPercent] = useState(50);
  const containerRef = useRef<HTMLDivElement>(null);
  const dividerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);
  const previousCursor = useRef("");
  const previousUserSelect = useRef("");

  const setSplitFromPosition = useCallback((clientX: number) => {
    if (!containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const availableWidth = rect.width - DIVIDER_WIDTH_PX;
    const minPx = PANE_MIN_WIDTH_PX;
    const maxPx = availableWidth - minPx;

    if (maxPx < minPx) {
      setSplitPercent(50);
      return;
    }

    const newX = Math.max(minPx, Math.min(maxPx, clientX - rect.left));
    setSplitPercent((newX / availableWidth) * 100);
  }, []);

  const startDragging = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    isDragging.current = true;
    previousCursor.current = document.body.style.cursor;
    previousUserSelect.current = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    dividerRef.current?.classList.add("active");
    event.currentTarget.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  }, []);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      if (!isDragging.current || !containerRef.current) return;
      setSplitFromPosition(event.clientX);
    };

    const handlePointerUp = () => {
      if (isDragging.current) {
        isDragging.current = false;
        document.body.style.cursor = previousCursor.current;
        document.body.style.userSelect = previousUserSelect.current;
        dividerRef.current?.classList.remove("active");
      }
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      isDragging.current = false;
      document.body.style.cursor = previousCursor.current;
      document.body.style.userSelect = previousUserSelect.current;
      dividerRef.current?.classList.remove("active");
    };
  }, [setSplitFromPosition]);

  const getSplitBounds = useCallback(() => {
    const width = containerRef.current?.getBoundingClientRect().width;
    if (!width) return { min: 32, max: 68 };

    const availableWidth = width - DIVIDER_WIDTH_PX;
    return {
      min: (PANE_MIN_WIDTH_PX / availableWidth) * 100,
      max: ((availableWidth - PANE_MIN_WIDTH_PX) / availableWidth) * 100,
    };
  }, []);

  const adjustSplit = useCallback(
    (delta: number) => {
      const bounds = getSplitBounds();
      setSplitPercent((current) =>
        Math.max(bounds.min, Math.min(bounds.max, current + delta))
      );
    },
    [getSplitBounds]
  );

  return (
    <div className="split-pane-container" ref={containerRef}>
      <div
        className="split-pane-left"
        style={{
          flexBasis: `calc(${splitPercent}% - ${(splitPercent * DIVIDER_WIDTH_PX) / 100}px)`,
        }}
      >
        {left}
      </div>
      <div
        className="split-pane-divider"
        ref={dividerRef}
        onPointerDown={startDragging}
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft" || event.key === "ArrowDown") {
            event.preventDefault();
            adjustSplit(-5);
          } else if (event.key === "ArrowRight" || event.key === "ArrowUp") {
            event.preventDefault();
            adjustSplit(5);
          } else if (event.key === "Home") {
            event.preventDefault();
            setSplitPercent(getSplitBounds().min);
          } else if (event.key === "End") {
            event.preventDefault();
            setSplitPercent(getSplitBounds().max);
          }
        }}
        role="separator"
        aria-orientation="vertical"
        tabIndex={0}
        aria-valuemin={Math.round(getSplitBounds().min)}
        aria-valuemax={Math.round(getSplitBounds().max)}
        aria-valuenow={Math.round(splitPercent)}
      />
      <div className="split-pane-right">{right}</div>
    </div>
  );
}