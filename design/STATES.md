# States, bundles and chapters

Status: Agreed, with open questions (below). Reviewed decision by decision with Bruce on 2026-11-23. Nothing here is built yet. Items marked *proposed* are the commander's suggestions that Bruce hasn't confirmed yet; everything else Bruce has agreed to.

`design/SECTIONS.md`, agreed in outline by Bruce and under trial, shows content by matching `## When:` headings against the game state, replaces actions and responses with reply sections, and retires the `<!-- thwarter:... -->` markers. This document now uses sections throughout; where the two disagree, `design/SECTIONS.md` wins.

## Summary

A room, object or character is either one Markdown file or a folder of them (a bundle). A bundle holds versions of the thing: its states (bread that is fresh or stale) and its changes over the story (chapter overlays). Major changes are separate files; minor ones are variables in the prose. An author never writes conditions into prose, and rarely writes them anywhere.

This replaces the earlier idea of conditional text blocks (`{{< when >}}` shortcodes) and conditions on exits. The one condition that survives is `requires:` on an action.

## Principles

The principles now live in `REQUIREMENTS.md` › 4. Principles, so every component's requirements answer to the same list. They were first agreed here, with Bruce, on 2026-11-23.

Review status: reviewed with Bruce group by group, all six groups done. Files and bundles: reviewed and kept. Inheritance: reviewed, with two changes (list replacement and placement-based overlays now warn). Chapters: reviewed, with two items reopened (below). Changing state: reviewed, with doors and templates designed in outline. Variables: reviewed; undeclared variables and shared ids warn rather than fail. Console: reviewed; full editing may take the whole terminal, headers defined, conflicts handled by git and safe reloads. All 2026-11-23.

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

The number prefix only orders the chapters; the chapter's name is the rest (`after-the-king`), and that's what overlays and actions use. Leave gaps (10, 20, 30) so a chapter can be inserted by naming one file. A root chapter can exist before any overlay does, as a placeholder for the arc. Root chapters don't change individual things; that's what a thing's own overlays are for. What a root chapter can change is world-level settings (starting room, title, intro text) and story values (Bruce, 2026-11-23).

The world's definition gets its own `world/` folder (Bruce, 2026-11-23). The running game is a virtual `game/` folder the console shows but that isn't on disk. A real `game/` folder, if one exists, can only set the game's starting state.

The layout of `world/`, settled by the commander for Phase 4 (2026-11-23; Bruce may overrule):

```
world/
  index.md      # the world: title, author, starting room; the intro is its prose
  story.md      # story. values, declared with their starting values
  player.md     # the player: a character, with player. values
  styles/       # writing styles for the writing helpers
  commands/     # the world's own console commands, for its authors in edit mode (proposed; CONTENT_FORMAT.md › type: command)
```

```yaml
# world/index.md
title: Castle Adventure
type: world
author: bob                      # a persona id (REQ-WS-5)
start: castle/courtyard
---
The intro a player reads when a new game starts.
```

```yaml
# world/story.md
type: story
vars: { king: alive }
---
<!-- Story notes for the author. -->
```

```yaml
# world/player.md
title: Traveller                 # the player's default name
type: character
character_type: player
vars: { health: 100 }
---
What the player sees when they examine themselves.
```

- Values use `vars:` everywhere, as things do, so `story.king` is declared in `story.md` and `player.health` in `player.md`. `{{ player.title }}` is the player's name.
- `world.toml` keeps tool settings only, such as the default map style. The world's title, author, version and starting room move to `world/index.md`, and its description becomes the intro prose.
- The test world's `world.toml` also holds counts and flags that nothing reads (`locations_count`, `relationship_system` and the like). They're dropped in the move, and `CHANGELOG.md` records them.
- Built in 0.16.0 (7c7772d), and confirmed by the commander: `world.toml` MAY list more paths for the loader to ignore, as `ignore = ["drafts", "notes/old"]` from the world root. Hugo's own folder names are ignored only at the root, so `castle/static/` is still an area. A world with no `world/index.md` still reads `world.toml`'s old `[metadata]` and starting location, quietly, so older worlds keep working; a deprecation warning can come later. A player with no `title` is called "you". Unknown keys in `world/` files warn, and so do unknown files there, except under `styles/`.

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

