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
├── console/              # The console: edit and play modes (design/CONSOLE_REQUIREMENTS.md)
├── render.rs             # Room view text, shared by `thwart show` and the console
├── writer/               # Round-trip file writer: edits frontmatter in place (REQ-ENG-7)
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

The engine's interface to the console (`design/ENGINE_REQUIREMENTS.md` › REQ-ENG-9), as agreed between the coder and the console coder on 2026-11-23. The Phase 2 part is built; play state and action results come with Phase 4.

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

### Play (Phase 4, 0.7.0)

```rust
// models::game
pub struct PlayState {
    location: String, inventory: Vec<String>,
    taken: BTreeMap<String, Vec<String>>,          // room -> objects taken from it
    flags: BTreeMap<String, serde_yaml::Value>,    // unused until narrative state
}
pub enum PlayAction { Look, Go(String), Take(String), Examine(String), Inventory }
pub struct ActionResult { text: String, changes: Vec<Change>, note: Option<String> }  // note: author-only detail
pub enum Change { Moved { from, to }, Took { object, from } }

// engines
GameEngine::start(&self, &WorldEngine, location) -> Result<PlayState>
GameEngine::parse_command(&self, &WorldEngine, line, &PlayState) -> Parsed  // Action(PlayAction) | Reply(String) | Unknown(String)
GameEngine::act(&self, &WorldEngine, &mut PlayState, &PlayAction) -> ActionResult
GameEngine::view(&self, &WorldEngine, &PlayState) -> Option<LocationView>
PlayState::reconcile(&mut self, &WorldEngine) -> Vec<String>
save_game(&Path, &PlayState) -> Result<()>
load_game(&Path, &WorldEngine) -> Result<(PlayState, Vec<String>)>
saves::{save_named, load_named, save_path, list_saves, DEFAULT_SAVE}  // by save name, for a world root
render::RenderStyle { Author, Player }; render_location_as(&LocationView, RenderStyle) -> String
```

- The world is read-only to play (REQ-CON-3). `taken` records what the player removed from each room as a difference from the authored contents, so an author's edits during a game still show. A test checks that playing leaves every file's bytes unchanged.
- `take` needs a card and a `weight` other than `immovable`. Scenery can be examined but not taken.
- Nouns match by id, then title, then a unique word. A tie gets a "Which do you mean" reply.
- `text` always reads as the game; author-only detail goes in `note` (REQ-CON-10).
- Saves are YAML marked `thwarter_save: 1`, written atomically to a path the console chooses (REQ-CON-11). Loading reconciles the save with the current world and returns warnings for anything that's gone.

## File writer

`thwarter::writer` writes entities back to their files (`design/ENGINE_REQUIREMENTS.md` › REQ-ENG-7), built 2026-11-23 (0.4.0).

```rust
WorldWriter::edit(&self, engine: &WorldEngine, kind: EntityKind, id: &str, edits: &[Edit]) -> Result<PathBuf>
WorldWriter::create(&self, new: &NewEntity, edits: &[Edit]) -> Result<PathBuf>
new_entity_relative_path(kind: EntityKind, id: &str) -> PathBuf   // the one place the new-file rule lives
enum Edit { SetScalar, SetList, SetMapEntry, RemoveMapEntry, RemoveKey, SetBody, ReplaceAll }
```

