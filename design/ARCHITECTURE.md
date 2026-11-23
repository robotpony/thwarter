# Architecture

System design and current implementation status. For "why Rust" and other standing decisions, see `design/DECISIONS.md`. For the content file format, see `design/CONTENT_FORMAT.md`.

## Principles

- All world entities are defined in Markdown files with YAML frontmatter (see `design/CONTENT_FORMAT.md`)
- A location's file path is its address; a URI like `castle/courtyard` should read as a place, not an opaque ID
- Locations, characters, and objects are parsed by the same mechanism (frontmatter `type` field selects the interpretation), but modeled as distinct Rust types because they carry genuinely different data
- Plot points and narrative arcs exist to let an author track story-level state (decisions made, themes progressing) the same way locations track spatial state

## Modules

The library crate (`thwarter`) and binary crate (`thwart`) split along these lines:

```
src/
├── bin/thwart.rs        # CLI entry point, argument parsing, dispatch
├── models/               # Data structures, no behavior beyond simple constructors
│   ├── world/            # Location, WorldGraph, Coordinate, GameObject
│   ├── character/        # Character, Player, CharacterState
│   ├── narrative/        # PlotPoint, NarrativeArc, GameState, StoryVariable
│   └── game/              # Game, GameSession, Action
├── engines/               # Behavior over the models
│   ├── world_engine.rs    # Load, validate, and query the world graph
│   ├── character_engine.rs # Character placement and movement
│   ├── narrative_engine.rs # Plot point and story variable state
│   └── game_engine.rs      # Orchestrates the other three, processes actions
├── parsers/
│   ├── markdown_parser.rs  # Frontmatter + body parsing, dispatches on `type`
│   └── metadata_parser.rs  # Standalone YAML/JSON/TOML parsing (config files)
└── cli/
    ├── commands/           # One file per subcommand
    ├── config.rs           # thwart.toml / world.toml handling
    └── output.rs           # Text/JSON/YAML output formatting
```

## Engine responsibilities

**World Engine.** Owns the `WorldGraph` (locations and their connections) and, pragmatically, the object table (there's no separate Object Engine; objects don't have enough independent behavior to justify one, and their spatial placement is a world-graph concern). `load_world` walks a directory, parses every `.md` file, and sorts locations/objects/characters accordingly, character parsing results are handed back to the caller rather than stored here, since character placement is `CharacterEngine`'s job.

**Character Engine.** Owns loaded `Character` definitions and their live `CharacterState` (current location, mood, health). Takes parsed `Character` values from a `WorldEngine::load_world` call; the two engines don't parse independently of each other.

**Narrative Engine.** Owns `GameState` (story variables, completed plot points), the plot point registry, and narrative arc registry. Plot point triggering checks prerequisites; consequence *execution* (what actually happens when a plot point fires) is not implemented.

**Game Engine.** Orchestrates the other three and processes player `Action`s (move, take, look, examine, ...). Action dispatch exists; the actual effects of each action type do not.

## Implementation status

This table exists because status claims in this repo have previously drifted from what the code does. Update it in the same commit as any change that makes an entry true.

| Component | Status | Notes |
|---|---|---|
| Location / Character / GameObject models | Done | |
| Markdown/frontmatter parser | Done | Frontmatter-first, dispatches on `type`. See `design/CONTENT_FORMAT.md`. |
| `WorldEngine::load_world` | Done | Walks a directory, parses all content, reports per-file errors instead of aborting on the first one. |
| `WorldGraph::find_path` | Not implemented | Returns `None` unconditionally. |
| `WorldEngine::broken_connections` | Done | Flags connections pointing at unloaded location IDs. Backs `thwart validate`. |
| `CharacterEngine` | Partially done | Loading, placement, and movement work; nothing calls it from the CLI yet (only `WorldEngine` is wired into `validate`/`show`). |
| `NarrativeEngine` | Model only | Plot point prerequisite checking works; consequence execution and alternative-text condition evaluation are stubs. |
| `GameEngine` | Model only | Action dispatch exists; move/take/look/examine handlers are stubs that report success without doing anything. |
| `thwart validate` | Done | Loads the world, reports parse errors and broken connections; `--strict` additionally checks character `default_location` references. `--fix` and location-scoped validation are not implemented. |
| `thwart show` | Partially done | Loads and displays the world/a single location. `--player` (perspective) and `--state` (saved state) are not implemented. |
| `thwart map` | Not implemented | Reports that it isn't implemented and exits non-zero. |
| `thwart search` | Not implemented | Reports that it isn't implemented and exits non-zero. |
| `thwart debug` | Not implemented | Reports that it isn't implemented and exits non-zero. For reference checking, use `thwart validate --strict` today. |
| Save/load game state | Not implemented | |
| Dialogue trees | Not implemented | `dialogue_tree` is loaded as an opaque string reference; nothing parses or executes what it points to. |
| `ConfigManager` / `thwart.toml` | Built but unused | `src/cli/config.rs` can load and save `thwart.toml` and `world.toml`, but `src/bin/thwart.rs` never constructs a `ConfigManager`; the CLI's default world path is a hardcoded constant instead. `thwart.toml`'s `[world].path` is currently inert. |

## World content layout

World content lives at `authors/<author>/<world-name>/`, e.g. `authors/mx/test-world/`. This formalizes what the repo already did in practice rather than the single-world `authors/world/` layout earlier docs described (a layout no content in this repo ever actually used). `thwart.toml`'s `[world].path` points at the world to use by default; `--world <path>` overrides it per-invocation.

## Next steps

The build order lives in `PLAN.md`, and known defects in `BUGS.md`.
