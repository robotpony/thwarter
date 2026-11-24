# Content requirements

Requirements for the content of a world: the file format, how things are laid out, and the world model of states, bundles and chapters. Owned by the commander. The field-by-field reference is `design/CONTENT_FORMAT.md`, and the reasoning behind the world model is `design/STATES.md`. RFC 2119 keywords are used as in `REQUIREMENTS.md`, and every requirement answers to its principles.

Split out of `REQUIREMENTS.md` on 2026-11-23. The REQ-CF and REQ-WS ids are stable. A change another component needs here goes to the commander as a message.

## Files

`design/SECTIONS.md`, agreed in outline by Bruce and under trial, changes REQ-CF-6, 17, 28 and 29 (content shown by matching `## When:` headings against game state, no marker comments). The requirements here are rewritten when the trial confirms it.

- REQ-CF-1: Every content file MUST be a Markdown file whose frontmatter is a `---`-delimited YAML block starting on the file's first line.
- REQ-CF-2: Frontmatter MUST include `id` (stable identifier), `title` (display name), and `type` (`location`, `character`, or `object`). A file missing any of these three MUST fail to load with an error identifying the file and the missing field, not load with a default or placeholder value.
- REQ-CF-3: The Markdown body following the closing `---` MUST hold everything a player reads: the entity's description, then any response blocks (REQ-CF-17). The format MUST NOT require or expect a title heading in the body; `title` lives in frontmatter exactly once, and is the only content frontmatter may hold.
- REQ-CF-4: For `type: location`, `connections` (direction to target location `id`), `characters` (list of character `id`s present by default), `objects` (list of object `id`s present by default), and `coordinates` (`[x, y, z]`) are OPTIONAL. Direction names in `connections` MUST be treated as free text, not a closed set (`north`/`south`/etc. are convention, not a required vocabulary).
- REQ-CF-5: For `type: character`, `character_type` (one of `npc`, `player`, `villain`, `hero`, defaulting to `npc`), `default_location`, `attributes` (name to integer), `dialogue_tree` (opaque reference, format unspecified, see §8), `inventory` (list of object `id`s), and `relationships` (list of single-key maps, other character `id` to a free-text label) are OPTIONAL.
- REQ-CF-6: For `type: object`, `object_type` (free text, e.g. `weapon`, `container`, `environmental`) is REQUIRED; `category`, `weight`, `value`, and `actions` (named action to its mechanics, REQ-CF-17) are OPTIONAL. Object content MAY include arbitrary additional frontmatter fields (`properties`, `stats`, `effects`, and similar); a conforming parser MUST preserve these rather than rejecting the file or silently discarding them, since object content varies more than location or character content does and a fixed schema would be false precision.
- REQ-CF-7: A loader encountering a file that fails REQ-CF-1 or REQ-CF-2 MUST record it as a per-file error and continue loading the rest of the world, rather than aborting the whole load. A single bad file MUST NOT prevent validation from reporting problems in every other file.
- REQ-CF-8: An object `id` named in a location's `objects` or a character's `inventory` MAY have no object file (a card). Such an object is scenery: it has a name and nothing else, and it MUST NOT be reported as an error. When an author first edits it, the tool MUST create its card (REQ-CON-6).

## World structure and addressing

- REQ-WS-1: World content MUST live under `authors/<author-name>/<world-name>/`, one directory per author, so a single repository MAY hold more than one author's world.
- REQ-WS-2: A location's `id` SHOULD read as a place a human would recognize (`castle/courtyard`), not an opaque identifier (a UUID or sequence number).
- REQ-WS-3: A world directory MAY include a `world.toml` (title, author, starting location, and world-specific settings) and the tool's own `thwart.toml` (default world path, output preferences); neither is required for a world to load.

