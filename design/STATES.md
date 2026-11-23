# States, bundles and chapters

Status: Draft, from a design discussion between Bruce and the commander on 2026-11-23. Nothing here is built. Items marked *proposed* are the commander's suggestions that Bruce hasn't confirmed yet; everything else Bruce has agreed to.

## Summary

A room, object or character is either one Markdown file or a folder of them (a bundle). A bundle holds versions of the thing: its states (bread that is fresh or stale) and its changes over the story (chapter overlays). Major changes are separate files; minor ones are variables in the prose. An author never writes conditions into prose, and rarely writes them anywhere.

This replaces the earlier idea of conditional text blocks (`{{< when >}}` shortcodes) and conditions on exits. The one condition that survives is `requires:` on an action.

## Principles

Agreed by Bruce, 2026-11-23. Every decision in this document should follow from at least one.

1. **Lineage only.** A thing takes values from global definitions, the folders above it, and templates it names; never from a sibling beside it.
2. **Explicit, never hidden.** Inheritance is written down, and the console shows where every value came from.
3. **Predictable from the tree.** An author can tell what applies by looking at the folders.
4. **Major changes are files, minor ones are values.** States and chapters are files; small changes are variables.
5. **Files are the truth; the console is how you edit them.** Structure changes go through commands, and the files stay plain Markdown.
6. **Play never changes content.** Everything that happens in a game lives in the game state.
7. **Keep the condition language small.** Only `requires: { has: ... }`, until a real case needs more.

Review status: decisions are being reviewed with Bruce group by group. Files and bundles: reviewed and kept. Inheritance: reviewed, with two changes (list replacement and placement-based overlays now warn). Chapters: reviewed, with two items reopened (below). Changing state: reviewed, with doors and templates designed in outline. Variables: reviewed; undeclared variables and shared ids warn rather than fail. All 2026-11-23.

## Terms

- **Thing**: a room, object or character.
- **Bundle**: a thing stored as a folder with an `index.md`, rather than as one file.
- **State**: a named version of a thing, stored as a file in its bundle (`stale.md`). The file name is the state's name.
- **Chapter**: a named stage of the story arc. Chapters are defined at the world root.
- **Overlay**: a file that changes a thing for a chapter. It overrides only what it states.
- **Template**: any file another file inherits from, named by `inherits:`.

## Files

### Bundles

```
objects/
  lantern.md                 # a simple thing: one file
  bread/                     # a bundle: the folder is the thing
    index.md                 # shared attributes, starting state
    fresh.md
    stale.md
castle/
  _index.md                  # an area, not a thing
  vault/
    index.md
    closed.md
    open.md
```

As in Hugo, a folder containing `index.md` is one thing (a "leaf bundle"). A folder without one, or with `_index.md`, is an area. The bundle's `id` comes from the folder (`bread`, `castle/vault`). State files have names, not ids.

`index.md` holds what every state shares and names the starting state:

```yaml
# objects/bread/index.md
id: bread
title: Loaf of bread
type: object
object_type: food
state: fresh
```

A state file states its template and only what differs:

```yaml
# objects/bread/stale.md
inherits: index
vars: { tasty: no }
---
The loaf has gone hard. It would still keep you alive.
```

### Chapters

Chapters follow the same lineage as everything else: they can exist at the world root or inside any thing (Bruce, 2026-11-23).

At the root, a chapter defines the chapter itself and what changes for the whole world (story values, and later anything global). It is one file, the same shape as a thing's chapter overlay, or a folder when it has sub-chapters (below):

```
chapters/
  10-the-summons.md
  20-after-the-king.md
```

```yaml
# chapters/20-after-the-king.md
title: After the king
sets: { story.king: dead }          # story values on entering the chapter
---
<!-- Arc notes: the court splits; the guard captain takes charge. -->
```

The number prefix only orders the chapters; the chapter's name is the rest (`after-the-king`), and that's what overlays and actions use. Leave gaps (10, 20, 30) so a chapter can be inserted by naming one file. A root chapter can exist before any overlay does, as a placeholder for the arc. Root chapters don't change individual things; that's what a thing's own overlays are for. What a root chapter can change is world-level settings, the kind `world.toml` holds (starting room, title, intro text), plus story values (Bruce, 2026-11-23). Whether world-level content also gets a `world/` or `game/` folder is open.

