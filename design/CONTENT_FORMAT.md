# Content File Format

This is the single source of truth for how world content is written. `README.md` and `design/ARCHITECTURE.md` link here rather than repeating field lists; if they ever disagree with this document, this document wins.

## Summary

Every content file is a Markdown file with YAML frontmatter at the top (Hugo-standard), followed by a body of plain prose. The frontmatter's `type` field decides how the rest of the file is interpreted: `location`, `character`, or `object`.

```
---
id: castle/courtyard
title: Castle Courtyard
type: location
connections:
  north: castle/great-hall
  south: forest/path
characters: [guard-captain]
objects: [fountain, stone-bench]
---

You stand in the grand courtyard of an ancient castle. Tall stone walls
rise around you, covered in ivy and weathered by countless storms.
```

The body is the entity's description, used as-is. There is no separate H1 title in the body; `title` lives in frontmatter so it isn't duplicated or allowed to drift from it.

## Why frontmatter-first

This repo's first pass put frontmatter at the *end* of the file. That matched neither the README's own example (which showed frontmatter-first) nor common Hugo/Jekyll convention, and it meant the shipped parser, which only recognized frontmatter opening the file, could never read the shipped sample content. Frontmatter-first removes that mismatch and matches what most contributors will already expect from a "Hugo-style" tool.

## Fields common to every type

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Stable identifier. By convention, matches the file's path relative to the world root, without extension (`castle/courtyard`, `npcs/king` → `king`, see note below). The loader allows any path; `thwart validate` warns when a location's `id` isn't its path. |
| `title` | yes | Human-readable name. |
| `type` | yes | `location`, `character`, or `object`. |

## Keys, order and file locations

Keys are lowercase and case-sensitive. The loader ignores a key it doesn't know, so `Inventory:` is dropped rather than read as `inventory:`; `thwart validate` should warn about a key that differs only in case from a known one (`PLAN.md` › Phase 3). Objects may carry any extra keys (see `type: object`), so only the case check applies to them.

When a tool writes a file, it writes the keys in the order of this document's tables: `id`, `title`, `type`, then the type's own fields in table order, then any extra keys in the order they already had. Editing a file keeps its existing order; only a key being added is placed by this rule.

A new entity's file goes where its `id` says:
- a location at `<id>.md` from the world root (`castle/gate` → `castle/gate.md`)
- a character at `npcs/<id>.md`, whatever its `character_type`
- an object at `objects/<id>.md`

Existing files can live anywhere in the world; the loader finds them by walking the directory and keeps track of which file each `id` came from. An `id` must be unique within its type; a location and an object may share one, but `thwart validate` should warn when they do, since the console's `go <name>` can't tell them apart (`PLAN.md` › Phase 3). These defaults match the test world (decided by the commander, 2026-11-23; Bruce may change them).

## `type: location`

| Field | Required | Meaning |
|---|---|---|
| `coordinates` | no | `[x, y, z]`. Optional; only needed if you want coordinate-based distance or a coordinate-driven map layout. |
| `connections` | no | Map of direction name to target location `id`. Direction names are free text (`north`, `up`, `through the hedge`), not a closed set. |
| `characters` | no | List of character `id`s present by default. |
| `objects` | no | List of object `id`s present by default. |

## `type: character`

| Field | Required | Meaning |
|---|---|---|
| `character_type` | no, defaults to `npc` | `npc`, `player`, `villain`, or `hero`. |
| `default_location` | no | Location `id` the character starts at. |
| `attributes` | no | Map of name to integer, e.g. `health: 80`. |
| `dialogue_tree` | no | Reference to a dialogue file. Format not yet specified; dialogue authoring is P1 (see `design/ARCHITECTURE.md`). |
| `inventory` | no | List of object `id`s the character starts carrying. |
| `relationships` | no | List of single-key maps, other character `id` to a free-text relationship label: `- king: loyal`. |

## `type: object`

Object content is far more varied than location or character content (compare a sword to a stream), so only a few fields are fixed. Everything else is preserved and handed to whatever engine code cares, rather than forced into a rigid schema.

