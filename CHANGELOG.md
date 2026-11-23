# Changelog

Thwarter's changes, newest first. This file also records what was removed or replaced: deleted files, retired formats, superseded plans and their history. Each removal names the commit to recover it from, so the working docs can describe only what's true now.

Versions are `0.x.y` semver, pre-1.0. Sections are `### Added`, `### Changed`, `### Fixed` and `### Removed`. Changes not yet in a versioned build go under `## Unreleased`.

## Unreleased

### Added

- `PLAN.md`, the phased work queue, and `BUGS.md`, the defect log, both in Space Command's task format.
- This changelog.
- `.claude/hooks/check-commit-message.sh`, which refuses a commit that credits Claude as a co-author. Copied from warped-space-command.

### Changed

- The phase plan: authoring first, then play, in one console with an edit and a play mode; engine work only as they need it (Bruce, 2026-11-23). `map`, `search` and the remaining `validate` checks moved to a "Tooling, as needed" phase, and tests moved into each phase.
- `REQUIREMENTS.md` gains §4.6 Console (REQ-CON-1 to 11), the file writer and in-memory changes (REQ-ENG-7 to 9), and objects without cards as allowed scenery (REQ-CF-8). The separate P1 play REPL and P2 editing REPL are now one console.
- `tasks/unit-testing-plan.md` moved to `design/UNIT_TESTS.md`.
- `tasks/ISSUES.md` became `BUGS.md`. Its open items that are missing features, not defects (no tests, `map`/`search`/`debug`, `CharacterEngine` and `ConfigManager` not wired in), moved to `PLAN.md`.

### Removed

- `tasks/cli-first-pass-TODO.md`, the original 20-day, six-phase plan. It described a `src/lib/` layout that never existed. Replaced by `PLAN.md`; recover with `git show 884ae96:tasks/cli-first-pass-TODO.md`.
- The tooling-first phase order (Tests, Static-world tooling, Game loop), replaced the same day by authoring first. Recover with `git show e43fbbf:PLAN.md`.
- `design/ARCHITECTURE.md` › Next steps, moved to `PLAN.md`. Recover with `git show 884ae96:design/ARCHITECTURE.md`.
- The notes at the top of `TESTING.md` and the testing plan explaining what their earlier versions got wrong. That history is under 0.1.0 below.

## 0.5.1 (2026-11-23)

### Added

- `validate`, `show` and the console warn when two kinds share an id (a location and an object, say), because the console's `cd <id>` can't tell them apart. It's a warning, not a problem: ids stay scoped by kind. It appears in `WorldLoadReport::warnings` and `WorldEngine::current_warnings()`, and `WorldEngine::shared_ids()` lists the ids.

## 0.5.0 (2026-11-23)

### Added

- In-memory changes to a loaded world (REQ-ENG-8). `WorldEngine::set_file(path, text)` replaces whatever a file defined with what new text defines, and `remove_file(path)` forgets a file. Neither touches the disk. Duplicates, last-one-wins and parse errors resolve exactly as a fresh load would, because the engine now keeps each file's parse result in load order.
- `WorldEngine::current_problems(strict)` and `current_warnings()` check the world as it is now, with no load report needed. Right after a load they equal `problems(report, strict)` and `report.warnings`.
- `WorldWriter::edit_loaded` and `create_loaded` write the file and then apply the same text to the loaded engine. Both return a `ChangeReport` with the path written, every problem after the change, and the problems it `added` and `resolved` (REQ-CON-5). A refused or failed write leaves the engine untouched.
- `WorldEngine::world_root`, the directory `load_world` walked.
- 10 tests. `validate` and `show` output is unchanged.

## 0.4.0 (2026-11-23)

### Added

- `thwarter::writer`, the round-trip file writer (REQ-ENG-7). `ContentFile` edits one file's frontmatter as text, one field at a time: `SetScalar`, `SetList`, `SetMapEntry`, `RemoveMapEntry`, `RemoveKey`, `SetBody`, and `ReplaceAll` for full editing. Field order, comments, spacing, line endings, unmodelled fields and the body stay as they were.
- The writer checks every edit by re-parsing. The frontmatter must read back as exactly the old mapping with that one change, and the file must still parse as the same entity, or the edit is refused and nothing changes. It also refuses edits to YAML shapes it doesn't handle (block scalars, non-empty flow maps, nested list items, anchors), and changes to `id` or `type`.
- An added key goes in `design/CONTENT_FORMAT.md` table order, with extra keys at the end. Existing order never changes.
- `WorldWriter`: `edit` writes to the file an entity was loaded from, refusing duplicate ids and files that changed since the load. `create` writes a new entity at `<id>.md`, `npcs/<id>.md` or `objects/<id>.md` (`writer::new_entity_relative_path`, the one place that rule lives). Both refuse ids and symlinks that would land outside the world.
- Saves are atomic: a temporary file beside the target, renamed over it, keeping the original's permissions. A new file is linked into place, so it never overwrites one that appeared meanwhile.
- 31 writer tests. They cover byte-identical round trips of every content file in the repo, a one-field edit of every entity in the test world, each operation and refusal, CRLF files, quoting, permissions, the path guard, and the safety net itself.

