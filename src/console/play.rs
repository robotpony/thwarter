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
        let offered = std::mem::take(&mut self.topics);
        if let Some(topic) = picked_topic(&offered, line) {
            let action = crate::models::game::PlayAction::Ask { person: topic.person.clone(), topic: topic.topic.clone() };
            if let Some(state) = self.game.as_mut() {
                let result = self.game_engine.act(&self.world.engine, state, &action);
                return self.reply(result);
            }
        }
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
            return super::session::suggest(&self.t("no_game_start_one", &[]), &["/play here"]);
        };
        let result = self.game_engine.act(&self.world.engine, state, &action);
        self.reply(result)
    }

    /// A play result as text. Topics a talk reply ends with become a
    /// numbered menu, kept for the next line to pick from (REQ-PC-31).
    fn reply(&mut self, mut result: ActionResult) -> String {
        if !result.topics.is_empty() {
            let mut lines: Vec<String> = result.text.lines().map(str::to_string).collect();
            for (n, topic) in result.topics.iter().enumerate() {
                if let Some(line) = lines.iter_mut().rev().find(|l| l.trim() == format!("- {}", topic.label)) {
                    *line = self.t("topic_row", &[("number", &(n + 1).to_string()), ("label", &topic.label)]);
                }
            }
            lines.push(String::new());
            lines.push(self.t("topic_pick", &[]));
            result.text = lines.join("\n");
            self.topics = std::mem::take(&mut result.topics);
        }
        // On a terminal, the author note goes to the sidebar; scripts and
        // transcripts keep it in the output.
        if self.interactive {
            self.notes = result.note.iter().cloned().collect();
            return result.text;
        }
        self.show(result)
    }

    /// The player's room, as the player sees it.
    pub(super) fn play_look(&self) -> String {
        let Some(state) = &self.game else {
            return super::session::suggest(&self.t("no_game_start_one", &[]), &["/play here"]);
        };
        match self.game_engine.view(&self.world.engine, state) {
            // The player's text, in the world's own wording (`[play]` in
            // world.toml); ids and author detail go to the sidebar.
            Some(view) => {
                let engine = &self.world.engine;
                crate::render::play_blocks(&view, &|n| engine.link_for(n), engine.play_text()).to_plain()
            }
            None => self.t("player_nowhere", &[("room", &state.location)]),
        }
    }

    /// Starts a game in `room`, replacing any game in progress.
    pub(super) fn start_game(&mut self, room: &str) -> Result<(), String> {
        let state = self.game_engine.start(&self.world.engine, room).map_err(|e| self.t("cant_start_there", &[("reason", &e.to_string())]))?;
        self.game = Some(state);
        Ok(())
    }

    /// `/play`: resumes the game where it was, or starts one in the room
    /// edit mode is in (else the world's starting room).
    pub(super) fn play(&mut self) -> String {
        if self.mode == Mode::Play {
            return self.t("already_playing", &[]);
        }
        if self.game.is_none() {
            let Some(room) = self.edit_room().or_else(|| self.world.default_room()) else {
                return self.t("no_rooms_to_play", &[]);
            };
            if let Err(e) = self.start_game(&room) {
                return e;
            }
        }
        self.mode = Mode::Play;
        self.t("play_mode", &[("view", &self.play_look())])
    }

    /// `/play here`: starts the game in, or moves the player to, the room
    /// edit mode is in, keeping what they carry (REQ-CON-12).
    pub(super) fn play_here(&mut self) -> String {
        let Some(room) = self.edit_room() else {
            return super::session::suggest(&self.t("play_here_needs_room", &[]), &["/play here"]);
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
        self.t("play_mode", &[("view", &self.play_look())])
    }

    /// `/save [name]`.
    pub(super) fn save_game_as(&self, args: &[&str]) -> String {
        let Some(state) = &self.game else {
            return self.t("nothing_to_save", &[]);
        };
        let name = match args {
            [] => DEFAULT_SAVE,
            [name] => name,
            _ => return super::session::suggest(&self.t("usage", &[]), &["/save [{name}]"]),
        };
        // Saves are the engine's, so every player client keeps them alike (REQ-PC-18).
        match save_named(&self.world.root, name, state) {
            Ok(_) => self.t("game_saved", &[("name", name)]),
            Err(e) => self.t("game_not_saved", &[("reason", &format!("{:#}", e))]),
        }
    }

    /// `/load [name]`: replaces the game in progress and switches to play.
    pub(super) fn load_game_named(&mut self, args: &[&str]) -> String {
        let name = match args {
            [] => DEFAULT_SAVE,
            [name] => name,
            _ => return super::session::suggest(&self.t("usage", &[]), &["/load [{name}]"]),
        };
        let path = match save_path(&self.world.root, name) {
            Ok(path) => path,
            Err(e) => return self.t("game_not_loaded_name", &[("reason", &e.to_string())]),
        };
        if !path.exists() {
            let saves = list_saves(&self.world.root).unwrap_or_default();
            return if saves.is_empty() {
                self.t("no_save_no_saves", &[("name", name)])
            } else {
                self.t("no_save_called", &[("name", name), ("saves", &saves.join(", "))])
            };
        }
        let (state, warnings) = match load_game(&path, &self.world.engine) {
            Ok(loaded) => loaded,
            Err(e) => return self.t("game_not_loaded", &[("reason", &format!("{:#}", e))]),
        };
        self.game = Some(state);
        self.mode = Mode::Play;
        let mut out = vec![self.t("game_loaded", &[("name", name)])];
        out.extend(warnings.iter().map(|w| self.t("load_author_note", &[("note", w)])));
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

impl Console {
    /// An action's reply, then its author note on a line of its own.
    fn show(&self, result: ActionResult) -> String {
        match result.note {
            Some(note) => self.t("reply_with_note", &[("reply", &result.text), ("note", &note)]),
            None => result.text,
        }
    }
}

/// The offered topic `line` picks: its number, or its label in any case.
fn picked_topic<'a>(offered: &'a [crate::models::game::Topic], line: &str) -> Option<&'a crate::models::game::Topic> {
    let line = line.trim();
    if let Ok(n) = line.parse::<usize>() {
        return n.checked_sub(1).and_then(|i| offered.get(i));
    }
    offered.iter().find(|t| t.label.eq_ignore_ascii_case(line))
}

