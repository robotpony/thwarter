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
| `id` | yes | Stable identifier. By convention, matches the file's path relative to the world root, without extension (`castle/courtyard`, `npcs/king` → `king`, see note below). The loader does not currently enforce this convention; `thwart validate` should (tracked in `design/CLI_DESIGN.md`). |
| `title` | yes | Human-readable name. |
| `type` | yes | `location`, `character`, or `object`. |

## Keys, order and file locations

Keys are lowercase and case-sensitive. The loader ignores a key it doesn't know, so `Inventory:` is dropped rather than read as `inventory:`; `thwart validate` should warn about a key that differs only in case from a known one (`PLAN.md` › Phase 3). Objects may carry any extra keys (see `type: object`), so only the case check applies to them.

When a tool writes a file, it writes the keys in the order of this document's tables: `id`, `title`, `type`, then the type's own fields in table order, then any extra keys in the order they already had. Editing a file keeps its existing order; only a key being added is placed by this rule.

A new entity's file goes where its `id` says:
- a location at `<id>.md` from the world root (`castle/gate` → `castle/gate.md`)
- a character at `npcs/<id>.md`, whatever its `character_type`
- an object at `objects/<id>.md`

Existing files can live anywhere in the world; the loader finds them by walking the directory and keeps track of which file each `id` came from. An `id` must be unique within its type; a location and an object may share one, but `thwart validate` should warn when they do, since the console's `cd <id>` can't tell them apart (`PLAN.md` › Phase 3). These defaults match the test world (decided by the commander, 2026-11-23; Bruce may change them).

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
| `actions` | no | List of single-key maps, action name to response text: `- wield: "The sword feels like an extension of your arm"`. |
| anything else | no | Preserved verbatim (e.g. `properties`, `stats`, `effects`, `contents`, `security` in the sample world). Not validated. |

## What the loader does today

`WorldEngine::load_world` walks a world directory, parses every `.md` file, and:
- Fails the individual file (collected as an error, not an abort) if `type` is missing or unrecognized, or if a type-required field is missing
- Loads `location` files into the world graph
- Loads `object` files into an object table
- Returns `character` files for the caller to hand to `CharacterEngine`

`thwart validate` reports every file that failed to parse, plus every connection that points at a location `id` that was never loaded.

## What's still open

- **`id` vs. file path.** Nothing currently checks that `id` matches the file's location. An author who copies a file and forgets to update `id` gets a silent collision (last one loaded wins) instead of an error. Duplicate ids get reported along with the file writer (`PLAN.md` › Phase 3); checking a location's `id` against its path waits for Phase 5.
- **Objects without cards are scenery.** A location's `objects: [...]` or a character's `inventory: [...]` can name an object `id` with no file. That's allowed: the object is scenery with a name only, and the console creates its card the first time an author edits it (`REQUIREMENTS.md` › REQ-CF-8). Tools should label it "no card", not report it as missing.
- **Dialogue tree format is unspecified.** `dialogue_tree` is loaded as an opaque string. What it points to, and how it's structured, is undecided.
