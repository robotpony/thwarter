# Command Line Document Processor Interface Design

## Overview

This document defines the command-style interface for the Thwarter command line document processor, supporting interactions in the format `thwart action location --param1=1 --param2=2`.

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
thwart show --player=john --state=save1.json  # Show state from save file
thwart show --format=json                      # Output as JSON
```

**Options:**
- `--player=<name>` - Show from specific player perspective
- `--state=<file>` - Load from saved game state
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

#### `search` - Find Content
```bash
thwart search <query> [location] [options]
```

**Examples:**
```bash
thwart search "golden key"                     # Search for objects/text
thwart search npc castle/                      # Find NPCs in location tree
thwart search --type=character "wizard"        # Search for specific character
thwart search --missing-connections            # Find broken location links
```

**Options:**
- `--type=<character|object|location|text>` - Limit search scope
- `--case-sensitive` - Enable case-sensitive matching
- `--regex` - Use regular expression matching
- `--missing-connections` - Find broken location references

### Interactive Actions

#### `do` - Execute Game Actions
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
path = "authors/world/"
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
```toml
[metadata]
title = "My Interactive Fiction"
author = "Author Name"
version = "1.0.0"

[settings]
starting_location = "home/bedroom"
default_map_style = "ascii"

[characters]
default_health = 100
default_mood = "neutral"
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
# Show current world state
thwart show

# Move player to specific location
thwart move castle/throne-room --save-state=checkpoint1.json

# Execute action and save state
thwart exec "examine throne" --state=checkpoint1.json --save-state=checkpoint2.json

# Generate map from player perspective
thwart map --player=default --output=player_map.txt
```

### Development Workflow
```bash
# Generate new location
thwart generate location castle/secret-chamber --name="Secret Chamber"

# Validate the new content
thwart validate castle/secret-chamber

# Test the location
thwart test --script=chamber_tests.yml castle/secret-chamber

# Generate updated world map
thwart map --output=updated_map.txt
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