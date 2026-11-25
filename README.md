# Thwarter Interactive Fiction

Thwarter is a Hugo style engine for interactive fiction (IF), driven by Markdown files and YAML/JSON metadata. The system breaks down storytelling into components that can be reused like variables, including maps, alternative text, and other common IF mechanisms.

## Architecture


### Principles

- All world entities are defined in Markdown files with YAML frontmatter; see `design/CONTENT_FORMAT.md` for the field-by-field spec
    - location descriptions and metadata are in files and folders that map to locations
    - character cards can be placed in locations, or globally as core NPCs, villains, and heroes
    - object cards behave similarly, but carry different fields (weight, value, actions) since object content varies more than location or character content
    - plot points are used for authors to track scope, and to define story inflection points, decisions, and other "variables"
    - narrative arcs behave similarly, but track long running themes
- URIs are world locations that read well to humans
- Concepts for rooms, objects, NPCs, locations, etc.

See `design/ARCHITECTURE.md` for what's actually implemented today versus still planned; several of the above (plot points, narrative arcs, dialogue) exist as data models but have no working engine behind them yet.

### Modules and services

- (P0) command line document processor that shares models with other components
    - can show the current world and game in any given state (for testing)
    - can help debug and diagnose story files (linting, path checking, etc.)
    - can show specific states and screens as needed
    - can draw simple ASCII maps with FOW (fog of war) style visibility, or entire world-view (for testing and the REPL client)
- (P0) shared models and components (world engine, character engine, narrative engine, game engine)
- (P1) the console, one interactive terminal client for writing a world (edit mode) and playing it (play mode), opened with `thwart edit` or `thwart play`; authoring comes first
- (later) RESTful world API as an abstraction to the models (for the console and future clients), following RESTful API norms.


## Basic Usage

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd thwarter

# Build the project
cargo build --release
```

### Quick Start

World content lives at `authors/<your-name>/<world-name>/` (see World Structure below). There's no built-in default world: a command uses `--world`, then `[world] path` in `thwart.toml` (or your personal `config.toml`), then the world the current folder is in. With none of those, it lists the worlds it can find and stops (REQ-CLI-21).

```bash
# Start a world: authors/<you>/tiny-castle, with you as its author
thwart new tiny-castle

# Write it, or play it, in the console
thwart edit authors/mx/tiny-castle
thwart play authors/mx/tiny-castle

# Validate a world's files (broken connections, parse errors)
thwart validate --world authors/mx/test-world

# Show the whole world state, or one location
thwart show --world authors/mx/test-world
thwart show --world authors/mx/test-world bedroom
```

`map` draws the world as ASCII and `search` finds things by id, title or prose. `debug` is specified in `design/CLI_DESIGN.md` but not implemented yet; running it reports that honestly rather than printing placeholder output. See `design/ARCHITECTURE.md` for current status.

### World Structure

Each author's content lives under their own name, so a single repo can hold more than one author's world:

```
authors/
└── mx/
    └── test-world/
        ├── world.toml          # Tool settings
        ├── world/
        │   └── index.md        # The world: title, author, starting room
        ├── bedroom.md          # Location files
        ├── closet.md
        ├── hallway.md
        └── objects/
            ├── walls.md        # Object files
            └── photo-1.md
thwart.toml                     # CLI preferences, including an optional default world path
```

### Example Location File

See `design/CONTENT_FORMAT.md` for the full field-by-field spec across locations, characters, and objects. A minimal location:

```markdown
---
id: bedroom
title: Bedroom
type: location
connections:
  north: closet
objects: [chair, bed, old-shirt]
---

You are in a small bedroom. It's not all that interesting, yet here you are.
```

#### Future components that are out of scope

- web frontend
- databases and data stores (we'll work in memory, directly from files for now)