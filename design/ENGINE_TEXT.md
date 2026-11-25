# Text as data

Every sentence thwarter writes, in play or in the console, MUST come from a text table, not from a string in the Rust source (Bruce, 2026-11-24). The defaults are compiled in, and the same shape of table is used everywhere, so the engine, play and the console each override their text the same way (Bruce: "console messages should be configurable too, even if text is compiled in. the same shape of config should be used everywhere, if possible").

Owner: commander (design). The coder builds the shared type and the engine's tables; the console coder moves the console's text. Status: built in Phase 8, 2026-11-24 to 25; tooling text and the thing-kind labels follow in `PLAN.md` › Phase 11.

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

Edit mode's room view reads the same as play's, with no title line of its own (Bruce, 2026-11-24): titles are for search, listings and the inventory, and play's `show_room_names` decides whether the room's name shows, in either mode. It uses `[play]`'s sentences ("You can go {or}.", "You see {list}.") and adds no lines under them, so the author judges the prose as a player meets it. The author's detail (the room's title and id, each thing's id, each exit's target, door and state, and what isn't written yet) goes to the console as data, a `LocationView`, which shows it in the sidebar for the current room and in full through `/inspect <thing>` (`design/CONSOLE_REQUIREMENTS.md`). The console's wording for that detail is `[console]`'s.

A world's story still answers first: a `## When:` reply section in a thing, its room or its template wins over `[engine]`, as it does today. `[engine]` replaces only the built-in fallback. Default-reply lists by chapter or area (`design/SECTIONS.md` › Default replies) are `## When: #nothing` sections, since they're story; the world-wide list is `[engine]`'s `nothing`.

### Story overrides of built-in replies

Rules for a `## When:` section that replaces an `[engine]` reply (from the coder, 2026-11-24):
- **Exact match.** Each built-in reply has one outcome tag, such as `#take #immovable` or `#go #no-exit`. A section replaces it only when its tags are exactly that set, most specific winning, so a thing's `## When: #take` doesn't also swallow every take refusal. The outcome tag is the reply's stable name, which the guard test and the unused-key warning use.
- **Each reply has a chain.** A reply about a thing ("The chest won't budge.", "You can't kick the cook.") can be replaced by the thing, its room, the chapter, or the world. A reply with no thing (``I don't understand "<line>".``, "Take what?", "There's no lamp here.", "You can't go west from here.") can be replaced only by the chapter or the world's `[engine]`. The step 2 list gives each key its chain.
- **Outcome names are reserved.** `#no-noun`, `#no-exit`, `#immovable`, `#not-here` and the rest join `#ambiguous` and `#searched` as reserved tags (`engines::layers::RESERVED_TAGS`), and `validate` warns about a state named like one.
- **Slots.** In a TOML table a slot is `{noun}`. In a section's prose it's the prose form, `{{ turn.noun }}`, `{{ turn.direction }}`, `{{ turn.door }}`, so an author learns one slot syntax per file type.
- **Whole replies only.** A reply built from parts, such as search's "{opening}, but find nothing new.", becomes one whole reply per case, so each can be reworded on its own.

Not covered in Phase 8: author notes (`ActionResult::note`), validation warnings and `thwart` command-line output. They're tooling messages, and they move to tables in a later phase (Bruce, 2026-11-24; `PLAN.md` › Phase 11).

## Order of work

1. **The shared type.** `Texts` with the three tables and their compiled-in defaults; `[play]` moves into it unchanged. [coder]
2. **Engine strings.** Every player-facing literal in `src/engines/` and `src/render.rs` becomes an `[engine]` key with its outcome tag and override chain, the default text unchanged so transcripts stay the same. The defaults file is the list of every use, which covers Bruce's "track all uses" route. [coder]
3. **Console strings.** About 200 literals in `src/console/` become `[console]` keys, likewise unchanged. [console]
4. **Guard.** A test fails on a new user-facing string literal outside the defaults files (› The guard test). [coder, console]
5. **Overrides.** `world.toml`'s `[engine]` and the personal config's `[console]` replace defaults, and `validate` warns about a key nothing uses. [coder, console]
6. **Story overrides.** A `## When:` section replaces an `[engine]` reply by its exact outcome tag along the reply's chain, the outcome names join `RESERVED_TAGS` with `validate`'s warning, and section prose fills `{{ turn.x }}` slots (› Story overrides of built-in replies). Its own step because it's a feature, not a move of strings. [coder]

## The guard test

A heuristic scan with a checked-in allowlist (Bruce, 2026-11-24), widened in 60c6e88 once the world picker had landed, as Bruce chose. A false positive costs one allowlist line.

- **Where it looks.** String literals in `src/engines/`, `src/render.rs` and `src/console/`, outside `#[cfg(test)]` code.
- **What it flags.** Any literal holding two or more words of letters once its `{placeholders}` are set aside, whatever its case or punctuation, so labels ("Story values"), key hints and short replies ("Bye.") count as text.
- **Not text.** A file template starting `---` and a newline; a command line (a literal starting `/` and a letter that doesn't end like a sentence); a literal after a `// text-guard: not text` comment; and `unimplemented!` and `todo!` panics.
- **The allowlist.** `tests/text_allowlist.txt`, one literal per line, each with a comment saying why it's exempt (a log line, a format spec). The test also fails on an allowlist line that matches no literal any more, so the list doesn't rot.
- **Exempt by call site.** Author notes, validation warnings and `thwart` command-line output, while they stay in code (› Not covered, above). They're exempted by file or by the function they're passed to, not one by one.
- **Who keeps it.** The test is `tests/text_guard.rs` (REQ-ENG-58). Each lane adds its own allowlist entries, and a change to the rule comes with the string moves it needs in the same commit, so the tree never fails on it.

## Settled questions

1. Validation warnings and `thwart` command-line output don't move to tables in Phase 8. They're a later item in `PLAN.md` › Phase 11 (Bruce, 2026-11-24).
2. A chapter doesn't override an `[engine]` key outside a `## When:` section. It rewords a built-in reply with a `## When: #outcome` section, as a thing or a room does, so there's one way to do it (Bruce, 2026-11-24).
3. The thing-kind labels in `world.rs` stay in code for now, allowlisted, since they're also words authors type; they're a Phase 11 item (Bruce, 2026-11-25).