Inside a thing, a chapter is an overlay for that thing:

```
castle/courtyard/
  index.md
  chapters/
    after-the-king.md        # the courtyard in chapter 2
npcs/guard/
  index.md
  friendly.md
  hostile.md
  chapters/
    after-the-king.md        # the guard in chapter 2
```

`chapters` is a reserved folder name everywhere in a world (Bruce, 2026-11-23): at the root it holds the chapters, and anywhere else it holds that folder's or thing's chapter versions. An overlay's file name matches a root chapter's name, without its number, for now; local or "secret" chapters, defined inside an area or around an object rather than at the root, are wanted but not yet designed (open question below). A thing that's one file becomes a bundle when it gets its first state or overlay; the console does that move (see Console).

Chapters are siblings: one chapter's changes never carry into the next (Bruce, 2026-11-23). A chapter applies to another only when it is its parent. A root chapter with sub-chapters is a folder, and its sub-chapters inherit from it:

```
chapters/
  10-the-summons.md
  20-after-the-king/
    _index.md                # the chapter itself
    21-the-funeral.md        # a sub-chapter; after-the-king applies too
```

A thing's overlays follow the same shape: `castle/courtyard/chapters/after-the-king.md` applies in `after-the-king` and in its sub-chapters, and `castle/courtyard/chapters/after-the-king/the-funeral.md` only in `the-funeral`. So an author can tell what applies from the folder tree.

### What applies, in order

A thing's current version is built in layers, each overriding only what it states. Stacking is inheritance, and it follows lineage only: global definitions, then the folders above the thing, then the thing itself, never a sibling beside it (Bruce, 2026-11-23). An author can think about parent folders and globals, and nothing else.

For the courtyard, in the sub-chapter `the-funeral`, in its `burning` state:

```
1. globals ................ chapters/20-after-the-king/_index.md, then 21-the-funeral.md
                            (global settings only)
2. folders above it ....... castle/chapters/after-the-king.md, then .../the-funeral.md
3. the thing .............. castle/courtyard/index.md
4. its current state ...... castle/courtyard/burning.md
5. its chapter overlays ... castle/courtyard/chapters/after-the-king.md, then .../the-funeral.md

  never: castle/gardens/*, or chapters/10-the-summons.md
```

Within each level, a parent chapter comes before its sub-chapter. An area can therefore change every room in it for a chapter (the whole castle in mourning) through its own `chapters/` folder (Bruce, 2026-11-23).

An overlay applies on top of whichever state is current; it can't vary by state. Bruce sees no case for that yet (2026-11-23), so it's out of scope.

Frontmatter merges by the inheritance rules below. Prose replaces the layer below only if the layer has prose; a body that is empty once author comments are removed counts as no prose, so an overlay holding only a note never blanks a room.

## Inheritance

Inheritance is never implicit (Bruce, 2026-11-23). A file inherits only if its frontmatter says `inherits:`.

- Inside a bundle, a state file says `inherits: index`. The console writes this when it creates a state.
- Across things, `inherits: objects/box` makes a thing start from another thing, as a template. A bare name (`index`) means a file in the same bundle; anything else is a path from the world root without `.md`. A template passes down its attributes and its prose, never its `id`; a bundle used as a template passes down only its `index.md`, never a state.
- A file has one parent. Cycles are an error. Template chains are at most 4 levels deep (*proposed*); a bundle's own index, state and overlay layers don't count toward that.
- A scalar or a list in the child replaces the parent's, so a state that lists `objects:` replaces the whole list; `validate` warns (never errors) each time a layer replaces an inherited list (Bruce, 2026-11-23). Maps (`connections`, `vars`) merge key by key, all the way down. `null` removes an inherited key; `validate` warns about each removal, since a bare `north:` with no value also means `null`.
- Overlays inherit from whatever version is current, by definition, and don't need `inherits:`. Because that bends principle 2, `validate` warns about each overlay that applies by placement, for now; if that proves confusing or noisy, it gets adjusted (Bruce, 2026-11-23).
- The console always shows the result and where each part came from (`north: castle/hall (from index)`). Nothing inherited is hidden.

