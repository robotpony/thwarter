# Engine requirements

Requirements for the engine: loading a world, building each thing's current version, play, saves and the writer. Owned by thwarter coder. RFC 2119 keywords are used as in `REQUIREMENTS.md`, and every requirement answers to its principles. The content rules the engine implements are in `design/CONTENT_REQUIREMENTS.md`; the engine rules settled during the states design are in `design/STATES.md` › Engine rules, still to be written here as REQ-ENG items.

Split out of `REQUIREMENTS.md` on 2026-11-23. The REQ-ENG ids are stable. A change another component needs here goes to the commander as a message.

## Requirements

- REQ-ENG-1: `WorldEngine` MUST own the location graph (`WorldGraph`) and MUST be able to load an entire world directory in one pass, per REQ-CF-7's error-collection requirement.
- REQ-ENG-2: `WorldEngine` MUST be able to report every location whose `connections` target an unloaded location `id` (this backs REQ-CLI-3).
- REQ-ENG-3: `CharacterEngine` MUST own loaded character definitions and their live state (current location, mood, health) separately from the static `Character` data parsed from content files, so that a character's runtime state can change during a session without mutating the authored definition.
- REQ-ENG-4: `NarrativeEngine` MUST be able to register plot points and narrative arcs and check a plot point's prerequisites against a set of completed plot points before allowing it to trigger.
- REQ-ENG-5: `GameEngine` MUST orchestrate the other three engines and dispatch player actions (at minimum: move, take, look, examine) to per-action-type handlers.
- REQ-ENG-6: Object placement and lookup (which world entity owns "does object X exist, and where is it") is the World Engine's responsibility, per REQ-ENG-1; there is no separate Object Engine, since objects don't have enough independent behavior to justify one.
- REQ-ENG-7: The engine MUST be able to write an entity back to its file. Changing one field MUST leave every other part of the file as it was: field order, comments, fields the engine doesn't model (REQ-CF-6) and the body. Full editing (REQ-CON-6) MAY replace a file's whole text, but only after the new text parses. Changing an entity's `id` or `type` is refused until renaming is specified, since either moves the file. The writer MUST refuse to edit an `id` defined in more than one file. A new entity MUST be written in the format of `design/CONTENT_FORMAT.md`, at the path that document gives for its type, and MUST refuse any path that resolves outside the world directory (an `id` containing `..`, for example).
- REQ-ENG-8: The engine MUST be able to change the loaded world in memory (add, change or remove an entity or an exit) and report the problems `validate` would find after that change, without reloading the world from disk.
- REQ-ENG-9: The engine's interface to the console MUST offer at least: a description of a location (title, body, exits with whether each target exists, characters, objects with whether each has a card); the play state (current location, inventory, story flags); and an action result (the text to show, and what changed). The coder and console coder agree its Rust shape between them and record it in `design/ARCHITECTURE.md`.

Requirements REQ-ENG-4 and REQ-ENG-5 describe required capability, not required completeness: an engine satisfying "MUST be able to dispatch player actions" needs a dispatch mechanism, but nothing here requires every action's *effects* to be implemented. Current status against every requirement in this section is tracked in `design/ARCHITECTURE.md`'s implementation status table, not duplicated here, so status and specification don't drift apart again (see `design/DECISIONS.md` for why that drift happened once already).
