# Testing Guide

What you can run today, with a pointer to what's planned. The test plan is `design/UNIT_TESTS.md`.

## Unit testing with Cargo

```bash
cargo test                  # all tests
cargo test -- --nocapture   # with println! output visible
cargo test parsers::        # one module
cargo test --release        # release mode
```

There are currently zero unit tests. See `design/UNIT_TESTS.md` for what's planned and in what order.

## Console transcripts

The console's acceptance tests (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-9) are transcripts in `tests/transcripts/*.txt`: a console session written exactly as the console prints it, with input lines as `> line`. Each starts with a `# world:` and `# mode:` header and runs against a fixture world in `tests/transcripts/worlds/`. The format is described in the header comment of `tests/transcripts.rs`.

```bash
cargo test --test transcripts                     # run every transcript
THWART_BLESS=1 cargo test --test transcripts      # accept new output after a deliberate change, then review the diff
```

To add one, write the session you expect as a new `.txt` file. A transcript is the place to pin a console behaviour that a unit test can't reach.

A transcript that changes content (`/new`, `/set`, `/link`, `/unlink`) MUST add `# copy: yes` to its header, so it runs against a temporary copy of the fixture world rather than the fixture itself. The runner fails a transcript that uses those commands without it. Saves made during a transcript go to a temporary folder, never `~/.local/state`.

## Exercising the CLI against real content

The commands below are real: they're the ones `src/bin/thwart.rs` actually defines, run against the sample world at `authors/mx/test-world/`.

```bash
# Validate the whole world: parse errors and broken location references
thwart validate --world authors/mx/test-world

# Same, plus checking character default_location against the world graph
thwart validate --world authors/mx/test-world --strict

# Show every loaded location's id
thwart show --world authors/mx/test-world

# Show one location's full parsed data
thwart show --world authors/mx/test-world castle/courtyard

# Same, as JSON
thwart show --world authors/mx/test-world castle/courtyard --format json
```

Expected result against the shipped sample world: `validate` exits non-zero and reports five broken connections (see `BUGS.md`); `show` lists 28 locations.

### What a broken world looks like

```bash
# Point at a directory with no content: 0 locations, 0 errors, exits clean
mkdir -p /tmp/empty-world && thwart show --world /tmp/empty-world

# A file with no frontmatter delimiter is a parse error, not a crash
echo "no frontmatter here" > /tmp/empty-world/bad.md
thwart validate --world /tmp/empty-world
```

## Not implemented yet

`map`, `search`, and `debug` exist as CLI subcommands (`thwart map`, `thwart search <query>`, `thwart debug`) but each one now reports "not implemented" and exits non-zero rather than producing output. Running them is a valid way to confirm that behavior, but there's nothing further to test until they're built. See `design/ARCHITECTURE.md`'s implementation status table for what's planned and in what order, and `design/CLI_DESIGN.md` for the target interface (including flags that don't exist yet, like fog-of-war maps or regex search).

## CI

No CI pipeline exists yet. A minimal one worth adding once there's more than zero tests:

```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build
      - run: cargo test
      - run: ./target/debug/thwart validate --world authors/mx/test-world --strict
```

Note that last step is expected to fail today (five broken connections in the sample world, see `BUGS.md`); either fix the sample content first or drop `--strict`/accept non-zero before wiring this in for real.
