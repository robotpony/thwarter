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
- [ ] The `/` command menu with argument completion [console] (REQ-CON-33)
- [ ] The `game/` folder, and `/set` in it changing only the running game [console] (REQ-CON-24, REQ-CON-25)
- [ ] Scope: a sticky default, per-command overrides, `/move` [console] (REQ-CON-26 to 29)
- [ ] Bundle editing as one document, `/state` and `/chapter` commands [console] (`design/STATES.md` › Console)
- [ ] Response hints: which responses an object needs and could have [console] (`design/STATES.md` › Responses)
- [ ] Renaming an id with every reference updated, and proposed renames for shared ids [coder, console] (`design/ENGINE_REQUIREMENTS.md` › Ids and values)
- [ ] Tag search, once designed with Bruce [console] (REQ-CF-30)
- [ ] Showing a template or built-in in the console (`/template`) [console] (REQ-CF-21)

## Phase 6 · Doors and play

Done when a locked-door puzzle plays from both sides.

- [ ] Doors as their own type, with sides, `passable:` and `transparent:` [coder] (REQ-CF-19, `design/STATES.md` › Doors)
- [ ] Door built-ins: `door`, `locked-door`, `portal` [coder] (REQ-CF-21)
- [ ] Linking two rooms through a door template from the console [console] (`design/STATES.md` › Doors)
- [ ] Keys for locked doors: a variable inside `requires:` [commander, coder] (`design/STATES.md` › Open questions)
- [ ] Clocks and game values: time, weather, randomness [commander, coder] (`design/STATES.md` › Open questions)

## Phase 7 · A real sample world

Bruce authors it with the console coder, starting now, using what exists and growing as features land. Done when it plays start to finish and has a transcript test.

- [ ] Write the world: a short story that uses states, chapters, templates and a door [Bruce, console]
- [ ] Fix or remove the test world's five broken exits along the way [Bruce] (`BUGS.md`)
- [ ] A transcript that plays it through [console] (REQ-CON-9)

## Phase 8 · Writing helpers

AI tools for prose in edit mode. Done when an author can review and rewrite a room's prose from the console.

- [ ] Review the writing helper requirements with Bruce [console] (REQ-CON-13 to 21)
- [ ] Review, rewrite, flesh out and apply a style [console] (REQ-CON-13 to 17)
- [ ] Styles as files in `world/styles/` [console, coder] (REQ-CON-18)
- [ ] Turning helpers on per world, with the service and model shown [console] (REQ-CON-19, REQ-CON-21)

## Phase 9 · Authors and sharing

Done when a persona can package a world, and someone else can install it and play it.

- [ ] Persona files, and `author:` in `world/index.md` naming one [coder] (REQ-WS-4, REQ-WS-5)
- [ ] Personal settings in `~/.config/thwarter/config.toml` [coder] (REQ-CFG-4)
- [ ] First run asks who's writing; `/author` switches persona; the status bar shows it; packaging says which contact details ship [console] (REQ-CON-34 to 37, REQ-WS-4)
- [ ] Packaging a world with its persona, and installing one to play [coder] (REQ-CFG-5, REQ-CFG-7)
- [ ] Play data for installed worlds kept apart from them [coder, console] (REQ-CFG-6)

## Phase 10 · Tooling, as needed


Built when authoring or play asks for them, most likely as console commands. Until then they stay as they are.

- [ ] `WorldGraph::find_path` and a map [coder] (`design/CLI_DESIGN.md` › `map`)
- [ ] Search over locations, characters and objects [coder] (`design/CLI_DESIGN.md` › `search`)
- [ ] `CharacterEngine` wired into `show` and `validate --strict` [coder] (`design/ARCHITECTURE.md` › Character Engine)
- [ ] Read `thwart.toml` through `ConfigManager` [coder] (REQ-CFG-1)
- [ ] `validate` checks a location's `id` against its file path [coder] (`design/CONTENT_FORMAT.md` › What's still open)
- [ ] Minimal CI: `cargo build` and `cargo test` on push [coder] (`TESTING.md` › CI)

## Open questions

Open questions live with their component: `design/STATES.md` (the world model), `design/CONTENT_REQUIREMENTS.md` (the format) and `design/CONSOLE_REQUIREMENTS.md` (the console).
