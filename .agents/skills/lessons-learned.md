---
name: lessons-learned
description: Apply at the start of a session and whenever a mistake, near-miss, or discovery is worth remembering beyond the current task. A growing, project-agnostic log of transferable tips phrased as rules rather than history: cross-platform code, tests and verification, CI and automation, contracts and documentation, and agent workflow. Append new tips after a session; the file is written to be copied into any repository as-is.
---

# Lessons (transferable tips)

A running log of tips worth keeping, phrased as rules for the next
change rather than as a story about the last one. Entries must be
portable: if a tip only makes sense in one repository, it belongs in
that repository's docs or task files instead. Another project can copy
this file as-is and keep appending to it.

## Adding a tip

- Phrase it as a rule someone can apply without knowing what incident
  produced it. No dates, no commit hashes, no narrative.
- Put it under the closest category below; add a new category once one
  earns itself.
- One line where possible; expand only when the rule needs a "how".
- Delete a tip once tooling enforces it mechanically (a lint, a type, a
  test). The goal is to stop needing the tip, not to accumulate rules.

## Cross-platform code

- Never decide "is this path safe" with the standard path library
  alone: `is_absolute` and path-component prefixes answer for the host
  platform only, so `C:\img.png` looks like a plain relative filename
  when the code runs on Unix. Write platform-neutral string rules
  (normalize separators, reject a leading separator, a drive prefix
  such as `C:` or `C:\`, and any `..` segment however it is separated),
  and treat native checks as an optional extra, never the whole check.
- Same trap, different costume: line endings, filename case
  sensitivity, reserved Windows names (`CON`, `PRN`), locale-dependent
  formatting and sorting, and argument quoting when shelling out (use
  argument arrays, never a shell string that each shell parses
  differently).
- Ask the question out loud: "what does this exact input do on the
  other platforms?" If answering it requires platform knowledge, do not
  reach for platform knowledge; reach for a rule that is true
  everywhere.
- Construct paths with join operations only, never string concatenation
  with a hardcoded separator, and never in the layer that should not
  touch the filesystem at all.

## Tests and verification

- Assume every test line runs on every supported platform and every CI
  leg. An input that is platform-specific (a Windows path, a
  backslash, a case variant) must behave identically everywhere by
  design; if it cannot, gate the test per platform with a comment
  saying why. "Passes here" is evidence about one machine, not about
  the matrix.
- When a rule rejects a category, test one representative per syntax
  inside that category. The extra cases cost seconds; rediscovering the
  category after a rewrite costs a failed run.
- Test the pure function directly, not only through the larger flow
  that happens to exercise it. A bug invisible on your platform hides
  inside integration tests and surfaces first in someone else's run.
- Run the exact commands CI runs, from the exact working directories CI
  uses, before pushing. A command run from the wrong directory proves
  nothing, and a green result from a different command than CI's is not
  a green result.
- After a refactor, run the linter with warnings denied, not just the
  compiler. An unused import is as fatal as a logic bug wherever CI
  lints with warnings as errors.
- A local pass proves behavior on one OS only. Say which legs were
  verified, and which are left to CI or a human, in the commit or PR
  description.

## CI and automation

- Path filters decide what can fail at all. A filter missing a code
  directory makes changes to it run zero jobs, which is worse than a
  red run because it looks green. Cover every code directory, the
  dependency manifests, the lockfiles, and the workflow file itself,
  and update filters in the same change that adds a directory.
- A filter typo fails silently by doing nothing; after changing one,
  confirm the expected runs actually appear.
- Make a trigger change test itself: including the workflow's own path
  means the push shipping the fix runs it.
- Report "the job is red" and "the job never ran" as different
  findings; they have different fixes.
- Keep secrets, credentials, and network access out of a project by
  configuration (capability scopes, permission lists, dependency
  policy), not by convention someone has to remember. Auditable beats
  documented.

## Contracts, docs, and types

- When a data shape crosses a boundary (API, IPC, file on disk), keep
  one authoritative document for it, and land code, document, and tests
  in the same change. A code change that makes the document wrong is an
  incomplete change, not documentation debt.
- Serialization formats carry hidden constraints the type system does
  not show: a tagged enum cannot serialize a newtype variant holding a
  string or a sequence, for example. Write a round-trip test per
  container type early, before four subsystems depend on the shape.
- Give anything built from untrusted input private fields plus a
  validating constructor returning a result; a public field is a bypass
  waiting for the next call site. Keep wire format independent of
  language-level visibility so this costs nothing at the boundary, and
  re-validate deserialized values at the deserialization site.
- Make counters and identity values constructible only by the code that
  owns them (first/next, never a public field), so no path can issue a
  stale or arbitrary value.
- Prefer a small honest doc note over a large undocumented decision: if
  implementation forced a shape change, record why in the same commit
  as the change.

## Working as an agent

- Read the project's entry-point documentation before changing
  anything; its hard constraints are cheap to follow and expensive to
  unwind after the fact.
- Keep the two kinds of memory separate: session state (environment
  changes, unverified items, decisions pending review) goes in a
  handoff log with the newest entry on top, while durable rules such as
  these live here. A log is history; this file is rules.
- Commit in scoped, conventional commits with changelog-style bodies,
  and leave pushing to a human unless explicitly told otherwise.
- Ask before guessing on anything the project treats as non-negotiable
  (licensing, network policy, data-loss rules). Interface details are
  cheap to redo; schema and policy decisions are not.
- Prefer asking over building a workaround when a task appears to
  require something forbidden; the workaround is what becomes expensive
  to unwind.
- Re-read the exact current target before applying a patch after a long
  session or after other edits; stale context often makes a plausible
  patch fail or land against the wrong structure.
