# Known Issues

Replaces a prior version of this file dated 2025-07-22 that reported a "PASSING," 100%-pass-rate test run, including specific memory and timing figures, on the same day the project was first committed. Those figures came from CLI commands that printed hardcoded strings regardless of input; nothing in the codebase at that time measured anything. This file now tracks real, currently-true issues instead.

**Last updated**: 2026-08-23, alongside the format/parser rewrite described in `design/DECISIONS.md`.

## Open issues

### No unit tests exist
`cargo test` finds zero test cases. `tasks/unit-testing-plan.md` describes a target test suite; none of it is implemented yet.
- **Severity**: High
- **Impact**: No regression protection. The five broken location references found by `thwart validate` below existed silently in the sample content for over a year because nothing, including a human, ever ran anything against it.

### `map`, `search`, and `debug` are not implemented
All three now report "not implemented" and exit non-zero rather than fabricating output (previously: a hardcoded 3x3 ASCII grid, three hardcoded search results, and invented memory/performance numbers, in every case regardless of the actual input).
- **Severity**: Medium
- **Impact**: `design/CLI_DESIGN.md`'s interface isn't fully realized. Tracked as build order in `design/ARCHITECTURE.md`'s Next Steps.

### `CharacterEngine` isn't wired into the CLI
`WorldEngine::load_world` returns parsed `Character` values, but no CLI command hands them to `CharacterEngine`. `thwart validate --strict` checks a character's `default_location` against the loaded world graph directly rather than through the engine.
- **Severity**: Medium
- **Impact**: Character placement/state isn't exercised by any command yet.

### `ConfigManager` / `thwart.toml` is unused
`src/cli/config.rs` can read and write `thwart.toml`, but `src/bin/thwart.rs` never constructs a `ConfigManager`. The CLI's default world path is a hardcoded constant instead.
- **Severity**: Low
- **Impact**: `thwart.toml`'s `[world].path` currently has no effect; `--world` is the only way to override the default.

### Sample world has five broken location references
Found by the now-real `thwart validate`, previously invisible because nothing ever parsed the content: `forest/hollow` → `forest/abandoned-camp`, `forest/pond` → `forest/rocky-outcrop`, `forest/ravine` → `forest/cave-entrance` and `forest/rocky-outcrop`, `forest/waterfall` → `forest/cave-behind-falls`. None of these five target locations exist.
- **Severity**: Low (sample content, not engine code)
- **Impact**: Left as-is rather than patched, since whether these become new rooms or get removed is an authoring decision, not an engineering one. `thwart validate` will keep flagging them until one or the other happens.

### Compiler warnings
`cargo build` succeeds but emits ~12 warnings, mostly unused parameters in `game_engine.rs`'s stub action handlers.
- **Severity**: Low
- **Impact**: Noise, not a correctness problem. Worth a `cargo fix --lib` pass when those handlers get implemented for real, since several of the unused parameters will become used at that point anyway.

## Resolved (2026-08-23)

- **Project did not build.** `Cargo.toml` had `[bin]` instead of `[[bin]]`, introduced by commit `d6b9e1b` ("Adds a missing file"), which silently broke `cargo build`/`cargo test` for the whole project. Fixed.
- **Compiled binary tracked in git.** `bin/thwart`, a platform-specific Mach-O executable, was committed directly. Untracked; `bin/thwart` is now gitignored and rebuilt via `build.sh`.
- **`Cargo.lock` was gitignored.** Backwards for a binary crate, where the lockfile should be committed for reproducible builds. Now tracked.
- **Every CLI command was a mock.** `validate`, `show`, `map`, `search`, and `debug` all printed hardcoded success text and, in several cases, hardcoded fake data, regardless of input. `validate` and `show` now do real work; the other three now report their own absence instead of fabricating output.
- **Markdown parser was unreachable and, if called, broken.** Nothing in the codebase called `MarkdownParser`. Had it been called, it still would have failed silently on every sample file: the parser expected frontmatter at the top of the file, but every sample location/character/object file put it at the bottom. Rewritten as part of the frontmatter-first format change; now wired into `WorldEngine::load_world` and exercised by `validate`/`show`.
- **Three disagreeing content format specs.** `README.md`, `design/DESIGN.md`, and the actual sample content each described or used a different frontmatter shape. Collapsed to one, documented in `design/CONTENT_FORMAT.md`.
- **`authors/world/` referenced everywhere, existed nowhere.** All docs pointed at a directory that was never created; real content lived at `authors/mx/test-world/`. Formalized `authors/<author>/<world-name>/` as the actual convention and updated defaults and docs to match.