## Engine rules

Settled by the commander from the coder's review (2026-11-23).

**Loading.**
- An entity's `id` comes from its `index.md` frontmatter, as today. The folder is only how the loader finds the files. `validate` warns when the `id` and the folder disagree.
- A bundle is a leaf. It may contain state files and one `chapters/` folder, nothing else; a nested `index.md` is an error. `index`, `_index` and `chapters` are reserved and can't be state names.
- The loader knows what a file is from where it sits: next to an `index.md` it's a state, under a thing's `chapters/` it's an overlay, and at the root `chapters/` it's a chapter. State and overlay files have no `id` or `type`.
- `_index.md` marks an area. Its frontmatter MAY have a `title` and author notes, and nothing else yet.
- Layers merge as raw YAML before the result is typed as a location, object or character. Every key remembers the file it came from, for error messages and for the console's "(from index)" labels.

**Current version.**
- A thing's current version depends on play state, so it is built on request, not stored at load.
- Edit mode shows a thing's starting version (starting state, no chapter) unless a playtest is running, and then the version the game is in; a header always says which (`castle/vault · open · after-the-king`). `show` and `validate` default to the starting version. `thwart show --state <name> --chapter <name>` shows any other. `validate` checks every state against every chapter stack.
- Only `index.md` may set `state:`, and it means the starting state. No layer may change `id` or `type`. A bundle may leave out `state:`, in which case `index.md` alone is the version until something switches it.
- What a room holds is its current `objects:` list, minus anything the player holds or has taken, so a chapter overlay can never duplicate an item the player already has. With counts (see Templates and copies), it's the count minus the number taken.

**Ids and names.**
- Once states exist, an `id` shared by two kinds (a door and a room both called `vault`) is ambiguous, because `becomes:`, `requires:` and `{{ }}` name things by bare id. It's a warning in authoring, and the console proposes a rename to fix it (for example `vault` → `vault-door`) (Bruce, 2026-11-23). That needs renaming, which updates every reference to the old id (see Console).
- `game`, `story`, `player`, `location` and `this` are reserved and can't be ids. Ids can't contain a dot, so `{{ castle/vault.lid }}` splits cleanly at the last dot.

**Values.**
- Values are strings. Where values are compared, `yes`, `true`, `no` and `false` count as the same pair on purpose.
- `requires:` stays closed to `has:` until a real case needs more. Each new key would bring the condition language back.
- Variables work in prose only for now, not in frontmatter.
- Actions are a map, not a list of one-key maps: `actions: { pull: { text: ..., becomes: ... } }`. The existing object cards move to this shape (the coder migrates them).

**Play state and saves.**
- `PlayState` stores each thing's state only when it has left its starting state (like `taken`), the current chapter, values by level, visits per room and the turn count. A turn is one player action that the game understood; console commands and unknown verbs don't count. `flags` goes away.
- Saves become `thwarter_save: 2`. A version 1 save loads as is. On load, a state that no longer exists falls back to the starting state, a thing that's gone is dropped, and a chapter that's gone falls back to the nearest earlier one, each with a warning.

**Writing.**
- Turning a one-file thing into a bundle writes the new files first, then renames the old file to `index.md` last. If that fails partway, both files exist and `validate` reports a duplicate `id`, so a half-done move is visible, never silent. The file watcher sees it as a delete and a create.
- A bundle saves through one call that checks the merged thing before writing anything. It takes each file's text as it was when the editor opened it, and if any of the bundle's files has changed on disk since then, the whole save is a conflict. Only sections that changed are written, so a one-word edit to `open.md` leaves `index.md` untouched on disk and in git. Every changed file is written to a temporary file and checked, then renamed in turn; if a rename fails, the console reports exactly which files were written.
- Renaming or deleting a state is a command (`/state rename`, `/state delete`). A rename also updates every `becomes:` and `state:` that names it.

