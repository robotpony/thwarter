# Plan

The phased work queue. Each task is one line that points to its spec, with its lane in brackets. Phases are `##` headings with checkboxes, so Space Command can read this file. Defects go in `BUGS.md`, what each component does today in `design/ARCHITECTURE.md` › Implementation status, and retired plans in `CHANGELOG.md`.

Authoring comes first, play second (Bruce, 2026-11-23). Each phase from 4 on says when it's done. Phase 7, the sample world, runs alongside Phases 4 to 6, and what Bruce needs while writing it can reorder their work. The console is where both happen. Engine work is done only when authoring, then play, needs it, never on its own until it needs refactoring. Each phase brings its own tests; the console's acceptance tests are transcripts (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-9).

## Phase 1 · Foundation

- [x] Done (squashed; 0.1.0–0.1.1, 2e4d080 to 41a583c): the build fixed, frontmatter-first content format and parser, real `validate` and `show` (a readable room view since 0.1.1), honest "not implemented" for `map`, `search` and `debug`, the design docs split. The original 20-day plan is in `git show 884ae96:tasks/cli-first-pass-TODO.md`.

## Phase 2 · Console shell

- [x] `thwart edit <world>` and `thwart play <world>`, handing off to `src/console/` [coder] (REQ-CON-1)
- [x] The console loop: `/edit`, `/play`, `/reload`, `/help`, `/quit` [console] (REQ-CON-2, REQ-CON-3, REQ-CON-7)
- [x] Edit-mode navigation by area and room, like folders [console] (REQ-CON-4)
- [x] Room view in the console, from the same renderer as `thwart show` [console, coder] (REQ-CON-8)
- [x] Transcript test runner [console] (REQ-CON-9)
- [x] Parser tests, before any writer work starts [coder] (`design/UNIT_TESTS.md` › 1)

## Phase 3 · Authoring

- [x] Round-trip file writer: change one field and keep everything else as it was [coder] (REQ-ENG-7)
- [x] The loader records which file each `id` came from, and `validate` reports an `id` defined in two files [coder] (REQ-ENG-7)
- [x] `validate` warns about a key that differs only in case from a known one, such as `Inventory:` [coder] (`design/CONTENT_FORMAT.md` › Keys, order and file locations)
- [x] Changes to the loaded world in memory, checked after each one [coder] (REQ-ENG-8)
- [x] `validate` warns when two kinds share an `id`, such as a location and an object [coder] (`design/CONTENT_FORMAT.md` › Keys, order and file locations)
- [x] `/new room <direction>`, `/new object`, `/new character` [console] (REQ-CON-5)
- [x] `/set` and `/link` for single fields and exits [console] (REQ-CON-5)
- [x] `/edit room`, `/edit object`, `/edit character`: full editing in the terminal [console] (REQ-CON-6)
- [x] An object without a card gets one on its first edit [console, coder] (REQ-CF-8)
- [x] File watching for `/reload` [console] (REQ-CON-7)
- [x] Line editing and history at the console prompt (arrow keys) [console] (REQ-CON-2)

## Phase 4 · World model

The engine side of `design/STATES.md` (run by the commander with the coder). Done when `validate` and `show` handle every construct in `design/STATES.md` except doors.

- [x] Done (squashed; 0.7.0 to 0.14.0, f987904 to e4e87e6): play verbs, action effects, and save and load from the console; the states, bundles and chapters design, agreed with Bruce; bundle loading, layers and merging with key sources; current versions on request, play state v2 and saves v2; actions as a map of mechanics with responses as body blocks, and the 9 object cards migrated. The old Phase 4 list is in `git show e4e87e6:PLAN.md`.
- [x] The `world/` folder: `index.md`, `story.md`, `player.md`, and the test world moved off `world.toml` [coder] (REQ-CF-20)
- [x] The loader ignores other tools' files: dot-folders and Hugo's own [coder] (REQ-CF-24)
- [x] Obsidian links wherever frontmatter takes an id, as a file path or an id [coder] (REQ-CF-25)
- [x] Variables by level, with warnings for undeclared ones [coder] (REQ-CF-16)
- [x] Templates and built-ins, with `scenery` the first built-in [coder] (REQ-CF-15, REQ-CF-21)
- [x] `visibility:` in play: faint, hidden and absent [coder] (REQ-CF-18)
- [x] `reply ambiguous` from the candidates' shared template, else an in-world default [coder] (`design/STATES.md` › Ambiguity)
- [x] Links in prose, Markdown rendered in play, and tags [coder] (REQ-CF-28 to 30)
- [x] `design/STATES.md` › Engine rules written as REQ-ENG items [coder] (`design/ENGINE_REQUIREMENTS.md`)