- REQ-WS-4: A persona is `authors/<persona>/index.md`, Markdown with frontmatter (Bruce, 2026-11-23): `type: author`, `title` (the pen name readers see), and contact details (`email`, `website` and similar), with the bio as its prose. It MAY also name a default writing style and a default world. A persona folder without `index.md` still works, with its folder name as the pen name. A TOML form may follow later if it's needed.
- REQ-WS-5: A world's `world/index.md` MUST name its author by persona id (`author: bob`), so the pen name and contact details come from the persona file and changing them there changes every world.
- REQ-WS-6: A persona's contact details are published with every archive of its worlds (REQ-CFG-7). The console MUST say so before packaging.

## World model

Written from `design/STATES.md` as agreed with Bruce on 2026-11-23. Where this list and that document disagree, it's a bug in one of them.

- REQ-CF-9: A room, object, character or door MUST be either one Markdown file or a bundle: a folder holding an `index.md`. A folder without an `index.md`, or with an `_index.md`, MUST be read as an area. `index`, `_index`, `chapters`, `templates`, `world` and `game` are reserved names.
- REQ-CF-10: A bundle MAY hold state files beside its `index.md`. A state's name MUST be its file name, and a state file MUST NOT set `id`, `type` or `state:`. Only `index.md` names the starting state, and MAY leave it out.
- REQ-CF-11: A file MUST inherit only when it says `inherits:`. A state file's `inherits: index` means its bundle's `index.md`. Any other bare name is looked up in `templates/` folders from the thing's own folder outward to the root, then in the engine's built-ins. Anything with a slash is a path from the world root. One parent only; cycles are an error.
- REQ-CF-12: A thing's current version MUST be built from its lineage only, in this order: globals, the folders above it, the thing's `index.md`, its current state, then its chapter versions, with a parent chapter before its sub-chapter. Nothing MUST come from a sibling beside it.
- REQ-CF-13: Frontmatter merges by key, all the way down for maps; a list or a scalar replaces the one below it, and `null` removes a key. The body merges by block: the description replaces the description below, and each response block replaces only its namesake. A thing's own layers replace prose; an area's layers add theirs after the description.
- REQ-CF-14: Chapters MAY be defined at the root or in any scope, in a `chapters/` folder. A numbered file defines a chapter in that scope; an unnumbered one is a thing's or an area's version for the nearest chapter of that name. Chapters are siblings and MUST NOT carry into each other; only a parent chapter applies to its sub-chapters.
- REQ-CF-15: A template MUST NOT appear in the world itself. A template that is a bundle passes down its states as well as its `index.md`; a thing's own state of the same name is layered over the template's. A file is a template if it lives in a `templates/` folder, in any scope, or says `template: true`.
- REQ-CF-16: Variables in prose MUST be written `{{ level.name }}`, with the level one of `game`, `story`, `player`, `location`, `this` or a thing's `id`. A variable that isn't declared MUST be reported as a warning, never an error, and MUST show as blank in play.
- REQ-CF-17: An action's mechanics (`becomes:`, `sets:`, `chapter:`, `requires:`) MUST be in frontmatter, and what the player reads MUST be in the body, after a `<!-- thwarter:action <name> -->` marker (`<name> refused` for a failed `requires:`). A response block MAY stand alone with no mechanics.
- REQ-CF-18: A state MAY set `visibility:` to `visible` (the default), `faint`, `hidden` or `absent`.
- REQ-CF-19: A door MUST be its own type, joining two rooms with one state seen from both sides; a portal MAY join more. Each room's exit names the door, and the door names its sides.
- REQ-CF-20: The world's definition MUST live in a `world/` folder: `index.md` (title, author, starting room, and the intro as its prose), `story.md` (`story.` values), `player.md` (the player, a character, with `player.` values) and `styles/`. `world.toml` MAY hold tool settings only.
- REQ-CF-21: The engine MUST supply built-in templates that any world can name without defining them, and a world template of the same name MUST override one. Phase 4 ships `scenery`; the door templates ship with doors.
- REQ-CF-22: An object, character or door `id` MUST be unique across the whole world, whatever scope its file lives in, so a room's `torch` always means one file. Moving a thing between scopes (`/move`) keeps its `id`. Templates and chapters are named per scope instead, and are looked up along the lineage (REQ-CF-11, REQ-CF-14). Decided by the commander, 2026-11-23, at the console coder's proposal.

