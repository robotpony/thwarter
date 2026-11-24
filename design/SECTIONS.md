# Sections: states written as headings

Status: Agreed in outline by Bruce, 2026-11-23, to be tested on real content. Bruce set the principles and answered seven questions through the console coder's session; the commander wrote it up and filled the gaps, marked *proposed*. Nothing here is built yet. Once the trial confirms it, this folds into `design/CONTENT_REQUIREMENTS.md` and `design/STATES.md`, and this file goes in `CHANGELOG.md` › Removed.

## Summary

A thing is one readable document. Its plain prose is the thing itself. Its states are sections under keyword headings (`## When: #locked`), each with its own prose and an optional `thwarter` code block holding the data that changes in that state. A state's section is also what the player reads when an action moves the thing into it. Markdown comments go back to being comments, so every `<!-- thwarter:... -->` marker goes. Characters are written `@guard-captain`, states and tags `#locked`, and places as links. Bundles still work: a `## When: #locked` section and a bundle's `locked.md` are the same state, written two ways.

## Why

The marker comments read badly to a person editing a file, and they hide structure inside something every Markdown tool treats as invisible. Bruce wants files that "read nicely to human editors", in Obsidian or any editor, with the structure visible as structure.

## Bruce's principles

1. `<!-- -->` is a comment, never syntax.
2. The default document is the thing itself. Variations are sections in it, properly marked, or extra documents (a bundle folder, or files with similar names). Many things will be one document.
3. Actions aren't caught by content, only by state: a state must change before an action does anything. The engine runs actions, world scripts and data, and those change state.
4. A thing's state starts as its frontmatter says (`state: closed`). States set limits and say what can happen, such as whether a door can be opened (locked, broken, disabled, unlocked).

The syntax a file may use is Markdown's own: inline `code`, fenced blocks with a type, headings and sections, and the `@` and `#` prefixes.

## The format

A door, with its states as sections:

````markdown
---
id: vault-door
title: Iron door
type: door
state: locked
sides:
  castle/hall:  { direction: down }
  castle/vault: { direction: up }
---
A heavy iron door, set into the floor.

## When: #locked
```thwarter
passable: no
actions:
  unlock: { requires: { has: brass-key }, becomes: { this: closed } }
```
A padlock hangs from the hasp. @guard-captain has the key.

## When: #closed
```thwarter
passable: no
actions:
  open: { becomes: { this: open } }
```
The padlock lies on the floor beside it.

## When: #open
```thwarter
passable: yes
```
The door stands open. Cold air rises from below.
````

`unlock` exists only while the door is locked, and `open` only while it's closed: the states say what can happen (principle 4).

### Keyword headings

- A heading whose text starts with `When:` or `Unless:` is a conditional section (Bruce). Any other heading is prose, as REQ-CF-28 says today.
- *Proposed*: only level-2 headings (`##`) are keyword headings, and the keywords are matched exactly, capital included. A section runs to the next level-1 or level-2 heading, so a `###` heading inside a section is prose within it. Text before the first keyword heading is the default section.
- `## When: #locked` holds while this thing is in state `locked`. `## Unless: #locked` holds while it isn't.
- *Proposed*: several tags in one heading must all hold (`## When: #open #night`). No `or` and no other operators, so principle 7 (a small condition language) still holds.

### What a section adds

- The default section's prose is always shown, and each section that holds adds its prose after it, in file order (Bruce's door example). So the locked door reads "A heavy iron door, set into the floor. A padlock hangs from the hasp."
- Data defaults to frontmatter (Bruce). A ` ```thwarter ` block in a section that holds adds to it, merging the way layers merge today (REQ-CF-13): maps key by key, lists and scalars replace. *Proposed*: one block per section, first in it; a block in the default section is a warning, since frontmatter is the place for it.
- A block's content is YAML with the keys frontmatter takes, except `id`, `type` and `state:`, which a state can't change (REQ-CF-10).
- Variables (`{{ this.key }}`) work in section prose as in any prose.

### Sections and state files

A `## When: #locked` section and a bundle's `locked.md` are the same thing, written two ways (Bruce). The loader builds the same state from either. A thing MUST NOT define one state both ways; `validate` reports it.

This changes one rule for state files. Today a state file's prose replaces the description below it (REQ-CF-13); for the two forms to be the same, a state file's prose adds to the default description, as a section's does. *Proposed*: a state that needs to replace the description entirely says so in its data (`replaces: description`), in either form.

The console's one-document view of a bundle (`design/STATES.md` › Console) uses `## When:` headings for its state sections, instead of `<!-- thwarter:section -->` comments.

### Replies

Action responses are `## When:` sections, replacing the `<!-- thwarter:action -->` markers and the `ambiguous` and `search` reply markers (Bruce). Since actions work through state (principle 3), the reply to an action is the prose of the state it moves the thing into: opening the door shows the `#open` section, "The door stands open. Cold air rises from below." The same text then describes the door for as long as it stays open.

