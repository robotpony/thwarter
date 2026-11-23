# Thwarter Requirements

Status: Draft. Consolidated from `README.md`, `design/ARCHITECTURE.md`, `design/CONTENT_FORMAT.md`, `design/DECISIONS.md`, `design/CLI_DESIGN.md`, and `CLAUDE.md` as of 2026-08-23. Those documents remain the detailed reference for their own areas; this document is what a reader should check first to know what the system is required to do, and RFC 2119 keywords (MUST, SHOULD, MAY) are used here precisely, even in places the source documents were looser. Where this document and a source document disagree, that's a bug in one of them, file it rather than picking one silently.

## 1. Abstract

Thwarter is a Hugo-style engine for interactive fiction: a Rust CLI (`thwart`) that loads a world of locations, characters, and objects from Markdown files with YAML frontmatter, validates the world's internal consistency, and (in later phases) runs it as a playable game. This document specifies what the system MUST, SHOULD, and MAY do, split into a P0 scope (document processing and shared models, buildable and testable without a player-facing game loop) and a P1 scope (an interactive client and API built on top of P0).

## 2. Motivation

Authoring interactive fiction today usually means either a dedicated authoring language (Inform, Twine) with its own syntax to learn, or hand-rolled code with no separation between content and engine. Thwarter's premise is that IF content is documents: a location is a page, a character is a page, an object is a page, and the relationships between them (exits, inventory, placement) are structured data attached to those pages, the same shape Hugo and other static site generators already use for content-driven sites.

That premise implies three requirements that shape everything else in this document: content MUST be plain Markdown files an author can write without touching Rust, structure (frontmatter) and prose (body) MUST be separated so tooling can validate the former without parsing the latter, and the loader MUST treat a malformed or missing file as a reportable error, not a crash, since authors will make mistakes far more often than the engine will.

## 3. Proposal

Build Thwarter in phases:

**P0: Command Line Document Processor + Shared Models.** A CLI that loads a world directory, validates it, and displays world state, backed by data models and engines (World, Character, Narrative, Game) shared with everything built later. This is the only phase specified in detail below, since it's the only phase with enough design consensus to be implementable as written.

**P1: Console, authoring first.** One interactive terminal client, the console, for both writing and playing a world, with an edit mode and a play mode (§4.6). Authoring comes first and play second; engine work is done only as authoring, then play, needs it (Bruce, 2026-11-23). This replaces the earlier split into a P1 play REPL and a P2 editing REPL. Build order is in `PLAN.md`.

**Later: RESTful API.** A RESTful API exposing the same models to other clients. No specification yet; see Open Questions (§8).

**Out of scope.** Explicitly out of scope for both phases, per the original design: a web frontend, and any database or persistent data store beyond the world's own files (state loads and saves are file-based, in memory during a session). These requirements are subject to change, once the initial POC is completed.

## 4. Specification

### 4.1 Content file format

Full field-by-field detail lives in `design/CONTENT_FORMAT.md`; this section states the normative requirements.

- REQ-CF-1: Every content file MUST be a Markdown file whose frontmatter is a `---`-delimited YAML block starting on the file's first line.
- REQ-CF-2: Frontmatter MUST include `id` (stable identifier), `title` (display name), and `type` (`location`, `character`, or `object`). A file missing any of these three MUST fail to load with an error identifying the file and the missing field, not load with a default or placeholder value.
- REQ-CF-3: The Markdown body following the closing `---` MUST be treated as the entity's description, used as-is (only leading/trailing whitespace trimmed). The format MUST NOT require or expect a title heading in the body; `title` lives in frontmatter exactly once.
- REQ-CF-4: For `type: location`, `connections` (direction to target location `id`), `characters` (list of character `id`s present by default), `objects` (list of object `id`s present by default), and `coordinates` (`[x, y, z]`) are OPTIONAL. Direction names in `connections` MUST be treated as free text, not a closed set (`north`/`south`/etc. are convention, not a required vocabulary).
- REQ-CF-5: For `type: character`, `character_type` (one of `npc`, `player`, `villain`, `hero`, defaulting to `npc`), `default_location`, `attributes` (name to integer), `dialogue_tree` (opaque reference, format unspecified, see §8), `inventory` (list of object `id`s), and `relationships` (list of single-key maps, other character `id` to a free-text label) are OPTIONAL.
- REQ-CF-6: For `type: object`, `object_type` (free text, e.g. `weapon`, `container`, `environmental`) is REQUIRED; `category`, `weight`, `value`, and `actions` (named action to response text) are OPTIONAL. Object content MAY include arbitrary additional frontmatter fields (`properties`, `stats`, `effects`, and similar); a conforming parser MUST preserve these rather than rejecting the file or silently discarding them, since object content varies more than location or character content does and a fixed schema would be false precision.
- REQ-CF-8: An object `id` named in a location's `objects` or a character's `inventory` MAY have no object file (a card). Such an object is scenery: it has a name and nothing else, and it MUST NOT be reported as an error. When an author first edits it, the tool MUST create its card (REQ-CON-6).
- REQ-CF-7: A loader encountering a file that fails REQ-CF-1 or REQ-CF-2 MUST record it as a per-file error and continue loading the rest of the world, rather than aborting the whole load. A single bad file MUST NOT prevent validation from reporting problems in every other file.

