# Command Line Document Processor Interface Design

## Overview

This document defines the command-style interface for the Thwarter command line document processor, supporting interactions in the format `thwart action location --param1=1 --param2=2`.

## Status

For what's built, see the implementation status table in `design/ARCHITECTURE.md`. As of 0.62: `validate`, `show`, `map` and `search` do real work; `edit` and `play` open the console (`design/CONSOLE_REQUIREMENTS.md`); `package`, `install` and `installed` share worlds (`design/CLI_REQUIREMENTS.md` › REQ-CLI-12 to 16); `debug` says it isn't implemented. `go`, `do`, `test` and `create` were never built, and the console replaced them: playing is `thwart play`, making things is `/new` and `/add` in `thwart edit`, and tests are console transcripts. Their sections below are kept as ideas, marked *not planned*.

## Command Structure

```
thwart <action> [location] [options]
```

### Base Command: `thwart`

The primary executable that provides access to all interactive fiction engine functionality.

## Core Actions

### World State Actions

#### `show` - Display World State
```bash
thwart show [location] [options]
```

**Examples:**
```bash
thwart show                                    # Show current player state
thwart show castle/throne-room                 # Show specific location state
thwart show castle/vault --state open --chapter after-the-king  # One version of a thing
thwart show master-sword                       # Any thing, by id or link
thwart show --template scenery                 # A template or built-in
thwart show --format=json                      # Output as JSON
```

**Options:**
- `--player=<name>` - Show from specific player perspective
- `--state=<name>` - Show the thing in that state (`design/STATES.md`)
- `--chapter=<name>` - Show the thing in that chapter, resolved from the thing outward
- `--kind=<location|object|character>` - Which kind, when an id is shared across kinds
- `--template=<name>` - Print a template or built-in as written, with where it came from
- `--format=<text|json|yaml>` - Output format (default: text)
- `--verbose` - Include detailed object/character information

#### `go` - Simulate Movement
```bash
thwart go <location> [options]
```

**Examples:**
```bash
thwart go castle/throne-room                 # Move to location
thwart go north --from=castle/courtyard      # Move in direction from location
thwart go --player=alice castle/library      # Move specific player
```

**Options:**
- `--from=<location>` - Specify starting location
- `--player=<name>` - Move specific player
- `--dry-run` - Show what would happen without changing state
- `--save-state=<file>` - Save resulting state to file

### World Analysis Actions

#### `validate` - Validate World Files
```bash
thwart validate [location] [options]
```

**Examples:**
```bash
thwart validate                                # Validate entire world
thwart validate castle/                        # Validate specific location tree
thwart validate --strict                       # Enable strict validation
thwart validate --output=report.json          # Save validation report
```

**Options:**
- `--strict` - Enable strict validation rules
- `--output=<file>` - Save validation report
- `--format=<text|json|yaml>` - Report format
- `--fix` - Attempt to auto-fix common issues

#### `map` - Generate Maps
```bash
thwart map [location] [options]
```

**Examples:**
```bash
thwart map                                     # Generate full world map
thwart map castle/                             # Generate map from location
thwart map --player=john                       # Player perspective (FOW)
thwart map castle/ --depth=2                   # Limit traversal depth
thwart map --output=worldmap.txt               # Save to file
```

**Options:**
- `--player=<name>` - Generate from player perspective (fog-of-war)
- `--depth=<n>` - Limit map depth/radius
- `--style=<ascii|unicode|compact>` - Map rendering style
- `--output=<file>` - Save map to file
- `--connections` - Show connection types between locations

`--player`, `--style` other than ascii, and `--connections` aren't built yet; each says so (REQ-CLI-7, REQ-CLI-17).

**Reading the map:**
- `@` marks the location the map starts from
- `?` marks an exit to a location that isn't written yet
- `+` marks a door, `#` a closed door
- `Also:` lists, below the grid, exits the grid can't draw (diagonals that collide, `up`, `behind` and other free-text directions)

#### `search` - Find Content
```bash
thwart search <query> [location] [options]
```

**Examples:**
```bash
thwart search "golden key"                     # Search for objects/text
thwart search --type=character castle/         # List the characters under castle/
thwart search castle/                          # List everything under castle/
thwart search --type=character guard castle/   # Characters under castle/ matching "guard"
thwart search --type=character "wizard"        # Search for specific character
thwart search --type=door gate                 # Doors matching "gate"
thwart search --missing-connections            # Find broken location links
```

**Options:**
- `--type=<location|object|character|door|text>` - Limit search scope; `text` searches prose only
- `--case-sensitive` - Enable case-sensitive matching (the default ignores case)
- `--regex` - Use regular expression matching (linear-time, so no pattern can hang it)
- `--missing-connections` - Find broken location references

The query matches ids, titles and prose, including object responses. A location limits the search to rooms in that folder, what they hold, and characters who start there. With no query, or an empty one, search lists everything in scope; `--type text` is the exception and needs a query. A single bare word is a location only when the world has a folder with exactly that name (case included) and nothing has it as an id; otherwise it's the query. No matches isn't an error (REQ-CLI-18).