## 0.3.0 (2026-11-23)

### Added

- The console watches the world's files and reloads when they change (REQ-CON-7). It prints what changed and what the reload said above the prompt, with the same rules as `/reload`. Only `.md` files and `world.toml` count; dotfiles and editor backups don't. Changes are settled for 300 ms, so one editor save is one reload. It watches only on a terminal, so piped runs and transcripts aren't affected. If watching can't start, the console says so, and `/reload` still works. New dependency: `notify-debouncer-mini`.

## 0.2.3 (2026-11-23)

### Added

- Line editing and history at the console prompt on a terminal: arrow keys, Ctrl-R history search, Ctrl-C to clear the line, and Ctrl-D to leave. It uses `reedline`, which is built on crossterm, the same backend ratatui uses, so a full-screen editor could share the terminal (`PLAN.md` › Open question 1). History lasts for the session. Piped input still runs the plain transcript loop. The `reedline` dependency itself landed in f5fe6aa, swept into that commit from the working tree.

### Changed

- Edit mode's file index comes from `WorldEngine::sources` instead of a second parse of every file. A `.md` file the engine didn't load is listed as `(doesn't parse)`. Key-case warnings (`WorldLoadReport::warnings`) count among the console's warnings.

## 0.2.2 (2026-11-23)

### Added

- `validate` and `show` warn about a frontmatter key that differs only in case from one its type defines, such as `Inventory:` for `inventory:`. A warning doesn't fail validation. Other unknown keys aren't warned about, since objects may carry any (REQ-CF-6).
- `WorldLoadReport::warnings`, the file-prefixed warnings from a load, kept apart from `errors` and from `WorldEngine::problems`.
- `parsers::known_keys(type)`, each type's keys in `design/CONTENT_FORMAT.md` table order. It's the one list the parser and the writer share. `MarkdownParser::parse_file_with_warnings` and `parse_content_with_warnings` return the warnings with the entity.

## 0.2.1 (2026-11-23)

### Added

- `WorldEngine::sources`, every file each `(EntityKind, id)` was loaded from, and `source_path(kind, id)`, which fails when an id is unloaded or defined twice. The round-trip writer and the console's folder navigation use it.
- `EntityKind` (`Location`, `Character`, `Object`). An id is unique within a kind.
- `validate`, and the console's `/reload` through `WorldEngine::problems`, report an id defined in more than one file. Before this, the last file loaded won silently.

## 0.2.0 (2026-11-23)

### Added

- The console loop behind `thwart edit` and `thwart play` (REQ-CON-2). `/edit` and `/play` switch modes and keep the current room, `/help` lists the commands for the current mode, and `/quit` (or end of input) leaves. `/edit room|object|character`, `/new`, `/set` and `/link` say they aren't built yet (Phase 3), and play mode says the same of its verbs (Phase 4).
- Edit-mode navigation of the world's folders, as plain input (REQ-CON-4): `ls`, `cd <folder, room, character or object>`, `cd <direction>` (with `n`, `ne`, ... `u`, `d`), `cd ..`, `cd /`, `pwd` and `look`. From a room, `cd <id>` reaches the characters and objects it holds, and an object with no card is reported as scenery.
- `/reload` (REQ-CON-7). It keeps the current room. It keeps the previous world if a file fails to parse or the current room is gone, and it shows broken references as warnings without blocking.
- Rooms in the console use the shared renderer (`render::render_location`, REQ-CON-8). Characters and objects get a short card view.
- Transcript tests (REQ-CON-9): `tests/transcripts.rs` runs each `tests/transcripts/*.txt` session against the `worlds/tiny` fixture. `THWART_BLESS=1` accepts new output. Also 4 unit tests for the `/reload` cases that change files mid-session.
- `console::run_script(world, mode, input)`, which runs the console over scripted input and returns its output.

## 0.1.6 (2026-11-23)

### Added

- Parser unit tests (`design/UNIT_TESTS.md` › 1), 16 of them. They cover each entity type's fields, missing `type` and required fields, unclosed or absent frontmatter, invalid YAML, a `---` inside the body, and the capitalized `Inventory:` key, which is not read.
- `MarkdownParser::parse_content(text, path)`, which parses text already in memory. `parse_file` is now a read followed by this.

## 0.1.5 (2026-11-23)

### Added

