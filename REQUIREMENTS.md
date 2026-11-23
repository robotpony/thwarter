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

**P1: Console, authoring first.** One interactive terminal client, the console, for both writing and playing a world, with an edit mode and a play mode (`design/CONSOLE_REQUIREMENTS.md`). Authoring comes first and play second; engine work is done only as authoring, then play, needs it (Bruce, 2026-11-23). This replaces the earlier split into a P1 play REPL and a P2 editing REPL. Build order is in `PLAN.md`.

**Later: RESTful API.** A RESTful API exposing the same models to other clients. No specification yet; see Open Questions (§9).

**Out of scope.** Explicitly out of scope for both phases, per the original design: a web frontend, and any database or persistent data store beyond the world's own files (state loads and saves are file-based, in memory during a session). These requirements are subject to change, once the initial POC is completed.

## 4. Principles

Agreed by Bruce, 2026-11-23. Every requirement in every component's document answers to these. A component MAY add principles of its own at the top of its requirements, such as the console's game-master principle (`design/CONSOLE_REQUIREMENTS.md`).

1. **Lineage only.** A thing takes values from global definitions, the folders above it, and templates it names; never from a sibling beside it.
2. **Explicit, never hidden.** Inheritance is written down, and the console shows where every value came from.
3. **Predictable from the tree.** An author can tell what applies by looking at the folders.
4. **Major changes are files, minor ones are values.** States and chapters are files; small changes are variables.
5. **Files are the truth; the console is how you edit them.** Structure changes go through commands, and the files stay plain Markdown.
6. **Play never changes content.** Everything that happens in a game lives in the game state.
7. **Keep the condition language small.** Only `requires: { has: ... }`, until a real case needs more.
8. **Content lives in prose, not frontmatter.** Frontmatter holds state, ids and mechanics; everything a player reads is in the Markdown body. `title` is the one exception (Bruce, 2026-11-23).
9. **A world is a folder any Markdown tool can read.** Worlds MUST stay readable in editors like VS Code and Obsidian, and SHOULD support their features where it's convenient, so that Hugo or a similar tool could sit on top of a world as a site (Bruce, 2026-11-23).

## 5. Components

Each component's requirements live in their own document, with one owner. A change that crosses components goes through the commander, never as an edit to another owner's document (Bruce, 2026-11-23). Every document uses RFC 2119 keywords, stable ids and numbered open questions.

| Component | Requirements | Ids | Owner |
|---|---|---|---|
| Overview, principles, configuration | this document | REQ-CFG | commander |
| Content and world model | `design/CONTENT_REQUIREMENTS.md` | REQ-CF, REQ-WS | commander |
| Engine | `design/ENGINE_REQUIREMENTS.md` | REQ-ENG | thwarter coder |
| CLI tools | `design/CLI_REQUIREMENTS.md` | REQ-CLI | thwarter coder |
| Console | `design/CONSOLE_REQUIREMENTS.md` | REQ-CON | thwarter console coder |

### 5.1 Configuration

- REQ-CFG-1: A global `thwart.toml` MAY set the default world path, default output format/style, validation strictness, and debug trace level; command-line flags MUST take precedence over it when both are given.
- REQ-CFG-2: A per-world `world.toml` MAY set world metadata (title, author, version) and world-specific settings (starting location, default map style); it is not required for a world to load or validate.
- REQ-CFG-3: Configuration parsing MUST NOT fail on unrecognized fields in either file; an author adding a custom `world.toml` section for their own tooling MUST NOT break `thwart`.

### 5.2 Authors, installed worlds and play data

Decided with Bruce, 2026-11-23. The person at the keyboard is the local user, and their identity and private settings live in their home folder, not in a world. Each folder in `authors/` is a persona: a pen name the person writes as. One person MAY have several. In a later online version each author gets their own instance, and personas work the same way inside it.

Paths follow the XDG base directories on every platform, falling back to `~/.config`, `~/.local/share` and `~/.local/state`.

