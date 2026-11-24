# Design

This document used to hold the full P0 design: language rationale, module architecture, and the content file format, all in one file. It's been split so each piece has a single home and a single source of truth:

- **`design/DECISIONS.md`**: standing decisions (language choice, format choice) with context, alternatives, and trade-offs
- **`design/ARCHITECTURE.md`**: module layout, engine responsibilities, and current implementation status
- **`design/CONTENT_FORMAT.md`**: the content file format authors write against
- **Requirements, one per component:** `design/CONTENT_REQUIREMENTS.md`, `design/ENGINE_REQUIREMENTS.md`, `design/CLI_REQUIREMENTS.md`, `design/CONSOLE_REQUIREMENTS.md`; the overview and principles are in `REQUIREMENTS.md`
- **`design/STATES.md`**: states, bundles, chapters, doors and keys, how things change over a story
- **`design/SECTIONS.md`**: `## When:` sections, scripts and replies, under trial
- **`design/PLAYER_COMMANDS.md`**: the command language every player client shares (REQ-PC)
- **`design/ENGINE_TEXT.md`**: every sentence as data in text tables (Phase 8)
- **`design/UNIT_TESTS.md`**: the testing approach
- **`design/CLI_DESIGN.md`**: the command-line interface, as built and as ideas

Start with `design/ARCHITECTURE.md`'s implementation status table if you want to know what works today versus what's still planned.
