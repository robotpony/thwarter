# Plan

The phased work queue. Each task is one line that points to its spec, with its lane in brackets. Phases are `##` headings with checkboxes, so Space Command can read this file. Defects go in `BUGS.md`, what each component does today in `design/ARCHITECTURE.md` › Implementation status, and retired plans in `CHANGELOG.md`.

Authoring comes first, play second (Bruce, 2026-11-23). Each phase from 4 on says when it's done. Phase 7, the sample world, runs alongside Phases 4 to 6, and what Bruce needs while writing it can reorder their work. The console is where both happen. Engine work is done only when authoring, then play, needs it, never on its own until it needs refactoring. Each phase brings its own tests; the console's acceptance tests are transcripts (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-9).

## Phase 1 · Foundation

- [x] Done (squashed; 0.1.0–0.1.1, 2e4d080 to 41a583c): the build fixed, frontmatter-first content format and parser, real `validate` and `show` (a readable room view since 0.1.1), honest "not implemented" for `map`, `search` and `debug`, the design docs split. The original 20-day plan is in `git show 884ae96:tasks/cli-first-pass-TODO.md`.

## Phase 2 · Console shell

- [x] Done (squashed): 6 items, from `thwart edit` and `thwart play` handing off to the console to the transcript runner and parser tests; the full list is `git show a33296b:PLAN.md`.

## Phase 3 · Authoring

- [x] Done (squashed): 11 items, from the round-trip writer to line editing at the prompt; the full list is `git show a33296b:PLAN.md`.

## Phase 4 · World model

The engine side of `design/STATES.md` (run by the commander with the coder). Done when `validate` and `show` handle every construct in `design/STATES.md` except doors.

- [x] Done (squashed): play verbs, states, bundles and chapters, the `world/` folder, variables, templates and built-ins, visibility, ambiguity, links and tags, and `thwart show` for every kind of thing. The 0.7.0 to 0.14.0 list is in `git show e4e87e6:PLAN.md`; the rest is in `git show a33296b:PLAN.md`.

## Phase 5 · Authoring the model

The console side (Bruce with the console coder). Done when an author can build a bundle with states and a chapter, and run the world, without touching a file by hand.

