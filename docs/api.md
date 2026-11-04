# Frontend and Backend Integration

This document is a practical guide to connecting frontend behavior to the
implemented Rust backend. It describes the callable Tauri commands, their
runtime behavior, the data flow between edits, and which user actions have
backend support. It does not prescribe UI layout or interaction design.

`docs/contracts.md` remains the canonical source for serialized type
shapes. `docs/specs.md` defines which LaTeX and DOCX constructs the core
supports. When this guide and those documents disagree with the registered
commands or their handlers, update the documentation and code together.

## Runtime boundary

The frontend runs in Tauri's webview and calls Rust through Tauri v2
`invoke`. There is no HTTP API, local web server, or event stream. Command
registration in `src-tauri/src/lib.rs` is the definitive list of callable
backend operations. Only these commands are registered:

| Command | Purpose |
|---|---|
| `open_project` | Open an existing project directory and load `main.tex`. |
| `new_project` | Create a project directory and starter document. |
| `parse_latex` | Replace the current document from full LaTeX source. |
| `apply_docx_edit` | Apply supported Document Model patches from the structured pane. |
| `save_project` | Save current state to the project files. |
| `get_highlight_tokens` | Classify LaTeX source for syntax highlighting. |
| `export` | Export current state to a file or return clipboard-ready text. |

No separate backend operation currently exists for project dialogs,
clipboard writes, DOCX import from a standalone file, image upload, undo,
redo, or granular nested-block editing. Do not invent an `invoke` command
for these actions. A UI feature that needs one requires a registered
command and corresponding contract, spec, and implementation first. File
and directory chooser dialogs are not registered in the current
capabilities; `open_project` and `new_project` accept paths supplied by
the caller.

Tauri serializes Rust command arguments using camelCase argument names at
the command boundary. For example, a Rust parameter named `request` is
passed as `{ request: ... }`. Fields inside request objects retain the
exact serde names documented in `docs/contracts.md`, generally snake_case.
Rust enum variant names serialize as their exact variant spelling, such as
`Latex`, `PlainText`, and `H1`, unless the contract explicitly shows a
tagged object such as `{"kind":"ClipboardPayload", ...}`.

All command errors reject the `invoke` promise with the serialized
`TesseraError` object from `docs/contracts.md`. Handle the object by its
`kind` tag, not by parsing an error string. A successful Rust `()` result
from `save_project` resolves as `null` in JavaScript.

## Command behavior

### Project lifecycle

`open_project({ path })` loads `<path>/main.tex` as the content source of
truth. It returns `ProjectState`. The path must be a non-blank directory
containing a readable `main.tex`. A missing or invalid
`.tessera/project.json` is reconstructed from `main.tex` and reported in
`warnings`; reconstruction is not a command failure. Image files
referenced from the project root may be copied into `.tessera/assets`.
Missing images and some copy failures are reported as warnings.

`new_project({ request })` expects a `NewProjectRequest`:

```json
{"path":"/absolute/or/relative/project/path","title":"Optional title"}
```

`title` may be `null` or omitted and defaults to `Untitled`. The command
creates `main.tex`, `main.docx`, and `.tessera/project.json`. It refuses to
overwrite a project that already has `main.tex`. It returns
`ProjectState`.

The backend stores one active project session per application window.
Opening or creating another project replaces the active session. Commands
that require a project fail with `ValidationError` on field `project` when
no project is open. There is no close-project command.

`save_project()` takes no payload. It writes `main.tex`, `main.docx`, and
`.tessera/project.json` from the current in-memory Document Model. Edits
are held in memory until this command is called. Exporting a file does not
save the project first.

### Editing and version flow

The frontend must keep the `Document` and `version` returned by the
backend as its authoritative shared state. Every edit request uses the
latest confirmed version. Do not independently mutate a LaTeX view and a
DOCX view as competing document states. The Document Model is the shared
source, as described in `docs/architecture.md`.

`parse_latex({ request })` takes `{ base_version, full_source }`. Send the
complete current LaTeX source. On success, `DocSyncResult` contains the
new version, full `document`, `latex_render`, and `warnings`. For a
LaTeX-origin edit, `latex_render` is the submitted source, so the backend
does not canonicalize the active source while the user types. Refresh the
structured pane and mirrored model from the response.

