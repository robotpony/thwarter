# Changelog

Thwarter's changes, newest first. This file also records what was removed or replaced: deleted files, retired formats, superseded plans and their history. Each removal names the commit to recover it from, so the working docs can describe only what's true now.

Versions are `0.x.y` semver, pre-1.0. Sections are `### Added`, `### Changed`, `### Fixed` and `### Removed`. Changes not yet in a versioned build go under `## Unreleased`.

## Unreleased

### Added

- `design/ENGINE_REQUIREMENTS.md` › REQ-ENG-58: the text guard covers `src/console/` too, since 1e40954.
- `design/CONSOLE_REQUIREMENTS.md`: REQ-CON-62, a world picker for `thwart edit` and `thwart play` when no world is given, in place of a built-in default.
- `design/CONSOLE_REQUIREMENTS.md`: the command registry is approved (REQ-CON-50 to 56), and a world command may be a Rhai `script` as well as a `run` list (REQ-CON-54). Authors are the machine user, in `authors:` (REQ-CON-34 to 36, 55).
- `design/CONSOLE_REQUIREMENTS.md` › To write, REQ-CON-57 to 59: a TO WRITE list in the sidebar, one entry per unwritten name, made with defaults from Cmd-1 to Cmd-9 or `/write`.
- `design/ENGINE_REQUIREMENTS.md` › Forgiving verbs: REQ-ENG-51 to 54, proposed, for aliases in a `[verbs]` table, the order a typed verb resolves in, unique prefixes that never beat an exact match, and did-you-mean for verbs only.
- `design/CONSOLE_REQUIREMENTS.md` › Command registry, REQ-CON-50 to 56, *Proposed*: one `type: command` Markdown file per command, loaded built-in, then world, then personal config; world commands as macros for the world's authors only; forgiving names.
- `LICENSE`: the MIT licence text, matching `Cargo.toml`'s `license = "MIT"`, copyright Bruce Alderson, 2025 to 2026.
- `design/SECTIONS.md`: agreed in outline by Bruce, to be tested; content is shown when a `## When:` heading's tags match the game state, reply sections replace actions and response markers, `@` names characters and `#` tags, and the marker comments retire.
- `design/ENGINE_TEXT.md`: every sentence the engine, play and the console write moves out of code into TOML text tables (`[play]`, `[engine]`, `[console]`) with compiled-in defaults, the same shape everywhere, at Bruce's request; planned as `PLAN.md` › Phase 8, which shifts the later phases up by one.
- `design/PLAYER_COMMANDS.md`: the player command language every client shares, REQ-PC-1 to 19, drafted from the console's play mode as built, at Bruce's request.
- `.github/workflows/ci.yml`: GitHub Actions builds every target and runs every test, with the lock file as committed, on each push and pull request.
- `design/CLI_REQUIREMENTS.md` › Packaging and installing worlds: REQ-CLI-12 to 16, for `thwart package`, `thwart install`, `thwart installed` and playing an installed world, agreed with the commander.
- `PLAN.md`, the phased work queue, and `BUGS.md`, the defect log, both in Space Command's task format.
- This changelog.
- `design/ENGINE_REQUIREMENTS.md` gains REQ-ENG-10 to 29, the engine rules settled during the states design (loading, chapters, building a version, ids and values, play and saves, writing bundles), and REQ-ENG-7 the writer's rule for files that name a template.
- `design/STATES.md`, a draft design for states, bundles and chapters. It replaces the earlier idea of conditional text in shortcodes and conditions on exits, which was never written into a doc.
- `.claude/hooks/check-commit-message.sh`, which refuses a commit that credits Claude as a co-author. Copied from warped-space-command.

### Changed

- Doc pass, at Bruce's request: `design/STATES.md`, `design/CONTENT_REQUIREMENTS.md` (REQ-CF-3, REQ-CF-17) and principle 7 describe `## When:` reply sections, not the retired marker comments; `design/CLI_DESIGN.md` matches the commands that exist and marks `do`, `test` and `create` not planned; `design/UNIT_TESTS.md` describes the tests as they are (the 2026-08 plan is `git show 2d36b93:design/UNIT_TESTS.md`); `TESTING.md` and `design/DESIGN.md` are current.
- The phase plan: authoring first, then play, in one console with an edit and a play mode; engine work only as they need it (Bruce, 2026-11-23). `map`, `search` and the remaining `validate` checks moved to a "Tooling, as needed" phase, and tests moved into each phase.
- `REQUIREMENTS.md` gains §4.6 Console (REQ-CON-1 to 11), the file writer and in-memory changes (REQ-ENG-7 to 9), and objects without cards as allowed scenery (REQ-CF-8). The separate P1 play REPL and P2 editing REPL are now one console.
- `tasks/unit-testing-plan.md` moved to `design/UNIT_TESTS.md`.
- `tasks/ISSUES.md` became `BUGS.md`. Its open items that are missing features, not defects (no tests, `map`/`search`/`debug`, `CharacterEngine` and `ConfigManager` not wired in), moved to `PLAN.md`.

### Removed

- `design/STATES.md` › Engine rules' text, now `design/ENGINE_REQUIREMENTS.md` › REQ-ENG-10 to 29 so there's one copy. Recover with `git show 1e2af89:design/STATES.md`.
- `REQUIREMENTS.md` › 4.1 to 4.4, split into one requirements document per component: `design/CONTENT_REQUIREMENTS.md` (REQ-CF, REQ-WS), `design/ENGINE_REQUIREMENTS.md` (REQ-ENG) and `design/CLI_REQUIREMENTS.md` (REQ-CLI), with ids unchanged; the principles moved from `design/STATES.md` to `REQUIREMENTS.md` › 4 (Bruce, 2026-11-23). Recover the old sections with `git show 1620350:REQUIREMENTS.md`.
- `PLAN.md`'s old Phase 4 (Play) and Phase 5 (Tooling), regrouped into Phases 4 to 9 with a "done when" each (Bruce, 2026-11-23). Recover with `git show e4e87e6:PLAN.md`.
- `REQUIREMENTS.md` › 4.6 Console, moved to `design/CONSOLE_REQUIREMENTS.md` with the console's open questions from `PLAN.md` (Bruce, 2026-11-23). The REQ-CON ids are unchanged. Recover the old section with `git show 7a10a8d:REQUIREMENTS.md`.
- `tasks/cli-first-pass-TODO.md`, the original 20-day, six-phase plan. It described a `src/lib/` layout that never existed. Replaced by `PLAN.md`; recover with `git show 884ae96:tasks/cli-first-pass-TODO.md`.
- The tooling-first phase order (Tests, Static-world tooling, Game loop), replaced the same day by authoring first. Recover with `git show e43fbbf:PLAN.md`.
- `design/ARCHITECTURE.md` › Next steps, moved to `PLAN.md`. Recover with `git show 884ae96:design/ARCHITECTURE.md`.
- The notes at the top of `TESTING.md` and the testing plan explaining what their earlier versions got wrong. That history is under 0.1.0 below.




























## 0.79.0 (2026-11-24)

### Added

- The world picker, `console::run_or_pick` (REQ-CON-62). When no world is given, a terminal shows the worlds `worlds::find_worlds` sees: ↑ and ↓ choose, Enter opens, Esc quits. Edit lists only the worlds you can write, and with none offers to make one in `authors/<you>/`. Play lists installed worlds too, and with none says how to install one. Without a terminal, the list goes back to the caller to print, as before. Its text is `[console]`'s `pick_*` keys.

## 0.78.0 (2026-11-24)

### Added

- Forgiving play verbs (REQ-ENG-51 to 54). Aliases live in `src/builtins/verbs.toml` (`l`, `x`, `inspect`, `get`, `walk`, `move`, `i`, `inv`), and a world adds its own in `world.toml`'s `[verbs]` (`yank = "pull"`). A unique prefix of two letters or more runs its verb (`exa pot`, `pu lever`), several ask ("Do you mean take or talk?"), and a verb one typo away gets "Did you mean \"look\"?", which runs nothing. Directions and exact verbs always win, so `in` is a direction. Only verbs forgive, never nouns, and a world verb counts only when a thing in scope answers to it.
- `GameEngine::verbs(world, state)`, every verb the player could type here, and `verbs_in_scope`, for clients.
- `validate` warns about a world alias that is a direction or a built-in verb, or whose verb nothing knows.
- `[engine]` keys `which_verb` and `did_you_mean`, with outcomes `#ambiguous-verb` and `#not-understood #suggest`, both reserved.

### Changed

- The parser's own alias list is gone; every alias comes from the table.

## 0.77.3 (2026-11-24)

### Changed

- `/new`, `/add` and moving a room no longer build their replies from pieces: each case is one whole `[console]` reply (`created_room_linked`, `created_object_placed`, `added_as_scenery`, `moved_room_updated` and the rest), so each can be reworded on its own. `/help`'s rows under each mode are `[console]` keys too. The default text is unchanged, except that when linking or placing fails, its line now comes before any new problems instead of after them.

### Removed

- The `[console]` keys `but_linking_failed` and `but_placing_failed`, replaced by whole replies per case. `added_to` no longer takes `{note}`.

## 0.77.2 (2026-11-24)

### Changed

- A console reply's kind (done, error, warning, note, refused) comes from its `[console]` key, in `src/console/kinds.rs`, not from what it says. Output is coloured by matching each line against the author's current wording, the editor's status line keeps its kind with it, and `/recipe` stops on a step whose reply was an error or a refusal, now including "Can't" replies. Rewording a reply in `config.toml` keeps its colour and doesn't hide a failed step.
- "Usage:" and the "Game: " note prefix are `[console]` keys.

## 0.77.1 (2026-11-24)

### Changed