- [x] `thwart show` for any thing (objects, characters, templates and built-ins), not only locations [coder] (`design/CLI_REQUIREMENTS.md`)
- [x] `thwart show` for the `world/` definition and the chapters [coder] (`design/CLI_REQUIREMENTS.md`)

## Phase 5 · Authoring the model

The console side (Bruce with the console coder). Done when an author can build a bundle with states and a chapter, and run the world, without touching a file by hand.

- [x] `/restart` onto a new build, keeping the session (0.15.0, 3818dfe) [console] (REQ-CON-30, REQ-CON-31)
- [x] Edit mode moves with play words (go, go back, look, examine), replacing cd, ls and pwd (0.17.0) [console] (REQ-CON-4)
- [x] Full-screen console: status bar, output, a sidebar with GAME and HERE, the editor inside it (0.25.0) [console] (REQ-CON-22, REQ-CON-23)
- [x] The `/` command menu with argument completion (0.38.0) [console] (REQ-CON-33)
- [ ] The `game/` folder, and `/set` in it changing only the running game [console] (REQ-CON-24, REQ-CON-25)
- [ ] Scope: a sticky default, per-command overrides, `/move` [console] (REQ-CON-26 to 29)
- [x] Turning a one-file thing into a bundle, and saving a bundle's sections together [coder] (REQ-ENG-28, REQ-ENG-29)
- [ ] Bundle editing as one document, `/state` and `/chapter` commands [console] (`design/STATES.md` › Console)
- [ ] Response hints: which responses an object needs and could have [console] (`design/STATES.md` › Responses)
- [x] Renaming an id with every reference updated: file or bundle moved, exits, lists, `default_location`, `start:`, prose links and variables (0.30.0, 318144e) [coder] (REQ-ENG-30)
- [x] Renaming also rewrites references inside actions (`becomes:`, `requires:`), which are reported for now [coder] (REQ-ENG-30)
- [ ] Proposing a rename when two kinds share an id [console] (`design/ENGINE_REQUIREMENTS.md` › Ids and values)
- [x] Removing a thing: its file or bundle deleted, and every reference to its id taken out (Bruce, 2026-11-24) [coder] (`design/ENGINE_REQUIREMENTS.md`, beside REQ-ENG-30)
- [ ] `/remove <thing>` deletes an object, room, character or door after the author confirms; no undo yet (Bruce, 2026-11-24) [console] (`design/CONSOLE_REQUIREMENTS.md`, beside REQ-CON-5)
- [ ] Tag search, once designed with Bruce [console] (REQ-CF-30)
- [ ] Showing a template or built-in in the console (`/template`) [console] (REQ-CF-21)
- [x] Objects hold objects: loading, scope, examine lists, one place per thing, unified with `keep()` [coder] (REQ-CF-31)
- [ ] `/move` and `/add` into an object, and held things in the sidebar [console] (REQ-CF-31)
- [x] Question sets: the loader reads `questions/` folders and built-ins, and `validate` checks them [coder] (REQ-CF-32, `design/CONTENT_FORMAT.md` › `type: questions`)
- [x] `/new` and `/add` ask a set's questions, run the answers and print the commands they stood for, with the four recipes (0.60.0, 0.61.0) [console] (REQ-CF-32)
- [x] Question sets chosen by id, then template, then kind, with no warning for a set whose name matches nothing yet (d2665a8) [coder] (REQ-CF-32)
- [x] Question sets: `set <key> <value>` with the value written in, sequenced with the console's `Construct::Set` (dc66a64, 0.62.1) [coder, console] (REQ-CF-32)
- [ ] Review the built-in question content with Bruce, once the mechanics work [commander, Bruce]

## Phase 6 · Doors and play

Done when a locked-door puzzle plays from both sides.

