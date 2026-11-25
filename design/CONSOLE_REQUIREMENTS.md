# Console requirements

Requirements for the console: the interactive terminal client for writing (edit mode) and playing (play mode) a world, in `src/console/`. Split from `REQUIREMENTS.md` › 4.6 on 2026-11-23 so Bruce and the console coder can work on them directly. RFC 2119 keywords (MUST, SHOULD, MAY) are used as in `REQUIREMENTS.md`.

The ids REQ-CON-1 onwards are stable; code, tests and other docs cite them. Requirements that cross into the content format or the engine (REQ-CF-*, REQ-ENG-*) stay in `REQUIREMENTS.md`, owned by the commander; a change here that affects one of those goes to the commander as a message.

## Requirements

The console is the interactive terminal client for writing and playing a world. It lives in `src/console/`. Decided by Bruce, 2026-11-23, except where marked as proposed.

The author is a player of sorts, like a game master running a table (Bruce, 2026-11-23). Editing SHOULD feel like being in the world and running it: standing in a room, adding things to it, checking how it plays, and nudging the game's state. It should not feel like filling in forms. New commands are judged against that, and each requirement below gives an example of how it reads at the prompt.

Output reads with air around it (Bruce, 2026-11-24): every block of output has one blank line before it and one after, never two, so a blank line follows what was typed and another comes before the next prompt. A reply that suggests a command puts each command on its own indented line after a blank line, with parameters named as in the `/` menu (`{id}`, `[{title}]`), never inside the sentence. `code` shows in the code style, without its backticks. The examples in this document are written compactly; transcripts record replies without the screen's blank lines, which the screen's own tests check.

```
in bedroom > /new room west hallway

bedroom already has a west exit, to door-bedroom-west. To change it:

  /link west {room}
```

