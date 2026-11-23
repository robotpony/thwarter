# Plan

The phased work queue. Each task is one line that points to its spec. Phases are `##` headings with checkboxes, so Space Command can read this file. Defects go in `BUGS.md`, what each component does today in `design/ARCHITECTURE.md` › Implementation status, and retired plans in `CHANGELOG.md`.

The order is documented tooling first, then the game loop: static-world tooling (Phases 2 and 3) is finished before action processing and narrative state (Phase 4), which together make a real game loop. Open question 1 below asks whether to change that.

## Phase 1 · Foundation

- [x] Done (squashed; 0.1.0, 2e4d080, 6aa58de, 884ae96): the build fixed, frontmatter-first content format and parser, real `validate` and `show`, honest "not implemented" for `map`, `search` and `debug`, the design docs split. The original 20-day plan is in `git show 884ae96:tasks/cli-first-pass-TODO.md`.

## Phase 2 · Tests

- [ ] Parser tests (`design/UNIT_TESTS.md` › 1)
- [ ] `WorldEngine` tests: `load_world` and `broken_connections` on a temp world (`design/UNIT_TESTS.md` › 2)
- [ ] CLI tests against a fixture world: exit codes and key output lines (`design/UNIT_TESTS.md` › 5)
- [ ] Minimal CI: `cargo build` and `cargo test` on push (`TESTING.md` › CI)

## Phase 3 · Static-world tooling

- [ ] Wire `CharacterEngine` into `show` and `validate --strict` (`design/ARCHITECTURE.md` › Character Engine)
- [ ] `WorldGraph::find_path` (`design/ARCHITECTURE.md` › Implementation status)
- [ ] `thwart map` built on `find_path` (`design/CLI_DESIGN.md` › `map`)
- [ ] `thwart search` over locations, characters and objects (`design/CLI_DESIGN.md` › `search`)
- [ ] Read `thwart.toml` through `ConfigManager`, with flags taking precedence (`REQUIREMENTS.md` › REQ-CFG-1)
- [ ] `validate` checks that `id` matches the file path, and flags `id` collisions (`design/CONTENT_FORMAT.md` › What's still open)
- [ ] `validate` checks object and inventory references (`design/CONTENT_FORMAT.md` › What's still open)

## Phase 4 · Game loop

- [ ] Action effects for move, take, look and examine (`REQUIREMENTS.md` › REQ-ENG-5)
- [ ] Narrative state: plot point consequences and alternative-text conditions (`design/ARCHITECTURE.md` › Narrative Engine)
- [ ] Save and load game state (`design/ARCHITECTURE.md` › Implementation status)

## Open questions

1. Should a thin `thwart play` loop (`look`, `go`, `take`, `inventory`, one story flag) come before Phase 3? Writing one small puzzle with it would show what the content format lacks (conditions, state-dependent text) before `map` and `search` are built on it. Raised 2026-11-23, not decided.
