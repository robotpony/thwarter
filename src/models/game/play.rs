//! A game in progress, kept apart from authored content (REQ-CON-3):
//! playing changes a `PlayState`, never a world file.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where the player is and what has changed since the game started.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayState {
    /// The current location's id.
    pub location: String,
    /// Object ids carried, in the order taken.
    #[serde(default)]
    pub inventory: Vec<String>,
    /// Room id to the object ids taken from it. A delta from the authored
    /// room contents, so an author's mid-game edits to a room still show.
    #[serde(default)]
    pub taken: BTreeMap<String, Vec<String>>,
    /// Story flags. Unused until narrative state is designed.
    #[serde(default)]
    pub flags: BTreeMap<String, serde_yaml::Value>,
}

impl PlayState {
    pub fn new(location: impl Into<String>) -> Self {
        Self { location: location.into(), ..Default::default() }
    }

    /// Whether `object` was taken from `room` in this game.
    pub fn was_taken(&self, room: &str, object: &str) -> bool {
        self.taken.get(room).map_or(false, |objects| objects.iter().any(|o| o == object))
    }
}

/// One player action, already parsed from what they typed. The console
/// owns verbs and aliases (REQ-CON-10); the engine owns effects (REQ-ENG-5).
#[derive(Debug, Clone, PartialEq)]
pub enum PlayAction {
    Look,
    /// A direction: `north`, or an abbreviation such as `n`.
    Go(String),
    /// A noun as typed: an id, a title, or a word of either.
    Take(String),
    Examine(String),
    Inventory,
}

/// What an action did: the text to show, and the state changes made.
/// No changes means the state is as it was.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionResult {
    /// For the player.
    pub text: String,
    pub changes: Vec<Change>,
    /// For the author playtesting, such as an exit to a room that doesn't
    /// exist yet. The console shows it apart from `text`.
    pub note: Option<String>,
}

impl ActionResult {
    pub fn say(text: impl Into<String>) -> Self {
        Self { text: text.into(), changes: Vec::new(), note: None }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Moved { from: String, to: String },
    Took { object: String, from: String },
}