#[cfg(test)]
mod tests {
    use super::super::session::Console;
    use super::super::Mode;
    use std::fs;
    use std::path::Path;

    #[test]
    fn a_talk_reply_offers_numbered_topics_to_pick() {
        let dir = tempfile::tempdir().unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        for entry in walkdir::WalkDir::new(&fixture).into_iter().filter_map(|e| e.ok()) {
            let to = dir.path().join(entry.path().strip_prefix(&fixture).unwrap());
            if entry.file_type().is_dir() {
                fs::create_dir_all(&to).unwrap();
            } else {
                fs::copy(entry.path(), &to).unwrap();
            }
        }
        fs::write(
            dir.path().join("npcs/elder.md"),
            "---\nid: elder\ntitle: The Elder\ntype: character\ndefault_location: village/square\n---\nA stooped woman.\n\n## When: #talk\n\"Ask, child.\"\n\n- The well -> ask #the-well\n- The lantern -> ask @lantern\n\n## When: #ask #the-well\n\"Dry these forty years.\"\n\n## When: #ask @lantern\n\"Keep it lit.\"\n",
        )
        .unwrap();
        let mut console = Console::open(dir.path(), Mode::Play).unwrap();
        let (menu, _) = console.handle("talk to elder");
        assert_eq!(menu, "\"Ask, child.\"\n\n  1. The well\n  2. The lantern\n\nPick a topic by its number or name.");
        assert_eq!(console.handle("2").0, "\"Keep it lit.\"");
        // A pick only follows the menu; after another reply, a number is
        // ordinary input again.
        assert_ne!(console.handle("1").0, "\"Dry these forty years.\"");
        console.handle("talk to elder");
        assert_eq!(console.handle("the WELL").0, "\"Dry these forty years.\"");
    }
}
