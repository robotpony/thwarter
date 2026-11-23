# Decisions

Decision records for choices that would be expensive to silently reverse. Newest first.

---

## Rust as the implementation language

**Decision:** Thwarter is implemented in Rust, as a single binary crate (`thwart`) plus a library crate (`thwarter`) that the binary depends on.

**Context:** The project needed a CLI tool that loads, validates, and eventually serves an interactive fiction world graph from Markdown files. Startup time and single-binary distribution mattered because the tool is meant to run frequently during authoring (every save, potentially), by authors who shouldn't need a language runtime installed.

**Options considered:** Not formally recorded at the time this decision was made. The original design doc stated Rust's benefits but didn't document what else was weighed against it (a scripting language with a packaged runtime, e.g. Go, was the most likely alternative given the same startup-time and single-binary goals). Recorded here as a gap rather than reconstructed after the fact.

**Rationale:**
- Startup time: a compiled binary starts in single-digit milliseconds, well under the <1s target for CLI tools run on every save
- Single-binary distribution: no runtime to install for content authors, which matters more here than for a typical dev tool since the audience includes non-engineers
- `serde` gives YAML/JSON/TOML handling from one set of derives, which the frontmatter-heavy content format leans on heavily
- Type-checked models catch a class of "world file references a field that doesn't exist" bugs at compile time, though this only helps the engine code; it does nothing for the author-facing content files, which are unchecked YAML until `thwart validate` runs

**Trade-offs accepted:**
- Slower iteration than a scripting language would give; every engine change needs a recompile
- Smaller pool of contributors comfortable with Rust, relevant if this becomes a community-authored tool
- No dynamic plugin loading without extra work (trait objects, or a scripting layer bolted on later) if content authors ever want to extend behavior without recompiling

**Reversibility:** Low. The models, engines, and CLI are all Rust-specific; changing language means a rewrite, not a port. This is worth stating plainly since the original doc didn't: if Rust turns out to be the wrong call, there's no cheap way back. Given the project's current early state (no shipped users depending on the binary), reversal cost is real but not yet high in absolute terms; it grows every release.

---

## Frontmatter position and field names (2026-08)

**Decision:** Content frontmatter goes at the top of the file (Hugo-standard), not the bottom. Full field spec lives in `design/CONTENT_FORMAT.md`.

**Context:** Three sources of truth disagreed: `README.md`'s example put frontmatter first with one set of field names, `design/DESIGN.md`'s spec put frontmatter last with a different set of field names, and the shipped sample content (27 files) matched the DESIGN.md shape, which the shipped parser could not actually read (the parser only recognized frontmatter opening the file). None of the three agreed with the other two.

**Options considered:**
- Keep frontmatter-last, matching the existing 27 sample files and DESIGN.md, and rewrite the parser and README to match. Lowest content-migration cost.
- Move to frontmatter-first, matching README and common static-site-generator convention. Requires migrating all sample content and rewriting the parser, but matches what a new contributor is most likely to guess.

**Rationale:** Frontmatter-first was chosen for external convention: nearly every tool a contributor has likely used (Hugo, Jekyll, most static site generators, most note-taking tools with frontmatter) puts it first. Matching that reduces the number of things a new author has to be told rather than able to guess.

**Trade-offs accepted:** All 27 sample location/character/object files needed migration (done as part of this decision, see repo history around 2026-08-23). Titles that were previously derived from an H1 heading in the body are now a required frontmatter field instead; bodies no longer repeat the title as a heading.

**Reversibility:** Moderate. Content migration is mechanical (a script did it in one pass); the cost is mostly in re-migrating again if this is reversed, not in re-architecting anything.
