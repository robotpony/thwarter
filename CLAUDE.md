# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Thwarter is a Hugo-style engine for interactive fiction (IF), driven by Markdown files and YAML frontmatter. The system breaks down storytelling into reusable components: locations, characters, objects, plot points, and narrative arcs.

For anything beyond this orientation, prefer these over re-deriving from source:
- `design/ARCHITECTURE.md` — module layout and, critically, its implementation status table. Check this before assuming a feature works.
- `design/CONTENT_FORMAT.md` — the content file format, single source of truth
- `design/DECISIONS.md` — why Rust, why frontmatter-first

## Project Structure

- `src/` - Rust source code for the IF engine
  - `src/bin/` - CLI binary executable (`thwart`)
  - `src/models/` - Core data models (World, Character, Narrative, Game)
  - `src/engines/` - Processing engines for game logic
  - `src/parsers/` - Markdown and metadata file parsers
  - `src/cli/` - Command-line interface implementation
- `design/` - Design documents and technical specifications
- `PLAN.md` - Phased work queue; `BUGS.md` - defect log; `CHANGELOG.md` - changes, plus a record of removed and replaced files
- `authors/<author>/<world-name>/` - World content, one directory per author. The shipped example is `authors/mx/test-world/`.
  - Example locations: `castle/courtyard.md`, `forest/path.md`
  - Character files: `npcs/king.md`, `npcs/guard-captain.md`
  - World configuration: `world.toml`

## Architecture Principles

- All world entities are defined in Markdown files with YAML frontmatter (see `design/CONTENT_FORMAT.md`)
- URIs are world locations that read well to humans
- Components include rooms, objects, NPCs, locations, plot points, and narrative arcs
- Location descriptions and metadata map to file/folder structure
- Character and object cards can be placed in locations or globally

## Priority 0 (P0) Modules

1. **Command Line Document Processor** - Shares models with other components, provides world state visualization, debugging tools, and ASCII map generation
2. **Shared Models and Components** - World engine, character engine, narrative engine, and game engine, including action processing and narrative state (this is broader than "document processor" alone; see `PLAN.md` for the build order within it)

## Priority 1 (P1) Modules

- The console (`src/console/`): one interactive client for writing and playing a world, with an edit mode and a play mode. Authoring first, play second; engine work only as they need it (`design/CONSOLE_REQUIREMENTS.md`)
- Later: RESTful world API abstraction

## Special Configuration

This project has Claude Code permissions configured in `.claude/settings.local.json` that allow specific bash commands (`ls`, `find`, `cat`). When working with this codebase, be aware that certain bash operations may be restricted based on these permission settings.

## Implementation Details

**Technology**: Rust with the following key dependencies:
- `clap` - Command-line argument parsing and subcommands
- `serde` - Serialization for YAML/JSON metadata handling
- `pulldown-cmark` - CommonMark-compliant Markdown parsing
- `walkdir` - Recursive world directory traversal
- `anyhow` - Comprehensive error handling
- `colored` - Terminal output formatting

