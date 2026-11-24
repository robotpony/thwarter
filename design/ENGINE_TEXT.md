# Text as data

Every sentence thwarter writes, in play or in the console, MUST come from a text table, not from a string in the Rust source (Bruce, 2026-11-24). The defaults are compiled in, and the same shape of table is used everywhere, so the engine, play and the console each override their text the same way (Bruce: "console messages should be configurable too, even if text is compiled in. the same shape of config should be used everywhere, if possible").

Owner: commander (design). The coder builds the shared type and the engine's tables; the console coder moves the console's text. Status: agreed; Bruce said go on 2026-11-24. Planned as `PLAN.md` › Phase 8.

## The shape

One mechanism for all text: a TOML table of `key = "template with {placeholders}"`, the shape `[play]` already has (`design/CONTENT_FORMAT.md` › How play words a room).

```toml
[engine]
no_exit = "You can't go {direction} from here."
no_noun = "{Verb} what?"
not_here = "There's no {noun} here."
not_understood = "I don't understand \"{line}\"."
nothing = ["Nothing happens.", "Nothing seems to happen.", "That does nothing."]
```

- A key's value is a string, or a list of strings so a reply doesn't repeat word for word. A list picks entry `turn % len`, deterministically and never at random, so a replay gives the same text and a transcript pins one wording.
- `{or}` and `{list}` join lists as they do in `[play]`. A placeholder the call doesn't supply, or a key the defaults don't have, is a warning.
- One type, `Texts`, holds every table and answers `text(key, &args)`. It's the coder's, built from `engines::play_text`.

## Where tables live

| Table | Defaults (compiled in) | Overridden in | Covers |
|---|---|---|---|
| `[play]` | `src/builtins/text/play.toml` | `world.toml` | Rooms, people, held things, exits, the inventory |
| `[engine]` | `src/builtins/text/engine.toml` | `world.toml` | Built-in verb replies, refusals, missing nouns, defaults when nothing answers, ambiguity questions, parser replies |
| `[console]` | `src/builtins/text/console.toml` | `$XDG_CONFIG_HOME/thwarter/config.toml` | The console's replies to `/` commands, usage lines, the sidebar, the problems pane |

Defaults are read with `include_str!`, so nothing has to be on disk. The console's text is the author's choice, not the world's, so it lives in the personal config and doesn't change with the open world.

A world's story still answers first: a `## When:` reply section in a thing, its room or its template wins over `[engine]`, as it does today. `[engine]` replaces only the built-in fallback. Default-reply lists by chapter or area (`design/SECTIONS.md` › Default replies) are `## When: #nothing` sections, since they're story; the world-wide list is `[engine]`'s `nothing`.

### Story overrides of built-in replies

Rules for a `## When:` section that replaces an `[engine]` reply (from the coder, 2026-11-24):
- **Exact match.** Each built-in reply has one outcome tag, such as `#take #immovable` or `#go #no-exit`. A section replaces it only when its tags are exactly that set, most specific winning, so a thing's `## When: #take` doesn't also swallow every take refusal. The outcome tag is the reply's stable name, which the guard test and the unused-key warning use.
- **Each reply has a chain.** A reply about a thing ("The chest won't budge.", "You can't kick the cook.") can be replaced by the thing, its room, the chapter, or the world. A reply with no thing (``I don't understand "<line>".``, "Take what?", "There's no lamp here.", "You can't go west from here.") can be replaced only by the chapter or the world's `[engine]`. The step 2 list gives each key its chain.
- **Outcome names are reserved.** `#no-noun`, `#no-exit`, `#immovable`, `#not-here` and the rest join `#ambiguous` and `#searched` as reserved tags (`engines::layers::RESERVED_TAGS`), and `validate` warns about a state named like one.
- **Slots.** In a TOML table a slot is `{noun}`. In a section's prose it's the prose form, `{{ turn.noun }}`, `{{ turn.direction }}`, `{{ turn.door }}`, so an author learns one slot syntax per file type.
- **Whole replies only.** A reply built from parts, such as search's "{opening}, but find nothing new.", becomes one whole reply per case, so each can be reworded on its own.

Not covered: author notes (`ActionResult::note`), validation warnings and `thwart` command-line output. Those are tooling messages and may follow later if Bruce wants.

## Order of work

1. **The shared type.** `Texts` with the three tables and their compiled-in defaults; `[play]` moves into it unchanged. [coder]
2. **Engine strings.** Every player-facing literal in `src/engines/` and `src/render.rs` becomes an `[engine]` key with its outcome tag and override chain, the default text unchanged so transcripts stay the same. The defaults file is the list of every use, which covers Bruce's "track all uses" route. [coder]
3. **Console strings.** About 200 literals in `src/console/` become `[console]` keys, likewise unchanged. [console]
4. **Guard.** A test fails on a new user-facing string literal outside the defaults files. [coder, console]
5. **Overrides.** `world.toml`'s `[engine]` and the personal config's `[console]` replace defaults, and `validate` warns about a key nothing uses. [coder, console]

## Open questions

1. Should validation warnings and `thwart` CLI output move to tables too? Proposed: not in Phase 8; they're for authors and change with the code.
2. A world's `[engine]` wording applies everywhere in that world. Should a chapter be able to override one key outside a `## When:` section? Proposed: no; sections already do that.
