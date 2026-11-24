//! Play mode: the game in progress, its verbs (REQ-CON-10), `/play here`
//! (REQ-CON-12) and saves (REQ-CON-11). The engine applies each action;
//! this module turns input into actions and results into text. Nothing
//! here writes a content file (REQ-CON-3).

use crate::engines::{list_saves, save_named, save_path, DEFAULT_SAVE};
use super::session::{Console, Place};
use crate::engines::Parsed;
use crate::models::game::PlayState;
use super::world::Kind;
use super::Mode;
use crate::engines::load_game;
use crate::models::game::ActionResult;

impl Console {
    /// One line of play-mode input.
    /// One line of play-mode input, read by the engine's parser, which
    /// every player client shares (`design/PLAYER_COMMANDS.md`, REQ-PC-1).
    /// Object actions ("light lantern") and `search` come through it too.
    pub(super) fn play_input(&mut self, line: &str) -> String {
        let empty = PlayState::default();
        let state = self.game.as_ref().unwrap_or(&empty);
        let action = match self.game_engine.parse_command(&self.world.engine, line, state) {
            Parsed::Action(action) => action,
            Parsed::Reply(reply) => return reply,
            Parsed::Unknown(reply) => {
                self.log_unknown("play", line);
                return reply;
            }
        };
        let Some(state) = self.game.as_mut() else {
            return "No game is in progress. /play here starts one in the room edit mode is in.".to_string();
        };
        let result = self.game_engine.act(&self.world.engine, state, &action);
        // On a terminal, the author note goes to the sidebar; scripts and
        // transcripts keep it in the output.
        if self.interactive {
            self.notes = result.note.iter().cloned().collect();
            return result.text;
        }
        show(result)
    }

    /// The player's room, as the player sees it.
    pub(super) fn play_look(&self) -> String {
        let Some(state) = &self.game else {
            return "No game is in progress. /play here starts one in the room edit mode is in.".to_string();
        };
        match self.game_engine.view(&self.world.engine, state) {
            // The player's text, in the world's own wording (`[play]` in
            // world.toml); ids and author detail go to the sidebar.
            Some(view) => {
                let engine = &self.world.engine;
                crate::render::play_blocks(&view, &|n| engine.link_for(n), engine.play_text()).to_plain()
            }
            None => format!(
                "You're nowhere: `{}` isn't in the world.\nAuthor note: /edit, then /play here from a room, moves the player.",
                state.location
            ),
        }
    }

    /// Starts a game in `room`, replacing any game in progress.
    pub(super) fn start_game(&mut self, room: &str) -> Result<(), String> {
        let state = self.game_engine.start(&self.world.engine, room).map_err(|e| format!("Can't start there: {}", e))?;
        self.game = Some(state);
        Ok(())
    }

    /// `/play`: resumes the game where it was, or starts one in the room
    /// edit mode is in (else the world's starting room).
    pub(super) fn play(&mut self) -> String {
        if self.mode == Mode::Play {
            return "Already in play mode.".to_string();
        }
        if self.game.is_none() {
            let Some(room) = self.edit_room().or_else(|| self.world.default_room()) else {
                return "This world has no rooms to play in.".to_string();
            };
            if let Err(e) = self.start_game(&room) {
                return e;
            }
        }
        self.mode = Mode::Play;
        format!("Play mode.\n\n{}", self.play_look())
    }

    /// `/play here`: starts the game in, or moves the player to, the room
    /// edit mode is in, keeping what they carry (REQ-CON-12).
    pub(super) fn play_here(&mut self) -> String {
        let Some(room) = self.edit_room() else {
            return "/play here needs edit mode to be in a room. /edit, go to one, then /play here.".to_string();
        };
        match self.game.as_mut() {
            Some(state) => state.location = room,
            None => {
                if let Err(e) = self.start_game(&room) {
                    return e;
                }
            }
        }
        self.mode = Mode::Play;
        format!("Play mode.\n\n{}", self.play_look())
    }

    /// `/save [name]`.
    pub(super) fn save_game_as(&self, args: &[&str]) -> String {
        let Some(state) = &self.game else {
            return "No game is in progress, so there's nothing to save.".to_string();
        };
        let name = match args {
            [] => DEFAULT_SAVE,
            [name] => name,
            _ => return "Usage: /save [name]".to_string(),
        };
        // Saves are the engine's, so every player client keeps them alike (REQ-PC-18).
        match save_named(&self.world.root, name, state) {
            Ok(_) => format!("Saved the game as `{}`.", name),
            Err(e) => format!("Not saved: {:#}.", e),
        }
    }

    /// `/load [name]`: replaces the game in progress and switches to play.
    pub(super) fn load_game_named(&mut self, args: &[&str]) -> String {
        let name = match args {
            [] => DEFAULT_SAVE,
            [name] => name,
            _ => return "Usage: /load [name]".to_string(),
        };
        let path = match save_path(&self.world.root, name) {
            Ok(path) => path,
            Err(e) => return format!("Not loaded: {}.", e),
        };
        if !path.exists() {
            let saves = list_saves(&self.world.root).unwrap_or_default();
            return if saves.is_empty() {
                format!("There's no save called `{}`, and this world has no saves yet.", name)
            } else {
                format!("There's no save called `{}`. Saves: {}.", name, saves.join(", "))
            };
        }
        let (state, warnings) = match load_game(&path, &self.world.engine) {
            Ok(loaded) => loaded,
            Err(e) => return format!("Not loaded: {:#}", e),
        };
        self.game = Some(state);
        self.mode = Mode::Play;
        let mut out = vec![format!("Loaded `{}`.", name)];
        out.extend(warnings.iter().map(|w| format!("Author note: {}", w)));
        out.push(String::new());
        out.push(self.play_look());
        out.join("\n")
    }

    /// After the world changed: drops what the game holds that no longer
    /// exists, returning a warning for each (REQ-CON-12).
    pub(super) fn reconcile_game(&mut self) -> Vec<String> {
        match self.game.as_mut() {
            Some(state) => state.reconcile(&self.world.engine),
            None => Vec::new(),
        }
    }

    /// The room id edit mode is in, if it's in one.
    fn edit_room(&self) -> Option<String> {
        let Place::File(path) = &self.place else { return None };
        self.world.files.get(path).filter(|e| e.kind == Kind::Room).map(|e| e.id.clone())
    }
}

/// An action's reply, then its author note on a line of its own.
fn show(result: ActionResult) -> String {
    match result.note {
        Some(note) => format!("{}\nAuthor note: {}", result.text, note),
        None => result.text,
    }
}
