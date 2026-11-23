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
├── console/              # The console: edit and play modes (REQUIREMENTS.md › 4.6)
├── render.rs             # Room view text, shared by `thwart show` and the console
├── models/               # Data structures, no behavior beyond simple constructors
│   ├── world/            # Location, WorldGraph, Coordinate, GameObject
│   ├── character/        # Character, Player, CharacterState
│   ├── narrative/        # PlotPoint, NarrativeArc, GameState, StoryVariable
│   └── game/              # Game, GameSession, Action
├── engines/               # Behavior over the models
│   ├── world_engine.rs    # Load, validate, and query the world graph
│   ├── location_view.rs   # LocationView: what a room shows, for show and the console
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

## Console interface

The engine's interface to the console (`REQUIREMENTS.md` › REQ-ENG-9), as agreed between the coder and the console coder on 2026-11-23. The Phase 2 part is built; play state and action results come with Phase 4.

```rust
// thwarter::engines (src/engines/location_view.rs)
pub struct LocationView {
    id: String, title: String, coordinates: Option<Coordinate>, body: String,
    exits: Vec<ExitView>, characters: Vec<EntityRef>, objects: Vec<EntityRef>,
}
pub struct ExitView { direction: String, target: String, target_exists: bool }
pub struct EntityRef { id: String, name: Option<String> }  // None: no card

WorldEngine::describe_location(&self, id: &str) -> Option<LocationView>
WorldEngine::problems(&self, report: &WorldLoadReport, strict: bool) -> Vec<String>
thwarter::render::render_location(&LocationView) -> String
thwarter::console::run(world_path: &Path, mode: Mode) -> anyhow::Result<()>  // Mode { Edit, Play }
```

- Exits are listed in compass order, then other directions A to Z.
- An `EntityRef` with no `name` has no card: scenery for an object (REQ-CF-8), a missing character for a character.
- `problems` is `validate`'s check, also used by `/reload` (REQ-CON-7).
- `render_location` gains a style argument (author or player view) in Phase 4.

Proposed for Phase 4, not built: `PlayState { location, inventory: Vec<String>, flags: BTreeMap<String, serde_yaml::Value> }` in models, and `ActionResult { text, changes: Vec<Change> }`.

## Implementation status

This table exists because status claims in this repo have previously drifted from what the code does. Update it in the same commit as any change that makes an entry true.

| Component | Status | Notes |
|---|---|---|
| Location / Character / GameObject models | Done | |
| Markdown/frontmatter parser | Done | Frontmatter-first, dispatches on `type`. See `design/CONTENT_FORMAT.md`. |
| `WorldEngine::load_world` | Done | Walks a directory, parses all content, reports per-file errors instead of aborting on the first one. |
| `WorldGraph::find_path` | Not implemented | Returns `None` unconditionally. |
| `WorldEngine::broken_connections` | Done | Flags connections pointing at unloaded location IDs. Backs `thwart validate`. |
| `CharacterEngine` | Partially done | Loading, placement, and movement work, but nothing calls it from the CLI. `WorldEngine` keeps character cards for lookups (names in the room view); placement is still `CharacterEngine`'s job. |
| `WorldEngine::describe_location` / `problems` | Done | The console interface above. `problems` backs both `validate` and `/reload`. |
| Room renderer (`render::render_location`) | Done | One renderer for `thwart show` and the console. |
| Console (`thwart edit`, `thwart play`) | Stub | The subcommands and `console::run` exist; `run` loads the world, then reports that the console isn't built. |
| Unit tests | Started | 5 tests, in `render.rs` and `world_engine.rs`. Parser tests are next (`design/UNIT_TESTS.md` › 1). |
| `NarrativeEngine` | Model only | Plot point prerequisite checking works; consequence execution and alternative-text condition evaluation are stubs. |
| `GameEngine` | Model only | Action dispatch exists; move/take/look/examine handlers are stubs that report success without doing anything. |
| `thwart validate` | Done | Loads the world, reports parse errors and broken connections; `--strict` additionally checks character `default_location` references. `--fix` and location-scoped validation are not implemented. |
| `thwart show` | Partially done | Loads and displays the world or a single location; text output is the shared room view. Objects without cards show "(no card)", missing exits "(missing)". `--player` (perspective) and `--state` (saved state) are not implemented. |
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
