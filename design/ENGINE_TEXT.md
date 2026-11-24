# Engine text as data

Every sentence the engine writes for a player MUST come from data a world can change, not from a string in the Rust source (Bruce, 2026-11-24). The engine ships that data as defaults: `world.toml`'s `[play]` table and the built-in bundles in `src/builtins/`. A world overrides a default the same way it overrides a built-in template.

Owner: commander (design); the coder builds it. Status: *proposed*, planned as `PLAN.md` › Phase 8.

## What counts

Engine text is anything a player reads that the world's own files didn't write:
- Room and inventory wording ("You see {list}.", "You aren't carrying anything."). Already data: `[play]` (`design/CONTENT_FORMAT.md` › How play words a room).
- Built-in verb replies: taking, refusals ("You can't go west from here."), the locked-exit line, missing nouns ("Take what?"), "There's no {noun} here.".
- Defaults when nothing answers: "Nothing happens." and its variants (`design/SECTIONS.md` › Default replies).
- Ambiguity questions ("Which key do you want to take, the brass key or the iron key?"). The template-section override exists; the built-in wording is still code (`GameEngine::ask_which`).
- Parser replies: ``I don't understand "<line>".``, and later typo suggestions (REQ-PC-24).
- The built-in templates' replies (`src/builtins/door`, `locked-door`, `portal`). Already data.

Not engine text: author notes (`ActionResult::note`), validation warnings, CLI output, and the console's own replies to `/` commands. Those are tools talking to an author, not the story talking to a player.

## Where each kind lives

| Kind | Home | Why |
|---|---|---|
| Fixed sentence shapes with slots (room, inventory, exits) | `[play]` in `world.toml` | Already there; one line each, easy to tune |
| Replies to verbs, refusals, defaults, ambiguity | A built-in bundle, `src/builtins/replies/`, with one `## When:` section per reply (`## When: #take #no-noun`, `## When: #go #no-exit`) | The same matching as every other reply, so a world, chapter or thing overrides one by writing the same section nearer the action |
| Variants of one reply | Several sections with the same heading; play picks one by turn | Matches `design/STATES.md` › Randomness in prose |

A world overrides a built-in reply by putting the same section in `world/replies.md` (or nearer: the chapter, the area, the thing). The usual lineage decides which wins, nearest first.

## Order of work

1. **Inventory.** The coder lists every player-facing string literal in `src/engines/` and `src/render.rs` in a table here (text, where, which kind above). Strings in tests don't count. About 100 candidates today, many of them built-in replies in `game_engine.rs` and `turn.rs`.
2. **Move.** Each string moves to its home, with its slots named (`{noun}`, `{direction}`, `{door}`). The default text doesn't change, so transcripts stay the same.
3. **Guard.** A test fails when a new player-facing literal appears in the engine outside the defaults, so the inventory doesn't drift.
4. **Override.** A world's `world/replies.md` replaces a built-in reply; `validate` warns about a section there that matches no built-in reply.

## Open questions

1. Does the console's own UI text (replies to `/` commands, usage lines, the sidebar) need the same treatment? Proposed: no, it's tooling; the console coder gathers it into one table in `src/console` so it's easy to find, not configurable.
2. Should `[play]`'s sentence templates also move into the replies bundle, so there's one home instead of two? Proposed: not now; `[play]` works and is simpler to edit.
3. Do default-reply lists (`design/SECTIONS.md` › Open question 4) take this form, several `## When: #nothing` sections? Proposed: yes, which settles that question too.