## Templates and copies

Agreed by Bruce, 2026-11-23.

Two different things that share a definition are two files, each naming a template:

```
objects/
  templates/book.md        # every book: can be read, weight: light
  red-cookbook.md          # inherits: objects/templates/book
  blue-diary.md            # inherits: objects/templates/book
```

A template is never placed in the world itself. *Proposed*: it's marked by living in a `templates/` folder, a reserved name like `chapters`. Most doors and many objects are expected to be global templates (`locked-door`, `magic-door`, `fortified-door`) that hold their whole definition, so a specific door or object is mostly just a name and a template.

Identical copies (three healing potions) are one thing with a count: `objects: { potion: 3 }`, with play state tracking how many are left. A copy that needs to differ becomes its own thing with its own file. Counts come after templates, when a world needs them.

## Doors

Agreed in outline by Bruce, 2026-11-23; details still open.

A door is its own type of thing, because doors can't be room states: a room with two doors would need four versions, and with three, eight. A door joins two rooms and has one state seen from both sides.

```yaml
# doors/vault-door.md
id: vault-door
title: Iron door
type: door
inherits: doors/templates/locked-door
sides:
  castle/hall:  { direction: down }
  castle/vault: { direction: up }
```

- Each room's exit names the door rather than a room: `connections: { down: vault-door }`. The door says where each side leads, so the two rooms always agree.
- Its states (`locked`, `shut`, `open`) are a bundle like any other thing's, with `passable:` saying whether the exit can be used. Unlocking is one of its actions: `unlock: { requires: { has: brass-key }, becomes: { vault-door: shut } }`.
- The room view shows the door's current text beside its exits; the room's own prose never changes for a door.
- Most doors will come from global templates (Bruce, 2026-11-23); doors defined per area are possible but expected to be rare.

## Scope

Where a thing's file lives is its scope (Bruce, 2026-11-23):
- **Global**: the world's root folders (`objects/`, `npcs/`, `doors/`) and their `templates/`.
- **Area**: inside an area folder, for things that belong to one part of the world.
- **Local**: inside a thing's bundle.

The console MUST make scope easy. While writing a room, an author can add several things and choose their scope as they go (for example, add three objects, all global), and can later change a thing's scope. Changing scope moves the file; because object, character and door ids don't include their folder, references to them stay valid. The commands are console work, still to be designed (`design/CONSOLE_REQUIREMENTS.md`).

## Changing state

How states switch is mostly invisible to the author, who works in the console (Bruce, 2026-11-23). Underneath:

- An action sets a thing's state directly: `becomes: { castle/vault: open }`.
- An action can need something first: `requires: { has: brass-key }`. This is the only remaining condition, a small YAML map.
- An action moves the story to a chapter: `chapter: after-the-king`. Every overlay for that chapter then applies, with no list to keep up to date. Since chapters are siblings, going from one chapter to another applies only the new chapter (and its parents); nothing from chapters in between. States and chapters can move forward or back (Bruce, 2026-11-23).
- An action changes a value: `sets: { knife.sharp: no }`. Values hold once set (Bruce, 2026-11-23).
- The game itself updates some values on its own: time, weather, randomness and similar, under `game.`.
- Doors are their own type (see Doors). A door's state switches like any other thing's, so a room never needs versions for its doors.
- Time-based change (bread going stale after 20 turns) needs a clock, which doesn't exist yet. *Proposed* for later: `after: { turns: 20, becomes: stale }` on a state.

```yaml
# objects/lever.md
actions:
  pull:
    text: "Gears grind somewhere below."
    becomes: { castle/vault: open }
```

## Variables

Prose can include variables, written `{{ name }}`. Every name starts with the level it belongs to, so the first word says where to look (Bruce, 2026-11-23):

| Level | Prefix | Examples | Declared in |
|---|---|---|---|
| Game | `game.` | `game.turn`, `game.score` | the engine; fixed |
| Story | `story.` | `story.chapter`, `story.king` | `world.toml` `[story]`, with starting values |
| Player | `player.` | `player.name`, `player.health` | `world.toml` `[player]` |
| Current room | `location.` | `location.title`, `location.visits` | the engine and the room |
| A thing | `<id>.` | `box.lid`, `knife.sharp` | the thing's `vars:` |
| This thing | `this.` | `this.lid` | the thing's `vars:` |

