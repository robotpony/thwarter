# Console requirements

Requirements for the console: the interactive terminal client for writing (edit mode) and playing (play mode) a world, in `src/console/`. Split from `REQUIREMENTS.md` › 4.6 on 2026-11-23 so Bruce and the console coder can work on them directly. RFC 2119 keywords (MUST, SHOULD, MAY) are used as in `REQUIREMENTS.md`.

The ids REQ-CON-1 onwards are stable; code, tests and other docs cite them. Requirements that cross into the content format or the engine (REQ-CF-*, REQ-ENG-*) stay in `REQUIREMENTS.md`, owned by the commander; a change here that affects one of those goes to the commander as a message.

## Requirements

The console is the interactive terminal client for writing and playing a world. It lives in `src/console/`. Decided by Bruce, 2026-11-23, except where marked as proposed.

The author is a player of sorts, like a game master running a table (Bruce, 2026-11-23). Editing SHOULD feel like being in the world and running it: standing in a room, adding things to it, checking how it plays, and nudging the game's state. It should not feel like filling in forms. New commands are judged against that, and each requirement below gives an example of how it reads at the prompt.

- REQ-CON-1: `thwart edit <world>` MUST open the console in edit mode and `thwart play <world>` in play mode. Both are subcommands of the one `thwart` binary.
- REQ-CON-2: Commands to the console itself MUST start with `/`; anything else is input to the current mode. `/edit` and `/play` MUST switch modes inside a running console (`/edit` followed by a kind, such as `/edit room`, opens full editing instead, REQ-CON-6; from play mode it switches to edit mode and opens the current room), and `/help` and `/quit` MUST work in both.
- REQ-CON-3: Edit mode MUST be able to see everything in the world, including hidden state, and change content. Play mode MUST NOT change content files; it MAY write game saves. (Play mode's limits beyond that are proposed, not decided: Open question 2 below.)
- REQ-CON-4: Edit mode MUST navigate the world with play mode's words, not shell words, since the author moves through the world like a game master (Bruce, 2026-11-23; this replaced `cd`, `ls` and `pwd`). Bare directions and `go <direction>` follow exits. `go <name>` goes to any room, area, folder, character, object or door, by its name or by a link in the prose. `go back` returns to the place before. `look` shows where edit mode is: a room, a thing, or what a folder holds. `examine <thing>` shows a thing here without going to it. A name that fits several things gets a question, never a guess. `go /` goes to the world root (built in 0.17.0). Commands that change content keep the `/` prefix.

  ```
  edit village/square> go north
  The Well (room)
  edit village/well> go elder
  The Elder (character), npcs/elder.md
  edit npcs/elder> go back
  edit village/well> go objects
  objects/  2 things: lantern, rope
  edit objects/> look
  lantern  Brass Lantern (object)
  rope     Rope (object)
  ```

- REQ-CON-5: Edit mode MUST offer commands that create or change one thing at a time, for example `/new room north <id>` (a new room, linked north of the current one), `/new object`, `/new character`, `/set` (one field), `/link` (add or change one exit) and `/unlink` (remove one exit). Each MUST save the change to the file (REQ-ENG-7) and report any problems it causes (REQ-ENG-8). The author always names a new entity's `id`; the console MUST NOT make one up, since an `id` can't be renamed yet. In a room, the first word after `/new room` is a direction; outside a room, it's the new room's `id`.
- REQ-CON-6: Edit mode MUST offer full editing of a whole entity, frontmatter and body, inside the console: `/edit room`, `/edit object`, `/edit character`. It MUST NOT hand off to an external `$EDITOR`. Editing an object without a card creates its card (REQ-CF-8). Full editing happens in the terminal, inside the console, and MAY take the whole terminal; it opens the file as it is (Bruce, 2026-11-23). A thing stored as several files (a bundle, `design/STATES.md`) opens as one document with a section per file; saving writes every file only if every section parses.
- REQ-CON-7: `/reload` MUST reload the world from disk, keeping the current location, inventory and flags. If a file fails to parse, or the current room no longer exists, the console MUST keep the previous world and show the problems. Broken references (exits to unwritten rooms, objects without cards) are normal while a world is being written: they MUST be shown as warnings and MUST NOT block the reload. The console SHOULD also reload when a world file changes on disk (after `/reload` works).
- REQ-CON-8: The console's room view SHOULD come from the same renderer as `thwart show`'s text output, so they can't drift apart.
- REQ-CON-9: Console behaviour MUST be testable from transcripts: a scripted input file and the output expected from it. Transcripts are the console's acceptance tests.
- REQ-CON-10: Play mode MUST accept at least `look` (`l`), `go <direction>` and bare directions (`north`, `n`, `ne` and so on, `up`/`u`, `down`/`d`), `take` (`get`), `examine` (`x`, `look at`) and `inventory` (`i`). An unknown command MUST get a short reply naming what it didn't understand (``I don't understand `dance`.``) and MUST NOT change any state. Replies are written for the player; detail only an author needs, such as an exit to a room not yet written, is printed on its own line as an author note. Whether play mode can hide author notes is part of Open question 2 below.
- REQ-CON-11: Play mode MUST be able to save a game and load it again, with `/save [name]` and `/load [name]` (`quicksave` when no name is given). Saves MUST NOT be written inside the world folder, since worlds are content repositories and saves would end up committed. They go in `$XDG_STATE_HOME/thwarter/saves/<world>/<name>.yaml` (`~/.local/state` when `$XDG_STATE_HOME` isn't set), where `<world>` is the world folder's name plus a short hash of its full path, so two worlds that are both called `test-world` don't share saves.
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
   Village Square                      │ at  village/square
   A small square of packed earth.     │ chapter  arrival
   An old well stands to the north.    │ carrying  lantern
   Exits: north (village/well),        │
     east (village/inn, not written)   │ HERE  village/square
   Here: The Elder.                    │ exits
   You see: brass lantern, bench.      │   north  village/well
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

- REQ-CON-23: The sidebar MUST show the play state under GAME: where the player is, the current chapter, what they carry, every thing that has left its starting state, and the values in play. It shows the game in both modes, so an author writing in edit mode can see the playtest they are running. In edit mode it MUST also show a HERE panel for where edit mode is, like a game master's notes for the room: its exits and which aren't written, the people and things in it with their scope, and its warnings. In play mode HERE is hidden and GAME gets the space. *Proposed*: on a terminal too narrow for both (under 100 columns), the sidebar folds into one line above the input, and `go /game` still shows everything.
- REQ-CON-24: The running game MUST appear as a virtual `game/` folder at the world root. It isn't on disk, and it is navigated like any other folder. A real `game/` folder on disk only sets the starting state (`design/STATES.md`).

  ```
  edit /> go /game
  edit game/> ls
  location     village/square
  chapter      arrival
  carrying/    1 thing
  things/      2 changed from their starting state
  values/      1 value
  edit game/> go things/bread
  edit game/things/bread> look
  bread: fresh (its starting state)
  ```

- REQ-CON-25: Inside `game/`, `/set` MUST change only the running game, never a file, and say so. It can set a thing's state, the chapter, the player's location, what they carry and values, so an author can set a scene up the way a game master would, then play it.

  ```
  edit game/things/bread> /set state stale
  Game: bread is now stale. No file changed.
  edit game/> /set chapter after-the-king
  Game: now in chapter after-the-king. No file changed.
  ```

## Scope

Where a thing's file lives is its scope: global, area or local (`design/STATES.md` › Scope). Decided by Bruce, 2026-11-23, except where marked as proposed.

- REQ-CON-26: `/scope global|area|local` MUST set the scope that the following `/new` commands use, and the prompt MUST show it. The default is global, since most objects and doors are expected to be global.

  ```
  edit castle/hall> /scope global
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

- REQ-CON-30: `/restart` MUST restart the console onto the `thwart` binary now on disk, in the same terminal, and put the author back where they were: the mode, edit mode's place, the running game, the scope default, the output's scrollback, the input history, and an open editor with its unsaved text and cursor. The console never restarts by itself. If the new binary doesn't start, the console stays on the one running and says why.

  ```
  (another pane: cargo build finishes)
  edit village/square> /restart
  Restarted on thwart 0.12.1 (built 14:02). You're where you were.
  edit village/square> _
  ```

- REQ-CON-31: If the new build can't take part of the session as it was, such as a game saved in a shape that changed or a place that no longer exists, that part MUST start fresh with a notice saying what was reset. Everything else carries over, and the restart goes ahead.

  ```
  edit village/square> /restart
  Restarted on thwart 0.12.1. The game was reset: its saved shape changed in this build.
  edit village/square> _
  ```

- REQ-CON-32: *Proposed*: when a newer `thwart` has been built while the console runs, the status bar says so (`newer build: /restart`).
- REQ-CON-33: Console commands MUST follow Claude Code's conventions for slash commands, such as `/restart` to restart (Bruce, 2026-11-23). Typing `/` opens a list of the commands that work right now, each with a one-line description, which narrows as you type. Tab or Enter picks one, Esc closes the list, and `/help` lists everything. Arguments complete from the world too: after `go ` or `/edit object `, the list offers the exits, rooms and things that fit.

  ```
   edit village/square [global]> /re_
     /reload    reload the world from disk, staying where you are
     /restart   restart the console on the newest build

   edit village/square> /edit object _
     lantern   brass lantern, global
     bench     scenery, no card yet
     rope      not here: objects/rope.md
  ```

## Authors

Who is writing, as a persona (`REQUIREMENTS.md` › 5.2, REQ-WS-4 to 6). Decided by Bruce, 2026-11-23; built after the lines above (`PLAN.md`).

- REQ-CON-34: On first run, the console MUST ask who's writing. It lists the personas in `authors/` or offers to create one, writes `authors/<name>/index.md` for a new one, and records the choice in `~/.config/thwarter/config.toml`.

  ```
  Who's writing?
    1. mx      Bruce Alderson
    2. (a new persona)
  > 1
  Writing as mx. /author switches.
  ```

- REQ-CON-35: `/author` MUST switch persona, and the status bar MUST show who you're writing as.
- REQ-CON-36: A new world MUST go under the current persona, with `author: <id>` in its `world/index.md`.
- REQ-CON-37: A persona's contact details ship with every packaged world (REQ-WS-6), so before packaging the console MUST say which details will go with it.

## Open questions

1. ~~What does full editing look like on screen?~~ Decided 2026-11-23 (Bruce): in the terminal, inside the console, and it may take the whole terminal; REQ-CON-6.
2. What can play mode do and not do? Assumed until Bruce says otherwise: in-game verbs only, no changes to content, game saves only (REQ-CON-3).
3. Should `/new room north` also add the return exit (`south`) to the new room? Proposed: yes for compass directions and `up`/`down`, no for free-text directions. Implemented as proposed in 0.6.1, pending Bruce.
4. Writing helpers: which model services to support (one, several, or a local model), and can "flesh out" also propose new things, such as rooms from an outline, or only prose?
5. Writing helpers: where styles live, until `world/`'s layout is settled (`design/STATES.md` › Open questions).
6. Scope: what does `local` mean in a room? Making an object local to a room turns the room into a bundle (`castle/hall.md` becomes `castle/hall/index.md`, with `torch.md` beside it). Proposed: `/new ... in here` asks first, or works only in a room that is already a bundle.
7. ~~Scope: must object, character and door ids be unique across the whole world?~~ Decided 2026-11-23: yes, whatever their scope, and `/move` keeps the id (`design/CONTENT_REQUIREMENTS.md` › REQ-CF-22). Templates and chapters are the exception: named per scope and looked up along the lineage.
