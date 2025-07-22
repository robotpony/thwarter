# Thwarter Interactive Fiction

Thwarter is a Hugo style engine for interactive fiction (IF), driven by Markdown files and YAML/JSON metadata. The system breaks down storytelling into components that can be reused like variables, including maps, alternative text, and other common IF mechanisms.

## Architecture


### Principles

- All world entities are defined in Markdown files
    - location descriptions and metadata are in files and folders that map to locations
    - character cards can be placed in locations, or globally as core NPCs, villans, and heros
    - object cards behave similarly (and are really the same thing, but with different defaults)
    - plot points are used for authors to track scope, and to define story inflection points, decisions, and other "variables"
    - narrative arcs behave similarly, but track long running themes
- URIs are world locations that read well to humans
- Concepts for rooms, objects, NPCs, locations, etc.

### Modules and services

- (P0) command line document processor that shares models with other components
    - can show the current world and game in any given state (for testing)
    - can help debug and diagnose story files (linting, path checking, etc.)
    - can show specific states and screens as needed
    - can draw simple ASCII maps with FOW (fog of war) style visibility, or entire world-view (for testing and the REPL client)
- (P0) shared models and components (world engine, character engine, narrative engine, game engine)
- (P1) command line REPL client, a fully interactive variant of the document processor that provides a standard interactive fiction interface
- (P1) RESTful world API as an abstration to the models (for the REPL and future clients), following RESTful API norms.


#### Future components that are out of scope

- web frontend
- databases and data stores (we'll work in memory, directly from files for now)