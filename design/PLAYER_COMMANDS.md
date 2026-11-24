# Player commands

The command language every client shares (Bruce, 2026-11-23): what a player can type, how a line becomes an action, and how the game replies. A terminal player comes first; later clients with a real interface (Godot, Swift, Node) send the same lines or the same actions. The engine parses and answers; a client only reads input and shows replies.

The console's play mode is one such client. Its own commands (`/edit`, `/play here` and the rest) are in `design/CONSOLE_REQUIREMENTS.md`, not here.

Owner: commander. Ids: REQ-PC. Each requirement says whether it's built, as of 0.43.2.

## Summary

A line is a verb and maybe a noun. The engine turns it into an action (look, go, take, examine, inventory, search, or an object's own named action) or into a reply that changes nothing. Nouns match things in scope by id, name or a unique word of either. Every reply is written for the player; detail only an author needs travels separately as an author note. Game commands that aren't part of the story, such as saving, start with `/` so they never collide with a story's verbs.

## How a line is read

- REQ-PC-1: The engine MUST parse a typed line into an action, a reply, or an unknown line, through one call any client can use: `GameEngine::parse_command(&WorldEngine, line, &PlayState) -> Parsed`. `Parsed` is `Action`, `Reply` (a verb missing its noun) or `Unknown` (REQ-PC-5's reply, kept apart so a client can log what players tried). Only an `Action` goes on to `act`, so only it can count as a turn. A client MAY also build actions directly (a button for "north"), skipping the parser. *Built* (0fe9754, REQ-ENG-32), and the console's play mode parses through it (a1e8f1b).
- REQ-PC-2: Parsing MUST ignore case and surrounding space, and split the line into words. *Built.*
- REQ-PC-3: An article (`the`, `a`, `an`) MUST be dropped before a noun: `take the lantern`, `look at a well`. *Built.*
- REQ-PC-4: A known verb with no noun MUST get a reply asking for one, and change nothing: `Take what?`, `Go where?`, `Look at what?`. A verb that some object has as an action counts as known: a bare `pull` gets `What do you want to pull?`. *Built.*
- REQ-PC-5: A line the engine can't read MUST get ``I don't understand `<line>`.``, and change nothing. It MUST NOT count as a turn (REQ-ENG-23). That includes a verb no thing has, alone (`dance`), and a built-in verb used another way (`i lamp`, `n please`), which never becomes an object action. *Built.*

## Verbs

| Verb | Also | Action | Built |
|---|---|---|---|
| `look` | `l` | Describe the room | yes |
| `look at <thing>` | `look <thing>`, `examine`, `x`, `inspect` | Describe a thing | yes |
| `look around` | `examine room`, `x here`, the room's name | Examine the room, finding faint things | yes |
| `search` | | Search the room, finding faint things | yes |
| `search <thing>` | | An object's own `search` action, if it has one (a desk) | yes |
| `go <direction>` | `walk`, `move`, a bare direction | Move through an exit | yes |
| `take <thing>` | `get`, `pick up` | Pick up an object | yes |
| `inventory` | `i`, `inv` | List what the player carries | yes |
| `<verb> <thing>` | | An object's own named action (`pull lever`) | yes |

- REQ-PC-6: The verbs and aliases above MUST be accepted by every client. The verb is the line's first word, apart from `look at` and `pick up`; multi-word verbs such as `turn on` aren't parsed (Open question 5). A client MUST NOT add story verbs of its own; a world adds them as object actions (REQ-PC-13).
- REQ-PC-7: Bare directions MUST move the player: `north`/`n`, `northeast`/`ne`, `east`/`e`, `southeast`/`se`, `south`/`s`, `southwest`/`sw`, `west`/`w`, `northwest`/`nw`, `up`/`u`, `down`/`d`, `in`, `out`. A bare direction followed by more words (`n please`) is not understood. *Built.*
- REQ-PC-8: `go` MUST also take an exit's free-text name, since direction names are free text (`design/CONTENT_FORMAT.md` › `connections`): `go behind`, `go through the hedge`. *Built.*

## Nouns

- REQ-PC-9: A noun MUST match among the things in scope, in this order: an exact id; the id with spaces for hyphens (`brass lantern` for `brass-lantern`); the name, ignoring case; then a whole word of the name or id that only one thing has (`lantern`). The first rule that matches wins. *Built* (`game_engine::resolve`).
- REQ-PC-10: In scope means what the player can see or carries: the room itself (by name, `room`, `here` or `around`), what's listed in it, faint things already found, and the inventory. Hidden and absent things follow `design/STATES.md` › Hidden things. *Built* (REQ-ENG-25).
- REQ-PC-11: A noun that matches several things MUST get a reply in the story's voice naming them, so the player can choose: ``Which sword do you want to take: the rusty sword or the old sword?`` The story MAY replace it (`design/STATES.md` › Ambiguity, REQ-ENG-26). *Built.*
- REQ-PC-12: A noun that matches nothing MUST get ``There's no <noun> here.`` *Built.*

## Object actions

- REQ-PC-13: A verb that isn't in the table, followed by a noun, MUST be tried as that object's named action (`light lantern`, `pull lever`; `design/STATES.md` › Changing state). The engine checks `requires:`, applies the action and replies with its response block (REQ-ENG-24). *Built* (`PlayAction::Use`, REQ-ENG-32).
- REQ-PC-14: When the noun matches a thing that doesn't have the action, the reply MUST be ``You can't <verb> the <thing>.`` When no thing in the world has that verb as an action, the reply MUST be REQ-PC-5's ``I don't understand `<line>`.``, so `dance wildly` doesn't answer "There's no wildly here." "In the world" means any state or chapter of any object or door, not only the current ones, so a verb a gong gains later (`hush`) is understood now, and `hush wind` gets ``There's no wind here.`` *Built* (REQ-ENG-33).
- REQ-PC-15: Object actions MUST NOT shadow the built-in verbs: an object with a `take` action is still taken by `take`. `search` is the exception, since a bare `search` is the built-in and `search <thing>` is the object's action. *Built* (REQ-ENG-32); whether a world may override a built-in verb for one object is Open question 3.

## Replies

- REQ-PC-16: Every reply MUST be written for the player, in the story's voice where the story gives one. *Built.*
- REQ-PC-17: Detail only an author needs, such as an exit to a room that isn't written yet, MUST travel apart from the reply as an author note (`ActionResult::note`). A client MAY hide author notes; a client for players SHOULD hide them by default. The console shows them. *Built in the engine.*

## Game commands

Commands about the game rather than in the story start with `/`, so a world's verbs never collide with them: a story can have a `save` action ("save the princess") and the player can still save the game.

- REQ-PC-18: Every client MUST offer saving and loading. A typing client MUST accept `/save [name]` and `/load [name]`, with `quicksave` when no name is given; a client with an interface MAY use buttons instead. The engine owns where saves go (REQ-CFG-6) and the save and load calls, so every client stores games the same way: `engines::saves` (`save_named`, `load_named`, `list_saves`, `save_path`, `DEFAULT_SAVE`). *Built* (28ff39f, REQ-ENG-34); the console's `/save` and `/load` use it.
- REQ-PC-19: A game command MUST NOT count as a turn or change the story's state, except that `/load` replaces it.

## Open questions

1. Which verbs come next? Candidates: `drop`, `wait` (`z`), `again` (`g`), `it` for the last thing named, `take all`, and talking to characters, which waits on the dialogue format (`design/CONTENT_FORMAT.md` › What's still open). Bruce to choose.
2. Should `search` and `look around` stay two verbs or become one? Both find faint things today; `search` has its own reply (`design/STATES.md` › Hidden things).
3. May a world override a built-in verb for one object, such as a `take` action on a cursed idol that refuses? Proposed: yes, through a `take refused` response block, the same way doors use `go refused`.
4. Which game commands beyond saving does every client need: `/quit`, `/restart`, `/help`, a list of saves?
5. Should multi-word verbs parse (`turn on lamp`, `look under rug`)? Today the first word is the verb, so a world would name that action `turn` and the player would type `turn lamp`.