- REQ-CFG-4: A person's own settings MUST live in `$XDG_CONFIG_HOME/thwarter/config.toml`, never in a world or a repository: the persona they write as by default, and the writing helpers' service settings and keys (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-19).
- REQ-CFG-5: A world installed to be played (someone else's, or a published one) MUST go in `$XDG_DATA_HOME/thwarter/worlds/<persona>/<world>/`. An author's own worlds stay where they write them, under `authors/<persona>/`.
- REQ-CFG-6: Play data MUST go in `$XDG_STATE_HOME/thwarter/`: saves as REQ-CON-11 already says, and any other per-player state, so neither an author's worlds nor installed ones ever change by being played (principle 6).
- REQ-CFG-7: A world MUST be shareable as one archive holding the world and its persona's file, so the author's details travel with the game. Packaging leaves out what the world's ignore list names. Installing an archive puts it under REQ-CFG-5's folder. The archive format and the commands are the coder's to specify in `design/CLI_REQUIREMENTS.md`. *Proposed*: `thwart package` and `thwart install`.

## 6. Alternatives considered

**Implementation language.** Rust was chosen for startup time (target: under 1 second, achieved: single-digit milliseconds) and single-binary distribution to content authors who shouldn't need a language runtime installed. No alternative was formally evaluated and recorded at the time; the most likely alternative given the same constraints would have been a compiled-to-native scripting language. See `design/DECISIONS.md` for the full record, including the acknowledgment that this gap in the original rationale exists.

**Frontmatter position.** Two positions were live options: frontmatter at the end of the file (what the original sample content and design spec used) versus the beginning (what most static-site tooling, and the original README's own example, used). Frontmatter-first was chosen to match the convention a new contributor is most likely to already know; see `design/DECISIONS.md` for the full trade-off record, including the migration cost this imposed on already-written sample content.

**Object schema strictness.** A closed, fully-typed schema for objects (mirroring `Character`'s fixed `attributes` map) was considered and rejected: real sample content already used a dozen distinct `object_type` values with non-overlapping extra fields (a sword has `stats.damage`, a stream doesn't), and forcing that into a fixed struct would either reject valid content or silently drop fields. REQ-CF-6's "preserve arbitrary extra fields" requirement is the result.

## 7. Security / risk considerations

Not previously assessed anywhere in this repo; recorded here as a starting point, not a completed review.

- **Trust boundary**: content files are currently assumed to come from a trusted author (yourself, or someone you've given write access to the world directory). Nothing in this specification requires sandboxing frontmatter parsing or body content against a malicious author. This assumption breaks the moment the RESTful API allows remote or multi-tenant content submission; that phase MUST revisit this section before shipping, not inherit it silently.
- **Path handling**: once the console writes files, an `id` is also a path to write to. REQ-ENG-7 requires the writer to refuse any path outside the world directory. Reading is not yet guarded the same way (an `id` or `connections` target containing `../` loads without complaint), which matters if content is ever fetched from an untrusted source or served over the API.
- **YAML parsing**: frontmatter is parsed with `serde_yaml` against locally-authored files. YAML deserialization libraries have a history of resource-exhaustion issues (deeply nested or self-referential structures); this specification does not currently require any size or depth limit on frontmatter. Worth revisiting if content is ever accepted from outside the local filesystem.
- **No authentication or authorization model exists**, because none is needed yet: everything in P0 scope runs as the invoking user against local files. The RESTful API introduces the first point where "who is allowed to do what" becomes a real question, and this document takes no position on it yet (see Open Questions, §9).

## 8. Implementation notes

This specification describes required behavior, not current status. Current status, command by command and engine by engine, is tracked in `design/ARCHITECTURE.md`'s implementation status table and MUST be kept there rather than duplicated here. As of 2026-08-23 (REQ-CF-8, REQ-ENG-7 to 9 and REQ-CON-* added 2026-11-23, none met yet): REQ-CF-1 to 7, REQ-WS-*, REQ-ENG-1, REQ-ENG-2, REQ-CLI-1 through REQ-CLI-7 are met. REQ-ENG-3 through REQ-ENG-6 are met at the model/engine level but not yet exercised from any CLI command. REQ-CLI-8 (`map`, `search`) and REQ-CFG-1 through REQ-CFG-3 (`ConfigManager` exists but `thwart.toml` isn't read by `main.rs` yet) are not yet met.

Build order is in `PLAN.md`: the console shell, then authoring (the file writer, REQ-ENG-7, is its largest engine task), then play. `map`, `search` and the remaining `validate` checks wait until authoring or play needs them.

## 9. Open questions

- **The RESTful API has no specification.** The console is specified in `design/CONSOLE_REQUIREMENTS.md`; the API is not, and needs its own RFC before implementation, given the security considerations in §7 it introduces.
- **Console open questions** are in `design/CONSOLE_REQUIREMENTS.md` › Open questions.
- **Dialogue tree format is unspecified.** REQ-CF-5 requires `dialogue_tree` to load as an opaque reference; nothing specifies what it should point to or how it should be structured. Blocks any real character interaction in play mode.
- **`id` uniqueness and path-matching are unenforced.** REQ-WS-2 says a location `id` SHOULD read as a place name, and by convention matches the file's path, but nothing requires it to, and nothing currently checks for `id` collisions across files. Should `thwart validate` add this check, and should it be a MUST rather than a SHOULD once added?
- **Multiplayer, web, and always-online play are not addressed anywhere in this document**, and the README explicitly places a web frontend out of scope. If broader ambitions beyond a local single-player CLI/REPL are real (this surfaced in an author project note, not in any document this specification was built from), they belong in a separate P2-or-later RFC, since they'd change the security model (§7), the persistence model (currently file-based, in-memory per session), and likely the language/architecture trade-offs in `design/DECISIONS.md`. Flagging rather than resolving, since it wasn't part of the source material this document was built from.
