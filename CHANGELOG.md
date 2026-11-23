# Changelog

Thwarter's changes, newest first. This file also records what was removed or replaced: deleted files, retired formats, superseded plans and their history. Each removal names the commit to recover it from, so the working docs can describe only what's true now.

Versions are `0.x.y` semver, pre-1.0. Sections are `### Added`, `### Changed`, `### Fixed` and `### Removed`. Changes not yet in a versioned build go under `## Unreleased`.

## Unreleased

### Added

- `PLAN.md`, the phased work queue, and `BUGS.md`, the defect log, both in Space Command's task format.
- This changelog.
- `design/STATES.md`, a draft design for states, bundles and chapters. It replaces the earlier idea of conditional text in shortcodes and conditions on exits, which was never written into a doc.
- `.claude/hooks/check-commit-message.sh`, which refuses a commit that credits Claude as a co-author. Copied from warped-space-command.

### Changed

- The phase plan: authoring first, then play, in one console with an edit and a play mode; engine work only as they need it (Bruce, 2026-11-23). `map`, `search` and the remaining `validate` checks moved to a "Tooling, as needed" phase, and tests moved into each phase.
- `REQUIREMENTS.md` gains §4.6 Console (REQ-CON-1 to 11), the file writer and in-memory changes (REQ-ENG-7 to 9), and objects without cards as allowed scenery (REQ-CF-8). The separate P1 play REPL and P2 editing REPL are now one console.
- `tasks/unit-testing-plan.md` moved to `design/UNIT_TESTS.md`.
- `tasks/ISSUES.md` became `BUGS.md`. Its open items that are missing features, not defects (no tests, `map`/`search`/`debug`, `CharacterEngine` and `ConfigManager` not wired in), moved to `PLAN.md`.

### Removed

- `REQUIREMENTS.md` › 4.6 Console, moved to `design/CONSOLE_REQUIREMENTS.md` with the console's open questions from `PLAN.md` (Bruce, 2026-11-23). The REQ-CON ids are unchanged. Recover the old section with `git show 7a10a8d:REQUIREMENTS.md`.
- `tasks/cli-first-pass-TODO.md`, the original 20-day, six-phase plan. It described a `src/lib/` layout that never existed. Replaced by `PLAN.md`; recover with `git show 884ae96:tasks/cli-first-pass-TODO.md`.
- The tooling-first phase order (Tests, Static-world tooling, Game loop), replaced the same day by authoring first. Recover with `git show e43fbbf:PLAN.md`.
- `design/ARCHITECTURE.md` › Next steps, moved to `PLAN.md`. Recover with `git show 884ae96:design/ARCHITECTURE.md`.
- The notes at the top of `TESTING.md` and the testing plan explaining what their earlier versions got wrong. That history is under 0.1.0 below.

## 0.15.0 (2026-11-23)

### Added

- `/restart` (REQ-CON-30, 31): the console restarts onto the `thwart` binary now on disk, in the same terminal, and puts you back where you were: the mode, edit mode's place, the running game and the input history (Up still recalls what you typed before). Build in another pane (`cargo build`), then `/restart`. It checks the new binary starts before handing over, and stays on the running one if it doesn't. The session goes through a resume file under `$XDG_STATE_HOME/thwarter/restart/`, used once; the game travels as an ordinary save, so a save format the new build can't read resets the game with a notice, and a place that's gone moves up a folder. In scripts and transcripts, which have no terminal, `/restart` says so.

## 0.14.0 (2026-11-23)

### Added

- Actions are mechanics in frontmatter and responses in prose (`design/STATES.md` › Changing state, Responses). `actions: { pull: { becomes, sets, chapter, requires: { has } } }` holds what an action does. Its reply is a body block after `<!-- thwarter:action pull -->`, and the reply when `requires:` isn't met is a block after `<!-- thwarter:action pull refused -->`. A block with no mechanics is a whole action. `becomes:` switches things to states by bare id, `sets:` sets values by full name (`this.` means the object itself), `chapter:` moves the story to a chapter seen from the object's folder, and `requires: { has: }` names objects the player must carry.
- Bodies merge block by block: a layer's description replaces the description below it, and each response block replaces only its namesake. A folder's chapter overlay adds its description after each thing's beneath it, rather than replacing it, so "the castle in mourning" reads as each room's own text plus the castle's. Its frontmatter still sits below the thing's.
- `PlayAction::Use { verb, noun }` runs an object's named action in play, for an object carried or in the room. With no response block, play makes a reply from the verb and the object's name ("You pull the iron lever."), varied by turn, and the same for a refusal. A target that can't be switched is skipped with an author note. `Change` gains `Became`, `Set` and `EnteredChapter`. Examine shows an object's `examine` response after its description.
- `validate` reports what an action names that doesn't exist: a `becomes:` thing or state, a `chapter:` not visible from the object, or a `requires: { has: }` object. A bare id shared by two kinds is reported as ambiguous. It warns about an action with no response, a `requires:` with no refusal, and a response with no mechanics.
- The "Which do you mean" reply is `GameEngine::ask_which(verb, candidates)`, replaceable per story.

