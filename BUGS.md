# Bugs

Working log of defects. An entry under `## Open` is one line: the bug in bold, when it was found, and a pointer to detail if any. Diagnosis and fixes go in commit messages. When a bug is fixed, it moves to `## Fixed`, trimmed to the date, version and one line, plus the commit when the fix landed in an earlier commit than the move. Missing features are work, not bugs, and go in `PLAN.md`.

Keep the `## Open` and `## Fixed` headings as they are. Space Command's task parser reads them.

## Open

- [ ] **Five connections in the test world point at rooms that don't exist**: `forest/abandoned-camp`, `forest/rocky-outcrop` (twice), `forest/cave-entrance`, `forest/cave-behind-falls`. An authoring decision for Bruce: write the rooms or cut the exits. Blocks using `validate` as a CI gate (2026-08-23)
- [ ] **About 11 compiler warnings**, mostly unused parameters in `game_engine.rs`'s stub action handlers; many go away once Phase 4 uses them (2026-08-23)

## Fixed

- 2026-11-23 (0.1.1): `--world`, `--config`, `-q` and `-v` are global, so they work after the subcommand as the docs write them.
- 2026-11-23 (0.1.1): `validate` prints its failure once and still exits non-zero.
- 2026-11-23 (0.1.1): `show <location>` text output reads as a room (name, description, exits, characters, objects) rather than a debug dump.
- 2026-11-23 (0.1.0, 6aa58de): HEAD didn't build on a fresh clone; `object.rs` was declared in `mod.rs` but never committed.
- 2026-08-24 (0.1.0, 2e4d080, squashed): 7 fixes from the August audit, from `Cargo.toml`'s `[bin]` breaking the build to the parser that couldn't read the sample content. Each is in `CHANGELOG.md` under 0.1.0, and in full with `git show 884ae96:tasks/ISSUES.md`.
