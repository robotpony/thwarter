# Thwarter P0 Design Document

## Overview

This document outlines the design for Priority 0 (P0) modules of the Thwarter interactive fiction engine. These modules form the foundation for all other components and must be implemented first.

## Technology Choice: Rust

### Rationale

Rust was selected as the implementation language for the Thwarter interactive fiction engine based on several key requirements and architectural considerations:

#### Performance Requirements
- **Fast Startup Time**: Target of < 1 second startup for CLI tools
- **Efficient Memory Usage**: Handle large world files and complex game state efficiently
- **Zero-Cost Abstractions**: Maintain performance while providing high-level game engine abstractions

#### Memory Safety and Reliability
- **Safe World State Management**: Prevent memory leaks when managing complex, interconnected game world data
- **Concurrent Processing**: Safe handling of multi-threaded operations for world updates and user interactions
- **Robust Content Handling**: Memory-safe processing of user-generated markdown and metadata files

#### Rich Ecosystem
- **CLI Framework**: `clap` for sophisticated command-line interfaces with subcommands and argument validation
- **Serialization**: `serde` for seamless YAML/JSON metadata processing and game state persistence
- **Markdown Processing**: `pulldown-cmark` for reliable CommonMark-compliant parsing
- **Error Handling**: `anyhow` for comprehensive, user-friendly error reporting and propagation

#### Cross-Platform Compatibility
- **Universal Deployment**: Single binary distribution across Linux, macOS, and Windows
- **No Runtime Dependencies**: Simplified deployment and installation for content creators
- **Consistent Behavior**: Uniform performance and functionality across all target platforms

#### Error Handling and Developer Experience
- **Result Types**: Comprehensive error propagation throughout the system
- **Graceful Degradation**: Clear error messages for malformed world files and invalid content
- **Type Safety**: Compile-time guarantees for world consistency and data integrity

### Architectural Benefits

The choice of Rust enables the modular architecture outlined in this document, where each engine (World, Character, Narrative, Game) can be implemented as safe, performant, and composable components. The language's ownership model naturally aligns with the game world's entity relationships, while its trait system supports the extensible design required for diverse interactive fiction mechanics.

## P0 Module 1: Command Line Document Processor

### Purpose
A command line tool that processes and validates IF story files, provides debugging capabilities, and visualizes world state.

### Key Features

#### World State Visualization
- Display current game state in any given condition
- Show player location, inventory, character positions
- Render world state as structured text output

#### Debugging and Diagnostics
- Lint story markdown files for syntax and structural errors
- Validate path references between locations
- Check for orphaned locations or unreachable content
- Verify character and object placement consistency

#### ASCII Map Generation
- Generate simple ASCII maps of the world
- Support fog-of-war (FOW) style visibility from player perspective
- Provide full world-view mode for testing and debugging
- Handle multi-level/hierarchical location structures

#### Testing Support
- Load and display specific game states
- Simulate player actions and show resulting state changes
- Export world state for testing purposes

### Technical Architecture

```
thwarter-cli
├── commands/
│   ├── validate.rs    # Story file validation
│   ├── visualize.rs   # World state display
│   ├── map.rs         # ASCII map generation
│   └── debug.rs       # Debugging utilities
├── parsers/
│   ├── markdown.rs    # Markdown file parsing
│   └── metadata.rs    # YAML/JSON metadata parsing
└── main.rs            # CLI entry point
```

### Command Interface
```bash
thwarter validate <story-dir>          # Validate story files
thwarter show --state <state-file>     # Show world state
thwarter map --location <uri>          # Generate map from location
thwarter map --full                    # Generate full world map
thwarter debug <story-dir>             # Run diagnostics
```

## P0 Module 2: Shared Models and Components

### Purpose
Core data structures and engines that model the interactive fiction world and game mechanics.

### Engine Components

#### World Engine
**Responsibility**: Manages locations, spatial relationships, and world structure

