# serial-task12: v1 polish and release preparation

## Prerequisites

`serial-task11-e2e-and-cross-platform-ci` complete. This is the final
task before a v1 tag.

## Goal

Close out remaining documentation drift, verify every hard constraint in
`AGENTS.md` still holds across the finished codebase, and prepare
release artifacts.

## Read first

- `AGENTS.md` in full, especially "Hard constraints" and "Before opening
  a pull request," applied here to the whole codebase rather than one
  change.
- `.agents/skills/contract-discipline.md`.
- `README.md`, to confirm it still accurately describes the finished
  application.

## Files touched

```
README.md (accuracy pass)
docs/*.md, docs/*.yml, docs/*.txt (accuracy pass against final
  implementation)
CHANGELOG.md (new, if not already present)
```

## Steps

1. Full documentation accuracy pass: read `docs/architecture.md`,
   `docs/contracts.md`, `docs/specs.md`, `docs/conventions.md`,
   `docs/techstack.yml`, and `docs/codebase.txt` against the actual,
   finished codebase. Fix any drift that accumulated across the eleven
   prior tasks; some is expected given how many tasks touched shared
   contracts, per `.agents/skills/contract-discipline.md`'s acknowledgment
   that this is where drift tends to happen.
2. Confirm every item in `AGENTS.md`'s "Hard constraints" section holds
   across the whole codebase, not just the files touched by the most
   recent task: no network access anywhere (full `cargo tree` and
   `tauri.conf.json` audit), every source file has the GPL header,
   `Opaque` handling has no silent-loss gaps, contracts and specs
   documentation is accurate.
3. Confirm `README.md`'s "Core features," "Non-features," build
   instructions, and project layout sections all match the finished
   application exactly.
4. Write `CHANGELOG.md` covering the work done across all twelve tasks
   at a user-relevant level of detail (what the application can do),
   not a task-by-task engineering log.
5. Verify version numbers are consistent across `Cargo.toml`,
   `package.json`, and `tauri.conf.json`.
6. Run the full `.agents/prompts/pr-checklist.md` against the entire
   codebase as a final gate, not just the latest diff.
7. Tag the release per whatever versioning scheme is appropriate for a
   v1.0.0; confirm CI builds produce installable artifacts for all three
   platforms from the tag.

## Definition of done

- [ ] Every `docs/*` file accurately reflects the finished
      implementation; no known drift remains.
- [ ] `AGENTS.md`'s hard constraints verified against the full,
      finished codebase.
- [ ] `README.md` is accurate end to end.
- [ ] `CHANGELOG.md` exists and accurately describes v1 functionality.
- [ ] Version numbers are consistent across all manifests.
- [ ] `.agents/prompts/pr-checklist.md`'s full checklist passes against
      the entire codebase.
- [ ] Release artifacts build successfully for Windows, macOS, and
      Linux from the tagged commit.
