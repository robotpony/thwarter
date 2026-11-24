# Sections: content shown by game state

Status: Agreed in outline by Bruce, 2026-11-23, to be tested on real content. Bruce set the principles and the model, and answered the open points through the console coder's session and directly; the commander wrote it up and filled the gaps, marked *proposed*. Nothing here is built in the engine yet (the editor draws the headings, f959c28). Once the trial confirms it, this folds into `design/CONTENT_REQUIREMENTS.md` and `design/STATES.md`, and this file goes in `CHANGELOG.md` › Removed.

## Summary

Content never tests anything. The game has a state, and content is shown when it matches that state (Bruce). A section under a keyword heading (`## When: #hit #window`) is shown when every tag in its heading is true right now; otherwise it isn't, and if nothing matches an action, nothing happens. A heading never creates a state. States live in bundles, as today. Markdown comments go back to being comments, so every `<!-- thwarter:... -->` marker goes. Characters are written `@guard-captain`, tags `#locked`, and places as links.

## Why

The marker comments read badly to a person editing a file, and they hide structure inside something every Markdown tool treats as invisible. Bruce wants files that "read nicely to human editors", in Obsidian or any editor, with the structure visible as structure.

## Bruce's principles

1. `<!-- -->` is a comment, never syntax.
2. The default document is the thing itself. Variations are sections in it, properly marked, or extra documents in its bundle. Many things will be one document.
3. Actions aren't caught by content, only by state. The engine runs actions, world scripts and data, and those change state; content is shown when it matches.
4. A thing's state starts as its frontmatter says (`state: closed`). States set limits and say what can happen, such as whether a door can be opened (locked, broken, disabled, unlocked).
5. Headings don't create state. A section is displayed if the state it names is true: it exists, and it matches.

The syntax a file may use is Markdown's own: inline `code`, fenced blocks with a type, headings and sections, and the `@` and `#` prefixes.

## The model

Bruce's example: the player is in a bedroom, holding a stick, and hits the window with it. At that moment the game state includes:

- the action, `#hit`, and its target, `#window`
- what the player holds, `#stick`
- where the player is, the bedroom
- each thing's current state, such as the window being `#closed`
- the chapter

The engine looks for sections that match. If the window or the bedroom has a section for hit and window, it's shown. Otherwise nothing happens.

```markdown
<!-- bedroom/window.md -->
---
id: window
title: Window
type: object
state: closed
---
A small window over the bed, its latch painted shut.

## When: #hit #stick
The glass cracks from corner to corner.
```

Only the tags in a heading are tested, and all of them must be true. *Proposed*: no `or` and no other operators, so principle 7 (a small condition language) holds. `## Unless:` is the same with every tag false.

### Two kinds of section

*Proposed*, so a room's state text doesn't repeat on every action:
- **A description section** names no action: `## When: #locked`. It's added to the thing's description whenever it matches, which is when the player looks, examines or enters.
- **A reply section** names an action: `## When: #hit #stick`. It's shown only on the turn that action happens, as the reply. Nothing else shows it.

A heading's tags are read against whose file it's in: in the window's file, the window is the target, so `#window` isn't needed; in the bedroom's file, the bedroom is where the player is, so the same reply is `## When: #hit #window #stick`.

### Which files answer an action

*Proposed*: for an action, the engine looks for matching reply sections in the target, then in what the action uses (the stick), then in the room, and shows every match in that order. If none matches, the reply is a built-in "Nothing happens." and the turn changes nothing.

### Changing state

A reply section MAY hold a ` ```thwarter ` block whose mechanics run when it's shown:

````markdown
## When: #hit #stick
```thwarter
becomes: { this: broken }
```
The glass shatters. Cold air pours in.
````

*Proposed*: this replaces the frontmatter `actions:` map. A world's verbs are the action tags its headings use, which is what the parser's REQ-PC-14 check already asks ("does any thing have this verb?"). `requires:` becomes tags in the heading: `## When: #unlock #brass-key` works only while the player holds the key, and `## When: #unlock` `## Unless: #brass-key` is the refusal. The `actions:` map and `requires:` then go.

A description section's block MAY hold data that applies while it matches, such as a door's `passable:`, merging over frontmatter the way layers merge today (REQ-CF-13). *Proposed*: one block per section, first in it.

### States

States come from bundles (Bruce: "bundles are simpler"). A state exists because its file does (`objects/window/broken.md`), or because frontmatter's `state:` or a `becomes:` names it. A heading only reads states; `## When: #broken` is shown while the window is broken, and never makes `broken` exist. `validate` warns about a heading tag that can never be true: no state, action, thing or chapter has that name, so `## When: #lockd` is a warning.

### Prefixes, links and names