### 4.2 World structure and addressing

- REQ-WS-1: World content MUST live under `authors/<author-name>/<world-name>/`, one directory per author, so a single repository MAY hold more than one author's world.
- REQ-WS-2: A location's `id` SHOULD read as a place a human would recognize (`castle/courtyard`), not an opaque identifier (a UUID or sequence number).
- REQ-WS-3: A world directory MAY include a `world.toml` (title, author, starting location, and world-specific settings) and the tool's own `thwart.toml` (default world path, output preferences); neither is required for a world to load.

### 4.3 CLI interface

Full command-by-command detail, including commands not yet built, lives in `design/CLI_DESIGN.md`. Normative requirements for the commands in P0 scope:

- REQ-CLI-1: The binary MUST expose, at minimum, `validate`, `show`, `map`, `search`, and `debug` subcommands.
- REQ-CLI-2: `--world <path>` MUST be available as a global option overriding the configured default world path for that invocation.
- REQ-CLI-3: `validate` MUST load the world and report, at minimum: files that failed to parse, and connections whose target location `id` was never loaded. `validate --strict` MUST additionally check that a character's `default_location` refers to a loaded location.
- REQ-CLI-4: `validate` MUST exit with a non-zero status if any problem was found, and zero otherwise, so it's usable as a CI gate.
- REQ-CLI-5: `show` MUST display either the whole loaded world (at minimum, the set of loaded location IDs) or, given a location argument, that location's full parsed data; it MUST error clearly, not silently return nothing, if the given location `id` doesn't exist in the loaded world.
- REQ-CLI-6: Output MUST support text, JSON, and YAML formats via `--format`, selectable independently per command.
- REQ-CLI-7: A command not yet implemented MUST say so and exit non-zero, and MUST NOT produce output that looks like a real result (a fabricated map, invented search matches, or made-up performance numbers). This requirement exists because the first implementation of this CLI violated it in `map`, `search`, and `debug`; see `CHANGELOG.md` › 0.1.0.
- REQ-CLI-8: `map` (ASCII rendering of the world graph, full or fog-of-war from a player's perspective) and `search` (content search across locations/characters/objects, with `--regex` and type filtering) are specified in `design/CLI_DESIGN.md` but have no implementation requirement attached to a milestone yet; see Open Questions (§8).

### 4.4 Engines and models

- REQ-ENG-1: `WorldEngine` MUST own the location graph (`WorldGraph`) and MUST be able to load an entire world directory in one pass, per REQ-CF-7's error-collection requirement.
- REQ-ENG-2: `WorldEngine` MUST be able to report every location whose `connections` target an unloaded location `id` (this backs REQ-CLI-3).
- REQ-ENG-3: `CharacterEngine` MUST own loaded character definitions and their live state (current location, mood, health) separately from the static `Character` data parsed from content files, so that a character's runtime state can change during a session without mutating the authored definition.
- REQ-ENG-4: `NarrativeEngine` MUST be able to register plot points and narrative arcs and check a plot point's prerequisites against a set of completed plot points before allowing it to trigger.
- REQ-ENG-5: `GameEngine` MUST orchestrate the other three engines and dispatch player actions (at minimum: move, take, look, examine) to per-action-type handlers.
- REQ-ENG-6: Object placement and lookup (which world entity owns "does object X exist, and where is it") is the World Engine's responsibility, per REQ-ENG-1; there is no separate Object Engine, since objects don't have enough independent behavior to justify one.
- REQ-ENG-7: The engine MUST be able to write an entity back to its file. Changing one field MUST leave every other part of the file as it was: field order, comments, fields the engine doesn't model (REQ-CF-6) and the body. A new entity MUST be written in the format of `design/CONTENT_FORMAT.md`, at the path its `id` implies, and MUST refuse any path that resolves outside the world directory (an `id` containing `..`, for example).
- REQ-ENG-8: The engine MUST be able to change the loaded world in memory (add, change or remove an entity or an exit) and report the problems `validate` would find after that change, without reloading the world from disk.
- REQ-ENG-9: The engine's interface to the console MUST offer at least: a description of a location (title, body, exits with whether each target exists, characters, objects with whether each has a card); the play state (current location, inventory, story flags); and an action result (the text to show, and what changed). The coder and console coder agree its Rust shape between them and record it in `design/ARCHITECTURE.md`.

Requirements REQ-ENG-4 and REQ-ENG-5 describe required capability, not required completeness: an engine satisfying "MUST be able to dispatch player actions" needs a dispatch mechanism, but nothing here requires every action's *effects* to be implemented. Current status against every requirement in this section is tracked in `design/ARCHITECTURE.md`'s implementation status table, not duplicated here, so status and specification don't drift apart again (see `design/DECISIONS.md` for why that drift happened once already).

### 4.5 Configuration

- REQ-CFG-1: A global `thwart.toml` MAY set the default world path, default output format/style, validation strictness, and debug trace level; command-line flags MUST take precedence over it when both are given.
- REQ-CFG-2: A per-world `world.toml` MAY set world metadata (title, author, version) and world-specific settings (starting location, default map style); it is not required for a world to load or validate.
- REQ-CFG-3: Configuration parsing MUST NOT fail on unrecognized fields in either file; an author adding a custom `world.toml` section for their own tooling MUST NOT break `thwart`.

### 4.6 Console

The console is the interactive terminal client for writing and playing a world. It lives in `src/console/`. Decided by Bruce, 2026-11-23, except where marked as proposed.

- REQ-CON-1: `thwart edit <world>` MUST open the console in edit mode and `thwart play <world>` in play mode. Both are subcommands of the one `thwart` binary.
- REQ-CON-2: Commands to the console itself MUST start with `/`; anything else is input to the current mode. `/edit` and `/play` MUST switch modes inside a running console, and `/help` and `/quit` MUST work in both.
- REQ-CON-3: Edit mode MUST be able to see everything in the world, including hidden state, and change content. Play mode MUST NOT change content files; it MAY write game saves. (Play mode's limits beyond that are proposed, not decided: `PLAN.md` › Open question 2.)
- REQ-CON-4: Edit mode MUST navigate the world by area and room, as a terminal navigates folders (an area such as `castle/` holds rooms such as `castle/courtyard`), and MUST be able to navigate to an object or character to edit it. Navigation stays in the terminal.
- REQ-CON-5: Edit mode MUST offer commands that create or change one thing at a time, for example `/new room north` (a new room, linked north of the current one), `/new object`, `/new character`, `/set` (one field) and `/link` (one exit). Each MUST save the change to the file (REQ-ENG-7) and report any problems it causes (REQ-ENG-8).
- REQ-CON-6: Edit mode MUST offer full editing of a whole entity, frontmatter and body, inside the console: `/edit room`, `/edit object`, `/edit character`. It MUST NOT hand off to an external `$EDITOR`. Editing an object without a card creates its card (REQ-CF-8). How full editing looks on screen is open (`PLAN.md` › Open question 1).
- REQ-CON-7: `/reload` MUST reload the world from disk, keeping the current location, inventory and flags. If the reloaded world fails validation, the console MUST keep the previous world and show the problems. The console SHOULD also reload when a world file changes on disk (after `/reload` works).
- REQ-CON-8: The console's room view SHOULD come from the same renderer as `thwart show`'s text output, so they can't drift apart.
- REQ-CON-9: Console behaviour MUST be testable from transcripts: a scripted input file and the output expected from it. Transcripts are the console's acceptance tests.
- REQ-CON-10: Play mode MUST accept at least `look`, `go <direction>`, `take`, `examine` and `inventory`, with the aliases `l`, `x`, `i` and bare compass directions (`n`, `north`). An unknown command MUST get a short reply naming what it didn't understand, and MUST NOT change any state.
- REQ-CON-11: Play mode MUST be able to save a game and load it again (Phase 4).

## 5. Alternatives considered

**Implementation language.** Rust was chosen for startup time (target: under 1 second, achieved: single-digit milliseconds) and single-binary distribution to content authors who shouldn't need a language runtime installed. No alternative was formally evaluated and recorded at the time; the most likely alternative given the same constraints would have been a compiled-to-native scripting language. See `design/DECISIONS.md` for the full record, including the acknowledgment that this gap in the original rationale exists.

**Frontmatter position.** Two positions were live options: frontmatter at the end of the file (what the original sample content and design spec used) versus the beginning (what most static-site tooling, and the original README's own example, used). Frontmatter-first was chosen to match the convention a new contributor is most likely to already know; see `design/DECISIONS.md` for the full trade-off record, including the migration cost this imposed on already-written sample content.

**Object schema strictness.** A closed, fully-typed schema for objects (mirroring `Character`'s fixed `attributes` map) was considered and rejected: real sample content already used a dozen distinct `object_type` values with non-overlapping extra fields (a sword has `stats.damage`, a stream doesn't), and forcing that into a fixed struct would either reject valid content or silently drop fields. REQ-CF-6's "preserve arbitrary extra fields" requirement is the result.

## 6. Security / risk considerations

Not previously assessed anywhere in this repo; recorded here as a starting point, not a completed review.

- **Trust boundary**: content files are currently assumed to come from a trusted author (yourself, or someone you've given write access to the world directory). Nothing in this specification requires sandboxing frontmatter parsing or body content against a malicious author. This assumption breaks the moment the RESTful API allows remote or multi-tenant content submission; that phase MUST revisit this section before shipping, not inherit it silently.
- **Path handling**: once the console writes files, an `id` is also a path to write to. REQ-ENG-7 requires the writer to refuse any path outside the world directory. Reading is not yet guarded the same way (an `id` or `connections` target containing `../` loads without complaint), which matters if content is ever fetched from an untrusted source or served over the API.
- **YAML parsing**: frontmatter is parsed with `serde_yaml` against locally-authored files. YAML deserialization libraries have a history of resource-exhaustion issues (deeply nested or self-referential structures); this specification does not currently require any size or depth limit on frontmatter. Worth revisiting if content is ever accepted from outside the local filesystem.
- **No authentication or authorization model exists**, because none is needed yet: everything in P0 scope runs as the invoking user against local files. The RESTful API introduces the first point where "who is allowed to do what" becomes a real question, and this document takes no position on it yet (see Open Questions, §8).

## 7. Implementation notes

This specification describes required behavior, not current status. Current status, command by command and engine by engine, is tracked in `design/ARCHITECTURE.md`'s implementation status table and MUST be kept there rather than duplicated here. As of 2026-08-23 (REQ-CF-8, REQ-ENG-7 to 9 and REQ-CON-* added 2026-11-23, none met yet): REQ-CF-1 to 7, REQ-WS-*, REQ-ENG-1, REQ-ENG-2, REQ-CLI-1 through REQ-CLI-7 are met. REQ-ENG-3 through REQ-ENG-6 are met at the model/engine level but not yet exercised from any CLI command. REQ-CLI-8 (`map`, `search`) and REQ-CFG-1 through REQ-CFG-3 (`ConfigManager` exists but `thwart.toml` isn't read by `main.rs` yet) are not yet met.

Build order is in `PLAN.md`: the console shell, then authoring (the file writer, REQ-ENG-7, is its largest engine task), then play. `map`, `search` and the remaining `validate` checks wait until authoring or play needs them.

## 8. Open questions

- **The RESTful API has no specification.** The console is specified in §4.6; the API is not, and needs its own RFC before implementation, given the security considerations in §6 it introduces.
- **Console open questions** are in `PLAN.md` › Open questions: how full editing looks on screen, play mode's limits, and return exits for `/new room`.
- **Dialogue tree format is unspecified.** REQ-CF-5 requires `dialogue_tree` to load as an opaque reference; nothing specifies what it should point to or how it should be structured. Blocks any real character interaction in play mode.
- **`id` uniqueness and path-matching are unenforced.** REQ-WS-2 says a location `id` SHOULD read as a place name, and by convention matches the file's path, but nothing requires it to, and nothing currently checks for `id` collisions across files. Should `thwart validate` add this check, and should it be a MUST rather than a SHOULD once added?
- **Multiplayer, web, and always-online play are not addressed anywhere in this document**, and the README explicitly places a web frontend out of scope. If broader ambitions beyond a local single-player CLI/REPL are real (this surfaced in an author project note, not in any document this specification was built from), they belong in a separate P2-or-later RFC, since they'd change the security model (§6), the persistence model (currently file-based, in-memory per session), and likely the language/architecture trade-offs in `design/DECISIONS.md`. Flagging rather than resolving, since it wasn't part of the source material this document was built from.
