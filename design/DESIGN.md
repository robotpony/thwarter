# Design

This document used to hold the full P0 design: language rationale, module architecture, and the content file format, all in one file. It's been split so each piece has a single home and a single source of truth:

- **`design/DECISIONS.md`** — standing decisions (language choice, format choice) with context, alternatives, and trade-offs
- **`design/ARCHITECTURE.md`** — module layout, engine responsibilities, and current implementation status
- **`design/CONTENT_FORMAT.md`** — the content file format authors write against
- **Requirements, one per component:** `design/CONTENT_REQUIREMENTS.md`, `design/ENGINE_REQUIREMENTS.md`, `design/CLI_REQUIREMENTS.md`, `design/CONSOLE_REQUIREMENTS.md`; the overview and principles are in `REQUIREMENTS.md`
- **`design/STATES.md`** — states, bundles and chapters: how things change over a story (draft)
- **`design/UNIT_TESTS.md`** — the unit test plan, in priority order
- **`design/CLI_DESIGN.md`** — the command-line interface spec, with a status column showing what's actually built

Start with `design/ARCHITECTURE.md`'s implementation status table if you want to know what works today versus what's still planned.