### Changed

- The test world's 9 object cards move their action strings into response blocks, each ending with a full stop. Frontmatter `actions` written as strings, `text:`, or the old list of one-key maps are now load errors that say where the reply goes.
- Locations and characters read only the description part of the body, before any response block.
- `Edit::SetBody` checks the body it wrote by reading back the raw body, response blocks included.

## 0.13.0 (2026-11-23)

### Added

- Play sees each thing's current version: its state in the game, in the game's chapter (`design/STATES.md` › Current version). `WorldEngine::describe_location_at`, `location_at`, `object_at` and `character_at` take an `At` (the starting versions, or a game's), and the play verbs use them, so a room's exits, prose and contents follow its state and chapter. A room never shows an object the player carries, so a chapter that adds one can't duplicate it.
- `PlayState` keeps each thing's state as a delta from its starting state, the current chapter, values by full name (`story.king`), visits per room and the turn count. `switch_state` and `enter_chapter` change them; both can go back as well as forward. Its unused `flags` are gone.
- Saves are `thwarter_save: 2`. Version 1 saves still load. On load, a state that no longer exists falls back to the starting state, a thing that's gone loses its entry, and a chapter that's gone falls back to the nearest earlier one in its scope, each with a warning.
- `thwart show <location> --state <name> --chapter <name>` shows another version of a location. `--state` used to be a placeholder for loading a saved game, which was never built.

## 0.12.0 (2026-11-23)

### Added

- Bundles, states and chapters load (`design/STATES.md` › Engine rules › Loading). A folder holding `index.md` is one thing; its other files are its states, and its `chapters/` folder holds its chapter overlays. In any `chapters/` folder, a numbered file (`20-after-the-king.md`, or a numbered folder with `_index.md` for one with sub-chapters) defines a chapter in that scope, and an unnumbered one is an overlay for the chapter of that name, found from its folder outward to the root. `_index.md` marks an area.
- Layers merge as raw YAML before typing (`engines/layers.rs`): maps merge key by key all the way down, scalars and lists replace, `null` removes an inherited key, and prose replaces the prose below only if something is left once author comments are removed. Every key remembers the file it came from.
- The world tables hold each thing's starting version (its starting state, no chapter). `WorldEngine::version(kind, id, state, chapter)` builds any other version, with `thing()` and `chapters()` to see what exists. Layers stack by lineage only: overlays from the folders above, the thing, its state, then its own overlays, a parent chapter before its sub-chapter.
- `validate` and the console's problems check every state against every chapter visible from the thing, and report misplaced files: state files that set `id`, `type` or `state:` or don't say `inherits: index`, a nested bundle, reserved names (`index`, `_index`, `chapters`, `templates`), a `state:` naming no state file, an overlay naming no chapter, an unnumbered root chapter, and a sub-chapter whose folder has no `_index.md`.
- New warnings: a layer that replaces an inherited list, an empty value that removes an inherited key, each overlay (it applies by where it sits), a bundle whose `id` doesn't match its folder, a scoped chapter that hides an outer one of the same name, and keys in an area's `_index.md` other than `title`.

### Changed

- The known keys (key order and case warnings) gain `inherits`, `state`, `vars` and `can`.
- `load_world` reads files in sorted path order, so which duplicate wins no longer depends on the file system.
- `world/` and `game/` at the root, any `templates/` folder, and files with `template: true` are kept out of the world, until their formats are built.

## 0.11.1 (2026-11-23)

### Changed

- The full-screen editor watches its file while open. A change on disk (a `git pull`, another editor) is taken in by itself when nothing in the editor is unsaved, with "Reloaded: changed on disk." on the status line. With unsaved edits, it warns once, and the next Ctrl-S saves over the change. This follows Bruce's rule in `design/STATES.md` › Engine rules: authors use git, so conflicts are rare and shouldn't need a second confirmation. A change that lands between that check and the write is still caught by the writer, and a second Ctrl-S overwrites it.

## 0.11.0 (2026-11-23)

### Added

- `/edit` opens a file that doesn't parse (any kind word works, since what the file holds is unknown), so an author can fix it in the console. Saving text that parses brings it into the world, and the save says what it now holds ("it parses now, as object `rope`"). Saves go through the writer's new `replace_file_loaded`, which also checks the file on disk still matches what the editor opened as it writes, closing the gap between the console's check and the write.

### Changed

- A full-edit save that changes the `id` or `type` is refused by the console with its own message, since ids can't be renamed yet.
- The console's parse-error list reflects files fixed since the load, so a fixed file no longer counts as blocking.

### Fixed

- The file-watcher test waited for the first notice, and on macOS that is sometimes one about a file written just before watching began. It now waits for the notice about the file it changed.

## 0.10.0 (2026-11-23)

### Added

- `WorldWriter::replace_file_loaded` replaces a whole content file found by its path in the world, not by its id, so the console can fix a file that doesn't parse. The new text must parse, and it may hold any id and type. The path must be an existing `.md` file inside the world; a path with `..`, an absolute path or a symlink out of the world is refused. Given the text the editor opened, it refuses the write if the file on disk no longer matches, which closes the gap between the console's own check and the write. Afterwards the engine holds the new file, so a fixed file stops being a load error, and the report says which problems the write fixed or added.

## 0.9.0 (2026-11-23)

### Added

- Full editing (REQ-CON-6): `/edit room`, `/edit object` and `/edit character` open the file edit mode is in, or the one named (`/edit character elder`), in a full-screen editor inside the terminal, built on ratatui and ratatui-textarea. Ctrl-S saves and stays open; Esc closes, asking once if there are unsaved changes. A save replaces the whole file through the writer, so text that doesn't parse, or that changes the `id` or `type`, is refused with the reason on the status line and the editor stays open. A file changed on disk since it opened is a conflict: a second Ctrl-S overwrites it. Each save's report, with any problems it adds, prints when the editor closes.
- Scenery gets its card on first edit (REQ-CF-8): `/edit object bench` for an object the room names without a card opens a new card with `object_type: scenery`, written on the first save.
- In scripts and transcripts, which have no screen, `/edit <kind>` says which file it would open.

### Changed

- The file watcher sends its notices after unlocking the console, so a burst of changes while the editor is open can't block its saves.

## 0.8.1 (2026-11-23)

### Changed

- Play replies lowercase object names mid-sentence ("You take the brass lantern."), even when the title is written in title case. Character names keep their case ("Which do you mean: King Aldric or Queen Aldric?"). Before this, a title with two capitalized words kept its capitals, so "Brass Lantern" sat oddly next to "the rope".

## 0.8.0 (2026-11-23)

### Added

- Play mode's verbs, v1 (REQ-CON-10): `look`/`l`, `go <direction>`, bare directions with `n` ... `nw`, `u`, `d`, `take`/`get`/`pick up`, `examine`/`x`/`look at`, and `inventory`/`i`. Articles are dropped (`take the lantern`). A bare `take` asks "Take what?", and anything else gets "I don't understand `...`." with no change. The engine's replies are player-facing, and its author notes print on their own line as `Author note: ...`.
- `/save [name]` and `/load [name]` (REQ-CON-11), `quicksave` by default, in `$XDG_STATE_HOME/thwarter/saves/<world>-<hash>/` (`~/.local/state` without it). The hash is a stable FNV-1a of the world's full path. Save names are letters, digits, `-` and `_` only. `/load` of a missing save lists the saves there are.
- `/play here` (REQ-CON-12) starts the game in, or moves the player to, the room edit mode is in.
- Transcripts: `play.txt`. The runner sends game saves to a temp folder, and fails a transcript that uses `/new`, `/set`, `/link` or `/unlink` without `# copy: yes`.

### Changed

- Play state and edit-mode navigation are separate (REQ-CON-12). `/play` resumes the game where it was, `/edit` returns to where edit mode was, and `cd` never moves the player. Play mode's room view is the player view (`RenderStyle::Player`).
- `/reload` refuses only when the player's room is gone, not edit mode's place. When edit mode's file or folder is gone, it moves up to the nearest folder that exists. After a reload or an authoring change, carried objects whose cards are gone are dropped, with a `Game:` warning.
- `modes.txt` now runs on a copy, because it uses `/set` and `/new`, even though both are refused there.

## 0.7.0 (2026-11-23)

### Added

- Play: `GameEngine::start(world, location)` and `GameEngine::act(world, &mut PlayState, &PlayAction) -> ActionResult` (REQ-ENG-5). Look, go, take, examine and inventory are implemented, in the shape agreed with the console. Playing changes only `PlayState` and never writes a world file (REQ-CON-3).
- `PlayState` (location, inventory, objects taken from each room, and flags, which stay unused until narrative state) and `PlayAction`. `ActionResult` has player `text`, its `changes` (`Moved`, `Took`) and an author `note`, for things like an exit to a room that doesn't exist yet. Any reply that isn't a success leaves the state untouched.
- Take works on an object with a card unless its `weight` is `immovable`. Scenery without a card can be examined but not taken. Nouns match an id, a title or a unique word, and two matches get "Which do you mean".
- `save_game(path, &PlayState)` and `load_game(path, &world)` (REQ-CON-11). Saves are versioned YAML written atomically to a path the console chooses. Loading reconciles with the current world. `PlayState::reconcile(&world)` drops carried objects that no longer exist and reports a vanished location, for `/reload` and mid-game edits.
- `render::RenderStyle { Author, Player }` and `render_location_as`. The player view shows names and directions only. `render_location` is unchanged.
- `title_from_id` (`stone-bench` → "Stone bench") and `expand_direction` (`n` → `north`) in `thwarter::engines`.
- 11 tests.

### Removed

- The stub `GameEngine::process_action` and its `ActionResult`, `StateChange` and `StateChangeType` models, which returned fixed text regardless of input. That removes 8 of the 11 compiler warnings. Recover with `git show 9584a28:src/engines/game_engine.rs` and `git show 9584a28:src/models/game/action.rs`.

## 0.6.2 (2026-11-23)

### Changed

- The room view (`thwart show` and the console) prints `(no description yet)` for a room with an empty body, as the console's character and object views do, instead of a run of blank lines.

## 0.6.1 (2026-11-23)

### Changed

- `/new`, `/set`, `/link` and `/unlink` apply each change to the loaded world in memory (`edit_loaded`, `create_loaded`, REQ-ENG-8) instead of reloading from disk after each save. The problems a command adds come from the engine's current problems and warnings, so they now include warnings such as a shared id.
- `/new object <id>` with only an id, for scenery the current room names, creates its card with `object_type: scenery`, a title from the id and an empty body (REQ-CF-8, `design/CONTENT_FORMAT.md` › What's still open). `cd` on scenery now suggests it. `/edit object` will use the same default once full editing exists.

## 0.6.0 (2026-11-23)

### Added

- Edit-mode commands that change one thing at a time (REQ-CON-5), from 656e143. Each saves through the writer, reloads, and reports any problems the change adds:
  - `/new room <direction> <id> [title]` creates a room through a new exit from the current one and moves there. Compass and up/down exits get a return exit; free-text ones like `behind` don't (`PLAN.md` › Open question 3, as proposed). Outside a room, `/new room <id> [title]` creates a room with no exits. An id without a `/` goes in the current folder.
  - `/new character <id> [title]` and `/new object <id> <type> [title]` create a card. In a room, they place it there and set a character's `default_location`. An object the room already names as scenery gets its card this way (REQ-CF-8).
  - `/set <key> <value>` sets one field of the current room, character or object. `/set attributes.health 80` sets one map entry, `/set inventory a, b` a list, and `/set body ...` the description. Numbers and booleans are typed.
  - `/link <direction> <room>` adds or changes an exit, and `/unlink <direction>` removes one.
- Transcripts can run on a throwaway copy of their world (`# copy: yes`), so authoring transcripts never change the fixture. New: `tests/transcripts/authoring.txt`.

### Changed

- The file watcher ignores files the console saved itself in the last 3 seconds, so a `/set` isn't announced twice.
- Characters and objects with no description show `(no description yet)`.

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
