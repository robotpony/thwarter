# Bugs

Working log of defects. An entry under `## Open` is one line: the bug in bold, when it was found, and a pointer to detail if any. Diagnosis and fixes go in commit messages. When a bug is fixed, it moves to `## Fixed`, trimmed to the date, version and one line, plus the commit when the fix landed in an earlier commit than the move. Missing features are work, not bugs, and go in `PLAN.md`.

Keep the `## Open` and `## Fixed` headings as they are. Space Command's task parser reads them.

## Open

- [ ] **The editor's quick menu hint follows edit mode's place, not the open file**: `panels.rs` shows "⌃M or /menu" only when edit mode is at a room or thing (`quick_menu()` reads `place`), but Ctrl-M in the editor acts on the open file. Opened with `/edit` from the world root or a folder, the editor shows no hint though Ctrl-M works, and Ctrl-M then moves edit mode's place to the file. In the editor the hint SHOULD follow the open file and show only ⌃M (Bruce, 2026-11-25) [console] (REQ-CON-63)
- [ ] **`validate` doesn't warn about a heading tag that can never be true**: `design/SECTIONS.md` › States says it does, but `## When: #wallpaper.peeled` in the test world's hallway passes with no warning and never shows. It should warn, and suggest `@wallpaper:peeled` when the tag is a thing's id and one of its states joined by a dot (found 2026-11-25) [coder]
- [ ] **A ```rhai block below a `## When:` heading is dropped silently**: the script runs only from above the first heading (`layers.rs` › `take_script` sees the description alone), so a block written after a section never runs, and `validate` passes. It should either count anywhere in the file or warn (found 2026-11-25 writing `design/CONTENT_FORMAT.md` › Adding a verb to an object) [coder]
- [ ] **`#refused` and `#acted` can't be told apart by verb**: `## When: #spin #refused` never matches, only a bare `## When: #refused`, so a thing with two verbs gated by `requires:` gives both the same refusal. `## When: #take #immovable` names its verb, so these should too (found 2026-11-25, same place) [coder]
- [ ] **An unknown question construct does nothing, silently**: `validate` checks only a world's own question sets, so a built-in like `src/builtins/questions/object.md` with an answer such as `-> location in room` (not one of `set`, `recipe`, `ask thing`, `ask value`) gets no warning. Built-in and world sets should both warn (`validate` does since 0.81.3); the console will also say when a picked answer does nothing (2026-11-24) [console] (`design/CONTENT_FORMAT.md` › type: questions)
- [ ] **One compiler warning left**, `narrative_engine`'s unused `conditions` in a stub; it goes when narrative state uses that code. The two in `world_graph::find_path` went with the map in 0.37.0. Down from 11 in 0.7.0 (2026-08-23)

## Fixed

- 2026-11-24 (0.70.2): TO WRITE keys work in the editor: Ctrl- or Alt-1 to 9 save the open document, then make and open the entry, like switching tabs; a failed save keeps the editor open with the reason (REQ-CON-59).
- 2026-11-24 (0.66.3): a new object no longer lands in two rooms. When `/add` made a thing with no card, the question runner made its card with `/new object`, which also placed it in edit mode's room; `/add desk hallway` from the bedroom put the desk in both. The card is now written without placing it. (Not `location in room`, which is an unknown construct and does nothing.)
- 2026-11-24 (0.66.3): `/add` of a thing already placed asks "desk is in bedroom: move it here? (y/n)" and moves it on yes, changing nothing on no (REQ-CON-41, Bruce). Only y or n answers; any other line drops the question and runs as typed.
- 2026-11-23 (0.50.2, 37a5c49): the test world's five connections to rooms that didn't exist went with the old content; the new test world validates clean.
- 2026-11-23 (0.27.1): `thwart show` no longer prints the world's warnings on standard output; it writes one count to standard error ("2 problems and 27 warnings; run `thwart validate` to list them"), so its result stands alone and `--format json` is one document.
- 2026-11-23 (0.1.1): `--world`, `--config`, `-q` and `-v` are global, so they work after the subcommand as the docs write them.
- 2026-11-23 (0.1.1): `validate` prints its failure once and still exits non-zero.
- 2026-11-23 (0.1.1): `show <location>` text output reads as a room (name, description, exits, characters, objects) rather than a debug dump.
- 2026-11-23 (0.1.0, 6aa58de): HEAD didn't build on a fresh clone; `object.rs` was declared in `mod.rs` but never committed.
- 2026-08-24 (0.1.0, 2e4d080, squashed): 7 fixes from the August audit, from `Cargo.toml`'s `[bin]` breaking the build to the parser that couldn't read the sample content. Each is in `CHANGELOG.md` under 0.1.0, and in full with `git show 884ae96:tasks/ISSUES.md`.