- `thwart edit [world]` and `thwart play [world]`, which open the console in edit or play mode through `thwarter::console::run` (REQ-CON-1). The world is the positional argument, then `--world`, then the default. Until the console loop lands they report that it isn't implemented and exit non-zero.

## 0.1.4 (2026-11-23)

### Added

- `WorldEngine::describe_location`, returning a `LocationView` (title, body, exits with whether each target exists, characters and objects with their card names, `None` for no card), as agreed with the console for REQ-ENG-9.
- `thwarter::render::render_location`, the room view shared by `thwart show` and the console (REQ-CON-8). It returns plain text without colour.
- `WorldEngine::problems(report, strict)`, the check behind `thwart validate`, for the console's `/reload` to share (REQ-CON-7).
- `WorldEngine::characters`, the loaded character cards by id.
- The first unit tests: the renderer, `describe_location` and `problems`.

### Changed

- `show` and `validate` call those shared functions. Their output is unchanged.

## 0.1.3 (2026-11-23)

### Added

- `src/console/`, the console's entry point: `console::run(world_path, Mode::Edit | Mode::Play)`, which `thwart edit` and `thwart play` will call (REQ-CON-1). It loads the world, then reports that the console isn't implemented yet; the loop follows in Phase 2.

## 0.1.2 (2026-11-23)

### Changed

- `show <location>` labels an object with no card `(no card)` instead of `(not found)`, since such an object is allowed scenery (REQ-CF-8). Characters with no card still say `(not found)`, and exits to missing rooms still say `(missing)`.

## 0.1.1 (2026-11-23)

### Fixed

- `--world`, `--config`, `-q` and `-v` are accepted after the subcommand (`thwart validate --world <path>`), as `README.md` and `TESTING.md` write them. They were top-level only.
- `validate` prints its failure once. The summary line stays; `main` no longer prints the returned error again, and still exits non-zero.
- `show <location>` in text format prints the room for an author to check: name and id, coordinates, description, exits in compass order, then characters and objects by name. Exits to unknown rooms say `(missing)`; character and object ids with no card say `(not found)`. JSON and YAML output are unchanged.

### Changed

- `show`'s own `--verbose` is replaced by the global `-v`/`--verbose`, which `show --verbose` still reaches.
- `thwart --version` reads the version from `Cargo.toml` instead of a hardcoded string.

## 0.1.0 (2026-08-24)

The August audit (2e4d080), plus files it left uncommitted (6aa58de, 884ae96). The version number dates from July 2025 but wasn't bumped until this file began.

### Fixed

- The project didn't build: `Cargo.toml` had `[bin]` instead of `[[bin]]`, from d6b9e1b.
- The parser couldn't read any sample file: it expected frontmatter at the top and every sample file had it at the bottom. Nothing called the parser, so no one noticed.
- `Cargo.lock` was gitignored; it's now tracked, as a binary crate's should be.

### Changed

- The content format: frontmatter moved to the top of each file, and `title` moved from an H1 in the body into frontmatter. All 27 sample files were migrated. See `design/DECISIONS.md`.
- World content lives at `authors/<author>/<world-name>/`. Docs had pointed at `authors/world/`, which never existed.
- `validate` and `show` do real work against the loaded world.
- `map`, `search` and `debug` say they're not implemented and exit non-zero.
- `design/DESIGN.md` split into `design/ARCHITECTURE.md`, `design/CONTENT_FORMAT.md` and `design/DECISIONS.md`, with `REQUIREMENTS.md` consolidating the requirements.

### Removed

- Fabricated output from every CLI command. `map` printed a hardcoded 3x3 grid, `search` three invented results, and `debug` made-up memory and timing figures, whatever the input. Recover with `git show d6b9e1b:src/cli/commands/map.rs` (also `search.rs`, `debug.rs`).
- The compiled `bin/thwart` binary from git; `build.sh` rebuilds it and it's gitignored.
- Three disagreeing content format specs (`README.md`, `design/DESIGN.md`, and the sample content each differed), replaced by `design/CONTENT_FORMAT.md`. Recover with `git show d6b9e1b:design/DESIGN.md`.
- `TESTING.md`'s list of flags that never existed in the CLI (`--fow`, `--check-refs`, `--scale`, `--with-npcs`, `--all-endings`, `--trace-movements` and more), and its profiling workflows. Recover with `git show d6b9e1b:TESTING.md`.
- A unit testing plan for 200+ tests, 95% coverage and six weeks of work, written when the project had no tests and didn't compile. Recover with `git show d6b9e1b:tasks/unit-testing-plan.md`.
- `tasks/ISSUES.md`'s report of a passing test run with memory and timing figures, dated the day of the first commit; none of it measured anything. Recover with `git show d6b9e1b:tasks/ISSUES.md`.
- `CLAUDE.md`'s claim that P0 was complete.