## Working with other tools

Principle 9 (Bruce, 2026-11-23). Much of the format already follows Hugo's conventions: leaf bundles with `index.md`, sections with `_index.md`, `title` in frontmatter, and plain Markdown bodies. Items marked *proposed* are the commander's reading of what "where convenient" covers; to confirm with Bruce.

- REQ-CF-23: World files MUST be plain Markdown with YAML frontmatter that VS Code, Obsidian and Hugo can open without errors. The format MUST NOT add syntax that breaks their preview. Response markers and section headers are HTML comments, which all three hide.
- REQ-CF-24: The loader MUST ignore folders and files that belong to other tools: any name starting with `.` (`.obsidian/`, `.vscode/`, `.git/`), and Hugo's own (`layouts/`, `static/`, `assets/`, `public/`, `hugo.toml`). A world MAY list more in `world.toml`. *Proposed.*
- REQ-CF-25: Wherever the format takes an `id` (exits, `objects:`, `inherits:`, `becomes:` and the rest), it SHOULD also accept an Obsidian link to the same file (`"[[castle/courtyard]]"`), so Obsidian's graph shows the world's map and links (Bruce confirmed links matter to Thwarter, 2026-11-23). A link resolves as an `id` first, then as a path from the world root (a bundle by its folder or its `index.md`), then, as Obsidian does, by bare file name when exactly one file has that name. Aliases (`|`), headings (`#`) and a `.md` suffix are allowed. A bare name that matches more than one file MUST be reported as ambiguous, naming the files. Built in 0.18.0.
- REQ-CF-26: Thing ids SHOULD match their file names, as REQ-CF-22 and the default layout already make them, because Obsidian identifies a note by its file name. *Proposed.*
- REQ-CF-28: Thwarter files are Markdown, and the mechanics of Markdown documents apply to them (Bruce, 2026-11-23). Headings, emphasis, lists and links in prose MUST render in play as a terminal can show them, and MUST NOT be shown as raw syntax.
- REQ-CF-29: A link in prose to a thing (`[[lever]]`, `[[castle/vault]]` or `[the lever](objects/lever.md)`) MUST mean that thing to Thwarter, whatever its kind: object, character, door, location. In play it shows as its text. In edit mode the console MUST be able to follow it, and `validate` MUST warn about a link to a thing that doesn't exist, as it does for exits. *Proposed*: a room's prose that links an object it doesn't place makes that object examinable from the room, as scenery, without placing it.
- REQ-CF-30: Tags (`#tag` in prose, or `tags:` in frontmatter, as Obsidian uses them) MUST be searchable and filterable in the console. *Proposed*: play shows a tag in prose as its plain word, without the `#`.
- REQ-CF-31: An object MAY hold other objects, listed in its own `objects:` the way a location lists them (`walls` holding `photo-1`), so the photos hang on the walls rather than sit in the hallway (Bruce, 2026-11-23). A thing starts in one place only: a room, a character's `inventory`, or an object; `validate` reports one placed twice. What an object holds is in scope whenever the object is, so `look at photo` works in the hallway, and examining the holder lists what it holds (form: `design/CONTENT_FORMAT.md` › What's still open). A thing kept in play (`keep()`, a key left in a lock) is the same relation, made during a game.
- REQ-CF-27: `{{ variables }}` are written so Hugo leaves them alone: Hugo only runs templates in layouts, and runs shortcodes only in the `{{< >}}` and `{{% %}}` forms. A Hugo site MAY render them through a small plugin of its own. *Proposed.*

## Open questions

The world model's open questions are in `design/STATES.md` › Open questions. Format questions:

1. Dialogue tree format is unspecified. REQ-CF-5 loads `dialogue_tree` as an opaque reference.
2. Should `validate` check a location's `id` against its path as a MUST once the check exists?
3. ~~Tags and links in prose~~: they matter to Thwarter (Bruce, 2026-11-23); REQ-CF-28 to 30.
