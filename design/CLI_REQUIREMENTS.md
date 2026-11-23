# CLI requirements

Requirements for the `thwart` command-line tools: `validate`, `show`, `map`, `search` and `debug`. Owned by thwarter coder. Command-by-command detail, including commands not built yet, is in `design/CLI_DESIGN.md`. RFC 2119 keywords are used as in `REQUIREMENTS.md`.

Split out of `REQUIREMENTS.md` on 2026-11-23. The REQ-CLI ids are stable. A change another component needs here goes to the commander as a message.

## Requirements

- REQ-CLI-1: The binary MUST expose, at minimum, `validate`, `show`, `map`, `search`, and `debug` subcommands.
- REQ-CLI-2: `--world <path>` MUST be available as a global option overriding the configured default world path for that invocation.
- REQ-CLI-3: `validate` MUST load the world and report, at minimum: files that failed to parse, and connections whose target location `id` was never loaded. `validate --strict` MUST additionally check that a character's `default_location` refers to a loaded location.
- REQ-CLI-4: `validate` MUST exit with a non-zero status if any problem was found, and zero otherwise, so it's usable as a CI gate.
- REQ-CLI-5: `show` MUST display either the whole loaded world (at minimum, the set of loaded location IDs) or, given a location argument, that location's full parsed data; it MUST error clearly, not silently return nothing, if the given location `id` doesn't exist in the loaded world.
- REQ-CLI-6: Output MUST support text, JSON, and YAML formats via `--format`, selectable independently per command.
- REQ-CLI-7: A command not yet implemented MUST say so and exit non-zero, and MUST NOT produce output that looks like a real result (a fabricated map, invented search matches, or made-up performance numbers). This requirement exists because the first implementation of this CLI violated it in `map`, `search`, and `debug`; see `CHANGELOG.md` › 0.1.0.
- REQ-CLI-8: `map` (ASCII rendering of the world graph, full or fog-of-war from a player's perspective) and `search` (content search across locations/characters/objects, with `--regex` and type filtering) are specified in `design/CLI_DESIGN.md` but have no implementation requirement attached to a milestone yet; see Open Questions (§8).
