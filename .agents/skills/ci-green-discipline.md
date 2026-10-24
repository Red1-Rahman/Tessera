---
name: ci-green-discipline
description: Apply whenever writing validation logic, cross-platform-sensitive tests, or editing .github/workflows. A checklist of mistakes that have already turned Tessera's CI red once, written as tips so the same mistake, or a close relative of it, is not repeated. Read before pushing anything that touches paths, separators, tests with platform-specific inputs, or workflow triggers.
---

# Keeping CI green

Tessera's three CI legs (Windows, macOS, Linux) run the same test suite
and the same lint commands. Anything that only behaves correctly, or
only passes, on the platform you happen to develop on is a red build on
the other two. This file is the standing list of the ways that has
already happened here, phrased as rules to apply next time.

## Validation logic: never delegate portability to the standard library

- `std::path` answers questions about the *host* platform's syntax, not
  about "can this path escape a directory". `Path::is_absolute` and
  `Component::Prefix` happily classify `C:\img.png` as a plain relative
  filename when the code runs on Unix. For anything security- or
  correctness-relevant (escape checks, asset paths, temp paths), write a
  platform-neutral string rule and treat the native check as an optional
  extra, never as the whole check.
- A portable rule for path safety is small: normalize `\` to `/`, then
  reject a leading separator (POSIX absolute and UNC), a drive prefix
  (`C:\x` and the drive-relative `C:x`), and any `..` segment however it
  is separated. Keep it in one pure function so it can be tested
  exhaustively.
- The same trap exists for anything separator- or case-sensitive:
  line-ending handling, filename case collisions, reserved Windows
  names. Ask "what does this exact input do on the other two platforms?"
  and if the answer needs platform knowledge, do not use platform
  knowledge; use a rule that is true everywhere.

## Tests: assume every line runs on three operating systems

- A test input that is a platform-specific literal (a Windows path, a
  backslash, a case variant) must be rejected or accepted on *every*
  platform by design. If that is impossible, gate the test on the
  platform explicitly and comment why; never let it ride on "this
  passes here".
- When a validator rejects a category, add one case per syntax inside
  that category: POSIX absolute, Windows drive absolute, drive-relative,
  UNC, and `..` with each separator style. The extra cases cost seconds;
  rediscovering the category after a rewrite costs a red CI run.
- Put platform-sensitive logic in a pure function and test that
  function directly. Integration tests that only exercise it through a
  larger flow will pass on the platform where the bug is invisible.

## Before pushing: run what CI runs, where CI runs it

- From `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings`, `cargo test --all-features`. From the repo root:
  `npm run lint`, `npm run typecheck`, `npm run test -- --run`,
  `npm run build`, `npm run test:e2e`. `.agents/prompts/pr-checklist.md`
  has the full list.
- A local pass proves the code compiles and the tests pass *on this
  OS*. Say so explicitly in the commit or PR description for anything
  path-, process-, or line-ending-related, so a reviewer knows which two
  legs are unverified.
- Compile warnings are clippy failures in disguise: CI lints with `-D
  warnings`, and `unused import` after a refactor is exactly as fatal as
  a logic bug. Run clippy, not just `cargo build`.

## Workflow triggers: a change that CI never runs is a change no CI has approved

- The `paths:` filters in `.github/workflows/build-*.yml` decide what
  can turn CI red at all. Rust lives in `src-tauri/`, frontend in
  `src/`, tests in `tests/` and `src-tauri/tests/`; the filter must
  cover every directory that holds code, plus `package.json`,
  `package-lock.json`, and the workflow files themselves (so a trigger
  fix is itself tested).
- When a new top-level code directory appears, updating the filters is
  part of adding the directory, not a follow-up. `docs/codebase.txt` is
  the list to check against.
- After changing triggers, confirm with a push that touches only the
  previously-missing path and check that all three workflows appeared in
  the run list; a filter typo fails silently by doing nothing.
