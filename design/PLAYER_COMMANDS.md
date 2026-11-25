# Player commands

The command language every client shares (Bruce, 2026-11-23): what a player can type, how a line becomes an action, and how the game replies. A terminal player comes first; later clients with a real interface (Godot, Swift, Node) send the same lines or the same actions. The engine parses and answers; a client only reads input and shows replies.

The console's play mode is one such client. Its own commands (`/edit`, `/play here` and the rest) are in `design/CONSOLE_REQUIREMENTS.md`, not here.

Owner: commander. Ids: REQ-PC. Each requirement says whether it's built, as of 0.62.1.

## Summary

A line is a verb and maybe a noun. The engine turns it into an action (look, go, take, examine, inventory, search, or an object's own named action) or into a reply that changes nothing. Nouns match things in scope by id, name or a unique word of either. Every reply is written for the player; detail only an author needs travels separately as an author note. Game commands that aren't part of the story, such as saving, start with `/` so they never collide with a story's verbs.

## How a line is read

- REQ-PC-1: The engine MUST parse a typed line into an action, a reply, or an unknown line, through one call any client can use: `GameEngine::parse_command(&WorldEngine, line, &PlayState) -> Parsed`. `Parsed` is `Action`, `Reply` (a verb missing its noun) or `Unknown` (REQ-PC-5's reply, kept apart so a client can log what players tried). Only an `Action` goes on to `act`, so only it can count as a turn. An object action is `PlayAction::Use { verb, noun, with }`, where `with` is what it's done with (`hit window with stick`), articles dropped from both. A client MAY also build actions directly (a button for "north"), skipping the parser. *Built* (0fe9754, REQ-ENG-32), and the console's play mode parses through it (a1e8f1b).
- REQ-PC-2: Parsing MUST ignore case and surrounding space, and split the line into words. *Built.*
- REQ-PC-3: An article (`the`, `a`, `an`) MUST be dropped before a noun: `take the lantern`, `look at a well`. *Built.*
- REQ-PC-20: A `to` or `at` straight after the verb MUST be dropped too, then any article: `listen to the chest`, `talk to the captain`. Prepositions that change the meaning (`in`, `into`, `on`, `under`) MUST NOT be dropped; `look in box` waits on Open question 5 (multi-word verbs). `go` is the exception, since an exit can be named "to the river": the engine tries the whole phrase as an exit first, then without the `to`. *Built* (7437e1f and 95a25f0, REQ-ENG-37).
- REQ-PC-4: A known verb with no noun MUST get a reply asking for one, and change nothing: `Take what?`, `Go where?`, `Look at what?`. A verb that some object has as an action counts as known: a bare `pull` gets `What do you want to pull?`. *Built.*
- REQ-PC-5: A line the engine can't read MUST get `I don't understand "<line>".`, with plain quotes, and change nothing. It MUST NOT count as a turn (REQ-ENG-23). That includes a verb no thing has, alone (`dance`), and a built-in verb used another way (`i lamp`, `n please`), which never becomes an object action. *Built.*

## Verbs

| Verb | Also | Action | Built |
|---|---|---|---|
| `look` | `l` | Describe the room | yes |
| `look at <thing>` | `look <thing>`, `examine`, `x`, `inspect` | Describe a thing | yes |
| `look around` | `examine room`, `x here`, the room's name | Examine the room, finding faint things | yes |
| `search` | | Search the room, finding faint things | yes |
| `search <thing>` | | An object's own `search` action, if it has one (a desk) | yes |
| `look <direction>` | `what's <direction>`, `what is <direction>` | Say what's that way without going | yes |
| `go <direction>` | `walk`, `move`, `enter`, a bare direction, the room it leads to (`go closet`) | Move through an exit | yes |
| `take <thing>` | `get`, `pick up` | Pick up an object | yes |
| `inventory` | `i`, `inv` | List what the player carries | yes |
| `where` | `where am i` | Name the current location | yes |
| `drop <thing>` | | Put a carried thing down here | yes |
| `give <thing> to <person>` | | Offer a carried thing to someone | yes |
| `take all` | | Take every loose thing here | yes |
| `wait` | `z` | Let a turn pass | yes |
| `again` | `g` | Repeat the last command | yes |
| `talk to <person>` | | Hear what someone says, or pick a topic | yes |
| `ask <person> about <topic>` | | Ask about a thing or a topic | yes |
| `throw`, `smash`, `break`, `stack`, `push`, `pull` | | Story verbs with a built-in effect (REQ-PC-30) | no |
| `jump`, `run`, `kick`, `hit` | | Story verbs with a default reply (REQ-PC-29) | no |
| `<verb> <thing>` | | An object's own named action (`pull lever`) | yes |

- REQ-PC-6: The verbs and aliases above MUST be accepted by every client. The aliases are a table, `src/builtins/verbs.toml`, which a world's `[verbs]` in `world.toml` adds to (built in 29592dc, 0.78.0; `design/ENGINE_REQUIREMENTS.md` › Forgiving verbs, REQ-ENG-51). The verb is the line's first word, apart from `look at` and `pick up`; multi-word verbs such as `turn on` aren't parsed (Open question 5). A client MUST NOT add story verbs of its own; a world adds them as object actions (REQ-PC-13).
- REQ-PC-7: Bare directions MUST move the player: `north`/`n`, `northeast`/`ne`, `east`/`e`, `southeast`/`se`, `south`/`s`, `southwest`/`sw`, `west`/`w`, `northwest`/`nw`, `up`/`u`, `down`/`d`, `in`, `out`. A bare direction followed by more words (`n please`) is not understood. *Built.*
- REQ-PC-8: `go` MUST also take an exit's free-text name, since direction names are free text (`design/CONTENT_FORMAT.md` › `connections`): `go behind`, `go through the hedge`. *Built.*

## Nouns

- REQ-PC-9: A noun MUST match among the things in scope, in this order: an exact id; the id with spaces for hyphens (`brass lantern` for `brass-lantern`); the name, ignoring case; then a whole word of the name or id that only one thing has (`lantern`). The first rule that matches wins. *Built* (`game_engine::resolve`).
- REQ-PC-10: In scope means what the player can see or carries: the room itself (by name, `room`, `here` or `around`; a bare room name examines it), what's listed in it, faint things already found, and the inventory. Hidden and absent things follow `design/STATES.md` › Hidden things. *Built* (REQ-ENG-25).
- REQ-PC-11: A noun that matches several things MUST get a reply in the story's voice naming them, so the player can choose. The built-in reply is one question: "Which key do you want to take, the brass key or the iron key?", and for people "Who do you want to examine, King Aldric or Queen Aldric?" The story MAY replace it (`design/STATES.md` › Ambiguity, REQ-ENG-26). *Built.*
- REQ-PC-12: A noun that matches nothing MUST get ``There's no <noun> here.`` *Built.*

## Object actions

- REQ-PC-13: A verb that isn't in the table, followed by a noun, MUST be tried as that object's named action (`light lantern`, `pull lever`; `design/STATES.md` › Changing state). A verb counts as an action when a thing has it in `actions:` or names it in a section heading (`## When: #hit @stick`). The engine checks `requires:`, applies the action, and replies with the matching reply sections in the target, then what it's done with, then the room (`design/SECTIONS.md` › The model). A `with` thing that matches nothing gets REQ-PC-12's reply. *Built* (`PlayAction::Use`, REQ-ENG-32, REQ-ENG-36).
- REQ-PC-14: When the noun matches a thing but no section answers the action, the reply MUST be ``Nothing happens.``, and the turn changes nothing (Bruce; `design/SECTIONS.md` › Default replies). When no thing in the world has that verb as an action, the reply MUST be REQ-PC-5's `I don't understand "<line>".`, so `dance wildly` doesn't answer "There's no wildly here." "In the world" means any state or chapter of any object or door, not only the current ones, so a verb a gong gains later (`hush`) is understood now, and `hush wind` gets ``There's no wind here.`` *Built* (REQ-ENG-33, REQ-ENG-36).
- REQ-PC-15: Object actions MUST NOT shadow the built-in verbs: an object with a `take` action is still taken by `take`. `search` is the exception, since a bare `search` is the built-in and `search <thing>` is the object's action. *Built* (REQ-ENG-32); whether a world may override a built-in verb for one object is Open question 3.

## Replies

- REQ-PC-16: Every reply MUST be written for the player, in the story's voice where the story gives one. *Built.*
- REQ-PC-17: Detail only an author needs, such as an exit to a room that isn't written yet, MUST travel apart from the reply as an author note (`ActionResult::note`). A client MAY hide author notes; a client for players SHOULD hide them by default. The console shows them. *Built in the engine.*

## Game commands

Commands about the game rather than in the story start with `/`, so a world's verbs never collide with them: a story can have a `save` action ("save the princess") and the player can still save the game.

- REQ-PC-18: Every client MUST offer saving and loading. A typing client MUST accept `/save [name]` and `/load [name]`, with `quicksave` when no name is given; a client with an interface MAY use buttons instead. The engine owns where saves go (REQ-CFG-6) and the save and load calls, so every client stores games the same way: `engines::saves` (`save_named`, `load_named`, `list_saves`, `save_path`, `DEFAULT_SAVE`). *Built* (28ff39f, REQ-ENG-34); the console's `/save` and `/load` use it.
- REQ-PC-19: A game command MUST NOT count as a turn or change the story's state, except that `/load` replaces it.

## Asked for in playtesting

From the console's unknown-input log, 2026-11-23, at Bruce's request. *Proposed*, queued behind his playtest of both modes.

- REQ-PC-21: `where` and `where am i` MUST name the current location, in play and in the console's edit mode. Logged: "where am i?" *Built*: play in d836d0d (0.81.0), edit mode in 5993d29 (0.81.1).
- REQ-PC-22: `enter <place>` MUST be an alias of `go <place>`, and `go` also finds an exit by the room it leads to (`enter the vault`). Logged: "enter closet". *Built* in d836d0d.
- REQ-PC-23: `look <direction>` and `what's <direction>` MUST say what's that way without going there: ``North: Closet.``, or the door's name and state for an exit through a door (``Down: a closed iron door.``), or ``You see no way up.`` Logged: "what's north?" *Built* in d836d0d, and in edit mode in 5993d29.
- REQ-PC-24: An unknown verb that's one small typo from a known one (one letter added, missing, swapped or changed) MUST get `Did you mean "look"?` (plain quotes, since players see no code) instead of REQ-PC-5's reply, and still change nothing. Two equally near name both: `Did you mean "take" or "talk"?` Logged: "lookl". Built in 29592dc (REQ-ENG-53).

## More verbs

Bruce chose these on 2026-11-25, answering Open question 1. A thing's, room's or chapter's own `## When:` section or action for the verb always answers first (REQ-PC-13); what follows is what happens when nothing does.

- REQ-PC-25: `drop <thing>` MUST put a carried thing down in the room, and `give <thing> to <person>` MUST hand it over only when a `## When: #give @thing` section says the person takes it; otherwise the player keeps it. *Built* in 578b312 (REQ-ENG-59, 60).
- REQ-PC-26: `take all` MUST take every loose thing here, one line each, leaving scenery and people. With nothing to take: ``There's nothing here to take.`` *Built* in dad1629.
- REQ-PC-27: `wait` (`z`) MUST let one turn pass (``Time passes.``). `again` (`g`) MUST repeat the last line that was read as a command, not a `/` game command; with none: ``There's nothing to repeat.`` *Built* in dad1629.
- REQ-PC-28: `it` MUST mean the last thing the player named (`take lamp`, then `light it`). With none: ``I'm not sure what "it" means.`` *Built* in dad1629 (REQ-ENG-63).
- REQ-PC-29: `jump`, `run`, `kick` and `hit` MUST be understood with or without a thing, so they never get REQ-PC-5's reply. With no section, each gets an `[engine]` reply: ``You jump on the spot.``, ``Kicking the door does nothing.`` `run` is a verb of its own, not `go` (Bruce).
- REQ-PC-30: These verbs MUST have a built-in effect when nothing answers them (Bruce):
  - `throw <thing>` puts a carried thing down here, as `drop` does, with its own reply (``You throw the lamp. It lands on the floor.``).
  - `smash` and `break` move a thing into its `broken` state if it has one. A thing without one refuses: ``The chest won't break.``
  - `stack <thing> on <thing>` puts a carried thing onto a thing that holds things, as an object that holds objects does today. Anything else refuses: ``You can't stack anything on the table.``
  - `push` and `pull <thing> <direction>` move a thing with `pushable: true` through that exit into the next room, and the player goes with it. Without a direction, or for a thing that isn't pushable, they get a default reply. An object's own `push` or `pull` action (a lever) still answers first.
- REQ-PC-31: `talk to <person>` MUST show the person's `## When: #talk` section, matched by state and `@` tags like any reply, so what they say changes with the story. With none: ``{Person} has nothing to say.`` The content can turn it into a menu (Bruce): a `#talk` section that ends with a list of topics, written like a question set (`- The lantern -> ask @lantern`, `- The war -> ask #the-war`), offers them, numbered, and the next line picks one by number or label and asks it. *Built*: the engine in 2748ecc (REQ-ENG-61), the console's typed menu in f34a451.
- REQ-PC-32: `ask <person> about <topic>` MUST match `## When: #ask @lantern` when the topic names a thing, or `## When: #ask #the-war` for a free-text topic by its slug. With no match: ``{Person} doesn't know anything about that.`` `ask <person>` alone: ``Ask {person} about what?`` A `#talk` or `#ask` section MAY change state with a ` ```thwarter ` block, as any reply section can (`design/SECTIONS.md` › Changing state). *Built* in 2748ecc (REQ-ENG-62).

## Open questions

1. ~~Which verbs come next?~~ Settled 2026-11-25 by Bruce: › More verbs. The topic list shape REQ-PC-31 proposed was built as written.
2. Should `search` and `look around` stay two verbs or become one? Both find faint things today; `search` has its own reply (`design/STATES.md` › Hidden things).
3. May a world override a built-in verb for one object, such as a `take` action on a cursed idol that refuses? Proposed: yes, through a reply section on the thing, `## When: #take`, the same way a door's `## When: #go` answers a refused exit, with the take's outcome named so the idol stays put.
4. Which game commands beyond saving does every client need: `/quit`, `/restart`, `/help`, a list of saves?
5. Should multi-word verbs parse (`turn on lamp`, `look under rug`)? Today the first word is the verb, so a world would name that action `turn` and the player would type `turn lamp`.