| Field | Required | Meaning |
|---|---|---|
| `object_type` | yes | Free text: `weapon`, `container`, `environmental`, `furniture`, etc. Not a closed enum. |
| `category` | no | Free text, finer-grained than `object_type`. |
| `weight` | no | Free text by convention (`light`, `heavy`, `immovable`), not a unit. |
| `value` | no | Free text; sample content mixes numbers (`1000`) and descriptions (`priceless`, `unknown`). |
| `objects` | no | List of object `id`s this object holds, such as photos on a wall (REQ-CF-31). |
| `holds` | no | The word play uses for what this object holds: `on` (the default) or `in`, as in "Coins (in the chest)". |
| `actions` | no | Map of action name to its mechanics (`becomes:`, `sets:`, `chapter:`, `requires:`). The text a player reads is in the body, in a `## When: #wield` reply section, never here. See `design/SECTIONS.md`. |
| anything else | no | Preserved verbatim (e.g. `properties`, `stats`, `effects`, `contents`, `security` in the sample world). Not validated. |

## `type: questions`

A question set tells the console what to ask when an author makes a thing with `/new` or `/add`, the way Claude Code asks a question with options (Bruce, 2026-11-24; REQ-CF-32). It's a file of its own, not frontmatter in a template.

```markdown
---
type: questions
---
## Where does it start?
- Loose, it can be taken (default)
- Hanging on a wall, pulled free -> recipe fixed-until pull
- On something here -> ask thing, then move into it

## What is it called?
- Lantern (default)
- Brass lantern -> set title Brass lantern
- Something else -> set title
```

- **Which set.** The file's name says what it sets up: the new thing's id, its template, or its kind, tried in that order. `questions/lantern.md` is asked when making a thing called `lantern` or one from a `lantern` template, `questions/object.md` for any object without a set of its own, `questions/door.md` for `/add door`. There's no `for:` key, since the name already says it, the way a template's name does. A set can exist before any thing or template of its name does.
- **Where sets live.** A `questions/` folder is reserved, like `templates/`: it can sit in any scope, and the loader reads its files as question sets, never as things. A set is looked up from the new thing's folder out to the world's root, then in the engine's built-ins (`src/builtins/questions/`, compiled in). A set of the same name overrides one farther out, and a world's overrides a built-in. Nothing merges: the nearest whole file wins.
- **Questions.** Each `##` heading is one question, asked in order. Prose under a heading is a hint shown with it.
- **What it acts on.** A set is always about the thing being made, and every construct acts on that thing, so a set names no ids: `set title` sets the new thing's title, the way `this` means the thing in a script.
- **Options.** Each list item is one option. `(default)` marks what Esc takes; with none, the first option is the default. An option's text is what the author sees. After `->` (or `→`) comes what it does, one of four constructs:
  - `set <key> <value>` sets a key to the value written in (`- Brass lantern (default) -> set title Brass lantern`); `set <key>` alone asks the author to type it.
  - `recipe <name> <args>` runs a recipe (below).
  - `ask thing` picks a thing here.
  - `ask value <name> <default>` asks for a value, offering the default. An option with no `->` changes nothing.
- **Recipes** are shorthand for console commands (`/state`, `/set … in <state>`, `/action`), and the console prints the commands an answer stood for. The four built-ins are `fixed-until`, `locked-with`, `switch` and `container`. Their exact forms are the console's (`design/CONSOLE_REQUIREMENTS.md`).
- `validate` warns about an option with an unknown construct and more than one `(default)` under a question. A set whose name matches nothing yet isn't a warning, since it may be waiting for its thing.

## `type: command`

Agreed by Bruce, 2026-11-24. A command file describes one console command: its name, how it's called and its help. The `/` menu, `/help` and usage replies all read it, and what the command does at run time is the console's (`design/CONSOLE_REQUIREMENTS.md` › Command registry, REQ-CON-50 to 56). A world can add its own commands, as lists of existing ones:

```markdown
---
type: command
name: stock
aliases: [st]
about: put a thing in a room and open its card
args: "{thing} {room}"
modes: [edit]
changes: true
run:
  - /add {thing} {room}
  - /edit object {thing}
---
Puts a thing in a room and opens its card to write.

    /stock desk bedroom
```