- The text guard scans `src/console/` too. The 13 console lines it still found (replies built around another reply, play mode's heading, the full-screen editor's status line, the watcher's notice) are `[console]` keys now, their default text unchanged.

## 0.77.0 (2026-11-24)

### Added

- Story overrides of built-in replies (REQ-ENG-57): a `## When:` section whose tags are exactly a reply's outcome tags rewords it, in the thing, its room or the chapter, before `[engine]`. `## When: #take #immovable` in an anvil gives "The anvil laughs at you."; `## When: #not-here` in a chapter rewords "There's no lamp here." The prose fills `{{ turn.noun }}`, `{{ turn.verb }}` and the reply's other slots. The parser's own replies follow the chapter's wording too.
- The outcome names (`#no-noun`, `#not-here`, `#immovable`, `#nothing` and the rest) are reserved tags, and `validate` warns about a state named like one.
- `tests/text_guard.rs` (REQ-ENG-58): fails on a sentence-like literal in `src/engines/` or `src/render.rs` outside tests, tooling call sites and `tests/text_allowlist.txt`, and on an allowlist line that matches nothing. `src/console/` joins it once its last sentences move.

## 0.76.0 (2026-11-24)

### Added

- The author's `config.toml` `[console]` table rewords the console's replies; an unknown key or a wrong kind is a warning in the console's greeting, and `[console.commands]` stays the command layer's (`PLAN.md` › Phase 8).
- `tests/text_allowlist.txt`, the guard test's allowlist, with the console's entries.

### Changed

- The console's sentences come from `[console]` keys in `src/builtins/text/console.toml` (about 160 keys), with the default text unchanged, so the transcripts don't move.

## 0.75.0 (2026-11-24)

### Added

- `worlds::resolve_world` and `worlds::find_worlds` (REQ-CLI-21): a command finds its world from `--world`, then a default the person set (`[world] path` in `thwart.toml` here, then in `config.toml`), then the world the current folder is in. Otherwise it lists the worlds in `authors/*/*` and the installed ones, with paths and titles, says "Name a world with --world <path>." and exits non-zero. The error is `WorldNotGiven { found }`, for the console's picker.

### Removed

- The built-in default world, `authors/mx/test-world`: `DEFAULT_WORLD_PATH` in `src/bin/thwart.rs` and `ThwartConfig`'s default path. Recover them with `git show 14ded1a:src/bin/thwart.rs` and `git show 14ded1a:src/cli/config.rs`.

## 0.74.2 (2026-11-24)

### Fixed

- The `[play]` reference copy the tests check is now in `tests/worlds/sample/world.toml`, not Bruce's test world, whose copy is his to reword.

## 0.74.1 (2026-11-24)

### Fixed

- The validate test loaded Bruce's `authors/mx/test-world`, so a draft with an exit to a room not written yet (`master-bedroom`) failed it. It now checks the `tests/worlds/sample` fixture, `--strict` too.

## 0.74.0 (2026-11-24)

### Added

- The command registry, first part (REQ-CON-50 to 53, 56; Bruce approved): each built-in command is one `type: command` file in `src/builtins/commands/`, with its aliases, `about`, `args`, modes, whether it changes content, its handler, and its help as the body. The `/` menu, `/help`, the edit-mode guard and the transcripts' list of commands that change content all read it. A test fails if a file names a handler that isn't dispatched or a handler has no file.
- Forgiving names: aliases (`/rm`, `/q`, `/mv` and others), a unique prefix (`/insp`), "could be" for a prefix several share, and "did you mean" for a near miss. `/help <command>` shows one command in full.

### Changed

- `/help` lists every command from the registry with its usage; the hand-written edit-mode command lists are gone, since they were a second copy. Arguments a command doesn't take answer with its usage from its file.

### Not yet

- World commands (`world/commands/`, REQ-CON-54, 55), the personal `[console.commands]` layer (REQ-CON-52), checking argument counts against `args` before a handler runs, and bare command names without the slash.

## 0.73.3 (2026-11-24)

### Removed

- `engines::door_id(from, to, direction)`, the old door id named from one side, now that `/add door` uses `WorldEngine::new_door_name` (REQ-ENG-47). Recover it with `git show faebdce:src/engines/mod.rs`.

## 0.73.2 (2026-11-24)

### Changed

- `/add door` names a door with the engine's `new_door_name` (the coder's faebdce): `door-<room>-<room>`, the same from either side, numbered when taken, and titled from the far room ("Hall door"). Existing doors keep their ids. REQ-CON-42 updated; transcripts reblessed.

## 0.73.1 (2026-11-24)

### Changed

- The sidebar's exits are one per line with an arrow to a common column (`east -----> bedroom    closed door`), so a direction, its room and a door no longer run together (Bruce). A door shows only when it's in the way; its name and id are `/inspect`'s. A line too long for the sidebar is cut with an ellipsis.

### Added

- `/inspect <direction>`: the exit from here, where it goes, and its door in full.

## 0.73.0 (2026-11-24)

### Added

- `engines::door_id_between(room, other)` and `WorldEngine::new_door_name(here, to, template)`: a new door's id is `door-<room>-<room>`, the rooms sorted so either side makes the same id, with `-2` and on for a second door between them, and its title comes from the template when it gives a real one ("Iron door"), else from the far room ("Bedroom door") (REQ-ENG-47). Existing doors keep their ids.

### Changed

- `engines::door_id(from, to, direction)` is kept only until the console's `/add door` moves to `new_door_name`.

## 0.72.1 (2026-11-24)

### Changed

- The world settings view lists every author from `authors:` (the coder's 14ce024), and the console takes the machine user from `settings::current_user()`. The `/start` transcript is back now the persona problem is gone.

## 0.72.0 (2026-11-24)

### Added

- `authors:` in `world/index.md`, one id or a list (`authors: mx`, `authors: [mx, alice]`), read into `WorldDef::authors` (REQ-WS-5, REQ-ENG-56). `WorldDef::author` stays as the first one for clients still reading it.
- `WorldEngine::is_author(user)`, true when `user` is in `authors:` and the world isn't installed, and `settings::current_user()` (`$THWART_USER`, `$USER`, `$USERNAME`) for the console's author checks (REQ-WS-7, 8). `WorldEngine::personas()` gives every author's persona.
- `validate` warns about a `world/index.md` with no `authors:`, and suggests renaming an old `author:` (REQ-CLI-20).
- `thwart package` ships `<id>/index.md` for every author with a persona file, listed in the manifest's `authors`; `install` puts each in `worlds/<id>/index.md` (REQ-CLI-12). A thwart older than this refuses a package with more than one author's file.

### Changed

- An author with no persona folder is no longer a warning: the id is the name (REQ-WS-5).
- `design/ENGINE_REQUIREMENTS.md` › Forgiving verbs: REQ-ENG-51 to 54 are approved, not yet built.

## 0.71.0 (2026-11-24)

### Added

- `/start` makes the room you're in the world's start (REQ-CON-60, Bruce). With no `world/index.md`, it makes one: the title, `authors:` with the machine user (`THWART_USER` stands in for `$USER` in tests), and `start:`.
- `/edit world` in a world with no `world/index.md` offers to make it (y/n) and opens it, instead of only saying it's missing.

### Changed

- In edit mode, an exit to a room not written yet is a TO WRITE entry only, not a problem in the sidebar (REQ-CON-61); play mode and `/validate` still show it.

## 0.70.2 (2026-11-24)

### Fixed

- TO WRITE keys did nothing in the editor (Bruce). They now save the open document and switch to the entry; if the save fails, the editor stays with the reason. The editor binds no Ctrl- or Alt-digit, so nothing clashes. REQ-CON-59 updated.

## 0.70.1 (2026-11-24)

### Changed

- TO WRITE keys are Ctrl-1 to Ctrl-9, shown ⌃1 (Bruce, 2026-11-24), with Alt-1 to 9 still working; Cmd-digits go back to the terminal. A terminal without the kitty keyboard protocol sends control codes for Ctrl-digits, which are never read as these keys, so Ctrl-3 stays Esc there. REQ-CON-59 updated.

## 0.70.0 (2026-11-24)

### Added

- TO WRITE (REQ-CON-57 to 59, Bruce, 2026-11-24): the sidebar lists each name the world mentions but doesn't define, once, with a count and a key, from the coder's `not_written()` (999e382). Cmd-1 to Cmd-9, or Alt-1 to Alt-9 in a terminal that keeps Cmd, and `/write [<name>]` make it with defaults: a room if an exit leads there, else a scenery card listed in the first room, thing or person whose file names it. Then it opens in the editor. Both modifiers always work; choosing one in `config.toml` isn't built yet.

## 0.69.0 (2026-11-24)

### Added

- `WorldEngine::not_written()` and `Validation::not_written` (REQ-ENG-55): every name a link, `@name` or exit points at that nothing defines, one entry per name with each mention's file, line, holder and kind, for the console's TO WRITE list.
- `validate` lists them under `Not written yet (n):`, apart from warnings and not counted as any (REQ-CLI-19). Bruce's test world goes from four warnings for two names to none, with two names listed.

### Changed

- A prose link or `@name` that names nothing is no longer a warning. An ambiguous one still is. An exit to nothing is still a problem, and is listed too.

## 0.68.1 (2026-11-24)

### Fixed

- The `/` menu test expected the command list from before `/remove`; 497f603 was committed with it failing.

## 0.68.0 (2026-11-24)

### Added

- `/remove <thing>` (REQ-CON-49, Bruce, 2026-11-24): deletes an object, room, character or door and the references to it, on the coder's `plan_remove` and `remove_loaded` (904a93f). It shows the plan and asks y/n first, since there's no undo; any other line keeps the thing. A room that still holds things, or the start room, is refused with what to do first. New transcript `removing.txt`.

## 0.67.0 (2026-11-24)

### Added

- Removing a thing, for the console's `/remove` (REQ-ENG-50). `WorldWriter::plan_remove` says what will go and what will change, writing nothing: the thing's file or whole bundle, each file whose references change ("objects: drops lamp", "connections.down removed"), and what's left for the author. `remove_loaded` does exactly that. An object or character leaves every list; a door takes the exits through it; a room takes the exits into it and every door with it on a side. A room that still holds people or things, and the world's start room, are refused with what to do first. Prose links, actions' `becomes:` and `requires:`, and a door's `key` are reported, not rewritten.
- `WorldEngine::script_paths()` lists the world's `actions.rhai` files.

## 0.66.3 (2026-11-24)

### Fixed

- A new object landed in two rooms: `/add desk hallway` from the bedroom put the desk in both, because the question runner made its card with `/new object`, which also places. The card is written without placing now.
- `/add` of a thing that's already somewhere asks to move it here (y/n) instead of putting it in a second place (REQ-CON-41, REQ-CF-31). A y/n question takes only y or n; any other line drops it and runs as typed.

## 0.66.2 (2026-11-24)

### Changed

- The room's title and id lead the sidebar's HERE ("HERE  Village Square (village/square)") and play's ROOM, since the room view now shows only what the player reads (Bruce, 2026-11-24; the coder's f17588d drops its title line). HERE also gives each person's and thing's state, the game's when one is running (`state_at`, 627f899): "lantern  hanging". Transcripts reblessed.

## 0.66.1 (2026-11-24)

### Changed

- Edit mode's room view and `thwart show <room>` drop their title line (Bruce: "Titles are for search, listing in inventory, etc."). They're exactly play's view: the room's name shows only if `[play] show_room_names` says so, in both modes. The title and id stay in the `LocationView` for the console's sidebar.

## 0.66.0 (2026-11-24)

### Added

- `EntityRef::state`: each person, thing and held thing in a `LocationView` carries its state, the game's in play and its starting state outside, so the console's sidebar can show "lantern  hanging" without a lookup per thing. `WorldEngine::state_at(kind, id, at)` gives it for any thing. Asked for by the console coder.

## 0.65.0 (2026-11-24)

### Added

- `/inspect [<thing>]` (REQ-CON-48, Bruce, 2026-11-24): one thing or door in full, for chasing a warning like "tshirt starts in more than one place". It shows the file, scope, where it starts (from `placement()`), its templates, states (the starting one marked), actions by state, `## When:` and `## Unless:` sections, a door's sides, its state in a running game, and every problem or warning that names it. With no name, the thing edit mode is at.

### Changed