A thing separates its values from its abilities:

```yaml
# objects/knife.md
vars: { sharp: yes }    # values; can change in play
can: [cut]              # abilities; checked by actions
```

A variable that isn't declared at its level is a warning, not an error: edit mode shows it in place, marked, with the warning beside it, and `validate` lists it. In play it shows as blank, never as raw `{{ }}` (Bruce, 2026-11-23). Author comments (`<!-- -->`) show in edit mode and are removed in play.

## Console

In files, a bundle is separate files. In the console, it is one document with a section per file, each section starting with its frontmatter (Bruce, 2026-11-23):

```
/edit room  castle/vault                         ctrl-s save · esc close

  --- index
  title: Vault
  connections: { north: castle/hall }
  ---

  --- state: closed (starting)
  inherits: index
  ---
  A heavy iron door is set into the floor, shut fast.

  --- state: open
  inherits: index
  connections: { down: castle/vault-stairs }
  ---
  The iron door stands open. Cold air rises from below.

  --- chapter: after-the-king
  ---
  The door hangs off one hinge now.
```

The section headers are not text. The editor draws each one (`--- state: open (starting)`) as a fixed separator, with one text area per section, and the cursor moves across them. So no body line ever needs escaping, and a header can't be renamed or deleted by editing. New states and overlays come only from commands (`/state new`, `/chapter overlay`), never by typing a header. Sections appear in this order: `index`, the starting state, the other states A to Z, then chapter overlays in chapter order. (Settled by the commander with the console coder, 2026-11-23.)

Saving splits the document back into its files. Every section must parse before any file is written; if one fails, nothing is saved. A file that doesn't parse still opens, since editing is how an author fixes it. A one-file thing opens as a document with one section.

Full editing is a full-screen editor in the terminal (Bruce, 2026-11-23; `design/CONSOLE_REQUIREMENTS.md` › REQ-CON-6), since a document with sections can't be edited inline.

Commands the console will need, *proposed*: `/state new <name>`, `/state set <name>` (the starting state), `/chapter new <number> <name>` at the root, and `/chapter overlay` inside a thing. Turning a one-file thing into a bundle (`lantern.md` → `lantern/index.md`) is a file move, which the writer refuses today; these commands are where it gets built.

## Validation

`thwart validate` (and `/reload`) will check:
- every `inherits:` names a file that exists; no cycles; chains within the limit
- every `becomes:` names a state that exists
- every chapter overlay matches a root chapter, and every `chapter:` action names one
- every bundle's `state:` names one of its state files
- every `{{ variable }}` is declared at its level

## Engine changes

- The loader reads bundles and root chapters, and builds each thing's current version from its layers.
- `PlayState` gains each thing's current state, the current chapter, and variable values; `flags` becomes these. Saves store all of it.
- The writer gains file moves (one file to a bundle) and multi-file saves for the combined document.
- The play engine and the edit engine both keep the current game and player state: the location, each thing's state, the chapter and values. Showing that state in the editor is central to authoring (Bruce, 2026-11-23); how it looks is console work, still to be designed.

## Open questions

1. **Doors, the details**: where a door's file lives when it isn't global, one-way and hidden doors, see-through windows, and whether a specific door can be just a template plus two rooms with no file of its own.
2. **How chapters overlay, and local chapters.** Authors may use chapters globally, but also add "secret" chapters or sub-chapters inside an area or tied to an object. Where are those defined, what starts them, and how do they stack with root chapters (Bruce, 2026-11-23)?
3. **World-level content**: `world.toml` alone, or also a `world/` or `game/` folder for world-level text and settings?
4. **Clocks and game values**: when do time, weather and randomness come in, and is time turns or game time?
5. **Showing play state in the editor**: what the author sees, and how they change it while playtesting.
6. **Templates**: a reserved `templates/` folder, or a `template: true` marker?
