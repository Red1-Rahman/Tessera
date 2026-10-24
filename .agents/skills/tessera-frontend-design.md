---
name: tessera-frontend-design
description: Apply whenever building or reviewing UI in src/: the split-pane editor, syntax highlighting, Opaque block treatment, sync feedback, toolbar, or export menu. Use for layout, color, typography, motion, and interaction decisions specific to Tessera's two-pane LaTeX/DOCX editor. Not a general web design skill; it encodes decisions specific to this app.
---

# Tessera Frontend Design

## Initial response

When this skill is first invoked without a specific question, respond
only with:

> I'm ready to help design Tessera's interface: a quiet, instrument-like
> split view where a LaTeX pane and a DOCX pane stay in lockstep.

Do not provide any other information until the user asks a question.

## The core idea

Tessera's UI has exactly one job: let someone forget they are looking at
two documents. The two panes are not two features bolted together, they
are one document shown through two lenses, and the interface has to
*feel* that way, through motion that visibly links an edit on one side to
its effect on the other, not just through the fact that both panes
happen to update.

The audience is a researcher mid-sentence in a dense paragraph of prose
and citations. The UI's job is to stay out of the way of that sentence.
Every rule below exists to serve one of three things: **trust** (the
sync is correct and the person can see that it's correct), **focus**
(nothing competes with the text being written), and **legibility**
(technical content, including math and code, must be genuinely readable,
not just present).

This skill assumes `docs/specs.md`, `docs/contracts.md`, and
`docs/conventions.md` as ground truth for behavior (debounce timing, the
Okabe-Ito palette, the `Opaque` block concept, `SyncWarning`). This skill
is about how those things look and move, not what they are.

## 1. Layout: the split is the whole app

- Two panes, LaTeX and DOCX, resizable, default 50/50. No third pane, no
  sidebar competing for width by default. A file tree or outline, if
  added later, is a collapsible overlay, not a permanent column, per
  `docs/specs.md`'s "Split view is the default and only layout in v1."
- The divider between panes is a real, grabbable affordance (8 to 10px
  hit target, 1px visible line), not a bare CSS border. On drag, resize
  live, don't wait for release, matching the direct-manipulation
  principle: the pane width should track the pointer 1:1.
- Each pane scrolls independently by default, but a synced-scroll toggle
  (keep both panes roughly aligned to the same document position) is a
  first-class toolbar control, not a hidden setting, since "where am I
  in the other pane" is the single most common orientation question a
  split-document tool has to answer.
- Toolbar is a single thin strip above both panes, not above each pane
  individually. Export, save, and the sync-status indicator (below) live
  here. Resist per-pane toolbars; they double the visual weight for no
  functional gain.
- Max content width inside each pane should still respect a comfortable
  measure for prose, around 70 to 90 characters per line for the DOCX
  pane's rendered text; the LaTeX pane is source code and can run wider,
  but both should never force the eye to track an uncomfortably long
  line.

```css
:root {
  --pane-min-width: 320px;
  --divider-width: 1px;
  --divider-hit-target: 10px;
  --measure-docx: 72ch;
}
```

## 2. Color: Okabe-Ito is the whole palette, not just the LaTeX theme

`docs/conventions.md` defines the eight Okabe-Ito colors and their LaTeX
token roles. Two things follow from that for the rest of the UI:

- **The DOCX pane's semantic highlights reuse the same eight colors for
  the same concepts**, not a separate color system. A citation in the
  LaTeX pane and the resolved citation marker in the DOCX pane should
  read as the same color family, so the eye can match a change across
  panes without reading the text. This is the main way the UI
  communicates "these are the same thing" without a label.
- **Chrome (toolbar, dividers, pane backgrounds) stays neutral and gets
  out of the palette's way.** The eight Okabe-Ito colors are a budget
  spent entirely on document content: syntax tokens, the Opaque tint,
  and sync-status accents. If toolbar icons or UI chrome start pulling
  from the same eight colors, content and interface compete for
  attention and the palette's whole purpose (letting color carry
  meaning reliably) breaks down.

