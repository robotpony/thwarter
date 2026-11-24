# Sections: content shown by game state

Status: Agreed in outline by Bruce, 2026-11-23, to be tested on real content. Bruce set the principles and the model, and answered the open points through the console coder's session and directly; the commander wrote it up and filled the gaps, marked *proposed*. Reading sections and matching them against each turn's game state are built (2dad20b and ce4c62e, 0.46.0, REQ-ENG-35 and 36), and the editor draws them (f959c28). Not built: tags on other things' states and values, mechanics in `thwarter` blocks, default-reply lists, and `@name` shown as a name in prose. Once the trial confirms it, this folds into `design/CONTENT_REQUIREMENTS.md` and `design/STATES.md`, and this file goes in `CHANGELOG.md` › Removed.

## Summary

Content never tests anything. The game has a state, and content is shown when it matches that state (Bruce). A section under a keyword heading (`## When: #hit @window`) is shown when every name in its heading is true right now; otherwise it isn't, and if nothing matches an action, nothing happens. A heading never creates a state. States live in bundles, as today. Markdown comments go back to being comments, so every `<!-- thwarter:... -->` marker goes. `#` names states (`#locked`) and `@` names things (`@guard-captain`, `@stick`), and places are links.

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

- the action, `#hit`, and its target, `@window`
- what the player holds, `@stick`
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

## When: #hit @stick
The glass cracks from corner to corner.
```

Only the tags in a heading are tested, and all of them must be true. *Proposed*: no `or` and no other operators, so principle 7 (a small condition language) holds. `## Unless:` is the same with every tag false.

### Two kinds of section

*Proposed*, so a room's state text doesn't repeat on every action:
- **A description section** names no action: `## When: #locked`. It's added to the thing's description whenever it matches, which is when the player looks, examines or enters.
- **A reply section** names an action: `## When: #hit @stick`. It's shown only on the turn that action happens, as the reply. Nothing else shows it.

A heading's tags are read against whose file it's in: in the window's file, the window is the target, so `@window` isn't needed; in the bedroom's file, the bedroom is where the player is, so the same reply is `## When: #hit @window @stick`.

### Which files answer an action

*Proposed*: for an action, the engine looks for matching reply sections in the target, then in what the action uses (the stick), then in the room, and shows every match in that order. If none matches, nothing happens (Bruce): the turn changes nothing, and the reply is a default, "Nothing happens." unless something nearer sets one.

### Default replies

"Nothing happens." is the built-in default (Bruce). The world's settings can set a list of defaults, and so can chapters, sections and the rest, so a ghost story's unanswered actions can read differently from a farce's (Bruce). *Proposed*: the nearest list wins, by the usual lineage: the section, the thing, its area, the chapter, then `world/index.md`. The engine picks one line from the list each time, varied by turn. The list's form is Open question 4.

### Changing state

A reply section MAY hold a ` ```thwarter ` block whose mechanics run when it's shown:

````markdown
## When: #hit @stick
```thwarter
becomes: { this: broken }
```
The glass shatters. Cold air pours in.
````

*Proposed*: this replaces the frontmatter `actions:` map. A world's verbs are the action tags its headings use, which is what the parser's REQ-PC-14 check already asks ("does any thing have this verb?"). `requires:` becomes names in the heading: `## When: #unlock @brass-key` works only while the player holds the key, and a section headed `## Unless: @brass-key` is the refusal. *Proposed*: `When:` and `Unless:` can't share one heading, so a refusal needs Open question 5's answer on combining them. The `actions:` map and `requires:` then go.

Until Bruce confirms this, mechanics stay in frontmatter `actions:` (settled with the coder, 2026-11-23). A reply section supplies the words: with a matching action it replaces the built-in reply, and without one it shows its text and changes nothing. An action tag in any heading counts as a verb the world knows (REQ-PC-14).

A description section's block MAY hold data that applies while it matches, such as a door's `passable:`, merging over frontmatter the way layers merge today (REQ-CF-13). *Proposed*: one block per section, first in it.

### Scripts