Chapters can be defined in any scope, not only at the root (Bruce, 2026-11-23). A chapter defined inside an area or a thing (a "secret" chapter) is an ordinary chapter with a narrow scope: it exists only there, and an action inside that scope moves to it. The same goes for objects, characters and everything else: anything defined in a limited scope, and any global too, can be hidden. Hidden only means it is found, and means something, in a specific state or chapter. The timeline is the playthrough that results from the states a player passes through; nothing else decides what's hidden.

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

Frontmatter merges by the inheritance rules below. Prose merges block by block (agreed by Bruce, 2026-11-23):
- A body is a description followed by `## When:` sections (`design/SECTIONS.md`). The description replaces the description below it; sections are kept apart from it and shown when their tags match, and a thing's own section replaces a template's with the same tags. So a state can change one reply without restating the description, and the other way round.
- A thing's own layers (its `index.md`, state and chapter versions) replace prose. An area's layers add it: an area chapter's description is added after the description of every room beneath it, so "the castle in mourning" reads as each room's own text plus `Black banners hang from every wall.`
- A description that is empty once author comments are removed counts as no prose, so a layer holding only a note never blanks a room.

## Inheritance

Inheritance is never implicit (Bruce, 2026-11-23). A file inherits only if its frontmatter says `inherits:`.

- Inside a bundle, a state file says `inherits: index`. The console writes this when it creates a state.
- Across things, `inherits: objects/box` makes a thing start from another thing, as a template. A bare name (`index`) means a file in the same bundle; anything else is a path from the world root without `.md`. A template passes down its attributes and its prose, never its `id`; a bundle used as a template also passes down its states (see Templates and copies).
- A file has one parent. Cycles are an error. Template chains are at most 4 levels deep (*proposed*); a bundle's own index, state and overlay layers don't count toward that.
- A scalar or a list in the child replaces the parent's, so a state that lists `objects:` replaces the whole list; `validate` warns (never errors) each time a layer replaces an inherited list (Bruce, 2026-11-23). Maps (`connections`, `vars`) merge key by key, all the way down. `null` removes an inherited key; `validate` warns about each removal, since a bare `north:` with no value also means `null`.
- Overlays inherit from whatever version is current, by definition, and don't need `inherits:`. Because that bends principle 2, `validate` warns about each overlay that applies by placement, for now; if that proves confusing or noisy, it gets adjusted (Bruce, 2026-11-23).
- The console always shows the result and where each part came from (`north: castle/hall (from index)`). Nothing inherited is hidden.

## Engine rules

The engine rules settled during this design, from the coder's review on 2026-11-23 onwards, are now requirements: `design/ENGINE_REQUIREMENTS.md` › REQ-ENG-10 to 29, grouped as loading, chapters, building a version, ids and values, play and saves, and writing bundles. That document is the one copy; this one no longer repeats them. The rules as they stood here are in `git show 1e2af89:design/STATES.md`.

## Templates and copies

Agreed by Bruce, 2026-11-23.

Two different things that share a definition are two files, each naming a template:

```
objects/
  templates/book.md        # every book: can be read, weight: light
  red-cookbook.md          # inherits: objects/templates/book
  blue-diary.md            # inherits: objects/templates/book
```

A template is never placed in the world itself. It's marked either way (Bruce, 2026-11-23): by living in a `templates/` folder, which can exist globally or in any scope and is a reserved name like `chapters`, or by `template: true` in its own frontmatter. Most doors and many objects are expected to be global templates (`locked-door`, `magic-door`, `fortified-door`) that hold their whole definition, so a specific door or object is mostly just a name and a template.