*Proposed* for the rest, to try:
- **A refused action** (its `requires:` isn't met) shows the current state's section, which says why: trying to unlock without the key shows the padlock. With no such section, a built-in reply ("The iron door won't open.").
- **An action with no prose** in the state it reaches gets the built-in reply made from the verb and the title ("You pull the iron lever."), as today.
- **Engine moments** that aren't states use reserved tags: `## When: #ambiguous` replaces `reply ambiguous` (in a template shared by the candidates, REQ-ENG-26), and `## When: #searched` replaces `reply search` (in a room). These are the only reserved tags, and `validate` warns about a thing that uses one as a state name.
- **An action that changes nothing** isn't an action under principle 3. A line like `listen` → "You hear coins shifting inside." becomes part of a description, or an action that moves the thing to a state (`listened`) with that prose.

### Prefixes, links and names

- `@id` names a character, or another thing with a voice, in prose (Bruce). In play it shows as the character's name.
- `#name` is a tag, and in a keyword heading it's a state of this thing (Bruce).
- `#` and `@` share one namespace, with a limit (Bruce): `#` can't reach a character's properties, and `@` can't reach properties only objects have. Since object, character and door ids are already unique across the world (REQ-CF-22), a name means one thing whichever prefix reaches it. *Proposed* reading of the limit: a thing's state is also one of its tags in play (search for `#locked` finds what's locked now), `#guard-captain` names the character but `#guard-captain.mood` is an error, and `@lantern.lit` is one too.
- `[[ ]]` and `[]()` links are for locations (Bruce). *Proposed*: a link still resolves to any kind of thing, as REQ-CF-29 says, since Obsidian needs that; the convention is links for places and `@` for characters.

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

Under this format, `examine` is the description itself, and the locked chest is a state:

````markdown
An ornate wooden chest bound with iron and secured with multiple locks. ...
The craftsmanship rivals the finest furniture.

## When: #locked
```thwarter
actions:
  unlock: { requires: { has: chest-key }, becomes: { this: closed } }
```
The locks hold firm. Something shifts inside when you nudge it: coins, by the sound.

## When: #closed
```thwarter
actions:
  open: { becomes: { this: open } }
```
The lid sits loose on its hinges now.
````

## Built-ins and keys

The built-in `door` becomes one file instead of a bundle of three:

````markdown
---
title: Door
type: door
state: closed
transparent: no
---
## When: #closed
```thwarter
passable: no
actions:
  open: { becomes: { this: open } }
```

## When: #open
```thwarter
passable: yes
actions:
  close: { becomes: { this: closed } }
```
````

The `locked-door` built-in follows the vault door above with `has: this.key` and `vars: { key: "" }`, so the key proposal in `design/STATES.md` › Keys still fits.

## What changes

- REQ-CF-13 (state prose adds), REQ-CF-17 (response markers), REQ-CF-28 (headings are prose) and REQ-CF-29 (links) change; new requirements cover keyword headings, `thwarter` blocks and `@`. The commander writes them once the trial confirms the format.
- Principle 4 gains sections ("major changes are files, minor ones are sections or values"), and principle 3 ("predictable from the tree") then means the tree and the headings.
- The loader reads sections and blocks, and the parser's marker code goes (coder).
- The editor draws `When:` and `Unless:` headings as separators, and colours `@`, `#` and `thwarter` blocks (console coder).
- No conversion (Bruce): the project is early, so the test world's nine files with markers and the `door` and `portal` built-ins are rewritten in the new form, and the loader stops reading markers. A leftover marker is an ordinary comment.

## Open questions

1. **Can a section test another thing's state, or a chapter?** For example a room text while the vault door is open (`## When: #vault-door.open`), or in a chapter (`## When: #after-the-king`). `design/STATES.md` decided rooms don't vary by their doors; this would reopen it. *Proposed*: this thing's states only, for the trial.
2. **Chapters as sections?** A chapter overlay could be a section too, but chapters apply by placement in `chapters/` folders today. *Proposed*: chapters stay files for the trial.
3. **How are states declared?** Today a state exists because its file does. With sections, a typo (`## When: #lockd`) makes a state nobody enters. *Proposed*: a state exists if it has a file or a section, and `validate` warns about any state that no `state:` or `becomes:` reaches.
4. **Files with similar names.** Bruce mentioned extra documents with similar names as well as subfolders. Does `vault-door.open.md` beside `vault-door.md` mean a state? *Proposed*: bundle folders only for the trial, since a dotted name is easy to make by accident.
5. **Hugo.** A keyword heading shows as a heading ("When: #locked") if Hugo renders the world as a site (principle 9). Acceptable, or should a Hugo layout hide them?
6. **Replacing prose.** Is `replaces: description` the right way for a state to replace the default text, or should a state file keep replacing by default and only sections add?
