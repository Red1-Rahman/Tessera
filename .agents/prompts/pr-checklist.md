# Prompt: Pre-PR checklist (expanded)

This is the command-level expansion of the checklist in `AGENTS.md`,
"Before opening a pull request." Run every relevant section before
proposing a pull request; skip only sections that genuinely do not apply
(a docs-only change does not need the Rust test suite, for example, but
still needs the em dash check).

## Rust

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If `cargo test` includes the `proptest` round-trip suite in
`tests/contracts.rs` and it is slow, that is expected; do not reduce the
case count to make it faster without flagging that change explicitly,
since case count is part of what makes these tests meaningful.

## Frontend

```bash
npm run lint
npm run typecheck
npm run test        # vitest
```

## Network ban verification

```bash
cd src-tauri
cargo tree | grep -iE "reqwest|hyper|ureq|tokio-tungstenite" || echo "clean"
```

A match here is not automatically disqualifying (a dependency might pull
in `hyper` transitively for something unrelated to networking, such as a
local-only use), but it needs explicit justification in the PR
description if it appears. Also check `tauri.conf.json` for any added
`capabilities` entry and confirm none grant `http` or network scope.

## Documentation sync

- If `src-tauri/src/core/model.rs` changed: confirm `docs/contracts.md`
  section 1 reflects the new shape.
- If `src-tauri/src/commands/*.rs` changed: confirm `docs/contracts.md`
  section 2 reflects the new command signatures.
- If LaTeX or DOCX support changed: confirm `docs/specs.md` reflects it.
- If a new top-level file or directory was added: confirm
  `docs/codebase.txt` has an entry.

## Style

```bash
grep -rn "—" README.md docs/ AGENTS.md .agents/ || echo "no em dashes found"
```

(This also catches em dashes accidentally introduced into source file doc
comments if you widen the grep path; widen it for the actual changed
files in your diff.)

Confirm new source files carry the GPL header from
`docs/conventions.md`, "Licensing headers."

## Final check

Re-read the diff once as a reviewer would: does every changed behavior
have a corresponding test, and does every changed contract have a
corresponding documentation update in the same change? If either is
missing, the PR is not ready yet.