- The sidebar's exits name a door by title, id and state (in the game, else its starting state): "down  vault  through Iron door (vault-door), closed". With the coder's da8728b, edit mode's room view is play's prose under its title, and the sidebar and `/inspect` hold the author's detail. Transcripts reblessed.

## 0.64.3 (2026-11-24)

### Changed

- Edit mode's room view and `thwart show <room>` read exactly as play does, under the room's title and id (Bruce, 2026-11-24). The indented detail lines from 0.64.1 and the coordinates line are gone: ids, exit targets, doors and their state, and what isn't written are the console's to show from the `LocationView`, in its sidebar and `/inspect`. The `[console]` `room_*` keys now belong to the console for that.

## 0.64.2 (2026-11-24)

### Changed

- Edit mode's room view uses the world's own text tables with the author's `[console]` over them (`render_location_with`, the coder's 4fdf9b3), so it reads in the world's `[play]` sentences while it's being written, with the author detail indented under each. Transcripts reblessed for the new view.

## 0.64.1 (2026-11-24)

### Changed

- Edit mode's room view and `thwart show <room>` read as the world does (Bruce's playtest): no "Exits:", "Characters:" or "Objects:" headings. Who's here, what's here (scenery too) and the ways out are the world's `[play]` sentences, each followed by the author's detail as indented lines from new `[console]` keys: ids, where each exit leads, and what isn't written yet ("(not written yet)", was "(missing)" and "(not found)").
- A door exit leads with the door's title, then its id, then its state: "down   castle/vault through Iron door (vault-door), locked". Before, it showed only the id and "not passable".
- `render::render_location_with(view, texts)` words the view with a world's `[play]` and the author's `[console]`; `render_location` uses the defaults. A door's `DoorView::state` is its starting state outside play.

## 0.64.0 (2026-11-24)

### Changed

- Text as data, step 2: every built-in reply the engine writes for a player is an `[engine]` key in `src/builtins/text/engine.toml`, its wording unchanged, each commented with its outcome tag and who may replace it. Parser replies, missing nouns, refusals, the locked and closed exits, ambiguity questions, search finds and "Nothing happens." all read from it, and every reply picks a list's wording by turn. A world rewords any of them in `world.toml`'s `[engine]` table.
- `[play]` gains `see_through` ("Through the {door} you can see the {room}.") and `no_description` ("(no description yet)"), which were literals in the room view. The test world's reference copy lists them.
- `GameEngine::ask_which` is now an `Option`: `None` uses `[engine] which`.

## 0.63.2 (2026-11-24)

### Fixed

- The sidebar lost characters where a long row wrapped ("play shows it i" for "in"): its wrapped rows were indented two more after wrapping and ran past the edge. They're wrapped two narrower now.

### Changed

- Air around every block of output (Bruce, 2026-11-24): a blank line after each reply as well as after what was typed, in every mode, and never two together.
- `code` in the output and the sidebar shows in the code style without its backticks.
- `design/CONSOLE_REQUIREMENTS.md`: the examples use the `in <place> > ` prompt, the output principle covers blank lines before and after every block and backticks, REQ-CON-24 says `look` not `ls`, REQ-CON-46 has the id-template-kind lookup and `set <key> [<value>]`, REQ-CON-34 names `$XDG_CONFIG_HOME`, and open question 5 is closed.

## 0.63.1 (2026-11-24)

### Changed

- The console's wording starts moving into `[console]` (Phase 8, REQ-ENG-49): the console loads the built-in table with the author's own `[console]` from `config.toml` over it, and the replies in `session.rs` (unknown commands, reload, edit-mode navigation, `/validate`'s headings and the root listing's labels) now come from keys in `src/builtins/text/console.toml`. The wording is unchanged.

## 0.63.0 (2026-11-24)

### Added

- Text as data, step 1 (`design/ENGINE_TEXT.md`, REQ-ENG-49): `engines::texts::Texts` holds the `[play]`, `[engine]` and `[console]` tables, with compiled-in defaults from `src/builtins/text/*.toml`. `text(table, key, &args)` fills `{name}` slots (`{Name}` capitalizes), `text_at(.., turn)` picks a list's entry `turn % len`, and `apply(table, value, source, &mut warnings)` puts a table such as `world.toml`'s over the defaults, warning about unknown keys and wrong kinds. `WorldEngine::texts()` gives the world's.
- `[play]`'s defaults move to `src/builtins/text/play.toml`, unchanged; `PlayText` is now a typed view of that table. `[engine]` and `[console]` are empty until steps 2 and 3 fill them.

## 0.62.1 (2026-11-24)

### Added

- At the world's root, `look` lists the rooms first, then the areas ("area: 16 room(s)"), then the world's own folders labelled for what they hold: "Global objects: 4", "Global characters: 18", "Global doors: 1", "World settings", "Question sets: 2", "Templates", "Chapters" (REQ-CON-47, Bruce, 2026-11-24). `/edit world` opens `world/index.md`, or says the settings are in `world.toml` when there's none.
- A question's option with its value written in (`-> set title Brass lantern`, the coder's dc66a64) asks nothing and sets it.

## 0.62.0 (2026-11-24)

### Added

- A question's `set <key> <value>` option carries its value, `Construct::Set { key, value: Some(..) }`, so `- Brass lantern (default) -> set title Brass lantern` needs nothing typed; `set <key>` alone still asks (REQ-CF-32). The console's test constructor gains `value: None`, agreed with the console coder.

## 0.61.2 (2026-11-24)

### Changed

- The question picker chooses a set through the engine's `questions_for_thing` (d2665a8): the new thing's id, then its template's, then its kind's.

## 0.61.1 (2026-11-24)

### Changed

- A question set is chosen by the new thing's id, then its template, then its kind (`WorldEngine::questions_for_thing`), as CONTENT_FORMAT now says. A set named for nothing that exists yet is no longer a warning, so a world can write `questions/lantern.md` before any lantern, and a question with no options isn't one either. The warnings left are an unknown construct and more than one `(default)`.

## 0.61.0 (2026-11-24)

### Added