- [x] Doors as their own type, with sides, `passable:` and `transparent:` [coder] (REQ-CF-19, `design/STATES.md` › Doors)
- [x] Door built-ins: `door` and `portal` [coder] (REQ-CF-21)
- [x] The engine matches `## When:` sections against each turn's game state, and stops reading markers (0.46.0) [coder] (`design/SECTIONS.md` › The model)
- [x] `@name` in prose shows the thing's name in play [coder] (`design/SECTIONS.md` › Prefixes, links and names)
- [x] Rewrite the `door` and `portal` built-ins and the test world's marker files as sections [coder] (`design/SECTIONS.md` › What changes)
- [x] The editor draws keyword headings and colours `@`, `#` and `thwarter` blocks (0.44.0) [console] (`design/SECTIONS.md`)
- [ ] Try sections on real content with Bruce, then fold `design/SECTIONS.md` into the content requirements [commander, Bruce]
- [x] Rhai scripts: `actions.rhai` and ```rhai blocks, functions as actions, outcomes, keep and the owned scope [coder] (`design/SECTIONS.md` › Scripts, Outcomes and replies)
- [x] The `locked-door` built-in as the vault door example shows [coder] (REQ-CF-21, `design/STATES.md` › Keys)
- [x] Connections between things for scripts (`connects:`) [coder] (`design/STATES.md` › One thing affecting another)
- [x] `/add door west hallway`: doors made from the console, with `door_id` from the engine (28692c1, 66a1b27) [console, coder] (`design/STATES.md` › Adding a door from the console, REQ-CON-42)
- [x] `/validate` in the console, on the engine's one validation call (74681d1) [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [x] The world's problems in a pane at the sidebar's foot, like an IDE (74681d1) [console] (REQ-CON-23)
- [x] Keys for locked doors: designed with Bruce as a scripted built-in [commander] (`design/STATES.md` › Keys)
- [ ] Clocks and game values: time, weather, randomness [commander, coder] (`design/STATES.md` › Open questions)
- [x] The player command parser moves into the engine as `parse_command` [coder] (REQ-PC-1)
- [x] An unknown verb with a noun no thing answers to says it doesn't understand [coder] (REQ-PC-14)
- [x] Play mode parses through the engine and runs object actions (0.42.1) [console] (REQ-PC-1, REQ-PC-13)
- [x] Save paths and the save and load calls move into the engine (0.43.2) [coder, console] (REQ-PC-18)
- [ ] Settle the player command open questions with Bruce [commander] (`design/PLAYER_COMMANDS.md` › Open questions)

## Phase 7 · A real sample world

Bruce authors it with the console coder, starting now, using what exists and growing as features land. Done when it plays start to finish and has a transcript test.

- [ ] Write the world: a short story that uses states, chapters, templates and a door [Bruce, console]
- [x] The old test content removed, and the test world restarted from Bruce's rooms (0.50.2, 37a5c49) [Bruce, coder]
- [ ] A transcript that plays it through [console] (REQ-CON-9)

Asked for in playtesting (from the unknown-input log, 2026-11-23). Queued: Bruce playtests edit and play modes before any of these start.

- [ ] `where` and `where am i` [coder, console] (REQ-PC-21)
- [ ] `enter <place>` as `go` [coder] (REQ-PC-22)
- [ ] `look <direction>` and `what's <direction>` [coder] (REQ-PC-23)
- [ ] Typo suggestions for verbs [coder] (REQ-PC-24)
- [ ] `/list objects|rooms|characters` in edit mode [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [ ] `/map` from the world's root draws the whole world, instead of asking for a room [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [ ] A bare console command name in edit mode (`edit`) runs it or asks "Did you mean /edit?" [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [ ] An unknown slash command suggests the nearest one [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [x] At the world's root, edit mode lists `objects/` and `world/` below the rooms with labels ("World settings", "Global objects: 4"), and `/edit world` opens `world/index.md` (Bruce, 2026-11-24; 0.62.1) [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [x] Edit mode's room view introduces exits, people and things with prose from `[play]` and `[console]`, not headings (Bruce, 2026-11-24) [coder] (shared renderer; `design/ENGINE_TEXT.md` › Where tables live)
- [x] One blank line before and after every block of output, the prompt included; the screen's tests check it, since transcripts are compact (881fe90, Bruce, 2026-11-24) [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [x] Sidebar fixes from Bruce's playtest: the problems pane's wrap and literal backticks (881fe90) [console]
- [ ] A door's exit shows its state ("closed", "locked"), not "not passable", and its name before its id [coder] (shared renderer)
- [x] Play reads rooms and replies as the story: people right after the prose, held things on a plain look, doors named in the ways out (95ed0c9, REQ-ENG-43, 45, 46)
- [ ] New verbs `drop`, `talk`, `ask` and `give` [coder] (`design/PLAYER_COMMANDS.md` › Open question 1; Bruce to choose)

## Phase 8 · Text as data

Every sentence the engine, play and the console write comes from a text table with compiled-in defaults, the same shape everywhere (Bruce, 2026-11-24). Started 2026-11-24, at Bruce's go. Done when no user-facing string is left in the source outside the defaults files and a test keeps it that way.

- [x] One `Texts` type for `[play]`, `[engine]` and `[console]` tables, defaults from `src/builtins/text/` [coder] (`design/ENGINE_TEXT.md` › The shape)
- [x] Engine and play strings become `[engine]` keys, the default text unchanged [coder]
- [ ] Console strings become `[console]` keys, the default text unchanged [console]
- [ ] A test that fails on a new user-facing string outside the defaults [coder, console]
- [x] Overrides: `world.toml`'s `[play]` and `[engine]` over the defaults, with validate's warning for an unknown key [coder]
- [ ] Overrides: the personal config's `[console]` over the defaults, with a warning for an unknown key [console]
- [ ] Story overrides: a `## When:` section replaces an `[engine]` reply by exact outcome tag, reserved outcome names, `{{ turn.x }}` slots [coder] (`design/ENGINE_TEXT.md` › Story overrides of built-in replies)
- [x] Edit mode's room view reads as play's plus its title line; ids, targets and door states go to the console as a `LocationView` [coder] (`design/ENGINE_TEXT.md` › Where tables live)
- [x] The sidebar shows the current room's ids, exits and doors, and `/inspect <thing>` shows one in full (0.65.0) [console] (`design/CONSOLE_REQUIREMENTS.md`)
- [x] Edit mode's room view drops its title line (f17588d, 0.66.2); `show_room_names` decides in both modes, and the room's title and id show in the sidebar [coder, console] (`design/ENGINE_TEXT.md` › Where tables live)

## Phase 9 · Writing helpers

AI tools for prose in edit mode. Done when an author can review and rewrite a room's prose from the console.

- [ ] Review the writing helper requirements with Bruce [console] (REQ-CON-13 to 21)
- [ ] Review, rewrite, flesh out and apply a style [console] (REQ-CON-13 to 17)
- [ ] Styles as files in `world/styles/` [console, coder] (REQ-CON-18)
- [ ] Turning helpers on per world, with the service and model shown [console] (REQ-CON-19, REQ-CON-21)

## Phase 10 · Authors and sharing

Done when a persona can package a world, and someone else can install it and play it.

- [x] Persona files, and `author:` in `world/index.md` naming one [coder] (REQ-WS-4, REQ-WS-5)
- [x] Personal settings in `~/.config/thwarter/config.toml` [coder] (REQ-CFG-4)
- [ ] First run asks who's writing; `/author` switches persona; the status bar shows it; packaging says which contact details ship [console] (REQ-CON-34 to 37, REQ-WS-4)
- [x] Packaging a world with its persona, and installing one to play [coder] (REQ-CFG-5, REQ-CFG-7)
- [x] Play data for installed worlds kept apart from them (0.36.1) [coder, console] (REQ-CFG-6)

## Phase 11 · Tooling, as needed


Built when authoring or play asks for them, most likely as console commands. Until then they stay as they are.

- [x] `WorldGraph::find_path` and a map [coder] (`design/CLI_DESIGN.md` › `map`)
- [x] Search over locations, characters and objects [coder] (`design/CLI_DESIGN.md` › `search`)
- [ ] `CharacterEngine` wired into `show` and `validate --strict` [coder] (`design/ARCHITECTURE.md` › Character Engine)
- [ ] Read `thwart.toml` through `ConfigManager` [coder] (REQ-CFG-1)
- [x] `validate` checks a location's `id` against its file path [coder] (`design/CONTENT_FORMAT.md` › What's still open)
- [x] Minimal CI: `cargo build` and `cargo test` on push [coder] (`TESTING.md` › CI)

## Open questions

Open questions live with their component: `design/STATES.md` (the world model), `design/CONTENT_REQUIREMENTS.md` (the format) and `design/CONSOLE_REQUIREMENTS.md` (the console).