Built in 0.22.0 (9ee03ed), and confirmed by the commander:
- A template counts as part of the thing that names it. Its layers sit between the area's layers and the thing's own `index.md`, most distant template first, so the thing's own prose replaces a template's.
- A bare template name is looked up in `templates/` folders from the thing's own folder out to the root, then in the built-ins. A name with a slash is a path from the root. A template's own `inherits:` is resolved from the template's folder. A state file's `inherits: index` still means its bundle's `index.md`.
- A template never passes down `id`, `template:` or `state:`.
- A bundle used as a template passes down its states too, so a template can hold a thing's whole definition, as Bruce asked for global door templates (`locked-door`). Changed by the commander, 2026-11-23, when doors needed it; it reverses the earlier "index only" rule. A thing has the template's states plus its own. Where both have a state of the same name, the template's state file is a layer under the thing's, so the thing overrides only what it changes. The template's starting state applies unless the thing names one.
- A chain of more than 4 templates is a problem.
- Built-ins are Markdown files in `src/builtins/`, compiled in. `scenery` has `title: Scenery` and `object_type: scenery`.

Identical copies (three healing potions) are one thing with a count: `objects: { potion: 3 }`, with play state tracking how many are left. A copy that needs to differ becomes its own thing with its own file. Counts come after templates, when a world needs them.

## Built-in mechanics

Every world gets a set of mechanics from the engine itself (Bruce, 2026-11-23): the basic kinds of thing (room, location, object, character) and a common selection of doors (`door`, `locked-door`, `portal` and similar). They are templates the engine supplies, so a world can use them by name without defining them, and a world template of the same name overrides one.

Settled by the commander for Phase 4 (2026-11-23; Bruce may overrule):
- The kinds of thing (location, object, character, door) are types the engine knows, not templates.
- Built-in templates ship with the engine as ordinary Markdown template files, compiled in, so an author can read one with `thwart show` and copy it to change it. A world never lists the built-ins it uses; naming one is enough.
- A bare template name is looked up along the lineage: first the same bundle, then `templates/` folders from the thing's scope outward to the root, then the built-ins. So `inherits: locked-door` finds the world's own `locked-door` if it has one, and the engine's otherwise.
- Phase 4 ships the mechanism and one built-in, `scenery`, the template a cardless object's new card starts from (`object_type: scenery`, an empty description). The door built-ins (`door`, `locked-door`, `portal`) ship with doors in Phase 6.

## Doors

Agreed by Bruce, 2026-11-23, to be tested in play.

A door is its own type of thing, because doors can't be room states: a room with two doors would need four versions, and with three, eight. A door joins two rooms and has one state seen from both sides.

```yaml
# castle/doors/vault-door.md
id: vault-door
title: Iron door
type: door
inherits: locked-door            # a built-in, or a world template
sides:
  castle/hall:  { direction: down }
  castle/vault: { direction: up }
```

- Each room's exit names the door rather than a room: `connections: { down: vault-door }`. The door says where each side leads, so the two rooms always agree.
- A specific door always has a file, but authors rarely write one. The console creates it when an author links two rooms through a door template. Most doors will come from built-ins, globals or templates.
- The file lives in the nearest scope that contains both rooms: `castle/doors/` for two castle rooms, the root `doors/` between the castle and the forest. The console chooses this.
- Door states are `locked`, `closed` and `open`, a bundle like any other thing's. `passable:` says whether the exit can be used in a state, and `transparent:` whether the other side can be seen through it (opaque is the default). A transparent door, such as a window or a grate, shows a line about the other side in the room view. Unlocking is one of its actions: `unlock: { requires: { has: brass-key }, becomes: { vault-door: closed } }`.
- A one-way door has one side that is never passable (a trapdoor you fall through, a door that locks behind you). To be tested.
- A standard door joins exactly two rooms. A portal is a special type of door that can join more.
- The room view shows the door's current text beside its exits; the room's own prose never changes for a door.
- Trying to go through a door that isn't passable gives the door's `## When: #go` section for its current state, so a closed door and a locked one can say different things; without one, the engine's reply naming the door ("The iron door is locked.").
- Built in 0.33.0 (bbd1e64): doors, portals, the built-in `door` and `portal` (bundles with `closed`, the starting state, and `open`), and the validation rules.

