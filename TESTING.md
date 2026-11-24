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

The console's acceptance tests (`design/CONSOLE_REQUIREMENTS.md` › REQ-CON-9) are transcripts in `tests/transcripts/*.txt`: a console session written exactly as the console prints it, with input lines as `> line`. Each starts with a `# world:` and `# mode:` header and runs against a fixture world in `tests/worlds/`, named by its path from the repository root (`# world: tests/worlds/tiny`). The engine's own tests use `tests/worlds/sample/`; neither depends on `authors/mx/test-world`, which Bruce rewrites. The format is described in the header comment of `tests/transcripts.rs`.

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
thwart show --world authors/mx/test-world bedroom

# Same, as JSON
thwart show --world authors/mx/test-world bedroom --format json
```

Expected result against the shipped sample world: `validate --strict` passes with no problems and no warnings; `show` names the world, "Test world", and its starting room, the bedroom. The world is small on purpose: Bruce is writing it afresh (2026-11-23), and tests that need more use the fixture in `tests/worlds/sample/`.

### What a broken world looks like

```bash
# Point at a directory with no content: 0 locations, 0 errors, exits clean
mkdir -p /tmp/empty-world && thwart show --world /tmp/empty-world

# A file with no frontmatter delimiter is a parse error, not a crash
echo "no frontmatter here" > /tmp/empty-world/bad.md
thwart validate --world /tmp/empty-world
```

## Not implemented yet

`debug` exists as a CLI subcommand but reports "not implemented" and exits non-zero rather than producing output; `map` and `search` are built. Running it is a valid way to confirm that behaviour, but there's nothing further to test until it's built. See `design/ARCHITECTURE.md`'s implementation status table for what's planned and in what order, and `design/CLI_DESIGN.md` for the target interface (including flags that don't exist yet, like fog-of-war maps or regex search).

## CI

`.github/workflows/ci.yml` builds and tests on every push and pull request (added in bca9973): `cargo build --locked --all-targets`, then `cargo test --locked`, on Ubuntu with the stable Rust toolchain and read-only repository permissions. It runs once `main` is pushed to GitHub.

It doesn't yet run `thwart validate --strict` against the test world as its own step; the test suite's `the_shipped_test_world_validates` covers it for problems.

The workflow's actions are pinned by version tag (`actions/checkout@v5` and the rest), not by commit; pinning by commit is the stricter choice if supply-chain risk ever matters more.