- [x] Done (squashed): 25 items, among them `/restart`, the full-screen console, the `/` menu, the `game/` folder, renaming and removing with every reference updated, forgiving verbs, the world picker, doors named from both sides, `/start`, TO WRITE, objects that hold objects, question sets and scope with `/move`, to 0.92.0; the full list is `git show a33296b:PLAN.md`, and scope is in `git show c485a58:PLAN.md`.
- [x] Bundle editing as one document, `/state` and `/chapter` commands [console] (`design/STATES.md` › Console)
- [ ] Response hints: which responses an object needs and could have [console] (`design/STATES.md` › Responses)
- [ ] Proposing a rename when two kinds share an id [console] (`design/ENGINE_REQUIREMENTS.md` › Ids and values)
- [ ] One command registry (built-ins, aliases, prefixes and did-you-mean done in 0.74.0; world commands and the config layer to go): every console command in a `type: command` file, world commands as `run:` lists for the world's authors, aliases, unique prefixes and did-you-mean (agreed by Bruce, 2026-11-24) [console] (REQ-CON-50 to 56)
- [ ] Author-only console features check `WorldEngine::is_author`; `authors:`, `thwart new` and the picker's `authors: $USER` are built (0.72.0 to 0.83.1) [console] (REQ-CON-55, REQ-WS-7)
- [ ] World commands MAY be Rhai scripts that read the world and run console commands, with the world scripts' limits (Bruce, 2026-11-24) [console] (`design/CONTENT_FORMAT.md` › type: command)
- [ ] Tag search, once designed with Bruce [console] (REQ-CF-30)
- [ ] Showing a template or built-in in the console (`/template`) [console] (REQ-CF-21)
- [ ] `/move` and `/add` into an object, and held things in the sidebar [console] (REQ-CF-31)
- [ ] A quick menu in the sidebar for where you are: add action, add state, add chapter version, link to, add a reply, add a thing or door, recipes, hide or reveal (a state's `visibility:`), starting state, play from here. Each entry asks its questions in a picker, not a typed command, at the bottom of the console, opened with Ctrl-M at the prompt and in the editor, or `/menu` (Bruce, 2026-11-25; Ctrl-Space didn't reach the console) [console] (`design/CONSOLE_REQUIREMENTS.md` › Quick menu, REQ-CON-63 to 68)
- [x] Quick menu questions get a hint line with examples, taken from the world where it has them (the thing's own states, nearby rooms), and choices say what each option means (Bruce, 2026-11-25) [console] (REQ-CON-65)
- [x] A drop-down console over the editor, opened with a key like the `~` of games, running any command (`/menu` among them) for the open file; the editor shows only keys that work in it (Bruce, 2026-11-25) [console] (requirement to come in `design/CONSOLE_REQUIREMENTS.md`)
- [x] Mouse clicks in the sidebar: quick menu entries, TO WRITE names and exits; Shift-drag selects text (Bruce, 2026-11-25) [console] (requirement to come in `design/CONSOLE_REQUIREMENTS.md`)
- [ ] Review the built-in question content with Bruce, once the mechanics work [commander, Bruce]

## Phase 6 · Doors and play

Done when a locked-door puzzle plays from both sides.

- [x] Done (squashed): 18 items, from doors as their own type and `## When:` sections to Rhai scripts, keys, `/add door`, `/validate`, the parser in the engine, and clocks with game time and weather (0.94.0); the full list is `git show a33296b:PLAN.md`, and clocks are in `git show c485a58:PLAN.md`.
- [ ] Try sections on real content with Bruce, then fold `design/SECTIONS.md` into the content requirements [commander, Bruce]
- [ ] Settle the player command open questions with Bruce: 1, 2, 3 and 5 settled 2026-11-25, 4 (game commands every client needs) to go [commander] (`design/PLAYER_COMMANDS.md` › Open questions)
- [ ] Multi-word verbs a world names (`#turn-on`), longest first, with built-in `look in` and `look under`; paused until Bruce restarts the coder [coder] (`design/PLAYER_COMMANDS.md` › Open question 5)
- [ ] `refuse: true` in a built-in verb's section stops the verb for that thing; paused until Bruce restarts the coder [coder] (`design/PLAYER_COMMANDS.md` › Open question 3)

## Phase 7 · A real sample world

Bruce authors it with the console coder, starting now, using what exists and growing as features land. Done when it plays start to finish and has a transcript test.

- [x] Done (squashed): 19 items, the test world restarted and every playtest ask so far: `where`, `enter`, `look <direction>`, typo suggestions, `/list`, `/map` of the whole world, bare command names, prose room views, door states, `drop`, `give`, `talk`, `ask`, `take all`, `wait`, `again`, `it` and the story verbs, to 0.90.0; the full list is `git show a33296b:PLAN.md`, and the story verbs are in `git show c485a58:PLAN.md`.
- [ ] Write the world: a short story that uses states, chapters, templates and a door [Bruce, console]
- [ ] A transcript that plays it through [console] (REQ-CON-9)

## Phase 8 · Text as data

Every sentence the engine, play and the console write comes from a text table with compiled-in defaults, the same shape everywhere (Bruce, 2026-11-24). Started 2026-11-24, at Bruce's go; done 2026-11-25. Done when no user-facing string is left in the source outside the defaults files and a test keeps it that way.

- [x] Done (squashed): 13 items, from the shared `Texts` type to the widened guard test, 2026-11-24 to 25; the full list is `git show 60c6e88:PLAN.md`. Thing-kind labels and tooling text went to Phase 11 (Bruce, 2026-11-25).

## Phase 9 · Writing helpers

AI tools for prose in edit mode. Done when an author can review and rewrite a room's prose from the console.

- [ ] Review the writing helper requirements with Bruce [console] (REQ-CON-13 to 21)
- [ ] Review, rewrite, flesh out and apply a style [console] (REQ-CON-13 to 17)
- [ ] Styles as files in `world/styles/` [console, coder] (REQ-CON-18)
- [ ] Turning helpers on per world, with the service and model shown [console] (REQ-CON-19, REQ-CON-21)

## Phase 10 · Authors and sharing

Done when a persona can package a world, and someone else can install it and play it.

- [x] Done (squashed): 4 items: personas, personal settings, packaging and installing, and play data kept apart; the full list is `git show a33296b:PLAN.md`.
- [ ] First run asks who's writing; `/author` switches persona; the status bar shows it; packaging says which contact details ship [console] (REQ-CON-34 to 37, REQ-WS-4)

## Phase 11 · Tooling, as needed

Built when authoring or play asks for them, most likely as console commands. Until then they stay as they are.

- [x] Done (squashed): 4 items: the map, search, the location id check and CI; the full list is `git show a33296b:PLAN.md`.
- [ ] `CharacterEngine` wired into `show` and `validate --strict` [coder] (`design/ARCHITECTURE.md` › Character Engine)
- [ ] Read `thwart.toml` through `ConfigManager` [coder] (REQ-CFG-1)
- [ ] Validation warnings, author notes and `thwart` command-line output move to text tables (Bruce, 2026-11-24) [coder] (`design/ENGINE_TEXT.md` › Settled questions)
- [ ] The console's thing-kind labels ("world settings", "chapter overlay" and the rest in `world.rs`) move to `[console]`; they double as words authors type (`/edit room`), so this needs a text-aware label at six call sites (Bruce, 2026-11-25) [console] (`design/ENGINE_TEXT.md` › Settled questions)

## Open questions

Open questions live with their component: `design/STATES.md` (the world model), `design/CONTENT_REQUIREMENTS.md` (the format) and `design/CONSOLE_REQUIREMENTS.md` (the console).
