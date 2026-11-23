# States, bundles and chapters

Status: Draft, from a design discussion between Bruce and the commander on 2026-11-23. Nothing here is built. Items marked *proposed* are the commander's suggestions that Bruce hasn't confirmed yet; everything else Bruce has agreed to.

## Summary

A room, object or character is either one Markdown file or a folder of them (a bundle). A bundle holds versions of the thing: its states (bread that is fresh or stale) and its changes over the story (chapter overlays). Major changes are separate files; minor ones are variables in the prose. An author never writes conditions into prose, and rarely writes them anywhere.

This replaces the earlier idea of conditional text blocks (`{{< when >}}` shortcodes) and conditions on exits. The one condition that survives is `requires:` on an action.

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

At the root, a chapter defines the chapter itself and anything that changes for the whole world:

```
chapters/
  1-the-summons/_index.md
  2-after-the-king/_index.md
```

```yaml
# chapters/2-after-the-king/_index.md
title: After the king
sets: { story.king: dead }          # story values on entering the chapter
---
<!-- Arc notes: the court splits; the guard captain takes charge. -->
```

The number prefix orders the chapters; the rest is the name. A chapter's `_index.md` can exist before any overlay does, as a placeholder for the arc.

Inside a thing, a chapter is an overlay for that thing:

```
castle/courtyard/
  index.md
  chapters/
    2-after-the-king.md      # the courtyard in chapter 2
npcs/guard/
  index.md
  friendly.md
  hostile.md
  chapters/
    2-after-the-king.md      # the guard in chapter 2
```

An overlay's file name MUST match a root chapter. A thing that's one file becomes a bundle when it gets its first state or overlay; the console does that move (see Console).

### What applies, in order

A thing's current version is built in layers, each overriding only what it states (order agreed by Bruce, 2026-11-23):

1. `index.md`
2. the current state's file, if the thing has states
3. chapter overlays: root chapters first, then the thing's own. *Proposed*: every chapter up to and including the current one, in chapter order, so a change made in chapter 2 still holds in chapter 3 unless chapter 3 changes it again

Frontmatter merges by the inheritance rules below. Prose replaces the layer below only if the layer has prose.

## Inheritance

Inheritance is never implicit (Bruce, 2026-11-23). A file inherits only if its frontmatter says `inherits:`.

- Inside a bundle, a state file says `inherits: index`. The console writes this when it creates a state.
- Across things, `inherits: objects/box` makes a thing start from another thing, as a template.
- A file has one parent. Cycles are an error. Chains are kept short (*proposed*: at most 4 levels).
- A scalar or a list in the child replaces the parent's. A map (`connections`, `vars`) merges key by key. `null` removes an inherited key.
- Overlays inherit from whatever version is current, by definition, and don't need `inherits:`.
- The console always shows the result and where each part came from (`north: castle/hall (from index)`). Nothing inherited is hidden.

## Changing state

How states switch is mostly invisible to the author, who works in the console (Bruce, 2026-11-23). Underneath:

- An action sets a thing's state directly: `becomes: { castle/vault: open }`.
- An action can need something first: `requires: { has: brass-key }`. This is the only remaining condition, a small YAML map.
- An action moves the story on: `chapter: 2-after-the-king`. Every overlay for that chapter then applies, with no list to keep up to date.
- A door and its room: the action changes both, by setting each one's state (option (a), Bruce leaning that way, to be tested). The alternative, an exit that goes `through:` a door object, stays open until testing.
- Time-based change (bread going stale after 20 turns) needs a clock, which doesn't exist yet. *Proposed* for later: `after: { turns: 20, becomes: stale }` on a state.

```yaml
# objects/lever.md
actions:
  - pull:
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

A variable that isn't declared at its level is a `validate` error. In play, a value that can't be resolved shows as blank, never as raw `{{ }}`. Author comments (`<!-- -->`) show in edit mode and are removed in play.

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

  --- chapter: 2-after-the-king
  ---
  The door hangs off one hinge now.
```

Saving splits the document back into its files. Every section must parse before any file is written; if one fails, nothing is saved. A one-file thing opens as a document with one section.

Full editing is a full-screen editor in the terminal (Bruce, 2026-11-23; `design/CONSOLE_REQUIREMENTS.md` › REQ-CON-6), since a document with sections can't be edited inline.

Commands the console will need, *proposed*: `/state new <name>`, `/state set <name>` (the starting state), `/chapter new <n-name>` at the root, and `/chapter overlay` inside a thing. Turning a one-file thing into a bundle (`lantern.md` → `lantern/index.md`) is a file move, which the writer refuses today; these commands are where it gets built.

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

## Open questions

1. **Overlay order** (*proposed* above): do chapter overlays stack, so chapter 2's change holds in chapter 3, or does only the current chapter's overlay apply?
2. **Areas**: can an area (`castle/`) have chapter overlays that apply to every room in it, for example the whole castle in mourning?
3. **The door**: option (a), setting both states from the action, to be tested against option (b), an exit through a door object.
4. **Clocks**: when do time-based changes come in, and is it turns or game time?
