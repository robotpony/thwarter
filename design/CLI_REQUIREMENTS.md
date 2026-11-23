# CLI requirements

Requirements for the `thwart` command-line tools: `validate`, `show`, `map`, `search` and `debug`. Owned by thwarter coder. Command-by-command detail, including commands not built yet, is in `design/CLI_DESIGN.md`. RFC 2119 keywords are used as in `REQUIREMENTS.md`.

Split out of `REQUIREMENTS.md` on 2026-11-23. The REQ-CLI ids are stable. A change another component needs here goes to the commander as a message.

## Requirements

- REQ-CLI-1: The binary MUST expose, at minimum, `validate`, `show`, `map`, `search`, and `debug` subcommands.
- REQ-CLI-2: `--world <path>` MUST be available as a global option overriding the configured default world path for that invocation.
- REQ-CLI-3: `validate` MUST load the world and report, at minimum: files that failed to parse, and connections whose target location `id` was never loaded. `validate --strict` MUST additionally check that a character's `default_location` refers to a loaded location.
- REQ-CLI-4: `validate` MUST exit with a non-zero status if any problem was found, and zero otherwise, so it's usable as a CI gate.
- REQ-CLI-5: `show` MUST display the world's summary (REQ-CLI-11) with no argument, or, given a name, that thing (REQ-CLI-9). It MUST error clearly, not silently return nothing, if the name matches nothing in the loaded world.
- REQ-CLI-6: Output MUST support text, JSON, and YAML formats via `--format`, selectable independently per command.
- REQ-CLI-7: A command not yet implemented MUST say so and exit non-zero, and MUST NOT produce output that looks like a real result (a fabricated map, invented search matches, or made-up performance numbers). This requirement exists because the first implementation of this CLI violated it in `map`, `search`, and `debug`; see `CHANGELOG.md` › 0.1.0.
- REQ-CLI-8: `map` (ASCII rendering of the world graph, full or fog-of-war from a player's perspective) and `search` (content search across locations/characters/objects, with `--regex` and type filtering) are specified in `design/CLI_DESIGN.md` but have no implementation requirement attached to a milestone yet; see Open Questions (§8).
- REQ-CLI-9: `show <name>` MUST show any thing: a location, object or character, named by id or by link (REQ-CF-25), in its starting version, or in another with `--state <name>` and `--chapter <name>` (REQ-ENG-15). When a name is the id of more than one kind, `show` MUST say so and take `--kind location|object|character` to choose. The text form is the author's view: a location as the shared room view (REQ-CON-8), an object with its fields and its actions' mechanics (marking an action with no response), a character with where it starts and what it carries, then, for a thing with any, its states (marking the starting and shown one), its chapter versions, and its templates in the order they apply. `--format json` and `yaml` give the version's typed data.
- REQ-CLI-10: `show --template <name>` MUST show a template or built-in (REQ-CF-21) exactly as written, so an author can copy it, and say where it was found (its file, or built in). A bare name is looked up from the world root as REQ-ENG-19 describes; a path from the root finds a template in any scope. An unknown name MUST be an error that lists the templates there are.
- REQ-CLI-11: `show` with no name MUST summarize the world: its `world/` definition (title, author, version, starting room, intro), its story values and player, every chapter with its order, title and scope, and the ids of every location, object and character, sorted. `--format json` and `yaml` MUST give the same as data.