- REQ-CON-1: `thwart edit <world>` MUST open the console in edit mode and `thwart play <world>` in play mode. Both are subcommands of the one `thwart` binary.
- REQ-CON-2: Commands to the console itself MUST start with `/`; anything else is input to the current mode. `/edit` MUST open where edit mode is in full editing (REQ-CON-6), whatever it is, with no kind word needed; from play mode it switches to edit mode at the player's room and opens that (Bruce, 2026-11-23). `/edit room`, `/edit object` and `/edit character`, with or without an id, name something else to open, and `/edit <name>` opens whatever the name means, as `go <name>` finds it, without moving edit mode (Bruce, 2026-11-23). `/play` MUST switch to play mode, and `/help` and `/quit` MUST work in both.
- REQ-CON-3: Edit mode MUST be able to see everything in the world, including hidden state, and change content. Play mode MUST NOT change content files; it MAY write game saves. (Play mode's limits beyond that are proposed, not decided: Open question 2 below.)
- REQ-CON-4: Edit mode MUST navigate the world with play mode's words, not shell words, since the author moves through the world like a game master (Bruce, 2026-11-23; this replaced `cd`, `ls` and `pwd`). Bare directions and `go <direction>` follow exits. `go <name>` goes to any room, area, folder, character, object or door, by its name or by a link in the prose. `go back` returns to the place before. `look` shows where edit mode is: a room, a thing, or what a folder holds. `examine <thing>` shows a thing here without going to it. A name that fits several things gets a question, never a guess. `go /` goes to the world root (built in 0.17.0). Commands that change content keep the `/` prefix.

  ```
  in village/square > go north
  The Well (room)
  in village/well > go elder
  The Elder (character), npcs/elder.md
  in npcs/elder > go back
  in village/well > go objects
  objects/  2 things: lantern, rope
  in objects/ > look
  lantern  Brass Lantern (object)
  rope     Rope (object)
  ```

- REQ-CON-47: At the world's root, edit mode MUST list the rooms first, then the areas, then the world's own folders labelled for what they hold (Bruce, 2026-11-24), and `/edit world` MUST open `world/index.md` directly.

  ```
  in / > look
  bedroom   Bedroom  (room)
  castle/   area: 16 room(s)
  objects/  Global objects: 4
  world/    World settings
  ```

- REQ-CON-48: The room view in edit mode MUST be exactly play's view, with no title line unless the world's `show_room_names` asks for one (Bruce, 2026-11-24: titles are for search and lists); the author's detail goes to the sidebar and `/inspect` instead. HERE leads with the room's title and id, then shows its exits with where they lead, each door's title, id and state, and the people, things and scenery by id. `/inspect [<thing>]` MUST show one thing or door in full: its file, scope, where it starts, its templates, states, actions and sections, its sides for a door, its state in the game, and every problem or warning that names it.

  ```
  in village/square > /inspect lantern
  Brass Lantern (lantern), object
    file       objects/lantern.md
    scope      global
    starts in  room `village/square`
    actions    light
  ```

- REQ-CON-49: `/remove <thing>` MUST delete an object, room, character or door with every reference the writer can rewrite, and MUST confirm first, since there's no undo yet (Bruce, 2026-11-24). It shows the writer's plan (what else goes, files deleted, files changed, references left to fix by hand) and asks y/n; the next line answers, and any other line keeps the thing and runs as typed. Afterwards it says what changed. A room that still holds people or things, or the start room, is refused with what to do first.

- REQ-CON-60: `/start` MUST make the room edit mode is in the world's start, in `world/index.md`'s `start:` (Bruce, 2026-11-24). A world with no `world/index.md` MUST be offered one wherever it's missing (`/edit world`, `/start`): its title, `authors:` with the machine user, and `start:` set to the current room, else the first. `/start` makes it without asking, since that's what was typed.
- REQ-CON-61: In edit mode an exit to a room nothing defines MUST show only as a TO WRITE entry, not in the problems pane (Bruce, 2026-11-24). Play mode and `/validate` still count it as a problem, as `thwart validate` and `package` do.

- REQ-CON-5: Edit mode MUST offer commands that create or change one thing at a time, for example `/new room north <id>` (a new room, linked north of the current one), `/new object`, `/new character`, `/set` (one field), `/link` (add or change one exit) and `/unlink` (remove one exit). Each MUST save the change to the file (REQ-ENG-7) and report any problems it causes (REQ-ENG-8). On a terminal, `/new` and `/add` then open the full-screen editor on what they made or added (Bruce, 2026-11-24); a setting to turn that off MAY follow. The author always names a new entity's `id`; the console MUST NOT make one up, since an `id` can't be renamed yet. In a room, the first word after `/new room` is a direction; outside a room, it's the new room's `id`.
- REQ-CON-41: `/add <thing> [room or character]` MUST put a person or thing into a room, the current one if none is named, or an object into a character's inventory (Bruce, 2026-11-23). A name with no card goes in as scenery (REQ-CF-8). A thing starts in one place (REQ-CF-31), so a thing already somewhere else MUST NOT be added a second time: `/add` asks "desk is in bedroom: move it here? (y/n)", moves it on yes, and changes nothing on no (Bruce, 2026-11-24).

  ```
  in / > /add chair bedroom
  Added chair to bedroom, as scenery with no card yet (/edit object chair gives it one).
  ```

- REQ-CON-42: `/add door` MUST make a door from the room edit mode is in, with its exits, so authors don't write door files by hand (Bruce, 2026-11-23; `design/STATES.md` › Adding a door from the console). The far side faces the opposite way unless given, a far room that doesn't exist yet is made, the id is the engine's `door_id`, and a state hint in the door word picks another template (`door(locked)` is `locked-door`). Without a direction it asks, and the next line answers. `/add <room> <door>` gives a one-sided door its other side.

  ```
  in village/square > /add door west village/hallway
  Added door-village-square-village-hallway-west: west of village/square, east of village/hallway.
  in village/square > /add door(locked) --(south)--> village/cellar
  in village/square > /add door up
  in village/square > /add village/loft door-village-square-up
  ```

- REQ-CON-40: `/move <thing> <room or character>` MUST put a person or thing somewhere else (Bruce, 2026-11-23): a character into another room, with its `default_location` following, or an object into a room, a character's inventory or another object (REQ-CF-31), scenery included, which gets its card first if it has none. It takes the thing out of every room list and inventory that held it, and the thing's own file stays where it is. `/move <id> global|area|local` changes scope instead (REQ-CON-29). A room moves into an area folder: `/move hallway house` makes it `house/hallway`, with its file moved and every exit to it updated, and a room as the target stands for its own area, so `/move hallway bedroom` puts hallway beside bedroom (Bruce, 2026-11-23). The writer's rename does the move, and the report lists every file it updated and any reference left to fix by hand.

  ```
  in village/square > /move lantern village/well
  Moved lantern: village/square → village/well.
  in village/square > /move lantern elder
  Moved lantern: village/well → carried by elder.
  in village/square > /move elder village/well
  Moved elder: village/square → village/well (and its default_location).
  ```

- REQ-CON-6: Edit mode MUST offer full editing of a whole entity, frontmatter and body, inside the console: `/edit room`, `/edit object`, `/edit character`. It MUST NOT hand off to an external `$EDITOR`. Editing an object without a card creates its card (REQ-CF-8). Full editing happens in the terminal, inside the console, and MAY take the whole terminal; it opens the file as it is (Bruce, 2026-11-23). A thing stored as several files (a bundle, `design/STATES.md`) opens as one document with a section per file; saving writes every file only if every section parses.
- REQ-CON-7: `/reload` MUST reload the world from disk, keeping the current location, inventory and flags. If a file fails to parse, or the current room no longer exists, the console MUST keep the previous world and show the problems. Broken references (exits to unwritten rooms, objects without cards) are normal while a world is being written: they MUST be shown as warnings and MUST NOT block the reload. The console SHOULD also reload when a world file changes on disk (after `/reload` works).
- REQ-CON-8: The console's room view SHOULD come from the same renderer as `thwart show`'s text output, so they can't drift apart.
- REQ-CON-9: Console behaviour MUST be testable from transcripts: a scripted input file and the output expected from it. Transcripts are the console's acceptance tests.
- REQ-CON-10: Play mode MUST accept the player commands in `design/PLAYER_COMMANDS.md` (REQ-PC), the one description every player client shares, and parse them through the engine once it does (REQ-PC-1). The console shows author notes on their own line (REQ-PC-17 lets a player client hide them). An unknown command MUST get a short reply naming what it didn't understand and MUST NOT change any state.
- REQ-CON-11: Play mode MUST be able to save a game and load it again, with `/save [name]` and `/load [name]` (`quicksave` when no name is given). Where saves go, and the calls that write and read them, are the engine's, so every player client keeps games alike (`design/PLAYER_COMMANDS.md` › REQ-PC-18): never inside the world folder, keyed by the world's folder and path, or by persona and world for an installed world. The console keeps the commands, their replies, the `/` menu's list of saves and `/status`'s.
- REQ-CON-12: Play state and edit-mode navigation are separate. `/play` resumes the game in progress where it was; `/play here` starts the game in, or moves the player to, the room edit mode is in, for playtesting from where the author is writing. Moving around in edit mode never moves the player. `/reload` during a game keeps the play state and drops anything that no longer exists, with a warning (REQ-CON-7).

## Writing helpers

Edit-mode tools that use a large language model (an AI) on the author's prose. Bruce asked for them on 2026-11-23: review prose, rewrite prose, flesh out points, and apply writing styles. The requirements below are the commander's first draft of that request, for the console coder to review with Bruce; items marked *proposed* go beyond what he asked.

- REQ-CON-13: Edit mode MUST offer four writing helpers on prose: **review** (comments on the text, never changes it), **rewrite** (proposes new wording for the same content), **flesh out** (turns notes or points into prose), and **apply a style** (rewrites the text in a named writing style).
- REQ-CON-14: A helper MUST NOT change a file by itself. It shows its proposal against the current text, and the author accepts it, edits it or discards it. An accepted proposal is saved through the writer like any other edit (REQ-ENG-7).
- REQ-CON-15: A helper works on prose only (principle 8): a selection, a thing's description, one response block, or every section of a bundle. It MAY propose a `title`, and MUST NOT change any other frontmatter.
- REQ-CON-16: A helper's proposal MUST keep every `{{ variable }}`, response marker and author comment in the text it changes. The console MUST check this and refuse a proposal that drops or alters one.
- REQ-CON-17: A helper SHOULD send the model the context the prose sits in: the thing's frontmatter and title, its area, the current chapter's notes, and the style being applied. Author comments are context, not text to rewrite.
- REQ-CON-18: A style is a Markdown file of guidance an author writes, such as voice, tense, tone and words to avoid. *Proposed*: styles live in `world/styles/<name>.md`, and a world, chapter or thing MAY name its default style.
- REQ-CON-19: Sending prose to a model service publishes it to that service. The console MUST say which service and model a helper uses, MUST NOT send anything until the author has turned helpers on for that world, and MUST work fully with helpers off. API keys MUST come from the author's environment or tool configuration, never from the world folder.
- REQ-CON-20: *Proposed*: helpers are authoring tools only. Play mode MUST NOT call a model, so generated text reaches a player only after an author has accepted it into a file (principle 6).
- REQ-CON-21: *Proposed*: the model is configurable, defaulting to a current Claude model.

## Screen and play state

Decided by Bruce, 2026-11-23, except where marked as proposed.

- REQ-CON-22: On a terminal, the console MUST be one full-screen view: a status bar across the top, scrolling output on the left, a sidebar on the right that is always visible, and the input line across the bottom (Bruce chose this layout, 2026-11-23). The status bar names the world and mode, counts warnings, and shows the running build. Watcher notices go into the output. The full-screen editor (REQ-CON-6) opens over the output and keeps the sidebar, where HERE shows the file's problems as they change and which responses the thing has, needs or could have (`design/STATES.md` › Responses). Scripts and transcripts stay line-based, with the same output and no sidebar (REQ-CON-9).

  ```
   Tiny World · edit · 2 warnings              thwart 0.12.0
  ─────────────────────────────────────┬──────────────────
   > look                              │ GAME
                                       │ at  village/square
   Village Square (village/square)     │ chapter  arrival
   A small square of packed earth.     │ carrying  lantern
   An old well stands to the north.    │
   The Elder is here.                  │ HERE  village/square
   You see a brass lantern.            │ exits
   You can go north or east.           │   north  village/well
                                       │   east   village/inn  (not written)
                                       │ people
                                       │   elder    global
                                       │ things
                                       │   lantern  global
                                       │   bench    scenery, no card
                                       │ warnings
                                       │   east exit leads nowhere yet
  ─────────────────────────────────────┴──────────────────
   edit village/square [global]> _
  ```

- REQ-CON-23: The sidebar MUST show the play state under GAME: where the player is, the current chapter, what they carry, every thing that has left its starting state, and the values in play. It shows the game in both modes, so an author writing in edit mode can see the playtest they are running. In edit mode it MUST also show a HERE panel for where edit mode is, like a game master's notes for the room: its exits and which aren't written, the people and things in it with their scope, and its warnings. In play mode HERE is hidden and GAME gets the space. The world's problems and warnings sit at the foot of the sidebar under a line, as an IDE keeps its problems pane (Bruce, 2026-11-23), and `/validate` runs the same check as `thwart validate` (REQ-ENG-44). Between them, MAP draws the rooms one exit out from edit mode's room (or the player's), small enough for the sidebar, and follows as you move; `/map` shows the whole map in the output (Bruce, 2026-11-23). In a folder, HERE lists what the folder holds. The edit-mode prompt is `in <place> > ` (Bruce, 2026-11-23). *Proposed*: on a terminal too narrow for both (under 100 columns), the sidebar folds into one line above the input, and `go /game` still shows everything.
- REQ-CON-24: The running game MUST appear as a virtual `game/` folder at the world root. It isn't on disk, and it is navigated like any other folder. A real `game/` folder on disk only sets the starting state (`design/STATES.md`).

  ```
  in / > go /game
  in game/ > look
  location     village/square
  chapter      arrival
  carrying/    1 thing
  things/      2 changed from their starting state
  values/      1 value
  in game/ > go things/bread
  in game/things/bread > look
  bread: fresh (its starting state)
  ```

- REQ-CON-25: Inside `game/`, `/set` MUST change only the running game, never a file, and say so. It can set a thing's state, the chapter, the player's location, what they carry and values, so an author can set a scene up the way a game master would, then play it.

  ```
  in game/things/bread > /set state stale
  Game: bread is now stale. No file changed.
  in game/ > /set chapter after-the-king
  Game: now in chapter after-the-king. No file changed.
  ```

## Scope

Where a thing's file lives is its scope: global, area or local (`design/STATES.md` › Scope). Decided by Bruce, 2026-11-23, except where marked as proposed.

- REQ-CON-26: `/scope global|area|local` MUST set the scope that the following `/new` commands use, and the prompt MUST show it. The default is global, since most objects and doors are expected to be global.

  ```
  in castle/hall > /scope global
  edit castle/hall [global]> /new object lantern, rope, key tool
  Created 3 objects in objects/, placed in castle/hall.
  ```

- REQ-CON-27: A trailing `in global`, `in area` or `in here` MUST override the scope for one command. `/new object a, b, c <type>` creates several things of one type in one command.

  ```
  edit castle/hall [global]> /new object torch tool in area
  Created object torch in castle/objects/, placed in castle/hall.
  ```

- REQ-CON-28: An area's things go in a folder for their kind inside the area (`castle/objects/torch.md`, `castle/npcs/guard.md`). The console writes them there, and `ls castle/` lists those folders after the rooms. A thing's file placed directly in the area (`castle/torch.md`) MUST also load and edit normally, for authors who prefer it.
- REQ-CON-29: `/move <id> global|area|local` MUST change a thing's scope by moving its file. References stay valid, since object, character and door ids don't include their folder. It refuses if a file is already at the target, and the file watcher reports the move as one change.

  ```
  edit castle/hall [global]> /move torch global
  Moved torch: castle/objects/torch.md → objects/torch.md.
  ```

## Live reload

The console restarts itself onto a new build in place, so it can be changed and tried without losing the session (Bruce, 2026-11-23). It is an early feature, so the console can be iterated on quickly. Decided by Bruce, 2026-11-23, except where marked as proposed.

- REQ-CON-43: The console MUST keep each world's command history between sessions (Bruce, 2026-11-24), so Up reaches commands from earlier sessions, and a fresh start shows the last few under "Last time:". It lives in the state folder beside the world's saves, never in the world.
- REQ-CON-30: `/restart` MUST restart the console onto the `thwart` binary now on disk, in the same terminal, and put the author back where they were: the mode, edit mode's place, the running game, the scope default, the output's scrollback, the input history, and an open editor with its unsaved text and cursor. The console never restarts by itself. If the new binary doesn't start, the console stays on the one running and says why.

  ```
  (another pane: cargo build finishes)
  in village/square > /restart
  Restarted on thwart 0.12.1 (built 14:02). You're where you were.
  in village/square > _
  ```

- REQ-CON-31: If the new build can't take part of the session as it was, such as a game saved in a shape that changed or a place that no longer exists, that part MUST start fresh with a notice saying what was reset. Everything else carries over, and the restart goes ahead.

  ```
  in village/square > /restart
  Restarted on thwart 0.12.1. The game was reset: its saved shape changed in this build.
  in village/square > _
  ```

- REQ-CON-32: *Proposed*: when a newer `thwart` has been built while the console runs, the status bar says so (`newer build: /restart`).
- REQ-CON-33: Console commands MUST follow Claude Code's conventions for slash commands, such as `/restart` to restart (Bruce, 2026-11-23). Typing `/` opens a list of the commands that work right now, each with a one-line description, which narrows as you type. Tab or Enter picks one, Esc closes the list, and `/help` lists everything. Arguments complete from the world too: after `go ` or `/edit object `, the list offers the exits, rooms and things that fit.

  ```
   edit village/square [global]> /re_
     /reload    reload the world from disk, staying where you are
     /restart   restart the console on the newest build

   in village/square > /edit object _
     lantern   brass lantern, global
     bench     scenery, no card yet
     rope      not here: objects/rope.md
  ```

## Authors

Who is writing, as a persona (`REQUIREMENTS.md` › 5.2, REQ-WS-4 to 6). Decided by Bruce, 2026-11-23; built after the lines above (`PLAN.md`).

- REQ-CON-34: An author is the machine user for now (REQ-WS-5, 7; Bruce, 2026-11-24): the persona id is `$USER`, with no first-run ask. Choosing a persona waits.
- REQ-CON-35: `/author`, switching persona, waits with REQ-CON-34. The status bar MAY show the user writing.
- REQ-CON-36: A new world MUST be written with `authors: $USER` in its `world/index.md`. `authors:` is one id or a list; `author:` is still read.
- REQ-CON-37: A persona's contact details ship with every packaged world (REQ-WS-6), so before packaging the console MUST say which details will go with it.

## Colour

Decided by Bruce, 2026-11-23.

- REQ-CON-38: The console MUST take its colours from the terminal: the default foreground and background and the 16 named ANSI colours, with bold, dim, italic, underline and reverse. It never sets an RGB colour, so whatever theme the terminal has, light or dark, the console follows it. `NO_COLOR` turns colour off and keeps the rest. Colours go by role:

  ```
  chrome ................ blue: bars are blue with bright white text;
                          the sidebar's border and line numbers are blue
  console commands ...... the `/` bold yellow, the name bold, arguments plain,
                          as typed and as echoed in the output
  your input (> look) ... bold
  room and thing titles . bold
  exits, links .......... cyan
  not written, missing .. yellow
  warnings .............. yellow
  errors, refusals ...... red
  author notes, Game: ... dim
  console messages ...... green  (Saved, Reloaded, Restarted)
  sidebar headings ...... bold
  prompt ................ bold green in edit mode, bold magenta in play mode
  ```

- REQ-CON-39: The editor MUST highlight what it shows, in the same colours. Frontmatter: `---` fences dim, keys blue, ids that resolve cyan and ids that don't yellow. Prose wraps at words (Bruce, 2026-11-23) and is styled as Markdown: headings bold with their `#` marks dim, emphasis italic, strong bold, ~~struck~~ crossed out, links cyan and underlined, `#tags` magenta, code dim, list markers and quote marks in the chrome colour with quoted words in italics, and rules dim. Section headings (`## When: #locked`, `## Unless:`, `design/SECTIONS.md`) are drawn as rules in the chrome colour, a ```thwarter block is read like frontmatter, `@name` is cyan when any thing by that id exists (object, character or door) and yellow when not, and `{{ variables }}` are cyan when declared and yellow when not. A bundle shown as one document uses the same `## When: #state` headings for its state files. Chrome: line numbers dim with the current one bold, the heading bar reversed, and the status line coloured by what it says (green saved, red refused, yellow a warning).

## Making things well

Bruce, 2026-11-24: console commands should solve common problems easily, through small verbs, recipes built from them, and questions that `/add` and `/new` can ask. The question file's format is in `design/CONTENT_FORMAT.md` (REQ-CF-32). REQ-CON-44 to 46 are built (46 in 0.61.0).

- REQ-CON-44: Small verbs MUST cover states and actions: `/state <name> [start]` makes a state (turning a one-file thing into a bundle first), `/set <key> <value> in <state>` changes one state's file, and `/action <verb> <from> --> <to>` writes the action and opens its reply section in the editor, making the `to` state if it doesn't exist.
- REQ-CON-45: Recipes MUST be shorthand for those verbs, and MUST print the commands they ran so an author learns them. First four, as Bruce picked: `fixed-until <verb>` (an immovable starting state and a free one), `locked-with <key>`, `switch <verbs> <states>` (two states with an action each way), and `container in|on`.

  ```
  in objects/lantern > /recipe fixed-until pull

  That was:

    /state hanging start
    /set weight immovable in hanging
    /action pull hanging --> loose
  ```

- REQ-CON-46: When `/add` or `/new` makes something new and there's a question file for it, they MUST ask its questions (placing a thing that exists asks nothing) in a picker like the `/` menu's, then run the answers and print the commands they stood for. Esc takes the defaults for the rest. The file is `questions/<id, template or kind>.md`, tried in that order (d2665a8) and found the way templates are, from the new thing's folder out to the root and then the built-ins compiled in from `src/builtins/questions/`; the nearest whole file wins, a template's set before its kind's (`design/CONTENT_FORMAT.md` › `type: questions`, REQ-CF-32). An answer uses one of four constructs: `set <key> [<value>]` (a written-in value asks nothing, dc66a64), `recipe …`, `ask thing`, or `ask value <name> <default>`.

  ```
   A lantern, from the built-in lantern questions                  2 of 4
   Where does it start?
   ▸ 1. Loose, it can be taken
     2. Hanging on a wall, pulled free
     3. On something here
   ───────────────── ↑↓ choose · enter pick · esc use defaults for the rest
  ```



## To write

Bruce, 2026-11-24. A name the prose mentions but nothing defines (`[[hole]]`, `@wallpaper`) is a warning per mention today, so hallway.md's three `wallpaper` mentions make three warnings.

- REQ-CON-57: The sidebar MUST show a TO WRITE list: each name the world mentions but doesn't define, once, with how many times it's mentioned when that's more than once, and its key. It comes from the engine's not-written-yet list (grouped by name, each mention's file), not from warning text.

  ```
  TO WRITE
    ⌃1  wallpaper  ×3
    ⌃2  hole
  ```

- REQ-CON-58: Picking an entry MUST make it with defaults and open it in the editor, as `/add` would but without the question picker: the card REQ-CF-8 makes (scenery, title from the id), listed in the first room whose prose names it. A name an exit leads to becomes a room instead. Changing it afterwards is a normal `/set`.
- REQ-CON-59: Each of the first nine entries MUST have its own key, Ctrl-1 to Ctrl-9 in list order, like browser tabs (Bruce, 2026-11-24: Ctrl, not Cmd): no leader, no letter. Alt-1 to Alt-9 MUST do the same. Entries past nine have no key; `/write <name>` reaches them, and `/write [<name>]` does the same for any entry from the prompt. Ctrl-9 is the ninth entry, not the last. Ctrl-digits arrive as themselves only through the kitty keyboard protocol (iTerm2 3.5+, Ghostty, kitty, WezTerm). Other terminals, such as Terminal.app, send NUL, Esc, control codes or plain digits instead, which MUST NOT be read as TO WRITE keys; Esc stays Esc. In those terminals Alt is the key. The sidebar shows Ctrl's (⌃1 on macOS). The keys MUST work in the editor too, like switching tabs: it saves the open document first, and if that save fails it stays open with the reason. The editor binds no Ctrl- or Alt-digit, so nothing clashes; if one ever does, the TO WRITE key wins.

## Command registry

Bruce approved this, 2026-11-24 (the author rule is REQ-WS-5, 7 to 9). Today one command is written in five places (the `/` menu table, the dispatch `match`, `/help`, the transcripts' list of commands that change content, and the handler's own usage text), and they drift: 23 menu entries, 31 help lines, 34 dispatch arms. This section replaces them with one registry.

- REQ-CON-50: Every console command MUST be described once, in a `type: command` Markdown file, and the `/` menu, `/help`, usage replies, argument completion, the mode guard and the test guard MUST all read that description. Nothing else lists commands.
- REQ-CON-51: A command file's frontmatter has `name`, `aliases`, `about` (the one-line summary the `/` menu and `/help` list show), `args` (named parameters as the `/` menu shows them: `{thing} [{room}]`), `modes` (`edit`, `play`), `changes` (whether it writes to the world), and one of `handler` (a built-in), `run` or `script` (a world command, REQ-CON-54). Its body is the help prose and examples, which `/help <command>` shows.

  ```markdown
  ---
  type: command
  name: remove
  aliases: [rm, delete]
  about: delete a thing, room or door and what refers to it, after asking
  args: "{thing}"
  modes: [edit]
  changes: true
  handler: remove
  ---
  Deletes a thing, room or door, and every reference the writer can
  rewrite. It shows what goes and asks first, since there's no undo.

      /remove lantern
  ```

- REQ-CON-52: Commands load in layers, later ones adding or changing: built-in `src/builtins/commands/<name>.md`; then the world's `world/commands/<name>.md`; then the person's `config.toml` `[console.commands.<name>]`, which MAY add aliases, reword `about` or hide a command, and MUST NOT add one. Per-reply message text stays in `[console]` (REQ-ENG-49).
- REQ-CON-53: Built-in handlers stay in Rust, registered by the `handler` name. Every built-in file MUST have a handler and every handler a file; a test fails on either gap. The console checks a command's arguments against `args` before its handler runs, and a wrong count answers with the usage built from `args`.
- REQ-CON-54: A world command MUST be either a `run:` list of existing console commands with `{param}` slots, run as if typed, or a Rhai `script` that reads the world and runs console commands, with the world scripts' limits (no files, no network). Either way each command's own confirmation still asks, so it does no more than the author could type (REQ-WS-9). It MUST NOT run `/author`, personal config, packaging or install, the writing helpers, or another world command. It MUST NOT shadow a built-in: the built-in wins, with a load warning. `/help` lists world commands under the world's title.
- REQ-CON-55: World commands work only in edit mode, and only when the machine user (`$USER`) is in the world's `authors:` and the world isn't installed (REQ-WS-7, 8; Bruce, 2026-11-24). Otherwise they're off, and naming one says why; with no `authors:` it offers to add the current user.
- REQ-CON-56: Commands SHOULD be forgiving (Bruce, 2026-11-24). Built-in aliases ship (`/rm`, `/ls`). A unique prefix runs its command. A mistyped name answers "did you mean". A bare command name with no slash in edit mode runs it when it isn't also a play verb, else asks.

## Open questions

1. ~~What does full editing look like on screen?~~ Decided 2026-11-23 (Bruce): in the terminal, inside the console, and it may take the whole terminal; REQ-CON-6.
2. What can play mode do and not do? Assumed until Bruce says otherwise: in-game verbs only, no changes to content, game saves only (REQ-CON-3).
3. Should `/new room north` also add the return exit (`south`) to the new room? Proposed: yes for compass directions and `up`/`down`, no for free-text directions. Implemented as proposed in 0.6.1, pending Bruce.
4. Writing helpers: which model services to support (one, several, or a local model), and can "flesh out" also propose new things, such as rooms from an outline, or only prose?
5. ~~Writing helpers: where styles live~~ Settled: in `world/styles/` (`design/STATES.md` › world/ layout, REQ-CON-18).
6. Scope: what does `local` mean in a room? Making an object local to a room turns the room into a bundle (`castle/hall.md` becomes `castle/hall/index.md`, with `torch.md` beside it). Proposed: `/new ... in here` asks first, or works only in a room that is already a bundle.
7. ~~Scope: must object, character and door ids be unique across the whole world?~~ Decided 2026-11-23: yes, whatever their scope, and `/move` keeps the id (`design/CONTENT_REQUIREMENTS.md` › REQ-CF-22). Templates and chapters are the exception: named per scope and looked up along the lineage.
