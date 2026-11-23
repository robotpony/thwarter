# Bugs

Working log of defects. An entry under `## Open` is one line: the bug in bold, when it was found, and a pointer to detail if any. Diagnosis and fixes go in commit messages. When a bug is fixed, it moves to `## Fixed`, trimmed to the date, version, commit and one line. Missing features are work, not bugs, and go in `PLAN.md`.

Keep the `## Open` and `## Fixed` headings as they are. Space Command's task parser reads them.

## Open

- [ ] **`--world` is rejected after a subcommand**, so every example in `README.md` and `TESTING.md` fails as written; it only works as `thwart --world <path> validate` (2026-11-23)
- [ ] **`show <location>` prints a Rust debug dump as its text output** rather than the room as an author would read it; `--format json` is fine (2026-11-23)
- [ ] **`validate` reports its failure twice**, once as a log line and again as the returned error (2026-11-23)
- [ ] **Five connections in the test world point at rooms that don't exist**: `forest/abandoned-camp`, `forest/rocky-outcrop` (twice), `forest/cave-entrance`, `forest/cave-behind-falls`. An authoring decision for Bruce: write the rooms or cut the exits. Blocks using `validate` as a CI gate (2026-08-23)
- [ ] **About 11 compiler warnings**, mostly unused parameters in `game_engine.rs`'s stub action handlers; many go away once Phase 4 uses them (2026-08-23)

## Fixed

- 2026-11-23 (0.1.0, 6aa58de): HEAD didn't build on a fresh clone; `object.rs` was declared in `mod.rs` but never committed.
- 2026-08-24 (0.1.0, 2e4d080, squashed): 7 fixes from the August audit, from `Cargo.toml`'s `[bin]` breaking the build to the parser that couldn't read the sample content. Each is in `CHANGELOG.md` under 0.1.0, and in full with `git show 884ae96:tasks/ISSUES.md`.