`apply_docx_edit({ request })` takes `{ base_version, patches }`. Patch
variants and their JSON shapes are specified in `docs/contracts.md`.
Currently only top-level body blocks can be replaced, inserted, or
removed. `Replace.path` must contain exactly one body index. `Insert.path`
and `Remove.path` must be empty. Nested list, quote, and table-cell paths
are rejected with `ValidationError`; do not send guessed paths. On
success, the response contains the updated model and freshly printed
LaTeX source in `latex_render`.

Both edit commands reject a request whose `base_version` is stale with
`StaleVersion`, including `requested` and `current` versions. Discard the
unconfirmed edit result, use the latest confirmed `Document` and version,
then reconcile or resubmit the user's edit. Requests for a future version
are rejected as `ValidationError`. Do not apply a response after the
frontend has already confirmed a higher version. Serializing edit calls
or otherwise ensuring only one outstanding edit per session simplifies
this protocol.

Warnings are successful results, not thrown errors. `SyncWarning` has a
human-readable `message` and `affected_path`, a vector of body-block
indices. An empty path can indicate a document-level warning. Preserve
warnings for a visible warning surface rather than treating them as
parse failure.

### Highlighting

`get_highlight_tokens({ request })` takes `{ source }` and returns an
array of `{ start, end, kind }`. Offsets are half-open UTF-16 code-unit
indices into the exact submitted JavaScript string, suitable for editor
ranges. Kinds are `Command`, `Bracket`, `Environment`, `Comment`, `Math`,
`Error`, and `Text`. This command requires an open project because it uses
the current document's labels to resolve cross-references. Malformed
source can produce `Error` tokens while processing continues.

### Export and clipboard handoff

`export({ request })` takes `{ format, destination }`, where `format` is
one of `Latex`, `Docx`, `Pdf`, `Markdown`, or `PlainText`.

- With a non-blank `destination`, output is written to that path and the
  result is `WrittenToFile` with `path`. PDF results also include
  `pdf_engine`, either `{ "type":"SystemTeX", "program":"..." }` or
  `{ "type":"BuiltIn" }`. Non-PDF results omit `pdf_engine`.
- With `destination: null`, LaTeX, Markdown, and plain text return
  `ClipboardPayload` with `format` and `content`. The backend does not
  write to the operating system clipboard; the frontend must do that if
  the product flow calls for it.
- DOCX and PDF require a destination. A missing destination rejects with
  `ExportError`.

The destination is a file path provided by the caller. The backend does
not open a save dialog or add a file extension. Export uses the current
in-memory model, whether or not `save_project` has run. Markdown and plain
text behavior, including formatting of supported and opaque nodes, is
specified in `docs/specs.md`.

## TypeScript call pattern

All direct imports and calls to `invoke` belong in
`src/ipc/commands.ts`, according to `docs/conventions.md`. Components and
stores call typed wrappers. The frontend wrapper module is not present in
the current source tree yet; this example illustrates the required
boundary shape, not an existing implementation:

```ts
import { invoke } from "@tauri-apps/api/core";

export function openProject(path: string): Promise<ProjectState> {
  return invoke<ProjectState>("open_project", { path });
}

export function parseLatex(request: LatexEditRequest): Promise<DocSyncResult> {
  return invoke<DocSyncResult>("parse_latex", { request });
}

export function saveProject(): Promise<void> {
  return invoke<void>("save_project");
}
```

Define the TypeScript interfaces and discriminated unions from
`docs/contracts.md`, preserving Rust field names and exact enum spellings.
Do not define a second, UI-specific wire shape or silently coerce values
at the call site. The generic type passed to `invoke` is compile-time
only; it does not validate the runtime response. Keep wrappers narrow and
cover every registered command as the frontend is implemented.

## Feature support checklist

Before wiring a frontend button, action, or editor behavior, locate its
backend operation in the registered command list above. Check that its
input and result are represented in `docs/contracts.md`, and that the
actual format behavior is in `docs/specs.md`. If any part is absent, the
feature is not currently integrated. Add the backend operation and update
the relevant contract and spec in the same change before wiring the
frontend. This avoids controls that appear to work but have no backend
effect, and frontend calls to commands that do not exist.
