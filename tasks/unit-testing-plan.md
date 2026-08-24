# Unit Testing Plan

Replaces a prior version of this plan that specified 200+ tests, 95% model coverage, a 6-week timeline, and a full CI/coverage/fuzzing pipeline, for a project that at the time had zero tests and didn't compile. That plan was sized for a team working on a mature codebase, not a solo P0 CLI tool. This version is sized for what's actually here.

## Priority order

Test the parser first. It's the piece most likely to silently disagree with `design/CONTENT_FORMAT.md` (it already did once), and it's the piece every other component depends on.

1. **`markdown_parser.rs`** — highest priority. Cover:
   - A well-formed location/character/object file each, asserting every field lands correctly
   - Missing `type` field (should error, not default to something)
   - Missing a type-required field (`id`, `title`, `object_type`)
   - Frontmatter never closed (no second `---`)
   - No frontmatter at all (file doesn't start with `---`)
   - The `Inventory:` / `inventory:` case-normalization edge case, since a real sample file had this
2. **`WorldEngine`** — `load_world` against a small `tempfile`-built world directory; `broken_connections` against a world with a known-bad connection.
3. **Models** (`Location`, `Character`, `GameObject`) — constructors and any non-trivial derived behavior. Thin right now; grows as engines gain real logic.
4. **`CharacterEngine`** — placement and `get_characters_at_location`, once it's actually wired into a CLI command (see `design/ARCHITECTURE.md` Next Steps). Not worth testing in isolation before that.
5. **CLI commands** — integration-style tests running the built binary against a fixture world directory, checking exit codes and key output lines (e.g., `validate` against a world with a known broken connection should exit non-zero and mention it).

Everything downstream of the stub engines (`NarrativeEngine` consequence execution, `GameEngine` action effects) isn't worth testing yet: there's no behavior there to test, only `TODO`s. Add tests when the implementation lands, not before.

## What "done" looks like for now

Not a coverage percentage. A test suite that would have caught the two real bugs this repo actually had:
- The parser/content-format mismatch (a fixture-based parser test would have caught this immediately)
- The broken `Cargo.toml` (`cargo test` running in CI catches this by definition, once CI exists)

## Running tests

```bash
cargo test              # all tests
cargo test parsers::     # one module
cargo test -- --nocapture
```

No CI pipeline exists yet. Adding a minimal one (`cargo build && cargo test` on push) is worth doing once there are tests for it to run; a CI config with nothing to check is its own kind of false signal.
