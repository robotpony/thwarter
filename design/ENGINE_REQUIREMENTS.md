# Engine requirements

Requirements for the engine: loading a world, building each thing's current version, play, saves and the writer. Owned by thwarter coder. RFC 2119 keywords are used as in `REQUIREMENTS.md`, and every requirement answers to its principles. The content rules the engine implements are in `design/CONTENT_REQUIREMENTS.md`; the engine rules settled during the states design are REQ-ENG-10 to 29 below.

Split out of `REQUIREMENTS.md` on 2026-11-23. The REQ-ENG ids are stable. A change another component needs here goes to the commander as a message.

## Requirements

- REQ-ENG-1: `WorldEngine` MUST own the location graph (`WorldGraph`) and MUST be able to load an entire world directory in one pass, per REQ-CF-7's error-collection requirement.
- REQ-ENG-2: `WorldEngine` MUST be able to report every location whose `connections` target an unloaded location `id` (this backs REQ-CLI-3).
- REQ-ENG-3: `CharacterEngine` MUST own loaded character definitions and their live state (current location, mood, health) separately from the static `Character` data parsed from content files, so that a character's runtime state can change during a session without mutating the authored definition.
- REQ-ENG-4: `NarrativeEngine` MUST be able to register plot points and narrative arcs and check a plot point's prerequisites against a set of completed plot points before allowing it to trigger.
- REQ-ENG-5: `GameEngine` MUST orchestrate the other three engines and dispatch player actions (at minimum: move, take, look, examine) to per-action-type handlers.
- REQ-ENG-6: Object placement and lookup (which world entity owns "does object X exist, and where is it") is the World Engine's responsibility, per REQ-ENG-1; there is no separate Object Engine, since objects don't have enough independent behavior to justify one.
- REQ-ENG-7: The engine MUST be able to write an entity back to its file. Changing one field MUST leave every other part of the file as it was: field order, comments, fields the engine doesn't model (REQ-CF-6) and the body. Full editing (REQ-CON-6) MAY replace a file's whole text, but only after the new text parses. A file that names a template (`inherits:`) MAY take required keys from it, so the writer MUST check such a file only for sound frontmatter, `id` and `type`, and leave typing it with its template to the engine, which reports any problem (REQ-ENG-8). Changing an entity's `id` or `type` is refused until renaming is specified, since either moves the file. The writer MUST refuse to edit an `id` defined in more than one file. A new entity MUST be written in the format of `design/CONTENT_FORMAT.md`, at the path that document gives for its type, and MUST refuse any path that resolves outside the world directory (an `id` containing `..`, for example).
- REQ-ENG-8: The engine MUST be able to change the loaded world in memory (add, change or remove an entity or an exit) and report the problems `validate` would find after that change, without reloading the world from disk.
- REQ-ENG-9: The engine's interface to the console MUST offer at least: a description of a location (title, body, exits with whether each target exists, characters, objects with whether each has a card); the play state (REQ-ENG-23); and an action result (the text to show, and what changed). The coder and console coder agree its Rust shape between them and record it in `design/ARCHITECTURE.md`.


### Loading (`design/STATES.md` › Files)

- REQ-ENG-10: The loader MUST decide what a file is from where it sits, never from its contents: a folder holding `index.md` is one thing (a bundle) whose other files are its states, and whose `chapters/` folder holds its chapter overlays; `_index.md` marks an area; in any `chapters/` folder a numbered file defines a chapter in that scope and an unnumbered one is an overlay (REQ-ENG-14); files in `world/` define the world (REQ-CF-20); everything else is a one-file thing. State and overlay files MUST NOT set `id`, `type` or `state:`; a layer that does is kept with those keys removed and reported, so one bad key doesn't cause more errors.
- REQ-ENG-11: A bundle MUST be a leaf: it MAY hold state files, one `chapters/` folder and a `templates/` folder, and nothing else. A nested `index.md` is an error. `index`, `_index`, `chapters` and `templates` MUST NOT be state names.
- REQ-ENG-12: A thing's `id` MUST come from its one file or its `index.md`; the folder is only how the loader finds the files. A bundle whose `id` doesn't match its folder MUST be warned about.
- REQ-ENG-13: The loader MUST skip other tools' files (REQ-CF-24), and MUST NOT walk into a skipped folder. Only the part of a path inside the world counts, so a world kept under a dot folder still loads. The engine MUST say, for any file it read, what that file is (a thing, a state, an overlay, a chapter, an area, a `world/` file, a template, or a problem with its reason), so the console needn't guess.

### Chapters

- REQ-ENG-14: A chapter MUST be identified by its scope (the folder whose `chapters/` defines it) and its name without the number, so names need only be unique within a scope. A chapter name in an overlay, a `chapter:` action or a version request MUST be looked up from the referring thing's folder outward to the root. A scoped definition that hides an outer chapter of the same name MUST be warned about. Mixing numbered and unnumbered parts in one chapter path, and an unnumbered file in the root `chapters/`, are errors. A sub-chapter's folder MUST have an `_index.md`.

### Building a version (`design/STATES.md` › What applies, Inheritance)

