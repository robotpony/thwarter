# Thwarter CLI First Pass Implementation Plan

## Overview

This document outlines the implementation plan for the first pass of the Thwarter command line tool based on the P0 requirements from `design/DESIGN.md` and the interface specification from `design/CLI_DESIGN.md`.

## Project Structure

```
src/
├── bin/
│   └── thwart.rs              # Main CLI executable entry point
├── lib/
│   ├── models/                # Shared data models (P0 Module 2)
│   │   ├── world/
│   │   │   ├── location.rs
│   │   │   ├── world_graph.rs
│   │   │   └── coordinate.rs
│   │   ├── character/
│   │   │   ├── character.rs
│   │   │   ├── player.rs
│   │   │   └── character_state.rs
│   │   ├── narrative/
│   │   │   ├── plot_point.rs
│   │   │   ├── narrative_arc.rs
│   │   │   ├── game_state.rs
│   │   │   └── story_variable.rs
│   │   ├── game/
│   │   │   ├── game.rs
│   │   │   ├── game_session.rs
│   │   │   └── action.rs
│   │   └── mod.rs
│   ├── engines/               # Core engines
│   │   ├── world_engine.rs
│   │   ├── character_engine.rs
│   │   ├── narrative_engine.rs
│   │   ├── game_engine.rs
│   │   └── mod.rs
│   ├── parsers/               # File parsing utilities
│   │   ├── markdown_parser.rs
│   │   └── metadata_parser.rs
│   ├── cli/                   # CLI-specific functionality
│   │   ├── commands/
│   │   │   ├── validate.rs
│   │   │   ├── show.rs
│   │   │   ├── map.rs
│   │   │   ├── search.rs
│   │   │   ├── debug.rs
│   │   │   └── mod.rs
│   │   ├── config.rs          # Configuration management
│   │   ├── output.rs          # Output formatting
│   │   └── mod.rs
│   └── lib.rs
└── Cargo.toml
```

## Implementation Phases

### Phase 1: Foundation Setup (Days 1-2)

**Goal**: Establish basic project structure and dependencies

**Tasks**:
1. Initialize Rust project with proper Cargo.toml
2. Set up external dependencies:
   - `clap` (CLI argument parsing)
   - `serde` (serialization/deserialization)
   - `pulldown-cmark` (Markdown parsing)
   - `anyhow` (error handling)
   - `toml` (configuration files)
   - `colored` (terminal output)
3. Create basic module structure
4. Implement basic CLI scaffolding with clap
5. Create configuration system (thwart.toml, world.toml)

### Phase 2: Core Data Models (Days 3-5)

**Goal**: Implement all P0 shared models

**Priority Order**:
1. **World Models**:
   - `Location` struct with metadata and connections
   - `Coordinate` for spatial positioning
   - `WorldGraph` for managing location relationships

2. **Character Models**:
   - `Character` base struct with attributes
   - `Player` extended character with inventory
   - `CharacterState` for dynamic properties

3. **Narrative Models**:
   - `StoryVariable` for dynamic story elements
   - `GameState` for overall progress tracking
   - `PlotPoint` for story decision nodes
   - `NarrativeArc` for long-term progression

4. **Game Models**:
   - `Action` for player action representation
   - `GameSession` for individual play sessions
   - `Game` top-level coordinator

### Phase 3: Parsing Infrastructure (Days 6-7)

**Goal**: Implement file parsing for world data

**Tasks**:
1. **Markdown Parser**:
   - Parse location markdown files
   - Extract YAML frontmatter metadata
   - Handle character card files
   - Support alternative text blocks

2. **Metadata Parser**:
   - Parse YAML/JSON metadata sections
   - Validate required fields
   - Handle optional configuration

3. **File System Integration**:
   - Scan world directory structures
   - Build file-to-URI mapping
   - Handle nested location hierarchies

### Phase 4: Core Engines (Days 8-12)

**Goal**: Implement the four core engines

**Priority Order**:
1. **World Engine** (Days 8-9):
   - Location loading and caching
   - Path resolution between locations
   - Spatial relationship management
   - URI-based addressing

2. **Character Engine** (Day 10):
   - Character placement and movement
   - Dialogue tree management
   - Character state tracking

3. **Narrative Engine** (Day 11):
   - Story variable management
   - Plot point triggering
   - Alternative text resolution
   - Game state transitions