- `@id` names a character, or another thing with a voice, in prose (Bruce). In play it shows as the character's name.
- `#name` is a tag. In a heading it's read against the game state; in prose it's an ordinary tag (REQ-CF-30).
- `#` and `@` share one namespace, with a limit (Bruce): `#` can't reach a character's properties, and `@` can't reach properties only objects have. Object, character and door ids are already unique across the world (REQ-CF-22), so a name means one thing whichever prefix reaches it.
- `[[ ]]` and `[]()` links are for locations (Bruce). *Proposed*: a link still resolves to any kind of thing, as REQ-CF-29 says, since Obsidian needs that; the convention is links for places and `@` for characters.

### Hugo

A keyword heading shows as an ordinary heading ("When: #locked") if Hugo renders a world as a site, and that's acceptable (Bruce). A site that wants them styled or hidden hooks headings with a `render-heading.html`, as Not a Chef's themes do.

## Before and after

The test world's treasure chest today ends with three marker blocks:

```markdown
An ornate wooden chest bound with iron and secured with multiple locks. ...

<!-- thwarter:action examine -->
The craftsmanship rivals the finest furniture.

<!-- thwarter:action attempt-open -->
The locks hold firm against your efforts.

<!-- thwarter:action listen -->
You hear the faint sound of coins shifting inside.
```

Under this format, `examine` is the description itself, and the other two are replies:

```markdown
An ornate wooden chest bound with iron and secured with multiple locks. ...
The craftsmanship rivals the finest furniture.

## When: #open #locked
The locks hold firm against your efforts.

## When: #listen
You hear the faint sound of coins shifting inside.
```

## What changes

- REQ-CF-6 and REQ-CF-17 (`actions:` and response markers), REQ-CF-28 (headings are prose) and REQ-CF-29 (links) change; new requirements cover keyword headings, the game-state match, `thwarter` blocks and `@`. The commander writes them once the trial confirms the format.
- Principle 7 changes from "only `requires: { has: ... }`" to "only all-of and none-of tags in a heading".
- The engine builds each turn's tags and matches sections (coder). The parser needs `with`: `hit window with stick` names the target and what's used.
- The editor draws `When:` and `Unless:` headings as rules, and colours `@`, `#` and `thwarter` blocks; built in f959c28 (console coder).
- No conversion (Bruce): the project is early, so the test world's nine files with markers and the `door` and `portal` built-ins are rewritten, and the loader stops reading markers. A leftover marker is an ordinary comment.

## Open questions

Bruce asked for more detail on three of the earlier questions (2026-11-23). They're restated here with the model above.

1. **Which tags can a heading test?** Bruce's example uses the action, its target and what the player holds. Could a heading also test another thing's state (a bedroom line while the window is broken, `## When: #window.broken`), the chapter (`## When: #after-the-king`), a value (`#story.king.dead`) or the time of day? Each one makes headings more powerful and harder to predict. *Proposed* for the trial: actions, targets, held things, this thing's own states and the chapter.
2. **Do chapters become sections?** Today a thing changes for a chapter through a file in its `chapters/` folder (`castle/courtyard/chapters/after-the-king.md`). If the chapter is a tag a heading can test (question 1), a small chapter change could be a section in the thing's own file instead: `## When: #after-the-king` with "Black banners hang from the walls." Then `chapters/` files would be only for big changes, as state files are. Should both work, or only files?
3. **Does a section add to the description, or replace it?** Under "added when it matches", a door that reads "The door is shut." by default and has `## When: #open` "The door stands open." reads "The door is shut. The door stands open." while open. Options:
   - **a.** Sections always add. The default text holds only what's always true ("A heavy iron door, set into the floor."), and every state-dependent line goes in a section. Simple to predict; the author has to write that way.
   - **b.** A section can replace the default by saying so in its block (`replaces: description`). More flexible; one more rule.
   - **c.** Sections add, and a state file in the bundle replaces, as state files do today. Major changes replace, minor ones add.
   *Proposed*: **a** for the trial, since it matches "shown if it's true".
4. **What happens on an action no section answers?** Nothing happens, and the turn changes nothing (Bruce). *Proposed*: the reply is a built-in "Nothing happens.", varied, which a world MAY replace with a `## When: #nothing` reply in a room or template. Is a silent turn better?
5. **A name that's both an action and a state.** A door has an `open` state and players type `open door`. Is `## When: #open` the description while the door is open, or the reply to opening it? With one namespace the heading can't say. Options:
   - **a.** Actions are written as a bare word before the tags: `## When: open #closed` is the reply to opening a closed door, and `## When: #open` is the open door's description. It also reads well: "When: hit #stick".
   - **b.** A state can't share a name with an action, and `validate` reports it. Doors would need states like `shut` and `ajar`.
   *Proposed*: **a**. The examples above use `#hit` until this is settled.
