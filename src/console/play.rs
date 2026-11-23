//! Play mode: the game in progress, its verbs (REQ-CON-10), `/play here`
//! (REQ-CON-12) and saves (REQ-CON-11). The engine applies each action;
//! this module turns input into actions and results into text. Nothing
//! here writes a content file (REQ-CON-3).

use super::saves::{list_saves, save_path, DEFAULT_SAVE};
use super::session::{Console, Place};
use super::verbs::{parse, Verb};
use super::world::Kind;
use super::Mode;
use crate::engines::{load_game, save_game};
use crate::models::game::{ActionResult, PlayAction};
use crate::render::{render_location_as, RenderStyle};

impl Console {
    /// One line of play-mode input.
    pub(super) fn play_input(&mut self, line: &str) -> String {
        let action = match parse(line) {
            Verb::Look => PlayAction::Look,
            Verb::Go(direction) => PlayAction::Go(direction),
            Verb::Take(noun) => PlayAction::Take(noun),
            Verb::Examine(noun) => PlayAction::Examine(noun),
            Verb::Inventory => PlayAction::Inventory,
            Verb::Incomplete(question) => return question,
            Verb::Unknown(typed) => return format!("I don't understand `{}`.", typed),
        };
        let Some(state) = self.game.as_mut() else {
            return "No game is in progress. /play here starts one in the room edit mode is in.".to_string();
        };
        let result = self.game_engine.act(&self.world.engine, state, &action);
        show(result)
    }

    /// The player's room, as the player sees it.
    pub(super) fn play_look(&self) -> String {
        let Some(state) = &self.game else {
            return "No game is in progress. /play here starts one in the room edit mode is in.".to_string();
        };
        match self.game_engine.view(&self.world.engine, state) {
            Some(view) => render_location_as(&view, RenderStyle::Player),
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
            return "/play here needs edit mode to be in a room. /edit, cd to one, then /play here.".to_string();
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
        let path = match save_path(&self.world.root, name) {
            Ok(path) => path,
            Err(e) => return format!("Not saved: {}.", e),
        };
        match save_game(&path, state) {
            Ok(()) => format!("Saved the game as `{}`.", name),
            Err(e) => format!("Not saved: {:#}", e),
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