**Core Models**:
- `Location` - Represents rooms, areas, and spatial containers
- `WorldGraph` - Manages location connections and navigation
- `Coordinate` - Handles multi-dimensional positioning

**Key Functions**:
- Location loading from markdown files
- Path resolution between locations
- Spatial relationship management
- URI-based location addressing

#### Character Engine
**Responsibility**: Manages NPCs, player characters, and character interactions

**Core Models**:
- `Character` - Base character with attributes, state, dialogue
- `Player` - Extended character with inventory and progress tracking
- `CharacterState` - Dynamic character properties (location, mood, etc.)

**Key Functions**:
- Character card loading and parsing
- Character placement and movement
- Dialogue tree management
- Character interaction resolution

#### Narrative Engine
**Responsibility**: Manages plot points, story progression, and narrative state

**Core Models**:
- `PlotPoint` - Story inflection points and decision nodes  
- `NarrativeArc` - Long-running theme and story progression tracking
- `GameState` - Overall progress and variable state
- `StoryVariable` - Dynamic story elements and flags

**Key Functions**:
- Plot point triggering and resolution
- Narrative arc progression tracking
- Story variable management
- Alternative text resolution based on state

#### Game Engine
**Responsibility**: Orchestrates all other engines and manages game loop

**Core Models**:
- `Game` - Top-level game instance
- `GameSession` - Individual play session with state
- `Action` - Player action representation and resolution

**Key Functions**:
- Game initialization from story files
- Action processing and state transitions
- Engine coordination and event handling
- Save/load state management

### Technical Architecture

```
shared/
├── models/
│   ├── world/
│   │   ├── location.rs
│   │   ├── world_graph.rs
│   │   └── coordinate.rs
│   ├── character/
│   │   ├── character.rs
│   │   ├── player.rs
│   │   └── character_state.rs
│   ├── narrative/
│   │   ├── plot_point.rs
│   │   ├── narrative_arc.rs
│   │   ├── game_state.rs
│   │   └── story_variable.rs
│   └── game/
│       ├── game.rs
│       ├── game_session.rs
│       └── action.rs
├── engines/
│   ├── world_engine.rs
│   ├── character_engine.rs
│   ├── narrative_engine.rs
│   └── game_engine.rs
├── parsers/
│   ├── markdown_parser.rs
│   └── metadata_parser.rs
└── lib.rs
```

### Data Format Specifications

#### Location Markdown Format
```markdown
# Location Name

Description of the location that players will see.

---
location_id: unique-location-uri
coordinates: [x, y, z]  # optional
connections:
  north: another-location-uri
  south: different-location-uri
characters:
  - character-id-1
  - character-id-2
objects:
  - object-id-1
---
```

#### Character Card Format
```markdown
# Character Name

Character description and background.

---
character_id: unique-character-id
type: npc  # or player, villain, hero
default_location: location-uri  # optional
attributes:
  health: 100
  mood: neutral
dialogue_tree: character-dialogue.yml
---
```

## Implementation Dependencies

### External Dependencies
- `clap` - Command line argument parsing
- `serde` - Serialization/deserialization for YAML/JSON
- `pulldown-cmark` - Markdown parsing
- `anyhow` - Error handling

### Internal Dependencies
- Command Line Processor depends on all Shared Model components
- All engines depend on their respective model components
- Game Engine orchestrates and depends on all other engines

## Success Criteria

### Command Line Processor
- [ ] Successfully validate well-formed story directories
- [ ] Detect and report common story file errors  
- [ ] Generate readable ASCII maps for test worlds
- [ ] Display accurate world state for any game position

### Shared Models
- [ ] Load location data from markdown files
- [ ] Manage character placement and movement
- [ ] Track narrative state and plot progression
- [ ] Coordinate all engines through unified Game interface

## Next Steps (P1)

After P0 completion, the foundation will support:
1. REPL client implementation using the Game Engine
2. RESTful API layer exposing the shared models
3. Advanced debugging and development tools