### Adding a door from the console

Decided by Bruce, 2026-11-23. An author never writes a door file by hand; the console's `/add` makes it.
- `/add door west hallway`, or `/add door --(west)--> hallway`, adds a door on the room edit mode is in, leading west to the hallway. `/add door bedroom` adds it to one room only, and `/add hallway door` links a second room to it later, so a door can be built in two steps (a one-sided door is a warning meanwhile, REQ-CF-19).
- The other side is the opposite direction (west to east, up to down). A special case overrides it with a command or by hand.
- The id is predictable: `door-<from>-<to>-<direction>`, such as `door-bedroom-hallway-west`. The engine names it (`door_id`), so every client names doors the same way.
- A room that doesn't exist yet is created, as a new room.
- A new door is a plain `door`. A state hint in brackets picks another template: `/add door(locked) west hallway` makes a `locked-door`.
- The exact command forms are the console's (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-42), including `/add <room> <door>` to give a one-sided door its other side and a bare `/add door` that asks for the direction.

Built in 0.56.0 (66a1b27), with `door_id` from the engine (28692c1).

## Hidden things

Hidden means several things in stories (Bruce, 2026-11-23), and they apply to any thing, not only doors. A state's `visibility:` says which (agreed by Bruce, 2026-11-23):

| `visibility:` | In the story | In play |
|---|---|---|
| `visible` (default) | plainly there | listed in the room |
| `faint` | hard to see: a faint outline, a shadow | not listed; found by examining the room or searching |
| `hidden` | properly invisible | not listed or found by looking; an action reveals it |
| `absent` | not there at all until something happens | can't be named or examined; only an action elsewhere brings it into being |

Built in 0.23.0 (a2e45a9), and confirmed by the commander:
- A faint thing is found by `search`, or by examining the room itself. Once found it stays found for the game, even if it later changes state, and it can't be named before it's found, so a player can't guess "take key".
- An absent thing can't be named by anything.
- Visibility covers objects, characters and exits (an exit by the visibility of the room it leads to). Things the player carries are always shown. Doors join in Phase 6.
- A hidden thing is never listed or found by search. An action that names it exactly still works (a secret lever, a magic word) if that action has no prerequisite (`requires:`) and the thing is of a type that can be found; otherwise it behaves as absent until something reveals it (Bruce, 2026-11-23). Which types can be found is to settle with the built-ins: *proposed*, objects, doors and exits can, characters can't unless they say so.
- Search replies are built in ("You search carefully and find the brass key and a way north."), and a room's `## When: #searched` section leads them. The built-in wording moves to `[engine]` in Phase 8 (`design/ENGINE_TEXT.md`).

Revealing is ordinary state switching: an action `becomes:` a state with a different `visibility:`. Special activation, such as magic, is a `requires:` on that action (`requires: { has: wand }`).

## Ambiguity

When a command could mean more than one thing ("open door" with two doors), the reply is part of the story, not a fixed system message (Bruce, 2026-11-23): "Which door do you want to open?", "You stare at both doors, not sure which to open." The engine has a set of such replies and varies them. *Proposed*: an author can override them per world, and per thing for a thing with something to say.

Built in 0.24.0 (79af69c): the reply is the `## When: #ambiguous` section of the nearest template every candidate shares, with its variables filled in, as Bruce decided. Otherwise one of the built-in replies, which always name the candidates in one question so the player can still choose ("Which key do you want to take, the brass key or the iron key?"; for people, "Who do you want to examine, King Aldric or Queen Aldric?"). A world-wide override comes with Phase 8's `[engine]` table (`design/ENGINE_TEXT.md`).

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
---
id: lever
title: Iron lever
type: object
actions:
  pull: { becomes: { castle/vault: open } }
---
A rusted iron lever juts from the wall.