- REQ-ENG-15: A thing's version MUST be built on request from its layers, in order: overlays from the folders above it (outermost first), its templates (most distant first), its `index.md`, its current state, then its own chapter overlays; within each level a parent chapter comes before its sub-chapter. Siblings never apply.
- REQ-ENG-16: Layers MUST merge as raw YAML before the result is typed: maps merge key by key all the way down, scalars and lists replace, and `null` removes an inherited key. Every key MUST remember the file it came from. A replaced list and a removed key MUST be warned about.
- REQ-ENG-17: Prose MUST merge by part: a layer's description replaces the description below it only if something is left once author comments are removed, and each response block replaces only its namesake. A folder's overlay adds its description after each thing's instead of replacing it.
- REQ-ENG-18: Only `index.md` MAY set `state:`, which names the starting state; a bundle MAY leave it out, and then `index.md` alone is the version until something switches it. The world tables MUST hold each thing's starting version (starting state, no chapter), and `validate` MUST check every state against every chapter visible from the thing.
- REQ-ENG-19: A template MUST be found by path from the world root, or by bare name in `templates/` folders from the thing's scope outward, then among the built-ins (REQ-CF-21). A template MUST NOT pass down its `id`, `template:` or `state:`. A name that finds nothing, a loop, and a chain more than 4 deep are problems.
- REQ-ENG-20: Wherever frontmatter takes an id, a name that isn't an id MUST be resolved as the thing whose file it names, by path from the root or by a file name only one file has (REQ-CF-25). A file name more than one file has MUST be reported as ambiguous, naming the files.

### Ids and values

- REQ-ENG-21: `game`, `story`, `player`, `location` and `this` MUST NOT be ids, and ids MUST NOT contain a dot. An id used by two kinds MUST be warned about, since bare references can't tell them apart.
- REQ-ENG-22: Values MUST be strings. Variables (REQ-CF-16) MUST read a value set in play first, then the declared starting value, and for a thing the value in its current version. An undeclared variable, and an undeclared value an action's `sets:` names, MUST be warned about, and play MUST show an unknown variable as blank. `requires:` MUST stay limited to `has:` until a real case needs more.

### Play and saves

- REQ-ENG-23: `PlayState` MUST keep the current location, the inventory, what was taken from each room, each thing's state only when it differs from its starting state, the current chapter, values by full name, visits per room, the turn count (one per player action the game understood) and the faint things found. Play MUST show each thing in its current version, and a room MUST NOT list what the player carries.
- REQ-ENG-24: An object's named action MUST check `requires:`, then apply `becomes:`, `sets:` and `chapter:`, and reply with its response block, or a built-in reply made from the verb and the object's name. States and chapters MAY go back as well as forward, and moving to a chapter applies only it and its parents.
- REQ-ENG-25: Play MUST honour `visibility:` (REQ-CF-18) for objects, characters and exits: faint things appear once found by searching or examining the room, hidden ones answer only to an action that names them exactly, and absent ones can't be named.
- REQ-ENG-26: When a noun matches several things, the reply MUST be the `reply ambiguous` block of the nearest template they share, else a built-in reply in the story's voice, varied, which a story MAY replace.
- REQ-ENG-27: Saves MUST be versioned (`thwarter_save: 2`) and MUST load older versions. On load, a state that no longer exists MUST fall back to the starting state, a thing that's gone MUST be dropped, and a chapter that's gone MUST fall back to the nearest earlier one in its scope, each with a warning.

### Writing bundles

- REQ-ENG-28: Turning a one-file thing into a bundle MUST write the new files first and rename the old file to `index.md` last, so a half-done move leaves a duplicate `id` that `validate` reports, never a silent loss.
- REQ-ENG-29: A bundle MUST save through one call that checks the merged thing before writing anything, writes only the files whose text changed (each to a temporary file, checked, then renamed), and reports exactly which files were written if a rename fails. Renaming or deleting a state MUST be a command, and a rename MUST update every `becomes:` and `state:` that names it.
- REQ-ENG-30: Renaming a thing's `id` MUST move its file, or its bundle folder with everything in it, to the path the new id gives (for a room, the id is the path, so moving a room into an area is a rename), and MUST update every reference to the old id: exits, `characters:`, `objects:`, `inventory`, `default_location`, `world/index.md`'s `start:`, and links and variables in prose, in the style each was written (a link stays a link). Every edit MUST be checked in memory before anything is written, and the rename MUST be refused if the new path or id is taken, or the new id is reserved. References inside an action's `becomes:` and `requires: { has: }` MUST be rewritten too, and checked by reading the file back as exactly what was expected; one that can't be rewritten that safely MUST be left as it is and reported for the author to fix. The report MUST list every file moved and written, and the engine MUST take the same change.

Requirements REQ-ENG-4 and REQ-ENG-5 describe required capability, not required completeness: an engine satisfying "MUST be able to dispatch player actions" needs a dispatch mechanism, but nothing here requires every action's *effects* to be implemented. Current status against every requirement in this section is tracked in `design/ARCHITECTURE.md`'s implementation status table, not duplicated here, so status and specification don't drift apart again (see `design/DECISIONS.md` for why that drift happened once already).
