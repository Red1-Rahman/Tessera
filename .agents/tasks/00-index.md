# Task index

This directory breaks Tessera's build into discrete, assignable tasks.
Each task is small enough for one agent (or one person) to own start to
finish, with an explicit prerequisite list so you always know what has to
exist before starting it, and whether it can run at the same time as
another task.

## Naming convention

- `serial-taskNN-name.md`: must run after all lower-numbered serial
  tasks (and any parallel group with the same or lower number) are
  complete. Do not start a serial task until its prerequisites say it is
  unblocked.
- `parallel-taskNN-X-name.md`: tasks sharing the same `NN` and differing
  only by the letter (`A`, `B`, `C`, ...) share the same prerequisites
  and do not depend on each other. They can be assigned to different
  agents or worked on at the same time. A `parallel-taskNN-*` group is
  gated by the same rule as a `serial-taskNN`: every task numbered lower
  than `NN` must be done first. The group as a whole must finish (all
  letters) before anything numbered higher than `NN` starts, unless a
  specific higher-numbered task's prerequisites say otherwise (checked
  per-task, see each file's "Prerequisites" section; a few only need one
  letter of a group, not all of them, and that is called out explicitly
  where it applies).

Numbers are not evenly spaced and are not meant to be; they exist purely
to encode order. Gaps may appear in later revisions of this plan without
meaning anything was removed.

## Full dependency graph

```
serial-task01-scaffolding
        │
        ▼
serial-task02-document-model
        │
        ├──────────────┬──────────────┬──────────────┐
        ▼               ▼               ▼              ▼
parallel-task03-A  parallel-task03-B  parallel-task03-C  parallel-task03-D
(latex parser)      (latex printer)    (docx reader)      (docx writer)
        │               │               │              │
        └──────────────┴──────────────┴──────────────┘
                        │
                        ▼
         serial-task04-roundtrip-contract-tests
                        │
                        ▼
          serial-task05-diff-and-sync-engine
                        │
        ┌──────────────┼──────────────┬──────────────┐
        ▼               ▼              ▼              ▼
parallel-task06-A  parallel-task06-B  parallel-task06-C  parallel-task06-D
(highlighter)        (pdf export)      (markdown export)  (plaintext export)
        │               │              │              │
        └──────────────┴──────────────┴──────────────┘
                        │
                        ▼
        serial-task07-tauri-command-layer
                        │
                ┌──────┴──────┐
                ▼              ▼
    parallel-task08-A    parallel-task08-B
    (frontend ipc/state)  (frontend shell)
                │              │
                └──────┬──────┘
                       ▼
        serial-task09-editor-pane-integration
                       │
                       ▼
        serial-task10-export-and-clipboard
                       │
                       ▼
        serial-task11-e2e-and-cross-platform-ci
                       │
                       ▼
        serial-task12-v1-polish-and-release-prep
```

## Quick table

| Task | Can start after | Can run alongside |
|---|---|---|
| `serial-task01` | nothing (first task) | nothing |
| `serial-task02` | 01 | nothing |
| `parallel-task03-A/B/C/D` | 02 | each other |
| `serial-task04` | all of 03-A/B/C/D | nothing |
| `serial-task05` | 04 | nothing |
| `parallel-task06-A/B/C/D` | 05 (06-A also needs 03-A specifically) | each other |
| `serial-task07` | all of 06-A/B/C/D | nothing |
| `parallel-task08-A/B` | 07 | each other |
| `serial-task09` | both 08-A and 08-B | nothing |
| `serial-task10` | 09 | nothing |
| `serial-task11` | 10 | nothing |
| `serial-task12` | 11 | nothing |

## Using this with subagents

If you are splitting work across subagents: one subagent per task file
is the right granularity. Hand each subagent only its own task file plus
`AGENTS.md` (the hard constraints apply to every task regardless) and the
specific `docs/*.md` sections its task file points to; you do not need to
give every subagent the entire `docs/` directory context up front.

A `parallel-taskNN-*` group is safe to hand to separate subagents
simultaneously precisely because each task file below declares its own
file ownership (see "Files touched" in each task) and those do not
overlap within a group. If two parallel tasks in the same group ever
need to touch the same file, that is a planning bug in this index, not
something to resolve by one subagent overwriting the other; stop and
split the file ownership first.

Each task file ends with a "Definition of done" checklist. A task is not
complete, and dependents should not start, until every item on that list
is checked.