## When: #pull
Gears grind somewhere below.
```

## Responses

What a player reads when they act is prose in the body, never frontmatter (principle 8; agreed by Bruce, 2026-11-23). It's written as `## When:` reply sections; the full model is `design/SECTIONS.md` › The model and › Outcomes and replies.

- A reply section names the action and, where it matters, the outcome: `## When: #pull`, `## When: #unlock #wrong-key`. Write the reply where the thing ends up, and if nothing changes, name why.
- A reply section can stand alone, with no mechanics in frontmatter: `## When: #kick` holding "Ow." is a whole action.
- An action with no reply that matches gets "Nothing happens." (REQ-PC-14). A failed `requires:` gets a built-in refusal. Both become `[engine]` text in Phase 8 (`design/ENGINE_TEXT.md`).
- Ambiguity replies an author overrides are sections too: `## When: #ambiguous`.
- In the editor, keyword headings are drawn as rules (REQ-CON-39).

## Variables

Prose can include variables, written `{{ name }}`. Every name starts with the level it belongs to, so the first word says where to look (Bruce, 2026-11-23):

| Level | Prefix | Examples | Declared in |
|---|---|---|---|
| Game | `game.` | `game.turn`, `game.score` | the engine; fixed |
| Story | `story.` | `story.chapter`, `story.king` | `world/`, with starting values |
| Player | `player.` | `player.name`, `player.health` | `world/` |
| Current room | `location.` | `location.title`, `location.visits` | the engine and the room |
| A thing | `<id>.` | `box.lid`, `knife.sharp` | the thing's `vars:` |
| This thing | `this.` | `this.lid` | the thing's `vars:` |