**CLI Interface**: The `thwart` command has five subcommands. What each one actually does today (versus what's specified in `design/CLI_DESIGN.md` but not yet built) is tracked in `design/ARCHITECTURE.md`'s implementation status table, not here, to avoid a second copy going stale.

**World Structure**: Content is organized in `authors/<author>/<world-name>/`, see `design/CONTENT_FORMAT.md` for file format and `design/ARCHITECTURE.md` for the layout rationale.

## Conventions

**Session lanes.** Three Claude sessions share this checkout, each with its own lane (Bruce, 2026-11-23). Work that crosses a lane is sent to its owner as a message, not edited across lanes.
- *thwarter commander*: docs and design (`README.md`, `REQUIREMENTS.md` with the principles, `design/` apart from the requirements below), the content requirements (`design/CONTENT_REQUIREMENTS.md`), `PLAN.md` and `BUGS.md` entries, conventions, and handing work to the coders.
- *thwarter coder*: the engine, models, parsers and data layers, plus the `thwart` command-line tooling (`src/bin/`, `src/cli/`), with its tests, version bumps and `CHANGELOG.md` entries. Owns `design/ENGINE_REQUIREMENTS.md` and `design/CLI_REQUIREMENTS.md`.
- *thwarter console coder*: the console, the interactive REPL client for reading (playing) and writing (authoring) worlds, including its UX and reload, with its own tests and `CHANGELOG.md` entries. Owns `design/CONSOLE_REQUIREMENTS.md`, which Bruce works on with it directly.

Each component's requirements have one owner (`REQUIREMENTS.md` › 5. Components). A change that crosses components goes through the commander as a message, never as an edit to another owner's document (Bruce, 2026-11-23).

A coder who fixes a bug or finishes a task ticks it in `PLAN.md` or moves it to `BUGS.md` › Fixed in the same commit, and updates `design/ARCHITECTURE.md`'s status table if the change makes an entry true.

**Git.** Commit straight to `main`. Stage only the files a task touched, by name, never `git add -A`. The subject line is one descriptive phrase with no conventional-commit prefix; a docs-only commit starts with the file (`PLAN.md: ...`). The why and how go in the body. Never add Claude as a co-author or a "Generated with Claude Code" line; `.claude/hooks/check-commit-message.sh` refuses those commits. Never force-push, amend a pushed commit or rewrite pushed history.

**Pushing.** Each session pushes its own work to `main` once a task is done, without asking first (Bruce, 2026-11-23, following space-command's convention). A task is usually one commit: the change, its tests, the version bump and the `CHANGELOG.md` entry together. Before committing code, run `cargo test`. If other commits have landed, `git pull` first (Bruce's global git config rebases on pull); commit your own work before pulling, since another session's uncommitted files can block the rebase. A change in `git status` you didn't make belongs to another session or to Bruce: leave it alone, except a checkbox or tag Bruce ticked in `PLAN.md` or `BUGS.md` from Space Command, which you commit along with your own change to that file and mention in the message.

**Tasks.** `PLAN.md` holds phased work (`## Phase N · Name` headings, `- [ ]` items) and `BUGS.md` holds defects (`## Open` and `## Fixed`; keep those headings). Space Command reads both, so a tag written in prose goes in backticks. Each task is one short line pointing to its spec, never the spec itself. Done items are ticked in place, then squashed into one "Done (squashed)" line naming the commit that has the full list. No inline TODO comments as a tracking system.

**Changelog.** `CHANGELOG.md` records changes by version and is also where removed or replaced things go: a deleted file, retired format or superseded plan gets a `### Removed` entry with the `git show <sha>:<path>` that recovers it. Working docs then describe only what's true now, without notes on what their earlier versions said.

**Versions.** `0.x.y` semver in `Cargo.toml`. A patch is a fix, tweak or refactor; a minor adds or removes a capability. Bump with each code change; docs-only changes go under `## Unreleased`. A bump changes `Cargo.lock` too, so commit both together: CI builds with `--locked` and fails on a stale lock file (seen 2026-11-23, after 3234c40, 8b33bce and 64945de).

**Docs.** Root working files are uppercase (`README`, `CLAUDE`, `PLAN`, `BUGS`, `CHANGELOG`, `TESTING`, `REQUIREMENTS`), as are files in `design/`. Writing follows Canadian spelling, no em-dashes, conclusions first.

## Project Status

Do not mark a `PLAN.md` item done without also updating `design/ARCHITECTURE.md`'s implementation status table to match; docs drifting from the code is the problem the August 2026 audit fixed (`CHANGELOG.md` › 0.1.0). Treat `design/ARCHITECTURE.md` as authoritative on what works; this section is a summary of it, not an independent claim.

**Current state** (0.4.0): builds, 58 tests plus console transcripts, `validate` and `show` do real work against real content, `thwart edit` and `thwart play` open a working console (navigation, reload, file watching), and the round-trip writer is built but not yet called by the console. `map`/`search`/`debug` report themselves as not implemented.

**Next**: `PLAN.md` › Phase 2, the console shell.