4. **Game Engine** (Day 12):
   - Engine coordination
   - Action processing pipeline
   - Save/load state management
   - Event handling

### Phase 5: CLI Commands Implementation (Days 13-17)

**Goal**: Implement core CLI functionality

**Priority Order**:
1. **validate** command (Day 13):
   - World file validation
   - Reference checking
   - Error reporting
   - Auto-fix capabilities

2. **show** command (Day 14):
   - World state display
   - Player perspective views
   - Multiple output formats
   - State loading from files

3. **map** command (Day 15):
   - ASCII map generation
   - Fog-of-war support
   - Different rendering styles
   - Depth limiting

4. **search** command (Day 16):
   - Content searching
   - Type-specific filtering
   - Reference validation
   - Missing connection detection

5. **debug** command (Day 17):
   - Diagnostic reporting
   - Memory usage tracking
   - Action tracing
   - Performance profiling

### Phase 6: Advanced Features & Polish (Days 18-20)

**Goal**: Complete remaining features and polish

**Tasks**:
1. **Additional Commands**:
   - `go` - Movement simulation
   - `do` - Action execution
   - `test` - Test scenario running

2. **Output Enhancement**:
   - Colorized terminal output
   - JSON/YAML format support
   - File output redirection

3. **Configuration System**:
   - Global and per-world configuration
   - Command-line option precedence
   - Environment variable support

4. **Error Handling**:
   - Comprehensive error types
   - Structured error output
   - Helpful suggestion messages
   - Proper exit codes

## First Pass Feature Subset

For the initial implementation, focus on these core features:

### Must-Have (P0 Critical)
- `thwart validate` - Basic world file validation
- `thwart show` - Simple world state display  
- `thwart map` - Basic ASCII map generation
- Location and character model loading
- Markdown/YAML parsing
- World and character engines

### Should-Have (P0 Important)
- `thwart search` - Content search functionality
- `thwart debug` - Basic diagnostics
- Multiple output formats (text, JSON)
- Configuration file support
- Proper error handling and reporting

### Could-Have (P0 Nice-to-Have)
- `thwart go` - Movement simulation
- `thwart do` - Action execution
- Advanced map rendering styles
- Performance profiling
- Auto-fix validation issues

### Won't-Have (P1 Features)
- `thwart create` - Content generation
- `thwart test` - Advanced test scenarios
- Interactive mode
- Shell completion
- Complex state management

## Success Criteria

### Functional Requirements
- [ ] Load and validate world files from `authors/world/` directory
- [ ] Parse location markdown with YAML frontmatter
- [ ] Parse character cards and place in locations
- [ ] Generate ASCII maps showing location connections
- [ ] Display world state from any location perspective
- [ ] Detect and report broken location references
- [ ] Support basic configuration via toml files

### Technical Requirements
- [ ] Proper error handling with structured output
- [ ] Modular architecture supporting P1 extensions  
- [ ] Memory-efficient file loading and caching
- [ ] Cross-platform compatibility (Linux, macOS, Windows)
- [ ] Documentation for all public APIs

### Quality Requirements
- [ ] Comprehensive error messages with suggestions
- [ ] Fast startup time (< 1 second for basic commands)
- [ ] Handles malformed input gracefully
- [ ] Consistent CLI interface following design specification

## Development Notes

### Architecture Decisions
- Use trait-based design for extensibility
- Implement lazy loading for large world files
- Cache parsed content to improve performance
- Use Result types throughout for error handling

### Testing Strategy
- Unit tests for all model structs
- Integration tests for file parsing
- CLI command tests with sample world data
- Create test world in `authors/test-world/` for validation

### Documentation
- Inline documentation for all public APIs
- Usage examples in CLI help text
- Architecture documentation in `design/IMPLEMENTATION.md`
- Sample world structure in `examples/`

## File Dependencies

This implementation plan requires these existing files to be present:
- `design/DESIGN.md` - Technical specification
- `design/CLI_DESIGN.md` - CLI interface specification  
- `CLAUDE.md` - Project guidelines and architecture principles
- `authors/world/` - World content directory (to be created)

This plan provides a structured approach to building the first pass of the Thwarter CLI tool while adhering to the P0 requirements and maintaining flexibility for future P1 enhancements.