Built in 0.21.0 (0ebc0e4), and confirmed by the commander:
- The engine supplies `game.turn`, `game.score` (0 until set), `story.chapter` (the chapter's name, or blank), `player.title` ("you" with no `player.md`), `location.title`, `location.visits`, and `<id>.title` for every thing.
- `location.` means the player's room in play. When checking declarations, it means the room the prose is in, or, for prose outside a room, any room.
- A thing's value is the one set in play if there is one, else its current version's `vars:` (state and chapter applied). Values hold once set.
- `sets:` on an undeclared name warns, like an undeclared variable. `this.` in `sets:` means the thing the action belongs to.

A thing separates its values from its abilities:

```yaml
# objects/knife.md
vars: { sharp: yes }    # values; can change in play
can: [cut]              # abilities; checked by actions
```

A variable that isn't declared at its level is a warning, not an error: edit mode shows it in place, marked, with the warning beside it, and `validate` lists it. In play it shows as blank, never as raw `{{ }}` (Bruce, 2026-11-23). Author comments (`<!-- -->`) show in edit mode and are removed in play.

## Console

In files, a bundle is separate files. In the console, it is one document with a section per state, written the way `design/SECTIONS.md` writes states in a single file (Bruce, 2026-11-23; heading format from the same day):

````
/edit room  castle/vault                         ctrl-s save · esc close

  ---
  title: Vault
  state: closed
  connections: { north: castle/hall }
  ---
  A heavy iron door is set into the floor.

  ─── When: #closed (starting) ··· castle/vault/closed.md
  Shut fast.

  ─── When: #open ··· castle/vault/open.md
  ```thwarter
  connections: { down: castle/vault-stairs }
  ```
  The iron door stands open. Cold air rises from below.
````

- The index is the default section: its frontmatter and prose, with no heading.
- Each state is a `## When: #name` section. A state that's its own file shows its path beside the heading; its frontmatter, apart from `inherits: index`, is shown as its `thwarter` block. In the editor, the heading is drawn as a rule in the chrome colour (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-39).
- Where a bundle has to be one piece of plain text (copying it out, or a transcript test), the headings are written as ordinary `## When: #name` headings, so the text is the same document a one-file thing would be.
- Saving writes each section back where it came from: a section from a state file to that file, and a section written in the index to the index. Typing a new `## When:` heading adds a section to the index, which shows when its tags match and never creates a state (`design/SECTIONS.md` › States); `/state new` makes a state file instead. A file-backed state's heading can't be edited, since renaming or deleting that state moves a file: that's `/state rename` and `/state delete`. (Agreed with the console coder, 2026-11-23.)
- Sections appear in this order: the default section, the starting state, the other states A to Z, then chapter overlays in chapter order. How a chapter overlay is headed waits on `design/SECTIONS.md` › Open question 2 (chapters as sections or files); until then it keeps its `<!-- thwarter:section chapter <name> -->` line.

Saving splits the document back into its files. Every section must parse before any file is written; if one fails, nothing is saved. A file that doesn't parse still opens, since editing is how an author fixes it. A one-file thing opens as a document with one section.

Full editing is a full-screen editor in the terminal (Bruce, 2026-11-23; `design/CONSOLE_REQUIREMENTS.md` › REQ-CON-6), since a document with sections can't be edited inline.

Commands the console will need, *proposed*: `/state new <name>`, `/state rename <old> <new>`, `/state delete <name>`, `/state set <name>` (the starting state), `/chapter new <number> <name>` at the root, and `/chapter overlay` inside a thing. Turning a one-file thing into a bundle (`lantern.md` → `lantern/index.md`) is a file move, which the writer refuses today; these commands are where it gets built.

## Keys and clocks (Phase 6)

Keys are agreed with Bruce (2026-11-23) and built in f84f5d5 (0.48.0), worked through as the vault door example below; it uses `design/SECTIONS.md`'s sections, scripts and outcomes. Clocks are still the commander's proposal, not yet reviewed.

### Keys

A lock is a reason a door won't open, and the reason varies: a key, a deadbolt, a stiff handle. So a key may or may not be needed (Bruce). The built-in `locked-door` has a script that decides; each door says which key, if any, opens it.

- `unlock door` works only if the player provides the key: `unlock door with key`. The engine never searches the player's pocket. The door's reply MAY ask or suggest ("Unlock it with what?"). *Proposed*: after a reply that asks, a bare noun on the next line is taken as the `with`.
- A key used on a door stays in the lock until the player takes it back (Bruce). Then the door holds it, and can see it: a script reaches only what its thing owns.
- `vars: { key: "" }` means no key is needed, for a deadbolt or a latch.
- `validate` warns about a locked door whose `key` names nothing.

The built-in, as a bundle:

```
locked-door/
  index.md        state: locked, vars: { key: "" }
  locked.md       passable: no, and replies for ending up or staying locked
  closed.md       passable: no, and replies for ending up closed
  open.md         passable: yes, and the open door's text
  actions.rhai    unlock, lock, open, close
```

`actions.rhai`, as built in f84f5d5 (0.48.0, REQ-ENG-38 to 41): each function returns a new state, a reason, or a list of changes. The thing used on the door is `turn.used`, since Rhai reserves `with` (and `this`, so a parameter can't be called that).

```rust
fn unlock(door, turn) {
    if door.state != "locked" { return "no-action"; }
    if door.key == "" { return become("closed"); }      // a deadbolt: no key
    if turn.used == () { return "no-key"; }
    if turn.used.id != door.key { return "wrong-key"; }
    [keep(turn.used), become("closed")]                 // the key stays in the lock
}

fn lock(door, turn) {
    if door.state != "closed" { return "no-action"; }
    if door.key == "" || door.holds(door.key) { return become("locked"); }
    if turn.used == () { return "no-key"; }
    if turn.used.id != door.key { return "wrong-key"; }
    [keep(turn.used), become("locked")]
}

fn open(door, turn)  { if door.state == "closed" { become("open") } }
fn close(door, turn) { if door.state == "open" { become("closed") } }
```

`locked.md`:

```markdown
---
inherits: index
passable: no
---
## When: #lock
The lock clicks shut.

## When: #unlock #no-key
It's locked. Unlock it with what?

## When: #unlock #wrong-key
That doesn't fit the lock.

## When: #open #no-action
It's locked.
```

A locked exit needs no section: going through a door that isn't passable gets the engine's reply naming it, "The iron door is locked."

`closed.md`:

```markdown
---
inherits: index
passable: no
---
## When: #unlock
The lock falls open.

## When: #close
You pull the door shut.

## When: #unlock #no-action
It isn't locked.

## When: #close #no-action
It's already shut.
```

`open.md`:

```markdown
---
inherits: index
passable: yes
---
## When: #open
It swings open.

## When: #open #no-action
It's already open.
```

A specific door names its key and tells its own story. Its replies replace the built-in's for the same action and outcome:

```markdown
---
id: vault-door
title: Iron door
type: door
inherits: locked-door
vars: { key: brass-key }
sides:
  castle/hall:  { direction: down }
  castle/vault: { direction: up }
---
A heavy iron door, set into the floor.

## When: #locked
A padlock hangs from the hasp.

## When: #unlock #closed
The padlock snaps open with a rusty clunk. The key stays in it.

## When: #open
Cold air rises from the dark below.
```

In play:

| Typed | Outcome | Reply |
|---|---|---|
| `unlock door` | `no-key` | It's locked. Unlock it with what? |
| `unlock door with spoon` | `wrong-key` | That doesn't fit the lock. |
| `unlock door with key` | `closed`; the key stays in the lock | The padlock snaps open with a rusty clunk. The key stays in it. |
| `open door` | `open` | Cold air rises from the dark below. |
| `take key` | the key comes back to the player | You take the brass key. |
| `down` | engine: `passable: yes` | The vault's description |

### One thing affecting another

A lever in the hall that opens the vault door can't reach the door by ownership. A connection between the two, in the lever's (or the door's) frontmatter or body, gives it that reach (Bruce). Frontmatter `connects: [vault-door]` in either thing is enough, and a script changes a connected thing by naming it; a `change()` to an unconnected thing is refused with an author note (built in f84f5d5):

```rust
fn pull(lever, turn) {
    if lever.state == "up" { [become("down"), change("vault-door", "open")] }
}
```

Whether a mention in the body (`@vault-door`) also connects is still open (`design/SECTIONS.md` › Open questions).

### Built-in verbs

`take`, `go`, `look` and the other built-in verbs stay the engine's, and no script replaces them (Bruce). Every action, built-in or not, adds its tags to the turn, and each turn shows all the content that matches the request, the location, what the player holds and the rest (Bruce). So `take key` MAY have a `## When: #take` reply in the key's file; where one exists, it replaces the built-in reply. Where none exists, the built-in reply stands.


### Clocks

Time is counted in turns, which the engine already keeps (`game.turn`). Everything else is built on states and values, with no new condition language.

- **Changes over time.** A state MAY say `after: { turns: 20, becomes: stale }`: 20 turns after a thing enters that state, it switches. The count is per thing, from when it entered the state, so bread bought late goes stale late.
- **A range** (`turns: 15-25`) picks a point in it, the first source of randomness.
- **Game time.** `world/index.md` MAY set `start_time: "08:00"` and `minutes_per_turn: 5`. The engine then supplies `game.time` and `game.day` from the turn count. A world that sets nothing has turns only.
- **Weather** is an ordinary bundle in `world/` with states and `after:`, so it changes the way bread spoils:

```yaml
# world/weather/clear.md
inherits: index
after: { turns: 10-30, becomes: rain }
---
The sky is clear.
```

  Rooms show it with `{{ weather.title }}` or a link, and actions can check it once `requires:` needs a state check.
- **Randomness in prose.** A response MAY have several blocks with the same name, and play picks one each time, so "Ow." doesn't repeat word for word. The built-in replies already vary this way.

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

1. **Clocks and game values**: proposed above (Keys and clocks), to review with Bruce. Phase 6.
2. **Showing play state**: it MUST be visible in a sidebar in the console and be queryable (Bruce, 2026-11-23). The layout and the query commands are console work for `design/CONSOLE_REQUIREMENTS.md`. Phase 5.
3. **Keys for built-in locked doors**: settled with Bruce, 2026-11-23 (Keys and clocks › Keys).
