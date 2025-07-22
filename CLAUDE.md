# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Thwarter is a Hugo-style engine for interactive fiction (IF), driven by Markdown files and YAML/JSON metadata. The system breaks down storytelling into reusable components like variables, including maps, alternative text, and other common IF mechanisms.

## Project Structure

- `src/` - Rust source code for the IF engine
  - `src/bin/` - CLI binary executable (`thwart`)  
  - `src/models/` - Core data models (World, Character, Narrative, Game)
  - `src/engines/` - Processing engines for game logic
  - `src/parsers/` - Markdown and metadata file parsers
  - `src/cli/` - Command-line interface implementation
- `design/` - Design documents and technical specifications
- `tasks/` - Implementation planning and task tracking
- `authors/world/` - Sample world content with locations and characters
  - Example locations: `castle/courtyard.md`, `forest/path.md`
  - Character files: `npcs/king.md`, `npcs/guard-captain.md`
  - World configuration: `world.toml`

## Architecture Principles

- All world entities are defined in Markdown files
- URIs are world locations that read well to humans
- Components include rooms, objects, NPCs, locations, plot points, and narrative arcs
- Location descriptions and metadata map to file/folder structure
- Character and object cards can be placed in locations or globally

## Priority 0 (P0) Modules

1. **Command Line Document Processor** - Shares models with other components, provides world state visualization, debugging tools, and ASCII map generation
2. **Shared Models and Components** - World engine, character engine, narrative engine, and game engine

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
- `anyhow` - Comprehensive error handling
- `colored` - Terminal output formatting

**CLI Interface**: The `thwart` command provides these core operations:
- `thwart validate` - Validate world files and check references
- `thwart show` - Display world state and location information
- `thwart map` - Generate ASCII maps with optional fog-of-war
- `thwart search` - Find content across locations and characters
- `thwart debug` - Debugging tools and diagnostics

**World Structure**: Content is organized in `authors/world/` with:
- Location files: `location-name.md` with YAML frontmatter metadata
- Character files: `npcs/character-name.md` with attributes and dialogue
- World configuration: `world.toml` for global settings
- CLI configuration: `thwart.toml` for tool preferences

## Project Status

**P0 Implementation: COMPLETE** ✅
- ✅ Rust project structure with all modules
- ✅ Core data models for World, Character, Narrative, and Game entities
- ✅ Four processing engines (World, Character, Narrative, Game)
- ✅ Markdown/YAML parsing infrastructure
- ✅ Complete CLI with all five core commands
- ✅ Output formatting system (text, JSON, YAML)
- ✅ Configuration management system
- ✅ Sample world content with castle and forest locations

**Ready for P1 Development**: The foundation is complete and ready for:
- REPL client implementation
- RESTful API development  
- Advanced world loading and validation
- Enhanced map generation and visualization