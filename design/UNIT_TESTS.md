# Testing approach

No coverage target: tests cover the parts with real behaviour, and each change brings its own (CLAUDE.md › Conventions). How to run them is in `TESTING.md`.

## Where tests live

- **Unit tests** sit beside the code, in each module's `#[cfg(test)]` block: the parser, layers and merging, the world and game engines, turns and sections, scripts, play text, the player command parser, saves, question sets and the console's pieces. Most build a small world in a temporary folder.
- **CLI tests** in `tests/cli_*.rs` run the built `thwart` binary and check exit codes and key lines (`show`, `package`).
- **Console transcripts** in `tests/transcripts/*.txt` are the console's acceptance tests (REQ-CON-9): a session written as the console prints it, input after `> `. A transcript that changes content says `# copy: yes` and runs on a copy.
- **Fixture worlds** are `tests/worlds/tiny` (a village square, its well and the elder) and `tests/worlds/sample` (a castle and forest with characters and objects, 12 files). Tests never use `authors/mx/test-world/`, since Bruce rewrites it.

## What to test

1. Anything that reads content: a fixture or temporary-world test for each format rule in `design/CONTENT_FORMAT.md`, including what's refused and what only warns.
2. Each player command and reply in `design/PLAYER_COMMANDS.md`, through `parse_command` and `act`.
3. Each console command, by transcript when it shows output, by unit test when it doesn't.
4. A bug, by a test that fails before its fix.

Text a player or author reads is pinned by transcripts, so a wording change is a reviewed `THWART_BLESS=1` diff. With Phase 8, a test also fails on a new user-facing string written in code (`design/ENGINE_TEXT.md`).

## CI

`.github/workflows/ci.yml` runs `cargo build --locked --all-targets` and `cargo test --locked` on every push (`TESTING.md` › CI). A version bump commits `Cargo.lock` with `Cargo.toml`, or `--locked` fails.
