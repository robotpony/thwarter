# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Thwarter is a Hugo-style engine for interactive fiction (IF), driven by Markdown files and YAML/JSON metadata. The system breaks down storytelling into reusable components like variables, including maps, alternative text, and other common IF mechanisms.

## Project Structure

- `src/` - Source code for the IF engine
- `bin/` - Binary/executable files
- `tasks/` - Task-related files
- `design/` - Design documents
- `authors/world/` - per-author worlds are stored in folders and files here

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

## Project Status

Project is in initial development phase with architecture defined. Focus on implementing P0 modules first: command line processor and shared model components.