```css
:root {
  --chrome-bg: #fafafa;
  --chrome-border: #e3e3e3;
  --chrome-text: #3a3a3a;
  --chrome-text-muted: #86868b;

  /* Okabe-Ito, mirrored from docs/conventions.md; do not redefine
     elsewhere, this is the single source for the frontend */
  --ok-black: #000000;
  --ok-orange: #e69f00;
  --ok-sky: #56b4e9;
  --ok-green: #009e73;
  --ok-yellow: #f0e442;
  --ok-blue: #0072b2;
  --ok-vermillion: #d55e00;
  --ok-purple: #cc79a7;
}

@media (prefers-color-scheme: dark) {
  :root {
    --chrome-bg: #1a1a1a;
    --chrome-border: #2e2e2e;
    --chrome-text: #e8e8e8;
    --chrome-text-muted: #9a9a9a;
    /* Okabe-Ito values are already designed to hold contrast on dark
       backgrounds; raise perceived brightness slightly rather than
       changing hues, so the LaTeX/DOCX color mapping stays identical
       in both themes */
    --ok-yellow: #f5ea5c;
  }
}
```

### Opaque blocks

An `Opaque` block (see `docs/contracts.md`) is reserved vermillion
territory in spirit but reddish purple (`--ok-purple`) in
`docs/conventions.md`'s actual assignment: use it as a low-opacity
background tint (8 to 12 percent), never as text color, with a thin
left border at full opacity for a clear edge. It must be visually
distinct from both an error (`--ok-vermillion`) and a warning, and it
must never look clickable-but-broken; it should look deliberately
fenced off, like a quoted block, not like a rendering failure.

```css
.opaque-block {
  background: color-mix(in srgb, var(--ok-purple) 10%, transparent);
  border-left: 3px solid var(--ok-purple);
  border-radius: 4px;
  padding: var(--space-sm) var(--space-md);
  cursor: default; /* never implies editability */
}
```

## 3. Typography

- LaTeX pane: a monospace font stack, tabular figures, generous line
  height for scanning nested braces (`1.6`). This is source code; treat
  it like source code.
- DOCX pane: a serif or well-hinted sans body face appropriate to
  academic manuscripts, because this pane is meant to approximate what a
  submitted Word document looks like. Match the tracking/leading
  discipline from general interface design: tighter leading on headings,
  looser on body text, size-specific tracking rather than one fixed
  value at every size.
- Both panes respect the user's OS text-size setting; scale in `rem`,
  never hardcode pixel font sizes for body text.

```css
:root {
  --font-mono: "JetBrains Mono", ui-monospace, "SF Mono", Consolas, monospace;
  --font-docx: "Source Serif 4", Georgia, serif;

  --text-latex-body: 400 0.9375rem/1.6 var(--font-mono);
  --text-docx-body: 400 1rem/1.5 var(--font-docx);
  --text-docx-h1: 600 1.75rem/1.2 var(--font-docx);
  --text-docx-h2: 600 1.375rem/1.25 var(--font-docx);
}
```

## 4. Motion: the sync has to be visible, not just true

The hardest UI problem in this app is not styling, it's making the user
*feel* that an edit on one side genuinely reached the other side,
without being so loud it breaks focus on a dense paragraph. Treat this
as the single most important motion design problem in the app.

- **Changed regions in the receiving pane get a brief highlight flash**
  (150 to 250ms fade, using the token's own Okabe-Ito color at reduced
  opacity, not a generic yellow flash) after a successful sync. This is
  the user's confirmation that their edit propagated, and it should fire
  every time, not just on the first edit of a session.
- **The highlight fades from the current render, never jumps.** If a
  second edit arrives before the first highlight finishes fading, blend
  into the new highlight rather than hard-cutting it; a hard cut reads
  as the sync stuttering even when it didn't.
- **No full-pane reflow animation on every keystroke.** Only the changed
  block(s) should visibly move or resize; a full-pane re-layout on every
  sync would make the DOCX pane feel like it's constantly being rebuilt
  under the reader, which actively undermines trust in the sync rather
  than demonstrating it.
- **Debounce is felt, not hidden.** `docs/conventions.md` sets a 250ms
  debounce before re-parse. During that window, show a quiet,
  non-blocking indicator (a small dot or bar in the toolbar transitioning
  from "editing" to "syncing" to "synced") rather than nothing. Silence
  during a quarter-second window reads as fine; silence that could be
  mistaken for a stall does not, especially on a longer document where
  parse time may vary.
- **Respect `prefers-reduced-motion`.** Replace the flash fade with a
  brief, static background-color change and immediate removal, no
  animated transition.