### Interactive Actions

#### `do` - Execute Game Actions

*Not planned*: `thwart play` and its saves cover this.

```bash
thwart do <action> [location] [options]
```

**Examples:**
```bash
thwart do "take key" castle/treasury         # Execute action at location
thwart do "talk wizard" --player=alice       # Execute as specific player
thwart do "look around" --state=game.json    # Execute from saved state
```

**Options:**
- `--player=<name>` - Execute as specific player
- `--state=<file>` - Load initial state from file
- `--save-state=<file>` - Save resulting state
- `--interactive` - Enter interactive mode after execution

#### `test` - Run Test Scenarios

*Not planned*: console transcripts (`tests/transcripts/`, REQ-CON-9) are the test scenarios.

```bash
thwart test <scenario> [location] [options]
```

**Examples:**
```bash
thwart test walkthrough                        # Run full game walkthrough
thwart test combat castle/dungeon              # Test combat scenarios
thwart test --script=test_suite.yml            # Run scripted test sequence
thwart test dialogue --character=wizard        # Test character dialogue
```

**Options:**
- `--script=<file>` - Load test script
- `--character=<name>` - Focus tests on specific character
- `--verbose` - Show detailed test output
- `--continue-on-error` - Don't stop on first test failure

### Development Actions

#### `debug` - Debugging Tools
```bash
thwart debug [location] [options]
```

**Examples:**
```bash
thwart debug                                   # Run full diagnostic
thwart debug castle/ --check-references        # Check location references
thwart debug --memory-usage                    # Show memory usage stats
thwart debug --trace-action="take key"         # Trace action execution
```

**Options:**
- `--check-references` - Validate all location/character references
- `--memory-usage` - Show memory usage statistics
- `--trace-action=<action>` - Trace specific action execution
- `--profile` - Enable performance profiling

#### `create` - Content Generation

*Not planned*: the console's `/new` and `/add` make things, with question sets (REQ-CF-32).

```bash
thwart create <template> <location> [options]
```

**Examples:**
```bash
thwart create location castle/new-room       # Generate location template
thwart create character npcs/guard           # Generate character template
thwart create world --name="My Adventure"    # Generate new world structure
```

**Options:**
- `--name=<name>` - Set name for generated content
- `--template=<file>` - Use custom template
- `--overwrite` - Overwrite existing files
- `--dry-run` - Show what would be generated

## Global Options

These options work with all commands:

- `--world=<path>` - Specify world directory (default: current directory)
- `--config=<file>` - Use custom configuration file
- `--quiet` - Suppress non-essential output
- `--verbose` - Enable verbose output
- `--help` - Show command help
- `--version` - Show version information

## Configuration

### Global Configuration File: `thwart.toml`
```toml
[world]
path = "authors/mx/test-world/"
default_player = "player"

[output]
format = "text"
style = "unicode"

[validation]
strict_mode = false
auto_fix = false

[debug]
trace_level = "info"
profile_enabled = false
```

### Per-World Configuration: `world.toml`

Tool and play settings only. The world's title, author and starting room are in `world/index.md` (`design/STATES.md` › world/), and `[play]` is in `design/CONTENT_FORMAT.md` › How play words a room. The test world's `world.toml` lists every key at its default.

```toml
[settings]
default_map_style = "ascii"

[play]
show_room_names = false
exits = "You can go {or}."
```

## Error Handling

### Exit Codes
- `0` - Success
- `1` - General error
- `2` - Command line argument error
- `3` - World file validation error
- `4` - Missing file/location error
- `5` - Game state error

### Error Output
All errors are written to stderr with structured format:
```
ERROR [location]: Description of error
  Suggestion: How to fix the issue
```

## Examples Workflow

### Basic World Validation
```bash
# Validate entire world
thwart validate

# Check specific location tree
thwart validate castle/ --strict

# Generate validation report
thwart validate --output=validation_report.json --format=json
```

### Interactive Testing
```bash
# Show the world, or one room
thwart show
thwart show bedroom

# Play it, or write it, in the console
thwart play
thwart edit
```

### Development Workflow
```
# In the console (thwart edit), from the bedroom:
in bedroom > /new room north closet
in bedroom > /validate

# Then, from the shell:
thwart map
```

## Implementation Notes

### Command Parsing
- Use `clap` crate for robust argument parsing
- Support both `--param=value` and `--param value` syntax
- Implement command aliases for common operations
- Provide shell completion support

### Location Resolution
- Support both relative and absolute location URIs
- Handle location wildcards (`castle/*`)
- Resolve current working location context
- Validate location existence before operations

### State Management
- Support loading/saving game state in JSON/YAML formats
- Implement state diffing for debugging
- Provide state rollback capabilities
- Handle concurrent access to state files

### Output Formatting
- Support multiple output formats (text, JSON, YAML)
- Implement colorized console output
- Provide machine-readable formats for tooling integration
- Support output redirection and piping