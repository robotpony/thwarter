# Plan

The phased work queue. Each task is one line that points to its spec, with its lane in brackets. Phases are `##` headings with checkboxes, so Space Command can read this file. Defects go in `BUGS.md`, what each component does today in `design/ARCHITECTURE.md` › Implementation status, and retired plans in `CHANGELOG.md`.

Authoring comes first, play second (Bruce, 2026-11-23). The console is where both happen. Engine work is done only when authoring, then play, needs it, never on its own until it needs refactoring. Each phase brings its own tests; the console's acceptance tests are transcripts (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-9).

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
- [ ] `/edit room`, `/edit object`, `/edit character`: full editing in the terminal [console] (REQ-CON-6)
- [ ] An object without a card gets one on its first edit [console, coder] (REQ-CF-8)
- [x] File watching for `/reload` [console] (REQ-CON-7)
- [x] Line editing and history at the console prompt (arrow keys) [console] (REQ-CON-2)

## Phase 4 · Play

- [x] Play-mode verbs and aliases, v1 [console] (REQ-CON-10)
- [x] Action effects for move, look, take, examine and inventory [coder] (REQ-ENG-5)
- [ ] Design states, bundles and chapters, replacing flags, conditions and alternative text; draft written, Bruce reviewing [commander] (`design/STATES.md`)
- [ ] Bundles, states, chapters and variables in the engine, once `design/STATES.md` is agreed [coder] (`design/STATES.md` › Engine changes)
- [ ] `/state`, `/chapter` and bundle editing in the console [console] (`design/STATES.md` › Console)
- [x] Save and load from the console [console, coder] (REQ-CON-11)

## Phase 5 · Tooling, as needed

Built when authoring or play asks for them, most likely as console commands. Until then they stay as they are.

- [ ] `WorldGraph::find_path` and a map [coder] (`design/CLI_DESIGN.md` › `map`)
- [ ] Search over locations, characters and objects [coder] (`design/CLI_DESIGN.md` › `search`)
- [ ] `CharacterEngine` wired into `show` and `validate --strict` [coder] (`design/ARCHITECTURE.md` › Character Engine)
- [ ] Read `thwart.toml` through `ConfigManager` [coder] (REQ-CFG-1)
- [ ] `validate` checks a location's `id` against its file path [coder] (`design/CONTENT_FORMAT.md` › What's still open)
- [ ] Minimal CI: `cargo build` and `cargo test` on push [coder] (`TESTING.md` › CI)

## Open questions

The console's open questions (how full editing looks, play mode's limits, return exits for `/new room`) are in `design/CONSOLE_REQUIREMENTS.md` › Open questions.
