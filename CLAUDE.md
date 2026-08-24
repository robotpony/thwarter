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
- `tasks/` - Implementation planning and task tracking
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
2. **Shared Models and Components** - World engine, character engine, narrative engine, and game engine, including action processing and narrative state (this is broader than "document processor" alone; see `design/ARCHITECTURE.md`'s Next Steps for the recommended build order within it)

## Priority 1 (P1) Modules

- Command line REPL client for interactive fiction interface  
- RESTful world API abstraction

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

## Project Status

Do not mark a checklist item done here without also updating `design/ARCHITECTURE.md`'s implementation status table to match; the two describing different realities is exactly the problem an August 2026 audit found and fixed (broken build, five CLI commands that printed fabricated success output regardless of input, sample content in a format the parser couldn't read). Treat `design/ARCHITECTURE.md` as authoritative on what works; this section is a summary of it, not an independent claim.

**Current state**: builds cleanly, zero unit tests, `validate` and `show` do real work against real content, `map`/`search`/`debug` honestly report themselves as not implemented rather than faking output. Full detail in `design/ARCHITECTURE.md`.

**Not ready for P1** until the P0 "Next steps" in `design/ARCHITECTURE.md` are further along, specifically `find_path`/`map`/`search`, then action processing and narrative state.
