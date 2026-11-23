# Changelog

Thwarter's changes, newest first. This file also records what was removed or replaced: deleted files, retired formats, superseded plans and their history. Each removal names the commit to recover it from, so the working docs can describe only what's true now.

Versions are `0.x.y` semver, pre-1.0. Sections are `### Added`, `### Changed`, `### Fixed` and `### Removed`. Changes not yet in a versioned build go under `## Unreleased`.

## Unreleased

### Added

- `PLAN.md`, the phased work queue, and `BUGS.md`, the defect log, both in Space Command's task format.
- This changelog.
- `.claude/hooks/check-commit-message.sh`, which refuses a commit that credits Claude as a co-author. Copied from warped-space-command.

### Changed

- The phase plan: authoring first, then play, in one console with an edit and a play mode; engine work only as they need it (Bruce, 2026-11-23). `map`, `search` and the remaining `validate` checks moved to a "Tooling, as needed" phase, and tests moved into each phase.
- `REQUIREMENTS.md` gains §4.6 Console (REQ-CON-1 to 11), the file writer and in-memory changes (REQ-ENG-7 to 9), and objects without cards as allowed scenery (REQ-CF-8). The separate P1 play REPL and P2 editing REPL are now one console.
- `tasks/unit-testing-plan.md` moved to `design/UNIT_TESTS.md`.
- `tasks/ISSUES.md` became `BUGS.md`. Its open items that are missing features, not defects (no tests, `map`/`search`/`debug`, `CharacterEngine` and `ConfigManager` not wired in), moved to `PLAN.md`.

### Removed

- `tasks/cli-first-pass-TODO.md`, the original 20-day, six-phase plan. It described a `src/lib/` layout that never existed. Replaced by `PLAN.md`; recover with `git show 884ae96:tasks/cli-first-pass-TODO.md`.
- The tooling-first phase order (Tests, Static-world tooling, Game loop), replaced the same day by authoring first. Recover with `git show e43fbbf:PLAN.md`.
- `design/ARCHITECTURE.md` › Next steps, moved to `PLAN.md`. Recover with `git show 884ae96:design/ARCHITECTURE.md`.
- The notes at the top of `TESTING.md` and the testing plan explaining what their earlier versions got wrong. That history is under 0.1.0 below.

## 0.1.5 (2026-11-23)

### Added

- `thwart edit [world]` and `thwart play [world]`, which open the console in edit or play mode through `thwarter::console::run` (REQ-CON-1). The world is the positional argument, then `--world`, then the default. Until the console loop lands they report that it isn't implemented and exit non-zero.

## 0.1.4 (2026-11-23)

### Added

- `WorldEngine::describe_location`, returning a `LocationView` (title, body, exits with whether each target exists, characters and objects with their card names, `None` for no card), as agreed with the console for REQ-ENG-9.
- `thwarter::render::render_location`, the room view shared by `thwart show` and the console (REQ-CON-8). It returns plain text without colour.
- `WorldEngine::problems(report, strict)`, the check behind `thwart validate`, for the console's `/reload` to share (REQ-CON-7).
- `WorldEngine::characters`, the loaded character cards by id.
- The first unit tests: the renderer, `describe_location` and `problems`.

### Changed

- `show` and `validate` call those shared functions. Their output is unchanged.

## 0.1.3 (2026-11-23)

### Added

- `src/console/`, the console's entry point: `console::run(world_path, Mode::Edit | Mode::Play)`, which `thwart edit` and `thwart play` will call (REQ-CON-1). It loads the world, then reports that the console isn't implemented yet; the loop follows in Phase 2.

## 0.1.2 (2026-11-23)

### Changed

- `show <location>` labels an object with no card `(no card)` instead of `(not found)`, since such an object is allowed scenery (REQ-CF-8). Characters with no card still say `(not found)`, and exits to missing rooms still say `(missing)`.

## 0.1.1 (2026-11-23)

### Fixed

- `--world`, `--config`, `-q` and `-v` are accepted after the subcommand (`thwart validate --world <path>`), as `README.md` and `TESTING.md` write them. They were top-level only.
- `validate` prints its failure once. The summary line stays; `main` no longer prints the returned error again, and still exits non-zero.
- `show <location>` in text format prints the room for an author to check: name and id, coordinates, description, exits in compass order, then characters and objects by name. Exits to unknown rooms say `(missing)`; character and object ids with no card say `(not found)`. JSON and YAML output are unchanged.

### Changed

- `show`'s own `--verbose` is replaced by the global `-v`/`--verbose`, which `show --verbose` still reaches.
- `thwart --version` reads the version from `Cargo.toml` instead of a hardcoded string.

## 0.1.0 (2026-08-24)

The August audit (2e4d080), plus files it left uncommitted (6aa58de, 884ae96). The version number dates from July 2025 but wasn't bumped until this file began.

### Fixed

- The project didn't build: `Cargo.toml` had `[bin]` instead of `[[bin]]`, from d6b9e1b.
- The parser couldn't read any sample file: it expected frontmatter at the top and every sample file had it at the bottom. Nothing called the parser, so no one noticed.
- `Cargo.lock` was gitignored; it's now tracked, as a binary crate's should be.

### Changed

- The content format: frontmatter moved to the top of each file, and `title` moved from an H1 in the body into frontmatter. All 27 sample files were migrated. See `design/DECISIONS.md`.
- World content lives at `authors/<author>/<world-name>/`. Docs had pointed at `authors/world/`, which never existed.
- `validate` and `show` do real work against the loaded world.
- `map`, `search` and `debug` say they're not implemented and exit non-zero.
- `design/DESIGN.md` split into `design/ARCHITECTURE.md`, `design/CONTENT_FORMAT.md` and `design/DECISIONS.md`, with `REQUIREMENTS.md` consolidating the requirements.

### Removed

- Fabricated output from every CLI command. `map` printed a hardcoded 3x3 grid, `search` three invented results, and `debug` made-up memory and timing figures, whatever the input. Recover with `git show d6b9e1b:src/cli/commands/map.rs` (also `search.rs`, `debug.rs`).
- The compiled `bin/thwart` binary from git; `build.sh` rebuilds it and it's gitignored.
- Three disagreeing content format specs (`README.md`, `design/DESIGN.md`, and the sample content each differed), replaced by `design/CONTENT_FORMAT.md`. Recover with `git show d6b9e1b:design/DESIGN.md`.
- `TESTING.md`'s list of flags that never existed in the CLI (`--fow`, `--check-refs`, `--scale`, `--with-npcs`, `--all-endings`, `--trace-movements` and more), and its profiling workflows. Recover with `git show d6b9e1b:TESTING.md`.
- A unit testing plan for 200+ tests, 95% coverage and six weeks of work, written when the project had no tests and didn't compile. Recover with `git show d6b9e1b:tasks/unit-testing-plan.md`.
- `tasks/ISSUES.md`'s report of a passing test run with memory and timing figures, dated the day of the first commit; none of it measured anything. Recover with `git show d6b9e1b:tasks/ISSUES.md`.
- `CLAUDE.md`'s claim that P0 was complete.