- The question picker (REQ-CON-46): when `/add` or `/new` makes something new, the console asks the nearest question set for it (the coder's `questions_for`, 79b367a): its own name first, as a template's set, then its kind's, then the built-in. Each question shows its hint and options; ↑↓ choose and Enter picks. An option that needs something asks for it: text for `set` and `ask value` (with its default filled in), a pick of what's in the room for `ask thing`. Esc takes the defaults for this question and the rest. The answers then run as the console commands they stand for, with edit mode at the new thing, and the output lists them under "The answers were:"; then the editor opens on the thing. Placing a thing that already exists asks nothing.

## 0.60.0 (2026-11-24)

### Added

- States and actions from the console (REQ-CON-44, Bruce, 2026-11-24). `/state <name> [start]` makes a state of the thing edit mode is at, turning a one-file thing into a bundle first, and `start` makes it the starting state. `/set <key> <value> in <state>` sets one key in one state's file. `/action <verb> <from> --> <to> [with <thing>]` puts the action in the `from` state's file, so it works only from there, makes the `to` state if needed, adds a `## When: #<verb> #<to>` reply section to the thing and opens the editor on it; `with` means the player must hold that thing.
- Recipes (REQ-CON-45), each shorthand for those verbs and printing the commands it ran: `/recipe fixed-until <verb> [<fixed> <free>]` (an immovable starting state until the verb), `/recipe locked-with <key>`, `/recipe switch <verb>/<verb> <state>/<state>`, and `/recipe container in|on`. New transcript: `making.txt`.

### Fixed

- Going to a bundled thing's folder (`go /village/square` once the square is a bundle) landed in the folder rather than on the thing; it now lands on its `index`.

## 0.59.0 (2026-11-24)

### Added

- Question sets (REQ-CF-32, REQ-ENG-48): the loader reads every `questions/<name>.md` (`type: questions`) as a set, never as a thing. Each `##` heading is a question, prose under it a hint, each list item an option with `(default)` and `->` or `→` before `set`, `recipe`, `ask thing` or `ask value`. `WorldEngine::questions_for(folder, template, kind)` gives the set to ask: the template's, else the kind's, each the nearest from the thing's folder out to the root, then the built-in. One built-in set, `object`, is the CONTENT_FORMAT example until Bruce reviews the content.
- `validate` reports a file in `questions/` without `type: questions` as a problem, and warns about a set named for no template or kind, an option with an unknown construct, more than one `(default)` under a question, and a question with no options.

## 0.58.3 (2026-11-24)

### Changed

- `/restart` greets in two lines (Bruce, 2026-11-24): "Restarted on thwart 0.58.3 (built 14:11)." then "You are precisely where you were when you left."

## 0.58.2 (2026-11-24)

### Changed

- An empty list isn't shown (Bruce): the author's room view leaves out "Exits: none", "Characters: none" and "Objects: none", `thwart show <object>` leaves out "Actions: none", and the world summary leaves out "Story values: none", "Chapters: none" and empty id lists.

## 0.58.1 (2026-11-24)

### Changed

- Edit mode's views leave out an empty list rather than saying "none" (Bruce, 2026-11-24): `Actions: none`, `Carries: none`, `Story values: none` and the like don't show, and a list that has entries still does.

## 0.58.0 (2026-11-24)

### Added

- Command history is kept between sessions (REQ-CON-43, Bruce, 2026-11-24), one file per world in `$XDG_STATE_HOME/thwarter/history/`, keyed as its saves are (`<world>-<hash>.txt`, or `<persona>/<world>.txt` installed). Up reaches earlier sessions' commands, and a fresh start shows the last five under "Last time:". `/quit` isn't kept. After `/restart`, the session's own history carries over as before.

## 0.57.6 (2026-11-24)

### Changed

- `/add` opens the editor on what it added, on a terminal, as `/new` does (Bruce, 2026-11-24): the door for `/add door ...` or `/add <room> <door>`, otherwise the thing; scenery with no card opens as a new card, written on the first save. Nothing opens after a refusal, or in scripts and transcripts.

## 0.57.5 (2026-11-24)

### Changed

- `/new room`, `/new character` and `/new object` open the editor on what they made, on a terminal (Bruce, 2026-11-24), so a new thing goes straight to its description. Scripts and transcripts only make it. A setting to turn this off can follow.

## 0.57.4 (2026-11-24)

### Removed

- `NarrativeEngine::evaluate_alternative_text`, an unused stub that always returned true and was the only build warning. Conditional text is `## When:` sections now (`design/SECTIONS.md`). Recover it with `git show cdbf701:src/engines/narrative_engine.rs`.

### Fixed

- A rename test assigned an engine it never read, the one warning when building the tests.

## 0.57.3 (2026-11-24)

### Changed

- Output has air around it (Bruce, 2026-11-24): a blank line follows what was typed in every mode, and every reply that suggests a command, usage lines included, puts each command on its own indented line after a blank line, with named parameters (`/link west {room}`), instead of inside the sentence. Recorded as a principle in `design/CONSOLE_REQUIREMENTS.md`.

## 0.57.2 (2026-11-24)

### Changed

- The `/` menu's keys (`↑↓ choose · tab or enter pick · esc close`) sit on its lower rule at the right, just above the prompt, rather than on a line of their own (Bruce, 2026-11-24).
- Command hints name their parameters, required ones in braces and optional ones in brackets: `/new room|character|object {id} [{title}]`, `/link {direction} {room}`, `/move {thing} {room|character|thing}`.

## 0.57.1 (2026-11-24)

### Changed

- `/add <room> <door>` writes the new side with the writer's `SetMapEntry`, which takes a small map now (492a954), instead of editing the door file's text.

## 0.57.0 (2026-11-24)

### Added

- The writer's `SetMapEntry` takes a small map of strings, numbers and booleans, written on one line flow-style, and can replace such an entry: `sides.castle/shed` becomes `castle/shed: { direction: west }`. A deeper value is still refused, for full editing. Asked for by the console's `/add <room> <door>`.

## 0.56.0 (2026-11-24)

### Added

- `/add door` (REQ-CON-42, Bruce, 2026-11-23): `/add door west hallway` or `/add door --(west)--> hallway` makes a door west of this room and east of the hallway, with an exit through it on each side; `/add door west` makes one side, a validate warning until `/add hallway <door>` gives it the other; `/add door` alone asks which way, and the next line answers. The far side faces the opposite way unless a direction is given after the room. A far room that doesn't exist is made. The id is the engine's `door_id` (28692c1), as in `door-bedroom-hallway-west`. A state hint picks the template: `door(locked)` inherits `locked-door`, and plain `door` and `portal` their own.

## 0.55.0 (2026-11-24)

### Added

- `/validate [strict]`: the world checked as `thwart validate` checks it, through the engine's `validation()` (6abdf97, REQ-ENG-44), with the console's unsaved-to-disk edits included. It lists the problems, then the warnings, then the summary line. The greeting now points at it.
- A problems pane at the foot of the sidebar, under a line, as in an IDE (Bruce, 2026-11-23): "PROBLEMS n · WARNINGS n", then every problem in red and every warning in yellow, taking at most a third of the sidebar, with `/validate` for the rest. HERE no longer repeats a room's warnings.

## 0.54.0 (2026-11-24)

### Added

- `engines::door_id(from_room, to_room, direction)` names a new door the same way in every client, `door-bedroom-hallway-west`, as a safe file name, for the console's `/add door` (REQ-ENG-47).

## 0.53.1 (2026-11-24)

### Fixed

- A thing's own `## When: #open` replaces its template's open-state reply, as STATES.md says, instead of both showing. An action that shares a state's name means ending up in it, and replies are now matched that way too. The vault door's `open` plays "Cold air rises from the dark below." alone. Found by the commander.

## 0.53.0 (2026-11-24)

### Changed

- Play reads a room in Bruce's order (REQ-ENG-43): the description with who's here, then what's here, then what things hold ("There's an old photo on the walls and some coins in the chest."), then the ways out, naming their doors ("You can go north, east through a shed door, or down through an open iron door."). Scenery is left out of "You see", since the prose has it. A door's own text shows when it's examined, not on every look.
- An examined holder says what it holds the same way: "Papered walls. There's an old photo on the walls."
- Built-in replies read as the story (REQ-ENG-45): "Which key do you want to take, the brass key or the iron key?" every time, "Who do you want to …" for people, "The iron door is locked." for a locked exit, and `I don't understand "xyzzy".` with plain quotation marks. Replies from several things run together as one paragraph: "It swings open. Cold air rises from the dark below."
- The `locked-door` built-in opens with "It swings open.", like `door`, and leaves a locked exit to the built-in "The iron door is locked."

### Added

- `[play]` gains `held` ("There's {list}.") and `door_exit` ("{direction} through {door}"), and any template may use `{or}` for a list joined with "or"; `exits` now defaults to "You can go {or}.". The test world's reference copy lists them.
- `validate` warns about a character whose `default_location` room doesn't list it in `characters:`, so it never appears (REQ-ENG-46).
- Typing the room's own name examines it, as `look around` does (REQ-PC-10).

### Fixed

- "You can't talk The cook." Talking to someone with nothing written now says "The cook has nothing to say.", and other verbs say "You can't kick the cook."
- Opening an open `locked-door` said "Nothing happens."; it says "It's already open.", and closing a closed one says "It's already shut."

## 0.52.0 (2026-11-24)

### Added

- `WorldEngine::validation(strict)` gives what `thwart validate` reports for the world as it is now: problems, warnings and a summary line. `thwart validate` uses it, and the console's `/validate` will, so the two never differ (REQ-ENG-44).

### Changed

- `thwart validate` ends with one summary that counts warnings too: "Validation passed: no problems, 3 warning(s)" or "Validation failed: 2 problem(s), 3 warning(s)".

## 0.51.4 (2026-11-23)

### Fixed

- The `door` and `portal` built-ins can be opened and closed: each has an `actions.rhai` with `open` and `close`, and replies where the door ends up ("It swings open.", "It's shut now.", "It's already open."). Before, a door from either stayed shut (REQ-CF-19).
- An exit that names a thing of the wrong kind says what it is: "the exit west names `door`, which is an object, not a room or a door", instead of "points at unknown location".

### Changed

- A thing's own frontmatter action runs instead of a template's script of the same name; its own script still runs first (REQ-ENG-38).

## 0.51.3 (2026-11-23)

### Changed

- A door with only one side is a `validate` warning, not a problem: it's half-built, as `/add door bedroom` will leave it until its other room is added, and going through it leads nowhere with an author note. A door with no sides, and a plain door with more than two, are still problems (REQ-ENG-31).

## 0.51.2 (2026-11-23)

### Changed

- The transcript fixture world moved from `tests/transcripts/worlds/tiny` to `tests/worlds/tiny`, beside `tests/worlds/sample`, so every fixture world sits under `tests/worlds/` (Bruce, 2026-11-23). A transcript's `# world:` header is now relative to the repository root (`# world: tests/worlds/tiny`). The two engine tests that named the old path follow it, with the coder's agreement.

## 0.51.1 (2026-11-23)

### Changed

- Play mode reads as a story (Bruce, 2026-11-23): the room view is the engine's player text (`render::play_blocks`), in the world's own wording from `world.toml`'s `[play]` table, with no room name unless the world turns them on, "You see …" and "You can go …" as sentences, and a blank line after what was typed and after each reply.
- The detail an author needs while playing moves to the sidebar: a ROOM section below GAME and MAP shows the player's room's exits and where they lead, its people, things and scenery by id with held things under their holders, and its warnings, and an "author notes" section holds the last action's notes. Scripts and transcripts still print author notes in the output.

## 0.51.0 (2026-11-23)

### Changed

- Play reads as plain sentences (Bruce, 2026-11-23; REQ-ENG-43). Looking shows no room name by default, then the description, then what's here on the next line ("You see a chair, a bed, and an old shirt."), then after a blank line the ways out ("You can go north and west."), each door's text, and who's here ("Captain Marcus is here."). Inventory is "You have a key and a lamp." Examining a holder adds "You see an old photo and a shelf." Every line is a template in `world.toml`'s `[play]` table (`engines::play_text::PlayText`, `WorldEngine::play_text()`), and the test world's `world.toml` lists each one at its default as the reference. `render::play_blocks` renders a room with a world's settings; `render_location_as(view, Player)` uses the defaults.

### Fixed

- Play's room text is cleaned of terminal control sequences when rendered directly, not only through `location_blocks`.

## 0.50.2 (2026-11-23)

### Changed

- The shipped test world starts again from Bruce's new rooms: `bedroom`, `closet` and `hallway`, with the `chair`, `old-shirt`, `photo-1` and `walls` objects, committed as he wrote them. `world/index.md` is now "Test world", starting in `bedroom`; `version` stays, since packaging names the file with it.
- Tests that need content load a small fixture, `tests/worlds/sample/` (six rooms, two characters, three objects, from the old world with exits to rooms it lacks removed), so they never depend on content Bruce rewrites. A test checks that `authors/mx/test-world` loads with no problems.

### Removed

- The July 2025 test content in `authors/mx/test-world`: `castle/` (16 rooms), `forest/` (12 rooms), `npcs/` (18 characters) and nine objects (`ancient-tomes`, `crystal-orb`, `flowing-water`, `forge-fire`, `golden-throne`, `master-sword`, `staff-of-oak`, `standing-stones`, `treasure-chest`), at Bruce's request. Recover it with `git checkout 83291af -- authors/mx/test-world/castle authors/mx/test-world/forest authors/mx/test-world/npcs authors/mx/test-world/objects authors/mx/test-world/world/index.md`.

## 0.50.1 (2026-11-23)

### Added

- `holds:` on an object or door says how it holds things: `on` (the default), `in`, or any word (Bruce; REQ-CF-31). Looking reads "Coins (in the chest)", and the built-in `locked-door` says `holds: in`, so a key reads "Brass key (in the iron door)". An empty `holds:` is a `validate` warning.

### Changed

- Edit mode's room view lists what objects hold, each under its holder and indented a step per level, with what doors hold after the objects. `WorldEngine::describe_location_at` now fills `LocationView.held` from the authored lists; play refills it from the game.

## 0.50.0 (2026-11-23)

### Added

- Objects hold objects in the console (REQ-CF-31, the coder's placement in 5c5a8a2): `/move photo-1 walls` and `/add photo-1 walls` put a thing into another object's `objects:`. Any object can hold things, scenery included; scenery a room lists with no card gets its card first (REQ-CF-8). `/move` now finds every place a thing starts through `placement()`, a room, an inventory or another object, and takes it from each. An object can't hold itself. The `/` menu offers objects as destinations, and HERE shows held things indented under their holder.

## 0.49.1 (2026-11-23)

### Changed

- Looking lists what visible objects hold too, by title, each right after its holder: "You see: Walls, Old photo (on the walls), Shelf (on the walls), Vase (on the shelf)." What doors hold comes last (a key in its lock). Hidden, faint and absent things follow their visibility, and the room view, examine and inventory all use titles (Bruce; REQ-CF-31). `LocationView` gains `held`.

## 0.49.0 (2026-11-23)

### Added

- Objects hold objects (REQ-CF-31, REQ-ENG-42): an object's `objects:` lists what it holds, as a location's does, so photos hang on the walls. `WorldEngine::placement(id)` gives every room, character and object a thing starts in (`Holder::Room`, `Character`, `Object`), for the console's `/move`. In play, what an object holds is in scope whenever the object is, however deep, for look at, take, examine and object actions; the room view lists only what's directly in it, and examining a holder adds "You see: …" for what it holds. A key kept in a lock is the same relation, made in play, and saves are unchanged.
- `validate` warns about a thing that starts in more than one place, skipping scenery (the test world has benches in two rooms), and reports an object that holds itself through what it holds.

## 0.48.0 (2026-11-23)

### Added

- Rhai scripts (`design/SECTIONS.md` › Scripts, REQ-ENG-38; `engines/scripts.rs`, the `rhai` crate). A bundle's `actions.rhai` or a ```rhai block in a thing's file holds a function per action, `fn unlock(door, turn)`, given the thing (`id`, `title`, `state`, its values, `held`) and the turn (`verb`, `used`) read-only. It returns `become("closed")`, a reason (`"wrong-key"`), nothing, or a list of changes. A template's script passes down, and a thing's own function replaces the template's. Scripts are sandboxed: no files, network, modules, `eval` or printing, and capped at 100,000 operations. A script that doesn't compile is a problem.
- Outcomes (REQ-ENG-39): replies now match after the action, against the state the target moved into or the reason it didn't. A reply in a state file that names no outcome means "ended up here", and a thing's own reply replaces its template's for the same action and outcome. `take` looks for a `## When: #take` reply.
- `keep(turn.used)` leaves what an action was used with in the thing (`PlayState.held`, saved), and the player takes it back with `take` (REQ-ENG-40).
- The built-in `locked-door`, as `design/STATES.md` › Keys shows: `index`, `locked`, `closed` and `open`, plus `actions.rhai` with `unlock`, `lock`, `open` and `close`. `vars: { key: "" }` means no key is needed; `validate` warns about a `key` that names nothing. The Keys table plays as written (`engines/turn.rs` tests).
- `connects: [vault-door]` lets a thing's script `change("vault-door", "open")`; without it the change is refused with an author note, and `validate` warns about a `connects:` entry that names nothing (REQ-ENG-41).

### Changed

- Rhai reserves `with` and `this`, so a script reads the used thing as `turn.used` (not `turn.with`) and can't name a parameter `this`.
- Every matching reply shows, in order: the target's, then what's used, then the room's.

## 0.47.2 (2026-11-23)

### Changed

- CI uses `actions/checkout@v5`, which runs on Node 24; v4 targeted Node 20, which GitHub has deprecated and warned about on every run. The other actions gave no warning.

## 0.47.1 (2026-11-23)

### Changed

- `go to <exit>` tries the whole phrase as an exit first, then without the `to` and an article (REQ-PC-20), so `go to the river` works whether the exit is `river` or `to the river`.

## 0.47.0 (2026-11-23)

### Added

- In play, `@id` in prose shows as that thing's name in its current version (`@guard-captain` → Captain Marcus), in rooms, doors, objects, characters and replies. An `@` after a letter or a dot (an email address) or inside inline code isn't one. An `@` that names nothing shows as the bare word, and `validate` warns about it, in a file's prose and its sections. Edit mode shows `@` as written.
- The parser drops a `to` or `at` straight after a verb, then any article (REQ-PC-20): `listen to the chest`, `talk to the captain`. `in`, `into`, `on` and `under` stay, since they change the meaning, and `go to` is left alone, since an exit's name is free text (`go to the river`).

## 0.46.1 (2026-11-23)

### Changed

- The test world's nine objects that had marker replies have them back, as `## When: #<verb>` sections with the author's text word for word; an `examine` reply is now part of the object's description. None had a refusal.
- A closed door's reply to `go` is its `## When: #go` section, if one matches, else the built-in "The door is closed." The `door` and `portal` built-ins had no replies to move, so they're unchanged.

## 0.46.0 (2026-11-23)

### Added

- Sections are shown by matching the turn's tags (`design/SECTIONS.md`, REQ-ENG-36; `engines/turn.rs`). Each turn has `@` for what the player holds and the room, `#` for the chapter, and `#` for the current state of the thing described; an action's turn adds `#<verb>`, `@<target>` and `@<what it's used with>`. Descriptions in play (rooms, doors, objects, characters) add each section that matches. An action's reply is the matching `#<verb>` sections in the target, then what it's used with, then the room: with a frontmatter action its mechanics apply and the section replaces the built-in reply; without one the section is the whole action; with neither, "Nothing happens." A `#verb` in any heading counts as a verb the world knows. `## When: #searched` in a room leads the reply to `search`.
- The parser reads `with`: `hit window with stick` is `PlayAction::Use { verb, noun, with }`.

### Changed

- An object action the thing doesn't have, with no section to answer it, now replies "Nothing happens." instead of "You can't lick the lever."

## 0.45.0 (2026-11-23)

### Added

- The loader reads keyword sections (`design/SECTIONS.md`, REQ-ENG-35): a `## When:` or `## Unless:` heading with `#states` and `@things` opens a section that runs to the next `#` or `##` heading, with an optional ` ```thwarter ` block of data first. Sections stay out of the description and never create states. A misplaced block, or `id`, `type` or `state` in one, is a warning; a bad heading or block is a problem. Matching against a turn's tags (all of them for `When:`, none for `Unless:`) is in place for the description and reply work that follows.
- `## When: #ambiguous` in a shared template is the reply when a noun matches several things, replacing the `reply ambiguous` marker.
- `becomes: { this: open }`: `this` is the thing whose action it is.

### Removed

- `<!-- thwarter:... -->` markers are no longer read: `<!-- -->` is only a comment, and play never shows one (Bruce, `design/SECTIONS.md`). Gone with them: `Body`, `response_marker`, `GameObject.responses`, `Door.responses`, actions defined only by a response block, the response warnings, `go refused` and `examine` blocks, and `thwart show`'s "(no response)". Frontmatter `actions:` still work and reply with a built-in reply. The test world's nine objects with markers have lost those replies until they're rewritten as sections (`PLAN.md` › Phase 6); there's no conversion. `--strip-comments` now strips every comment. Recover the marker code with `git show e930cb5:src/parsers/markdown_parser.rs`.

## 0.44.1 (2026-11-23)

### Changed

- In the editor, `@name` is cyan when any thing by that id exists, object, character or door, not only a character (SECTIONS.md, e930cb5).

## 0.44.0 (2026-11-23)

### Added

- The editor reads the heading format (`design/SECTIONS.md`, 9174b33): `## When: #locked` and `## Unless:` section headings are drawn as rules in the chrome colour, with their tags in magenta; a ```thwarter block is highlighted like frontmatter (keys blue, ids cyan or yellow); and `@name`, a character, is cyan when it exists and yellow when it doesn't.

## 0.43.2 (2026-11-23)

### Changed

- `/save` and `/load` use the engine's saves (`engines::saves`, 28ff39f, REQ-PC-18), which every player client shares; the paths are the same, so existing saves still load. The unknown-input log and `/restart`'s session file use the engine's state folder (`settings::state_dir`). REQ-CON-11 points at REQ-PC-18 for where saves go and keeps the console's side.

### Removed

- `src/console/saves.rs`, now `src/engines/saves.rs` with its tests. Recover with `git show 64945de:src/console/saves.rs`.

## 0.43.1 (2026-11-23)

### Added

- The editor wraps long lines at words (Bruce, 2026-11-23), and its highlighting follows a line across the rows it wraps onto. A word wider than the editor wraps by character. The text widget finds words by Unicode word boundaries, which count punctuation as a word, so a comma that lands right at the edge can start the next row.
- Fuller Markdown styling in the editor: headings bold with their `#` marks dimmed, `~~strikethrough~~` crossed out, links underlined as well as cyan, list markers (`-`, `*`, `+`, `1.`) and `>` quote marks in the chrome colour with quoted text in italics, and `***` or `___` rules dimmed.

## 0.43.0 (2026-11-23)

### Added

- Saves are the engine's, so every client stores games the same way (REQ-PC-18, REQ-ENG-34), as agreed with the console coder: `engines::saves` has `DEFAULT_SAVE`, `save_dir`, `save_path` and `list_saves`, moved from the console with the same paths so existing saves still load, and new `save_named` and `load_named`, which save and load by name for a world. The save folder now comes from `settings::state_dir`, which ignores a relative `XDG_STATE_HOME` as XDG asks. The console's `src/console/saves.rs` stays until `/save` and `/load` switch over.

## 0.42.1 (2026-11-23)

### Changed

- Play mode reads input through the engine's player command parser (`GameEngine::parse_command`, 0fe9754, REQ-PC-1), the one every player client shares, so objects' named actions ("light lantern", "pull lever", REQ-PC-13) and `search` now work in the console. Edit mode reads its words (go, look, examine and directions) through the same parser. Unknown input is still logged.

### Removed

- `src/console/verbs.rs`, the console's own parser, now in the engine as `src/engines/player_commands.rs` with its tests. Recover with `git show 858974e:src/console/verbs.rs`.

## 0.42.0 (2026-11-23)

### Added

- The player command parser is the engine's, for every client (`design/PLAYER_COMMANDS.md`, REQ-ENG-32, 33): `GameEngine::parse_command(&world, line, &state)` returns `Parsed::Action`, a `Reply` asking for a noun (`Take what?`) or `Unknown` (``I don't understand `<line>`.``), with the console's verbs, aliases and tests carried over (REQ-PC-1 to 8). New: bare `search`, and any other verb with a noun as an object's named action, `PlayAction::Use` (REQ-PC-13). A verb no object or door has in any state, on a noun that names nothing in scope, isn't understood instead of "There's no wildly here" (REQ-PC-14), and `take`, `go` and the other built-in verbs are read before object actions, so they can't be shadowed (REQ-PC-15). `WorldEngine::has_action` says whether any thing has an action. The console's `verbs.rs` stays until play mode switches over.

## 0.41.4 (2026-11-23)

### Changed

- Edit mode's views and arrival lines no longer print a file's path and type (`objects/chair.md, scenery`); the sidebar shows them (Bruce, 2026-11-23). HERE shows the file and type of the thing edit mode is at, and of what `examine` (`look <thing>`) showed last, under "looked at", until edit mode moves.
- HERE groups a room's contents as people, things and scenery, where scenery is anything built from the built-in `scenery` template or listed with no card yet (dimmed, without a label). A thing's scope shows only when it isn't global, the default.
- `design/CONSOLE_REQUIREMENTS.md` › REQ-CON-10 points at `design/PLAYER_COMMANDS.md` (REQ-PC) for play mode's commands instead of listing them.

## 0.41.3 (2026-11-23)

### Changed

- `/edit` takes a name as well as a kind (Bruce, 2026-11-23): `/edit bedroom`, `/edit lantern`, `/edit "old shirt"` or `/edit north` open whatever the name means, found as `go <name>` finds it (an exit, a person or thing here, a path, an id or a title anywhere; several matches ask which). Edit mode stays where it was. Scenery the room lists opens as a new card. The `/` menu offers the kinds first, then what's here and the rooms.

## 0.41.2 (2026-11-23)

### Fixed

- Quotes in console commands didn't hold words together, so `/add "old shirt" bedroom` was read as three arguments (Bruce, 2026-11-23). `"..."` and `'...'` now keep spaces inside one argument, for every command.

### Changed

- Ids can't hold spaces, so a name typed with spaces or capitals becomes an id: `old shirt` and `Old Shirt` are `old-shirt`. The name as typed becomes the title where one is made: `/new object "Iron Key" tool` is `iron-key`, titled Iron Key, and `/add "old shirt" bedroom` gives the new scenery a card titled "old shirt", so the name isn't lost. `/move` and `/edit` take names the same way.

## 0.41.1 (2026-11-23)

### Changed

- The editor opens with the cursor after what's written (Bruce, 2026-11-23): at the end of the last line of prose, or, for a file that's only frontmatter such as a new scenery card, on a fresh line after the closing `---`. That line doesn't count as an unsaved change.

## 0.41.0 (2026-11-23)

### Added

- `/add <thing> [room or character]` (REQ-CON-41, Bruce, 2026-11-23): puts a person or thing into a room (this one if none is named), or an object into a character's inventory, without taking it from anywhere else; `/move` is for taking it away too. A name with no card goes in as scenery, and the reply says `/edit object <id>` gives it one. It's in `/help`, the `/` menu (with the world's characters and objects offered) and the transcript guard.

## 0.40.1 (2026-11-23)

### Added

- Command-S saves in the editor, as Ctrl-S does, for Mac users (Bruce, 2026-11-23). The console asks the terminal to report keys with their modifiers (the kitty keyboard protocol), which kitty, WezTerm, Ghostty and iTerm2 3.5 and later honour; a terminal that doesn't ignores the request, and Terminal.app keeps Command keys for its own menus. Only Command-S is taken; Command-C and Command-V stay with the terminal. On a Mac the editor's heading says `⌘s or ctrl-s save`.

## 0.40.0 (2026-11-23)

### Added

- A MAP panel in the sidebar (Bruce, 2026-11-23): the rooms one exit out from edit mode's room, or the player's, drawn small from the coder's map layout (851a7f2). `@` marks where you are, `?` a room not written yet, `+` and `#` a door that's passable or not, and exits off the grid (up, down, the rest) follow as `pantry: north → courtyard`. It follows you as you move and after every reload.
- `/map` prints the whole map around you in the output, as `thwart map` draws it.

### Changed

- The edit-mode prompt is `in <place> > `, such as `in / > ` (Bruce, 2026-11-23). Play mode's stays `play <room>> `.

### Fixed

- HERE showed only its heading in a folder, so rooms at the world root, such as a new `bedroom`, didn't appear in the sidebar. In a folder, it now lists the folders and files there, with what each is.

## 0.39.2 (2026-11-23)

### Added

- The console logs input it didn't understand to `$XDG_STATE_HOME/thwarter/unknown-commands.log` (`~/.local/state/...` by default), so the console coder can review what authors and players reach for when Bruce asks (Bruce, 2026-11-23). Each line is tab-separated: when, the world's folder, the mode, where (edit mode's place or the player's room), the kind (`command` for an unknown `/command`, `edit` for a word edit mode doesn't know, `play` for a verb play doesn't understand) and what was typed. Only terminal sessions log, not scripts or transcripts. `/status` shows the log's path and how many lines it holds.

## 0.39.1 (2026-11-23)

### Fixed

- `thwart search` lists things: with no query, or an empty one, a `--type` or a location lists everything there (`thwart search --type character castle/`). A lone argument ending in `/`, or naming a folder in the world exactly that isn't also a thing's id, is taken as the location, not the query, so `thwart search castle/` lists the castle. A location now also covers things whose own files are in that folder (`objects/`), as well as what its rooms hold.

## 0.39.0 (2026-11-23)

### Added

- `thwart search <query> [location]` (REQ-CLI-18): finds every location, object, character and door whose id, title or prose (an object's responses included) contains the query, ignoring case unless `--case-sensitive`, or matches it as a pattern with `--regex`. `--type location|object|character|door` narrows the kinds, and `--type text` searches prose only. A location (`castle/`) limits it to rooms in that folder and the things they hold or where characters start. Each match says the thing's kind, id and title, and for prose the line it's in. `--missing-connections` lists every exit to a room not written yet. `WorldEngine::search` gives the same results as data. Tags wait on the console's tag-search design (REQ-CF-30).
- The `regex` crate is a new dependency, since its matching runs in linear time, so no pattern in someone's search can hang it.

### Changed

- `EntityKind` serializes as its lowercase type name.

## 0.38.0 (2026-11-23)

### Added

- The `/` command menu (REQ-CON-33), as in Claude Code: typing `/` opens a list of the commands that work in this mode, each with a line about it, narrowing as you type. After a command it shows the command's arguments as a hint and offers what fits from the world: `/edit` the kinds and then the rooms, objects or characters; `/move` the people and things here first, then the rooms and characters to put them in; `/link` directions and rooms; `/unlink` this room's exits; `/load` and `/save` the saves. Plain input completes too: `go` offers the exits first (marking a room not written yet), then `back`, what's here, the rooms nearest first by the map's distances (851a7f2), and the folders; `examine` and `take` offer what's in the room. ↑ and ↓ choose, Tab completes, Esc closes it until the line changes. Enter picks the command being named, or a suggestion chosen with ↑ or ↓, and otherwise runs the line as typed.

## 0.37.1 (2026-11-23)

### Added

- `validate` warns when a one-file room's `id` isn't its path from the world root without `.md` (`castle/kitchen.md` holding `id: castle/hall`), so a room copied without changing its id is caught before it collides (CONTENT_FORMAT.md's `id` row). It's a warning, since the loader allows files anywhere. Bundles were already checked against their folder.

### Fixed

- `thwart map`'s list of exits beside the grid named a room reached only off the grid by its id ("castle/library-upper: down → castle/library"). It now uses the room's title, as the other lines do. `Beside` gains `room_title`.

## 0.37.0 (2026-11-23)

### Added

- `thwart map [location]` draws an ASCII map (REQ-CLI-17): rooms as `[Title]` boxes on a grid placed by their compass exits, `---`, `|`, `/` and `\` between them, `+` on a link through a door and `#` through a closed one, `@` for the room it's drawn from (the world's start by default) and `?` for one not written yet. Exits that don't fit the grid (up, down, other directions, a cell already taken) are listed after it as `room: direction → target`. `--depth` limits how far it goes and `--output` writes it to a file. `--player`, other `--style`s and `--connections` say they aren't implemented yet.
- `WorldEngine::map_layout(from, depth, only)` gives the same map as plain data (cells, links, and the exits beside the grid), for the console to draw at its own width, with `only` limiting it to given rooms (such as the ones a player has visited). `WorldEngine::distances(from)` lists every reachable room with its distance, nearest first, and `exits_of(room)` a room's exits through doors. Agreed with the console coder.
- `WorldEngine::find_path` finds the shortest way between two rooms through every exit and door, and `WorldGraph::find_path`, which always said no, finds it by direct exits.

### Changed

- `design/CLI_REQUIREMENTS.md`: REQ-CLI-17 for `map`; REQ-CLI-8 now covers only `search`.

## 0.36.1 (2026-11-23)

### Changed

- An installed world's saves are kept by persona and world, `$XDG_STATE_HOME/thwarter/saves/<persona>/<world>/`, rather than by a hash of its path, so reinstalling it or installing it elsewhere keeps them (REQ-CFG-6). It uses the coder's `packaging::installed_id` (58ad7d6). Every other world keeps its `<world>-<hash>` folder, and no saves move.

## 0.36.0 (2026-11-23)

### Added

- `packaging::installed_id(world_root)` says which installed world a folder is, as `(persona, world)`, when it sits at `worlds/<persona>/<world>/` in the data folder, so the console can key an installed world's saves by name and keep them through `--replace` or a reinstall (REQ-CFG-6). Agreed with the console coder, who changes the save folder to use it.

## 0.35.1 (2026-11-23)

### Changed

- The `zip` crate moves from 2.6.1 to 9.8.2, its current major line, since it parses untrusted archives. Two adjustments: entry names are now read fallibly, and an absolute entry name (`/etc/x`, `C:x`) is refused explicitly, because zip 9 quietly makes such names relative where 2.x refused them. No advisory scan was run: `cargo-audit` isn't installed, and installing a tool system-wide waits on Bruce.

## 0.35.0 (2026-11-23)

### Added

- `thwart package [WORLD]` (REQ-CLI-12 to 14): writes the world and its persona into one `<persona>-<world>-<version>.thwarter` file, a zip anyone can rename and open, holding `thwarter-package.toml` (format, persona, world, title, version, the packaging `thwart` version and the date), `<persona>/index.md` and `<persona>/<world>/`. It leaves out what the loader skips and `world/styles/`, keeps `world.toml`, and refuses a world that fails `validate`. Before writing, it says which of the persona's details ship and how many author comments, then asks; `--yes` skips the question, and without a terminal it refuses rather than publish. `--strip-comments` leaves author comments out of the packaged copies, keeping response markers. `--output` picks the file.
- `thwart install <file>` (REQ-CLI-15, REQ-CFG-5, 8): checks the whole archive before writing anything (no absolute paths, `..`, links or files outside the layout; at most 200 MB, checked as it unpacks, and 10,000 files), then unpacks into `$XDG_DATA_HOME/thwarter/worlds/<persona>/<world>/`, beside the target first, so a failed install leaves an older copy as it was. An installed world is refused without `--replace`. The persona file is written beside the world, or updated, with a note.
- `thwart installed` lists installed worlds, and `thwart play <persona>/<world>` plays one when there's no such folder where it's run (REQ-CLI-16).
- The `zip` crate (deflate only) is a new dependency.
- `tests/cli_package.rs` runs packaging and installing end to end, and hostile archives.

## 0.34.1 (2026-11-23)

### Fixed

- A world's text can no longer drive the terminal (REQ-CFG-9). `render::clean_text` removes control characters, carriage returns and escape sequences (CSI and OSC whole, so no `[31m` is left behind), keeping line breaks and tabs. It applies to every block the renderer builds and its plain text, the author views of objects, characters and doors, every play reply (built-in replies quote titles), `thwart show`'s output and every warning, error and notice the command line prints. This holds for every world, not only installed ones.
- Loading refuses a content file over 1 MB and frontmatter nested more than 32 levels deep, each reported as a problem, and skips a link that points outside the world, with a warning (REQ-CFG-10). A link to a file inside the world is still read. YAML alias bombs were already refused by the parser; a test now checks it.

## 0.34.0 (2026-11-23)

### Added

- Personas (REQ-WS-4, 5): `authors/<id>/index.md` with `type: author`, a `title` that is the pen name, contact details (`email`, `website`, or any other single value), an optional default `style` and `world`, and the bio as its prose. A persona folder without `index.md` still works, with its folder name as the pen name. `world/index.md`'s `author:` names a persona by id, found as the world's own parent folder or a sibling of it, and `WorldEngine::persona()` returns it. `validate` warns when `author:` names no persona, and reports a persona file that isn't `type: author`. `engines::persona::list_personas` lists the personas in an `authors/` folder.
- Personal settings (REQ-CFG-4): `settings::PersonalConfig` reads `$XDG_CONFIG_HOME/thwarter/config.toml` (else `~/.config/thwarter/config.toml`), with the default persona (`persona = "bob"`) and the writing helpers' `[helpers]` table, and writes it back atomically, readable only by its owner, keeping every key it doesn't know. `settings::data_dir` and `state_dir` give `$XDG_DATA_HOME/thwarter` and `$XDG_STATE_HOME/thwarter`, for installed worlds and play data (REQ-CFG-5, 6).
- `thwart show`'s summary says the persona's pen name and id ("by B. Alderson (bruce)"), and its JSON includes the persona.

### Changed

- The test world names its persona, `author: mx`, the folder it sits in, instead of the free-text "Thwarter Testing Team".

## 0.33.2 (2026-11-23)

### Fixed

- Edit mode treated an exit through a door (`connections: { down: vault-door }`) as an exit to a room called `vault-door`, so `down` said it wasn't written yet and HERE marked it missing. It now goes to the room on the door's far side, and HERE shows `down  vault  through vault-door`. Door files list by their title, from the coder's doors (bbd1e64).

## 0.33.1 (2026-11-23)

### Added

- A door that isn't passable answers with its current state's `<!-- thwarter:action go refused -->` block, with variables filled, so a closed door and a locked one can say different things (`design/STATES.md` › Doors). Without one, play says "The iron door is closed." as before.

## 0.33.0 (2026-11-23)

### Added

- Doors (REQ-CF-19, `design/STATES.md` › Doors): `type: door`, joining two rooms with one state seen from both sides. `sides:` names each room and the direction of its exit (`castle/hall: { direction: down }`), and a room's exit names the door (`connections: { down: vault-door }`). `passable:` says whether the exit can be used in the door's current state, and `transparent:` whether the room beyond can be seen. A portal (`portal: yes`) joins more rooms, each side saying where it leads with `to:`. Doors have states, actions and responses like any other thing, and `thwart show <door>` shows one.
- In play, an exit through a door leads to its other side, and is refused while the door isn't passable ("The iron door is closed."). The room view shows each door's current text by the exits, and for a transparent door, "Through the iron grate you can see the Cell." The player can examine a door and use its actions ("open door").
- `validate` reports a plain door without exactly two sides, a portal side without `to:` in a portal with more than two, a side that isn't a room, and an exit through a door the room isn't a side of. It warns about a side whose direction differs from the room's exit, and a side whose room has no exit through the door.
- Built-in `door` and `portal` templates, each a bundle with `closed` (not passable, the starting state) and `open`.
- `ExitView` gains `door: Option<DoorView>` (the door's id, title, current text, whether it's passable or transparent, and the room beyond). `WorldEngine::doors` and `door_at` hold doors.

### Changed

- Templates pass their states down (Bruce, 2026-11-23, REQ-ENG-19): a thing has its templates' states as well as its own, a state file of its own with the same name is layered over the template's, and a template's `state:` is the thing's starting state unless the thing names one. Built-ins can now be bundles. `thwart show --template` shows a template bundle's state files after its index.

## 0.32.1 (2026-11-23)

### Changed

- Edit mode lists any kind of thing the engine adds beyond rooms, characters and objects as a door, ready for the coder's doors (`design/STATES.md` › Doors). Doors show in `look` as written and can't be `/edit`ed from the console yet.

## 0.32.0 (2026-11-23)

### Added

- `WorldWriter::make_bundle_loaded(engine, kind, id, first_state, strict)` turns a one-file thing into a bundle (REQ-ENG-28): `lantern.md` becomes `lantern/index.md` with the same id and permissions, optionally with a first state file (`inherits: index`) beside it. The index is left alone, so the thing has no starting state until one is set. The new files are written before the old one is removed, so a half-done move leaves a duplicate id that `validate` reports, never a lost file.
- `WorldWriter::save_bundle_loaded(engine, kind, id, sections, strict)` saves a bundle's files together (REQ-ENG-29), in the shape agreed with the console coder. Each `Section` (a path, its text, and the text the editor opened) must be one of the bundle's files, must parse as what it is there (the index as the same thing, a state or chapter file with no `id`, `type` or `state:`), and must still match the disk. The merged thing must still load, checked on a copy of the engine. Only then is anything written, and only the sections that changed; a section for a file that doesn't exist yet is created. Every refusal starts with its section's path, so the editor can put the cursor there.
- `WorldWriter::delete_state_loaded` removes a state (refused for the starting state) and lists the actions that switch to it. `WorldWriter::rename_state_loaded` renames a state's file, the index's `state:` when it's the starting one, and every `becomes:` that switches to it, each checked by reading back. `BundleReport` lists what was written, created, left unchanged, moved, deleted, and what's left for the author.

## 0.31.0 (2026-11-23)

### Added

- `/move <room> <area or room>` moves a room into an area folder (REQ-CON-40, Bruce, 2026-11-23): `/move hallway house` makes it `house/hallway`, moving its file (or its bundle folder) and updating every exit, room list, inventory, `default_location`, the world's `start:` and every link to it, through the writer's `rename_loaded` (318144e). A room as the target stands for its own area. The report names the files it updated and lists any reference left to fix by hand (inside an action's `becomes:` or `requires:`). Edit mode's place, `go back` and the game in progress follow the room to its new id, and the watcher doesn't announce the files the move touched.

## 0.30.1 (2026-11-23)

### Changed

- Renaming rewrites references inside actions too: `becomes:` keys and `requires: { has: }`, in the style each was written. The writer replaces the old id as a whole word within the `actions:` block, then checks the frontmatter reads back as exactly what was expected. When the same word appears where it means something else (a `sets:` value, say), it changes nothing there and reports the reference for the author, as before.

## 0.30.0 (2026-11-23)

### Added

- `WorldWriter::rename_loaded(engine, kind, old, new, strict)` renames a thing (REQ-ENG-30), for the console's `/move` (a room's id is its path, so `/move hallway house` is a rename to `house/hallway`) and for renaming. It rewrites the thing's `id`, moves its file, or its bundle folder with every state and chapter file, to the path the new id gives, and rewrites every reference: exits, `characters:`, `objects:`, `inventory`, `default_location`, `world/index.md`'s `start:`, and `[[links]]`, `[text](file.md)` links and `{{ id.value }}` variables in prose. A link stays a link, and comments beside a value stay. Every edit is checked in memory first. The rename is refused, with nothing written, if the new path or id is taken or the id is reserved. References inside an action's `becomes:` or `requires:` are listed for the author to fix. `RenameReport` lists every file moved and written, and the engine takes the same change.
- The writer can edit files that aren't whole things (states, overlays, chapters, `world/` files), checking only that their frontmatter still parses (`ContentFile::load_layer`).

## 0.29.1 (2026-11-23)

### Changed

- The console's chrome has its own colour, blue, so it never looks like text (Bruce, 2026-11-23): the status bar and the editor's heading bar are blue with bright white text, and the sidebar's border and the editor's line numbers are blue. Sidebar headings are now bold in the text colour, leaving blue to the chrome.
- Console commands stand out, as typed and as echoed in the output: the `/` is bold yellow and the command's name bold, with the arguments plain.

### Fixed

- `/move` on a room said nothing by that name was in the world. It now says the room is there, where it would go (a room as the target stands for its area, so `/move hallway bedroom` is already done when both sit at the root), and that moving a room's file waits on renaming.

## 0.29.0 (2026-11-23)

### Added

- `/move <thing> <room or character>` (REQ-CON-40, Bruce, 2026-11-23): puts a character into another room, with its `default_location` following, or an object into a room or into a character's inventory. It takes the thing out of every room list and inventory that held it, saves each file it changes and reports the problems the move adds, as `/link` does. The thing's own file doesn't move.

## 0.28.0 (2026-11-23)

### Added

- Colour from the terminal (REQ-CON-38): the console uses only the terminal's default colours and its 16 named ones, by role, so it follows whatever theme the terminal has. What you typed is bold; console messages (Saved, Reloaded, Restarted) green; errors and refusals red; warnings and what isn't written yet yellow; author notes dim; sidebar headings bold blue; the prompt bold green in edit mode and bold magenta in play mode. `NO_COLOR` drops the colours and keeps bold, dim and the rest.
- The editor highlights what it shows (REQ-CON-39): frontmatter fences dim, keys blue, ids cyan when they resolve and yellow when they don't; Markdown headings and strong bold, emphasis italic, links cyan, `#tags` magenta, code dim; Thwarter's markers dim; `{{ values }}` cyan when declared and yellow when not. The current line's number is bold, and the status line is green when saved, red when refused and yellow when something needs watching.
- `/status`: the world and its folder, the mode and where edit mode is, the game (where, chapter, turn, what's carried), the saves, problem counts and the build.
- `/clear` empties the output; the session, game and history carry on.

### Changed

- `/edit` with nothing after it opens where edit mode is, whatever it is, and from play mode switches to edit mode at the player's room and opens that (REQ-CON-2, Bruce, 2026-11-23). It used to only switch modes.
- Transcripts show the build as `thwart <version>`, so a version bump doesn't change them.

### Fixed

- The full-screen console cleared the screen at start with a call that asks the terminal for its cursor position, so a terminal that didn't answer stopped it starting. It now clears without asking.

## 0.27.1 (2026-11-23)

### Fixed

- `thwart show` printed every load problem and warning on standard output before its result, about 30 lines on the test world, so `--format json` wasn't one JSON document. It now writes one line to standard error, "2 problems and 27 warnings; run `thwart validate` to list them", and nothing for a clean world. `validate` still lists them all.

## 0.27.0 (2026-11-23)

### Added

- `thwart show <name>` shows any thing, not only locations (REQ-CLI-9): an object with its fields and actions, a character with where it starts and what it carries, by id or link, in any state and chapter. A thing with states, chapter versions or templates lists them, marking the starting and shown state. `--kind location|object|character` chooses when an id names more than one kind, which is otherwise an error that says so.
- `thwart show --template <name>` shows a template or built-in exactly as written, and where it was found (REQ-CLI-10). An unknown name lists the templates there are.
- `thwart show` with no name summarizes the world (REQ-CLI-11): its `world/` definition, story values, player, chapters, and every location, object and character id, sorted. It used to print only the location ids, unsorted and unlabelled.
- `render_object`, `render_character` and `render_thing_sources`, the author's text views, and `WorldEngine::find_template` and `template_names`.
- `tests/cli_show.rs` runs `thwart show` itself against a small world.

### Changed

- `design/CLI_REQUIREMENTS.md`: REQ-CLI-5 now covers any thing and the summary, with REQ-CLI-9 to 11 new.

## 0.26.1 (2026-11-23)

### Changed

- A hidden thing answers to its exact name only for an action with no `requires:`; for one with `requires:` it is as good as absent ("There's no lever here."), even with the needed object in hand (Bruce, 2026-11-23, `design/STATES.md` › Hidden things). Actions only act on objects, which can always be found, so `findable:` for characters has nothing to change yet.

## 0.26.0 (2026-11-23)

### Added

- The renderer builds styled blocks (REQ-CF-28): `Rendered` holds `Block`s (title, heading, paragraph, list item, quote, code, exits, here, notice, author note, response), each a list of `Span`s with a `Style` (strong, emphasis, code, a link with its target, tag, missing), in the shape agreed with the console coder. Nothing is wrapped. `Rendered::to_plain` prints them; `render_location` and `render_location_as` now go through it and print the same bytes as before, which a test checks for every room in the sample worlds. `location_blocks` gives the console the blocks.
- Play renders prose as Markdown (REQ-CF-28): room descriptions, examine text and responses show headings, emphasis, lists, quotes and code as text, never as raw syntax. The author view (`thwart show`, edit mode) still shows prose as written.
- Links in prose (REQ-CF-29): `[[lever]]`, `[[castle/vault|the vault]]` and `[the lever](objects/lever.md)` name a thing of any kind, by id, path or unique file name. In play a link shows as its alias, else the thing's title, else its name as written. `validate` warns about a link that names nothing, or names an ambiguous file name. `WorldEngine::link_for` resolves one.
- Tags (REQ-CF-30): `#tag` in prose shows without its `#` in play, and `WorldEngine::tags()` lists every tag, from prose and from `tags:` in frontmatter, with the things that carry it, for search.

### Changed

- `design/ENGINE_REQUIREMENTS.md` and `design/ARCHITECTURE.md` point at REQ-ENG-10 rather than `design/STATES.md` › Engine rules, which now points at the requirements.

## 0.25.0 (2026-11-23)

### Added

- The full-screen console (REQ-CON-22, 23), in the layout Bruce chose: a status bar across the top (world, mode, warnings, and with a game the chapter and turn; the build on the right, with "newer build: /restart" once one is built), the output on the left, a sidebar on the right, and the input line at the bottom. The sidebar's GAME panel shows where the player is, the chapter and what they carry, which stay put, then what has changed from its starting state and the values in play. In edit mode, HERE shows the room as a game master's notes: its exits and which aren't written, the people and things in it with their scope (global, area or local), and its warnings. HERE scrolls with Alt-Up and Alt-Down when the sidebar is short. Under 100 columns, the sidebar folds into one line above the input.
- The editor opens over the output and keeps the sidebar. Watcher notices go into the output. Page Up and Page Down scroll the output; Up and Down recall input; Ctrl-C clears the line and Ctrl-D on an empty line leaves.
- `/restart` also carries the output's scrollback, so the screen looks the same after it.

### Removed

- reedline, the line editor the console used before the full-screen view. Recover the old terminal loop with `git show aec429a:src/console/mod.rs`.

## 0.24.0 (2026-11-23)

### Added

- `reply ambiguous` (`design/STATES.md` › Ambiguity, Responses): when a noun matches several things, the reply is the `<!-- thwarter:reply ambiguous -->` block of the nearest template the candidates all share, with its variables filled.

### Changed

- The built-in ambiguity replies are in the story's voice and vary by turn: "Which sword do you want to take: the rusty sword or the old sword?" and "You look at the rusty sword and the old sword, not sure which to take." `GameEngine::ask_which` now takes the verb, the noun as typed, the candidates and the turn.

## 0.23.0 (2026-11-23)

### Added

- `visibility:` in play (REQ-CF-18, `design/STATES.md` › Hidden things), read from a thing's current version, for objects, characters and exits (by the room they lead to). `faint` things aren't listed until the player searches or examines the room; once found, they stay found. `hidden` things are never listed or found by looking, but an action may name one exactly ("pull lever"). `absent` things can't be named at all. Revealing one is ordinary state switching (`becomes:`).
- `PlayAction::Search` ("You search carefully and find the brass key and a way north."), and `examine` on the room itself (`room`, `here`, `around`, or its title or id), which shows the room and finds faint things the same way. `PlayState` keeps what's been found (`noticed`), and `Change` gains `Noticed`. `WorldEngine::visibility(kind, id, at)` gives a thing's visibility.
- A `visibility:` other than `visible`, `faint`, `hidden` or `absent` is a problem. `visibility` joins the known keys.

## 0.22.1 (2026-11-23)

### Changed

- A scenery object's new card, from `/new object <id>` or `/edit object <id>`, inherits the built-in `scenery` template (`inherits: scenery`, REQ-CF-21) instead of repeating `object_type: scenery`, so it picks up whatever the template says scenery is.

## 0.22.0 (2026-11-23)

### Added

- Templates (REQ-CF-15, `design/STATES.md` › Templates and copies, Inheritance). `inherits:` on a thing names a template: a path from the world root (`objects/templates/book`, or `"[[objects/templates/book]]"`), or a bare name looked up in `templates/` folders from the thing's scope outward to the root. A template's frontmatter and prose stack under the thing's own files; its `id`, `template:` and `state:` never pass down. Templates may name templates, up to 4 deep. A name that finds nothing, a loop, or a chain more than 4 deep is a problem. Templates never appear in the world.
- Built-in templates (REQ-CF-21): ordinary Markdown files compiled into the engine (`src/builtins/`), found after the world's own, so a world template of the same name overrides one. The first is `scenery` (`object_type: scenery`), for a cardless object's new card.
- A thing's declared variables include its templates' `vars:`.

### Changed

- The writer checks a file that names a template (`inherits:`) only for its frontmatter, `id` and `type`, since keys such as `object_type` may come from the template. The engine types it with its template and reports any problem. Other files must still parse whole.

## 0.21.0 (2026-11-23)

### Added

- Variables in prose (REQ-CF-16, `design/STATES.md` › Variables): `{{ level.name }}`, with the level `game` (`turn`, `score`), `story` (`chapter`, and `world/story.md`'s `vars:`), `player` (`title`, and `world/player.md`'s `vars:`), `location` (the current room: `title`, `visits` and its `vars:`), `this` (the thing whose file it is) or a thing's id or link (`{{ lamp.lit }}`, `{{ castle/vault.lid }}`, split at the last dot). A value set in play holds; otherwise the declared starting value is used, and for a thing, the value in its current state and chapter. Hugo's `{{< >}}` and `{{% %}}` shortcodes are left alone.
- Play fills variables in room descriptions, examine text and action responses (after the action's changes), and an unknown one shows as blank. `WorldEngine::fill`, `variable` and `undeclared` are there for the console.
- `validate` warns about each undeclared variable in prose (things' files, chapters, the intro, the player) and each undeclared value an action's `sets:` names. A thing whose id is a level name (`game`, `story`, `player`, `location`, `this`) or contains a dot is a problem.

## 0.20.1 (2026-11-23)

### Fixed

- Edit mode listed a bundle's state files, chapter overlays, chapters, templates, area `_index.md` files and `game/` files as files that don't parse. It now asks the engine what each file is (`file_role`), lists each as what it is ("State `stale` of bread"), and `look` shows it as written. `/edit` on one says editing those comes with bundle editing. A file with a real problem shows the engine's reason.
- The file list and the file watcher now apply the world's own `ignore` list from `world.toml`, through the engine's `is_ignored`.

## 0.20.0 (2026-11-23)

### Added

- `WorldEngine::file_role(relative)` and `file_roles()` say what each file the engine read is: a thing, a state, a thing's or an area's chapter overlay, a chapter, an area, a `world/` file, a template, a `game/` file, or a problem with its reason. The console's file list no longer has to guess from `sources`.
- `WorldEngine::is_ignored(relative)` applies the loader's ignore rules, the world's own `ignore` list included.
- A link by bare file name that more than one file has is reported as ambiguous, naming the files, for exits and `becomes:` (REQ-CF-25). `WorldEngine::link_ambiguity` gives the reason.

## 0.19.0 (2026-11-23)

### Added

- Edit mode shows the files in `world/` as the world's settings (title, author, version, starting room, intro, story and player values) rather than as files that don't parse, and `/edit` opens them with any kind word.

### Fixed

- The console read the world's title and starting room from `world.toml` only, so a world moved to `world/index.md` (as the test world now is) started play in the first room by path. It now asks the engine, which reads `world/index.md` and falls back to `world.toml`.
- Edit mode's file list and the file watcher follow the loader's ignore rule: dot-names and Hugo's `layouts/`, `static/`, `assets/` and `public/` are left out. The world's own `ignore` list in `world.toml` isn't applied by the console yet.

## 0.18.0 (2026-11-23)

### Added

- Obsidian links wherever frontmatter takes an id (REQ-CF-25): exits, a room's `characters` and `objects`, `default_location`, `inventory`, an action's `becomes:` and `requires: { has: }`, and `start:` in `world/index.md`. `"[[castle/hall]]"`, `[[castle/hall|the hall]]`, `[[hall#Exits]]`, `castle/hall.md` and YAML's unquoted `[[castle/hall]]` all name `castle/hall`. A name is an id first; if no thing has that id, it's the thing whose file it is, by its path from the world root (a bundle by its folder or its `index` file) or by its bare file name when only one file has it. A link to nothing is reported like any unknown id. `WorldEngine::resolve_name` does the lookup.

## 0.17.0 (2026-11-23)

### Changed

- Edit mode moves with play mode's words (REQ-CON-4, Bruce, 2026-11-23), so the author walks the world like a game master: directions and `go <direction>` follow exits, `go <name>` goes to any room, area, folder, character or object by its path, id, last path segment or title (a name that fits several asks which), `go back` returns to the place before, `go /` goes to the root, `look` shows where you are and `examine <thing>` (`x`) shows a person or thing in the room without going to it. Arriving at a folder lists what it holds; at a character or object, it names the file.

### Removed

- `cd`, `ls` and `pwd` in edit mode. Typing one says what replaced it. Recover with `git show 3818dfe:src/console/session.rs`.

## 0.16.0 (2026-11-23)

### Added

- The `world/` folder (REQ-CF-20, `design/STATES.md`). `world/index.md` (`type: world`) gives the title, author, version and starting room (`start:`), and its prose is the intro. `world/story.md` (`type: story`) declares `story.` values with `vars:`, and `world/player.md` (a character with `character_type: player`) declares `player.` values, with its `title` as the player's name. `WorldEngine::world()` returns all of it. `world/styles/` is left to the writing helpers. Wrong types, unread keys, other files in `world/`, values that aren't single values, and a starting room that doesn't exist are reported.
- The loader ignores other tools' files (REQ-CF-24): any name starting with `.` (`.obsidian/`, `.vscode/`, `.git/`, which it no longer walks into) and Hugo's `layouts/`, `static/`, `assets/` and `public/` at the world's root. A world can add folders with `ignore = ["drafts"]` in `world.toml`.

### Changed

- The test world's title, author, version and starting room move from `world.toml` to `world/index.md`, and its description becomes the intro. A world with no `world/index.md` still has its `world.toml` `[metadata]` and `[settings] starting_location` read, so older worlds and the transcript fixture keep working.

### Removed

- From the test world's `world.toml`, keys nothing read: `world_size`, `locations_count`, `npcs_count`, the `[areas]` counts, `[characters]` (`default_health`, `default_mood`, `relationship_system`), `[objects]` (`dedicated_definitions`, `object_interactions`, `containers_enabled`) and `[validation]` (`strict_references`, `coordinate_checking`, `connection_validation`). Recover them with `git show 02e8fe8:authors/mx/test-world/world.toml`.

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