```css
@keyframes sync-flash {
  from { background-color: var(--flash-color); }
  to   { background-color: transparent; }
}
.synced-region {
  animation: sync-flash 200ms ease-out;
}

@media (prefers-reduced-motion: reduce) {
  .synced-region { animation: none; }
}
```

## 5. Sync status and warnings: informative, never a modal

- Sync status (editing / syncing / synced / error) lives as a small,
  constant-position indicator in the toolbar. It never moves, never
  grows, so the eye can check it without hunting.
- A `SyncWarning` (see `docs/contracts.md`) renders as an inline
  annotation anchored to the affected block in both panes, consistent
  with `docs/specs.md`: "non-blocking inline annotation... never a modal
  dialog." Use a small marker (not a full-width banner) at the start of
  the affected block, expandable on hover or click for the full message.
- Never block typing to show a warning. The moment a warning needs a
  modal to be noticed, that's a sign that the architecture's
  non-blocking promise in `docs/specs.md` isn't being honored at the UI
  level either.
- Errors (`TesseraError`, see `docs/contracts.md`) that prevent a sync
  from completing at all are different from warnings: they get
  vermillion (`--ok-vermillion`), persist until resolved, and should
  clearly separate "this block has a problem" from "I don't know what
  happened," per `TesseraError`'s own tagged variants, so the message
  shown always matches a real, specific failure kind, never a generic
  catch-all.

## 6. Interaction details

- **Opaque blocks are inert by design** (see section 2); make that
  legible through cursor and absence of hover affordance, not through a
  disabled-looking gray wash that could be mistaken for a loading state.
- **Divider drag, pane resize, and sync-status transitions** should all
  use a critically damped spring or an equivalent short ease (150 to
  250ms), not a linear transition and not a bouncy one; nothing in this
  app is a gesture-driven throw, so no overshoot is earned anywhere.
- **Clicking a cross-reference (`Inline::CrossRef`) in either pane**
  scrolls the referenced block into view in both panes and gives it the
  same brief highlight flash as a sync update, reusing one visual
  language for "something in the document moved into focus" rather than
  inventing a second one.
- **Export menu** is a simple list (LaTeX, DOCX, PDF, Markdown, plain
  text, copy to clipboard), each item showing format name plus a
  one-line note when relevant (for example "Basic PDF, no TeX engine
  found" per `docs/architecture.md`'s PDF export strategy). No preview
  pane, no wizard. A researcher choosing an export format already knows
  what they want; the menu's job is to get out of the way quickly.

## 7. Accessibility and reduced states

- `prefers-reduced-motion`: covered above, applies to sync flashes,
  divider drag easing, and cross-reference scroll-into-view (use instant
  scroll, not smooth-scroll, when set).
- `prefers-contrast: more`: raise the Opaque block border and sync
  status indicator to fully opaque fills rather than tinted
  translucency.
- Every Okabe-Ito-colored token must also be distinguishable without
  color alone where it carries meaning beyond syntax highlighting (an
  `Opaque` block uses a border and padding, not tint alone; a
  `SyncWarning` marker uses an icon, not color alone), since the palette
  being colorblind-safe for hue discrimination doesn't substitute for
  not relying on hue as the only signal.
- Touch targets (divider, toolbar buttons, warning markers) stay at or
  above 44px effective hit area even though this is a desktop app with a
  mouse-first assumption; trackpad and touch-screen laptops are common
  enough on all three target platforms to warrant it.

## Quick reference

| Decision | Rule |
| --- | --- |
| Default layout | Two resizable panes, 50/50, single shared toolbar |
| Chrome color | Neutral grays only; Okabe-Ito reserved for document content |
| DOCX pane highlight colors | Same eight Okabe-Ito colors, same token-to-color mapping as the LaTeX pane |
| Opaque block | `--ok-purple` tint at ~10% + solid left border, `cursor: default` |
| Sync confirmation | Brief flash in the token's own color on the receiving pane, 150 to 250ms |
| Full-pane reflow on sync | Never; only the changed block(s) move |
| Debounce feedback | Toolbar status dot: editing → syncing → synced |
| SyncWarning | Inline marker anchored to the block, never a modal |
| TesseraError | Persistent, vermillion, matches the specific tagged variant |
| Cross-reference click | Scroll into view + same highlight flash language as sync |
| Reduced motion | Static color swap instead of animated fade; instant scroll instead of smooth |
| Export menu | Flat list, no preview, inline caveat text where relevant (for example PDF fallback) |