Decided by Bruce, 2026-11-23. When YAML and sections aren't enough, as for a door with a key, a thing MAY have a script.
- Scripts are written in [Rhai](https://rhai.rs), a scripting language made for embedding in Rust. It's sandboxed by default (no files or network), and the engine caps how long a script can run, so a world from a stranger is safe to play.
- A script lives in the thing's bundle, or in the thing's one file as a fenced ```rhai block (Bruce: bundles are clearer, or all in one file). *Proposed*: the bundle's file is `actions.rhai`, one per thing.
- Each function is an action, named by the verb (`fn unlock(door, turn)`). It gets the thing and the turn, read-only, and returns what happens. The engine applies it, so a script never writes a file (principle 6).
- A script never returns text. It returns a new state (`become("closed")`) or a reason, which is a state-like name (`"wrong-key"`). `no-action` is the common reason, and returning nothing means it. A world MAY use any reason that matters to its story.
- A script reaches only what its thing owns (Bruce): its own state and values, and the things it holds. A door can see a key in its lock, not one in the player's pocket; a room can see its objects, but can't do much with them. The scope is the same for every kind of thing. A thing used on another (`unlock door with key`) is handed to it for the action, and stays with it if its script keeps it, until the player takes it back (Bruce). A connection between two things, in frontmatter or the body, lets one reach the other (Bruce; `design/STATES.md` › One thing affecting another).
- The built-in verbs (`take`, `go`, `look` and the rest) stay the engine's; no script replaces them. Their turns still add tags, so a `## When: #take` reply replaces the built-in reply where one exists (Bruce).
- Scripts sit beside the YAML mechanics, for hard cases. A lever that `becomes: pulled` needs no script.
- A template's script passes down. *Proposed*: a thing's own function replaces the template's function of the same name.

### Outcomes and replies

*Proposed*, following Bruce's answers: an action's turn has one outcome, which is the state the thing moved into, or the reason the script returned, or `no-action`. A reply section matches the action and its outcome. For an author, that comes down to one rule: write the reply where the thing ends up, and if nothing changes, name why.
- In a state file, `## When: #unlock` means "unlock, ending up in this state".
- Anywhere, `## When: #unlock #wrong-key` means "unlock, for that reason", and `## When: #unlock #closed` means "unlock, ending up closed".
- With no matching reply, the default reply shows ("Nothing happens.", or the nearest list).

A thing's own reply is the story's content, so it replaces any default content: a template's or a built-in's reply for the same action and outcome (Bruce).

### States

States come from bundles (Bruce: "bundles are simpler"). A state exists because its file does (`objects/window/broken.md`), or because frontmatter's `state:` or a `becomes:` names it. A heading only reads states; `## When: #broken` is shown while the window is broken, and never makes `broken` exist. `validate` warns about a heading tag that can never be true: no state, action, thing or chapter has that name, so `## When: #lockd` is a warning.

### Prefixes, links and names

- `#` means a state and `@` means a thing, and neither searches the other's names (Bruce). So `#open` is always a state or the action of that name, and `@window` is always a thing; a state and a thing can share a name without clashing.
- `@id` in prose names a thing; in play it shows as the thing's name (`@guard-captain` → Captain Marcus). Object, character and door ids are unique across the world (REQ-CF-22), so `@` always means one thing.
- `#name` in prose is also an ordinary tag (REQ-CF-30).
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

1. **Which tags can a heading test?** Bruce's example uses the action, its target and what the player holds. Could a heading also test another thing's state (a bedroom line while the window is broken, `## When: @window #broken`), the chapter (`## When: #after-the-king`), a value (`#story.king.dead`) or the time of day? Each one makes headings more powerful and harder to predict. *Proposed* for the trial: actions, targets, held things, this thing's own states and the chapter.
2. **Do chapters become sections?** Today a thing changes for a chapter through a file in its `chapters/` folder (`castle/courtyard/chapters/after-the-king.md`). If the chapter is a tag a heading can test (question 1), a small chapter change could be a section in the thing's own file instead: `## When: #after-the-king` with "Black banners hang from the walls." Then `chapters/` files would be only for big changes, as state files are. Should both work, or only files?
3. **Does a section add to the description, or replace it?** Under "added when it matches", a door that reads "The door is shut." by default and has `## When: #open` "The door stands open." reads "The door is shut. The door stands open." while open. Options:
   - **a.** Sections always add. The default text holds only what's always true ("A heavy iron door, set into the floor."), and every state-dependent line goes in a section. Simple to predict; the author has to write that way.
   - **b.** A section can replace the default by saying so in its block (`replaces: description`). More flexible; one more rule.
   - **c.** Sections add, and a state file in the bundle replaces, as state files do today. Major changes replace, minor ones add.
   *Proposed*: **a** for the trial, since it matches "shown if it's true".
4. **What form does a list of default replies take?** *Proposed*: a `nothing:` list in the settings' or section's ```thwarter block (`nothing: ["Nothing happens.", "That does nothing."]`). The lines are short and systemic, like `title`, so they can bend principle 8. The alternative is prose: a `## When: #nothing` section whose paragraphs are the list, which keeps principle 8 but adds a reserved state name.
5. **Combining `When:` and `Unless:`.** Settled (Bruce, 2026-11-23): no nesting, and a heading stays one keyword with its names. Harder conditions go in code: inline code, or a fenced ```thwarter block acting as a gate. *Proposed* gate keys: `requires:` and `unless:`, so two `## When: #unlock` sections can be told apart by their blocks.

An action and a state sharing a name (`open`) isn't a concern (Bruce): `## When: #open` is the open door's description, and it reads as the reply to opening it too.
6. **Does a mention connect?** A connection can be in the frontmatter or the body (Bruce). *Proposed*: frontmatter `connects: [vault-door]` always connects; in the body, only a link or `@` mention in a section that holds connects, so a passing mention of the captain in a description doesn't give a lever reach over him. Or should any mention connect?