- **Fields.** `name` is what follows the `/`, and the file is named after it (`world/commands/stock.md`). `aliases` are other names for it. `about` is the one-line summary the `/` menu and `/help`'s list show. `args` names its parameters the way the `/` menu shows them, `{required}` and `[{optional}]`. `modes` lists `edit`, `play` or both. `changes` says whether it writes to the world. A built-in has `handler` (its Rust handler's name), and a world command has `run`. The body is the help that `/help <name>` shows.
- **Where commands live.** Built-ins are compiled in from `src/builtins/commands/`. A world's are in `world/commands/`, which is reserved like `questions/`, so its files are never read as things. The person's `config.toml` can add aliases to a command, reword its `about` or hide it, but it can't add a command.
- **World commands.** `run` is a list of console commands with the `{param}` slots from `args`, run in order as if the author typed them. A world command MAY instead have `script`, a Rhai script, which can read the world and run console commands, and has the world scripts' limits: no files, no network (Bruce, 2026-11-24). A world command works only in edit mode, for the world's authors (`design/CONTENT_REQUIREMENTS.md` › REQ-WS-7 to 9), and it can't do more than they could by typing. The console's REQ-CON-54 lists the commands it may not run. A world command named like a built-in is a load warning, and the built-in wins.
- `validate` warns about a world command whose `run` names an unknown command, uses a slot `args` doesn't have, or has a `handler`.

## How play words a room: `[play]` in `world.toml`

Play writes rooms and the inventory as plain sentences, each from a template in `world.toml`'s `[play]` table (REQ-ENG-43). A world MAY change any of them. The full list, with each default, is `src/builtins/text/play.toml`; copy a line from there into a world's `[play]` to reword it.

| Key | Default | Used for |
|---|---|---|
| `show_room_names` | `false` | The room's name above its description |
| `objects` | `You see {list}.` | What's here, scenery left out |
| `character`, `characters` | `{list} is here.`, `{list} are here.` | Who's here, right after the description |
| `held` | `There's {list}.` | What things hold: "There's an old photo on the walls." |
| `exits` | `You can go {or}.` | The ways out |
| `door_exit` | `{direction} through {door}` | One way out through a door: "down through an iron door" |
| `no_exits` | `There's no way out.` | A room with no exits |
| `see_through` | `Through the {door} you can see the {room}.` | A door you can see through |
| `no_description` | `(no description yet)` | A room with no prose |
| `inventory`, `empty_inventory` | `You have {list}.`, `You aren't carrying anything.` | The inventory |
| `serial_comma` | `true` | "a, b, and c" rather than "a, b and c" |

`{list}` joins with "and", `{or}` with "or", and both work in any template. An unknown key, or a value of the wrong kind, is a warning.

The built-in replies ("Take what?", "The chest won't budge.") are `world.toml`'s `[engine]` table in the same shape, with each default in `src/builtins/text/engine.toml`. `[engine]` rewords a reply across the whole world. To reword it for one thing, room or chapter, write a `## When:` section headed with the reply's outcome tags instead (`design/SECTIONS.md` › Rewording a built-in reply).

## What the loader does today

`WorldEngine::load_world` walks a world directory, parses every `.md` file, and:
- Fails the individual file (collected as an error, not an abort) if `type` is missing or unrecognized, or if a type-required field is missing
- Loads `location` files into the world graph
- Loads `object` files into an object table
- Returns `character` files for the caller to hand to `CharacterEngine`

`thwart validate` reports every file that failed to parse, plus every connection that points at a location `id` that was never loaded.

## What's still open

- **Objects without cards are scenery.** A location's `objects: [...]` or a character's `inventory: [...]` can name an object `id` with no file. That's allowed: the object is scenery with a name only, and the console creates its card the first time an author edits it (`design/CONTENT_REQUIREMENTS.md` › REQ-CF-8). Tools should label it "no card", not report it as missing. A card created this way starts with `object_type: scenery`, a `title` made from the id in sentence case (`stone-bench` → "Stone bench"), and an empty body (decided by the commander, 2026-11-23).
- **Dialogue tree format is unspecified.** `dialogue_tree` is loaded as an opaque string. What it points to, and how it's structured, is undecided.