- It edits the frontmatter as text, one key's lines at a time, and never re-serializes the file. After every edit it re-parses: the edited field must have the new value and every other top-level key must parse exactly as before, or the edit is refused and the file is untouched. Saves are atomic (temp file, then rename) and keep the file's permissions.
- It handles the shapes `design/CONTENT_FORMAT.md` uses: one-line scalars, flow and block string lists, and one-level block maps. Block scalars, non-empty flow maps, nested list items and anchors are refused at the key being edited; `ReplaceAll` is the way around that. `relationships` and `actions` (lists of single-key maps) can only be changed with `ReplaceAll` for now.
- An added list follows the file's existing style: block if the file already has a block list, flow otherwise. Added keys go in field-table order, from `parsers::known_keys`.
- It refuses changes to `id` or `type`, and edits to an id defined in more than one file. An id can't contain `..`, empty, hidden or absolute segments, backslashes or colons, and any path that resolves outside the world is refused, symlinks included.
- The engine records which file each id came from (`WorldEngine::sources`, `source_path`, `EntityKind`). Ids are unique within a kind; a location and an object may share one, with a warning (`WorldEngine::shared_ids`).
- `WorldWriter::edit_loaded` and `create_loaded` write the file, apply the same text to the loaded engine (REQ-ENG-8, `WorldEngine::set_file`), and return `ChangeReport { path, problems, added, resolved }`: the problems the change added and the ones it resolved. Nothing re-reads the world from disk. There's no writer-side delete yet.
- `WorldWriter::create_loaded_in` makes a thing in a chosen folder (an area's `castle/objects/`), and `move_loaded` moves a thing's file or bundle to another folder keeping its id (REQ-CON-28, 29). The loader finds things anywhere by their `type:`, so an area's `objects/` and `npcs/`, or loose in the area, all load.

## Implementation status

This table exists because status claims in this repo have previously drifted from what the code does. Update it in the same commit as any change that makes an entry true.

| Component | Status | Notes |
|---|---|---|
| Location / Character / GameObject models | Done | |
| Markdown/frontmatter parser | Done | Frontmatter-first, dispatches on `type`. See `design/CONTENT_FORMAT.md`. |
| `WorldEngine::load_world` | Done | Walks a directory, sorts each file into its role by where it sits (a thing, a bundle's state, a chapter, an overlay, an area; `design/ENGINE_REQUIREMENTS.md` › REQ-ENG-10), merges each thing's layers (`engines/layers.rs`, with the file each key came from) and loads its starting version. Reports per-file errors instead of aborting on the first one, and checks every state against every chapter visible from the thing. `WorldEngine::version` builds any other version on request. `world/` is read as the world's own definition (`engines/world_def.rs`; `WorldEngine::world()`), with `world.toml`'s old keys as a fallback for a world not yet moved. Other tools' files (dot-folders, Hugo's `layouts/`, `static/`, `assets/`, `public/`, and `ignore` in `world.toml`) are skipped, as is `game/`. Templates stay out of the world but stack under the things that name them (`inherits:`): a path from the root, or a bare name found in `templates/` folders from the thing's scope outward, then among the built-ins (`engines/builtins.rs`, `src/builtins/*.md`; `scenery` so far). |
| `WorldEngine::find_path`, `distances`, `map_layout` | Done | Shortest paths and distances through every exit and door (`engines/map.rs`); `WorldGraph::find_path` does the same by direct exits only. `map_layout` places rooms on a grid by their compass exits and lists the rest beside it, as plain data the console draws itself, agreed with the console coder. |
| `WorldEngine::broken_connections` | Done | Flags connections pointing at unloaded location IDs. Backs `thwart validate`. |
| `CharacterEngine` | Partially done | Loading, placement, and movement work, but nothing calls it from the CLI. `WorldEngine` keeps character cards for lookups (names in the room view); placement is still `CharacterEngine`'s job. |
| `WorldEngine::describe_location` / `problems` | Done | The console interface above. `problems` backs both `validate` and `/reload`. |
| Room renderer (`render`) | Done | One renderer for `thwart show` and the console. It builds `Rendered` blocks of styled spans (headings, emphasis, lists, quotes, code, links with their targets, tags, missing marks, author notes), agreed with the console coder, and `to_plain` prints them; plain text is byte-identical to before for prose with no Markdown. Play renders prose as Markdown (`prose_blocks`, `prose_plain`); the author view shows it as written. |
| File writer (`writer`) | Done | See File writer above. The console's `/new`, `/set` and `/link` call it through `edit_loaded` and `create_loaded`. `make_bundle_loaded` turns a one-file thing into a bundle, `save_bundle_loaded` saves a bundle's sections together, and `delete_state_loaded` and `rename_state_loaded` remove and rename states (REQ-ENG-28, 29). `rename_loaded` renames a thing's id, moving its file or bundle folder and rewriting every reference (REQ-ENG-30), for the console's `/move` and renaming. |
| In-memory world changes (REQ-ENG-8) | Done | `WorldEngine::set_file` and `remove_file` apply one file's new text and recompute only the affected ids, with the same results a fresh load would give. `current_problems` and `current_warnings` check the world as it is now. |
| Console (`thwart edit`, `thwart play`) | Partially done | The loop, `/edit`, `/play`, `/reload`, `/help` and `/quit` work, and edit mode moves through the world with play words (directions, `go <name>` to any room, area, folder, character or object by id or name, `go back`, `go /`, `look`, `examine`). Play mode reads input through the engine's parser, which every player client shares (`design/PLAYER_COMMANDS.md`): look, go and bare directions, take, examine, inventory, search, `where`, `look <direction>` and objects' named actions, with aliases; edit mode reads its words through the same parser, prints author notes on their own line, and keeps its own position: `/play` resumes and `/play here` moves the player to edit mode's room. `/save` and `/load` keep games outside the world folder. `/new room|character|object`, `/set`, `/link` and `/unlink` save through the writer and report the problems a change adds. `/edit room|object|character [id]` opens the file as it is in a full-screen editor (ratatui; ctrl-s saves, esc closes). A save must parse and still hold the same thing, takes a change on disk by itself when nothing is unsaved and otherwise warns once before saving over it, and reports problems as `/new` does; scenery opens as a new card with `object_type: scenery`, and a file that doesn't parse opens so it can be fixed. Bundles, with a section per file, aren't built (`design/STATES.md`). On a terminal the console is one full-screen view (ratatui): a status bar (world, mode, warnings, chapter, turn, build), the output, a sidebar with GAME (play state, its first lines fixed) and HERE (edit mode's room: exits, people and things with their scope, warnings), and the input line with history; under 100 columns the sidebar folds into one line. The full-screen editor opens over the output and keeps the sidebar, and highlights frontmatter (keys, ids that resolve or don't), prose (headings, emphasis, links, tags, code), markers and `{{ values }}`. Colours come from the terminal's 16 named colours by role, and `NO_COLOR` is honoured. `/edit` alone opens where edit mode is; `/status` and `/clear` work as in Claude Code. `go /game` shows the running game as a virtual folder, where `/set` changes only the game. `/scope global|area|local` sets where `/new` puts things, shown in the prompt, and `/move <thing> global|area|local` moves its file keeping its id; local puts it in the room's bundle. `/list <kind>` lists every room, character, object or door; `/map` at the world's root draws the whole world; a command name typed without its slash in edit mode runs it, or asks when it's also a play verb. Typing `/` opens a menu of the commands that work in this mode, narrowing as you type, and after a command or `go`, `examine` or `take` it offers the exits, rooms (nearest first), people, things, directions or saves that fit; Tab completes, Enter picks a command or a chosen suggestion, Esc closes. Input history isn't saved between sessions. `/restart` restarts the console onto the `thwart` binary on disk in the same terminal, keeping the mode, edit mode's place, the running game and the input history; what the new build can't take starts fresh with a notice. It also watches the world's files and reloads when they change, printing the result above the prompt. The quick menu (Ctrl-M at the prompt or in the editor, or `/menu`, REQ-CON-63 to 67) offers action, state, hide or reveal, starting state, link, add a thing or door, recipes and play from here, asking at the bottom of the console; add a reply (REQ-CON-68) and add chapter version wait on response hints and `/chapter`. In the editor, `~` on an empty line drops a console down that runs any command for the open file (REQ-CON-71, 72). A bundle edits as one document, a `## When:` section per state file, and saves each file back through `save_bundle_loaded` (`design/STATES.md` › Console); its own chapter overlays follow as `## When: #<chapter>` sections. `/state rename` and `/state delete` call the writer's `rename_state_loaded` and `delete_state_loaded`. Sidebar clicks on exits, TO WRITE names and the quick menu's hint do what their keys do, and the wheel scrolls under the pointer (REQ-CON-69, 70). Tested by transcripts in `tests/transcripts/` (edit navigation, modes, authoring, play) and unit tests for `/reload`, verbs, save paths, the editor's keys and full-edit saves. |
| Unit tests | Done | Over 300 unit tests: renderer, world engine, parser, `/reload`, the writer (including a byte-identical round trip of every `.md` in the repo and a one-field edit of every entity in the `tests/worlds/sample` fixture), and play. Tests that need content use that fixture; the shipped `authors/mx/test-world` is only checked to validate, since Bruce rewrites it. Console behaviour is also covered by transcripts. |
| `NarrativeEngine` | Model only | Plot point prerequisite checking works; consequence execution and alternative-text condition evaluation are stubs. |
| `GameEngine` | Partially done | Clocks run after each turn (`engines/clock.rs`, `GameEngine::run_clocks`; REQ-ENG-66 to 68): `after:` on a state, `at:` on a thing, a seed per game for ranges and same-headed sections, game time as `game.time` and `game.day`, `@thing:state` in headings, world-wide things in `world/` (weather), and `## When: #arrive` shown when a clock moves a thing the player can see (REQ-ENG-69). Look, go, take (and `take all`), drop, give, talk, ask, examine, inventory, wait, again, `it` and the story verbs (jump, run, kick, hit, throw, smash, break, stack, push, pull; REQ-ENG-64) work against `PlayState` (see Play above), each thing in its current version: its state in the game and the game's chapter (`WorldEngine::describe_location_at` and `location_at`/`object_at`/`character_at`). `PlayState` keeps states (as a delta from the starting state), the chapter, values, visits and the turn count; `switch_state` and `enter_chapter` change them, and so does `PlayAction::Use { verb, noun }`, which runs an object's named action: `requires: { has: }`, then `becomes:`, `sets:` and `chapter:`, replying with a matching reply section, else a built-in reply. `engines/turn.rs` builds each turn's tags (REQ-ENG-36): descriptions add the sections that match, and an action's reply is the matching `#<verb>` section in the target, then what it's used with (`hit window with stick`), then the room, else "Nothing happens."; the room's `#searched` section leads a search. In play, `@id` in prose shows as the thing's name (REQ-ENG-37). Play's room and inventory text is plain sentences from `world.toml`'s `[play]` templates (`engines/play_text.rs`, REQ-ENG-43), rendered by `render::play_blocks`. Scripts (`engines/scripts.rs`, REQ-ENG-38 to 41): a bundle's `actions.rhai` or a ```rhai block, run sandboxed in Rhai, returns a state, a reason or changes (`become`, `keep`, `change` for things it `connects:` to); replies match the action's outcome, and the built-in `locked-door` plays `design/STATES.md` › Keys. Objects hold objects (REQ-ENG-42): `GameObject.objects`, `WorldEngine::placement(id) -> Vec<Holder>`, and in play what a holder holds is in scope, listed after its holder when the player looks, and listed when it's examined; `keep()` is the same relation. `visibility:` decides what play shows: faint things are found by `PlayAction::Search` or examining the room, hidden ones only by name in an action, absent ones not at all. Prose in play has its `{{ level.name }}` variables filled (`engines/variables.rs`; `WorldEngine::fill`), unknown ones blank. When a noun matches several things, the reply is the `## When: #ambiguous` section of the nearest template they share, else `GameEngine::ask_which`, whose built-in replies are in the story's voice and vary by turn. Doors (REQ-ENG-31): an exit through a door leads to its other side (a portal's side says `to:`), is refused while the door isn't passable, and shows the door's text, and a line about the room beyond for a transparent door; doors take actions as objects do. `GameEngine::parse_command` (`engines/player_commands.rs`) turns a typed line into an action or a reply for any client (REQ-ENG-32, 33; REQ-PC-1 to 15): the built-in verbs first, then any other verb with a noun as an object action, and a verb no thing has, on nothing in scope, as not understood. `engines::saves` owns where games are saved for every client (REQ-ENG-34): `save_named` and `load_named` by save name, under `$XDG_STATE_HOME/thwarter/saves/`. |
| `thwart validate` | Done | Loads the world, reports parse errors and broken connections; `--strict` additionally checks character `default_location` references. It also reports an `id` defined twice within a kind, and warns (without failing) about a key that differs only in case from a known one. It warns about a state file nothing can switch its thing into, skipping things with a script of their own. `--fix` and location-scoped validation are not implemented. |
| `thwart show` | Done | With no name, the world's summary: the `world/` definition, story values, player, chapters and every thing's id. With a name, any location, object or character by id or link, in its starting version or another (`--state`, `--chapter`), with its states, chapter versions and templates; `--kind` chooses when an id names more than one kind. `--template <name>` shows a template or built-in as written. Text, JSON and YAML (REQ-CLI-5, 9 to 11). `--player` (perspective) is not implemented. |
| `thwart map` | Partially done | An ASCII map from a room (the world's start by default), out to `--depth`, saved with `--output` (`render::render_map`, REQ-CLI-17). `--player`, `--style` other than ascii, and `--connections` say they aren't implemented and exit non-zero. |
| `thwart search` | Done | Text, or a pattern with `--regex`, over every thing's id, title and prose (other states' and sections' included), case-insensitive unless `--case-sensitive`; `--type` narrows it, a location limits it to rooms in that folder and what they hold, and `--missing-connections` lists broken exits (`engines/search.rs`, REQ-CLI-18). Tags wait on the console's tag-search design. |
| `thwart new` | Done | Makes a world folder with `world/index.md` naming the machine user in `authors:`: a bare name under `authors/<you>/`, or a path (`worlds::new_world_folder`, `create_world`; REQ-CLI-22). |
| `thwart debug` | Not implemented | Reports that it isn't implemented and exits non-zero. For reference checking, use `thwart validate --strict` today. |
| Save/load game state | Done | `save_game` and `load_game` write and read `thwarter_save: 2`, with states, chapter, values, visits and turns; version 1 saves still load. On load, a state or chapter that's gone falls back (to the starting state, or the nearest earlier chapter) with a warning. The console's `/save` and `/load` call them. |
| Personas and personal settings | Done | `engines::persona` reads `authors/<id>/index.md` (`type: author`, pen name, contact details, bio), or a folder alone; `WorldDef::authors` is `world/index.md`'s `authors:` (one id or a list; an old `author:` still reads), `WorldEngine::personas()` each author's persona and `persona()` the first's, and `is_author(user)` with `settings::current_user()` says whether the person at the keyboard writes this world (REQ-WS-4, 5, 7, 8; REQ-ENG-56). `settings::PersonalConfig` reads and writes `$XDG_CONFIG_HOME/thwarter/config.toml` (default persona, helpers' settings), keeping keys it doesn't know; `settings::data_dir` and `state_dir` give the XDG folders for installed worlds and play data (REQ-CFG-4 to 6). The console's first-run and `/author` use them (Phase 10). |
| Untrusted content | Done | Every world's text is cleaned of terminal control characters and escape sequences before it's shown (`render::clean_text`, REQ-CFG-9): in the renderer's blocks and plain text, the author views, every play reply, `thwart show` and every formatter message. Loading caps each file at 1 MB and frontmatter at 32 levels of nesting, and skips links to outside the world (REQ-CFG-10). Installing checks archives (REQ-CFG-8; see `thwart install`). |
| `thwart package`, `install`, `installed` | Done | `packaging` writes a world, every author's persona file and a manifest into one `.thwarter` zip, leaving out what the loader skips and `world/styles/`, after saying which contact details and how many author comments ship (`--strip-comments`, `--yes`). `install` checks the whole archive first (paths, links, layout, 200 MB, 10,000 files; REQ-CFG-8), unpacks beside the target and swaps it in, and refuses an existing install without `--replace`. `installed` lists them, and `thwart play <persona>/<world>` finds one (REQ-CLI-12 to 16). |
| Removing a thing | Writer done | `WorldWriter::plan_remove` and `remove_loaded` delete a thing's file or bundle and take its references out of lists, exits and doors, refusing a room that holds things and the start room (REQ-ENG-50). The console's `/remove` isn't built yet. |
| Not written yet | Engine done | `WorldEngine::not_written()` groups every name a link, `@name` or exit points at that nothing defines, one per name with each mention's file, line, holder and kind; `validate` lists them apart from warnings (REQ-ENG-55, REQ-CLI-19). The console's TO WRITE list isn't built yet. |
| Forgiving verbs | Done | `engines::verbs` holds the built-in verbs and the alias table (`src/builtins/verbs.toml`, with `world.toml`'s `[verbs]` over it). `parse_command` reads a typed verb as a direction, a built-in, an alias, a world verb, then a unique prefix among the verbs in scope, asking when several fit, and suggests a verb one typo away without running it; `validate` warns about an alias that can't work (REQ-ENG-51 to 54). `GameEngine::verbs` lists the verbs for a client. |
| Question sets | Loader done | `engines::questions` reads each `questions/<name>.md` as a set, never a thing; `WorldEngine::questions_for(folder, template, kind)` finds the nearest whole set, then the built-in `object` set; `validate` checks them (REQ-ENG-48). The console doesn't ask them yet (`PLAN.md` › Phase 5). |
| Text as data | Built | `engines::texts::Texts` holds the `[play]`, `[engine]` and `[console]` tables with compiled-in defaults (`src/builtins/text/`); `[play]` reads from it. Play's and the engine's replies read from `[play]` and `[engine]`, which a world overrides in `world.toml`. The console's own replies read from `[console]`, which the author overrides in their `config.toml`, with a warning in the greeting for an unknown key. A `## When:` section whose tags are exactly a reply's outcome tags rewords it, along the reply's chain (the thing, its room, the chapter), with `{{ turn.x }}` slots (`GameEngine::story_reply`, REQ-ENG-57). `tests/text_guard.rs` fails on a new sentence in `src/engines/`, `src/render.rs` or `src/console/` (REQ-ENG-58). |
| Dialogue trees | Not implemented | `dialogue_tree` is loaded as an opaque string reference; nothing parses or executes what it points to. |
| `ConfigManager` / `thwart.toml` | Built but unused | `src/cli/config.rs` can load and save `thwart.toml` and `world.toml`, but `src/bin/thwart.rs` never constructs a `ConfigManager`. `thwart.toml`'s `[world] path` is read by `worlds::configured_world` instead. |
| Finding the world | Done | No built-in default: `worlds::resolve_world` takes `--world`, a configured default (`thwart.toml`, then `config.toml`), then the world the current folder is in, else fails with `WorldNotGiven`, which lists `worlds::find_worlds` (REQ-CLI-21). `thwart edit` and `thwart play` show the console's picker on it, `console::run_or_pick` (REQ-CON-62). |

## World content layout

World content lives at `authors/<author>/<world-name>/`, e.g. `authors/mx/test-world/`. This formalizes what the repo already did in practice rather than the single-world `authors/world/` layout earlier docs described (a layout no content in this repo ever actually used). `thwart.toml`'s `[world].path` points at the world to use by default; `--world <path>` overrides it per-invocation.

## Next steps

The build order lives in `PLAN.md`, and known defects in `BUGS.md`.
