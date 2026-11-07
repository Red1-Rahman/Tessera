// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

/**
 * Suppresses scroll-event echo while a pane applies a programmatic
 * (synchronization-driven) scroll.
 *
 * Setting `scrollTop` programmatically fires a scroll event on the
 * element just like a user scroll does. Without suppression, pane A
 * scrolling pane B makes pane B report the (integer rounded) ratio
 * back, pane A applies it, and the two panes ping-pong a few pixels
 * forever because their scroll ranges differ. `begin()` arms the
 * suppressor just before a programmatic `scrollTop` assignment and
 * clears it two animation frames later, which is after the scroll
 * events for that assignment have been delivered, while genuine user
 * scrolls before and afterwards are reported normally.
 */
export interface ScrollEchoSuppressor {
  /** Arms suppression immediately before a programmatic scrollTop write. */
  begin(): void;
  /** True between `begin()` and the second subsequent animation frame. */
  isSuppressed(): boolean;
}

export function createScrollEchoSuppressor(): ScrollEchoSuppressor {
  let suppressed = false;
  let frame: number | null = null;

  return {
    begin() {
      suppressed = true;
      if (frame !== null) window.cancelAnimationFrame(frame);
      frame = window.requestAnimationFrame(() => {
        frame = window.requestAnimationFrame(() => {
          suppressed = false;
          frame = null;
        });
      });
    },
    isSuppressed() {
      return suppressed;
    },
  };
}
