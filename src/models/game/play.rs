//! A game in progress, kept apart from authored content (REQ-CON-3):
//! playing changes a `PlayState`, never a world file.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where the player is and what has changed since the game started
/// (`design/STATES.md` › Play state and saves).
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
    /// Each thing that has left its starting state, by kind (`location`,
    /// `object`, `character`) and id, to the state it is in. Also a delta,
    /// so a thing never switched follows its author's starting state.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, BTreeMap<String, String>>,
    /// The chapter the story is in; `None` before the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter: Option<SavedChapter>,
    /// Values set in play, by full name (`story.king`, `knife.sharp`).
    /// Values are strings, and hold once set.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
    /// How many times the player has entered each room, the start included.
    #[serde(default)]
    pub visits: BTreeMap<String, u32>,
    /// Player actions the game understood; console commands don't count.
    #[serde(default)]
    pub turn: u64,
    /// Faint things the player has found by searching or examining a
    /// room, by kind and id. Once found, a faint thing stays found.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub noticed: BTreeMap<String, std::collections::BTreeSet<String>>,
}

/// A chapter as a save names it: the folder that defines it (empty for
/// the root), its names from the top-level chapter down, and the numbers
/// that ordered it, kept so a chapter later deleted can fall back to the
/// nearest earlier one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedChapter {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scope: String,
    pub path: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<u64>,
}

impl SavedChapter {
    pub fn name(&self) -> &str {
        self.path.last().map(String::as_str).unwrap_or("")
    }
}

impl PlayState {
    pub fn new(location: impl Into<String>) -> Self {
        let location = location.into();
        let mut visits = BTreeMap::new();
        visits.insert(location.clone(), 1);
        Self { location, visits, ..Default::default() }
    }

    /// Whether `object` was taken from `room` in this game.
    pub fn was_taken(&self, room: &str, object: &str) -> bool {
        self.taken.get(room).map_or(false, |objects| objects.iter().any(|o| o == object))
    }

    /// The state a thing has been switched to in this game, if any.
    pub fn state_of(&self, type_name: &str, id: &str) -> Option<&str> {
        self.states.get(type_name)?.get(id).map(String::as_str)
    }

    /// Record a thing's state. Switching back to `starting` clears the
    /// entry, so the delta holds only what differs.
    pub fn set_state(&mut self, type_name: &str, id: &str, state: &str, starting: Option<&str>) {
        if Some(state) == starting {
            if let Some(by_id) = self.states.get_mut(type_name) {
                by_id.remove(id);
                if by_id.is_empty() {
                    self.states.remove(type_name);
                }
            }
        } else {
            self.states.entry(type_name.to_string()).or_default().insert(id.to_string(), state.to_string());
        }
    }

    pub fn has_noticed(&self, type_name: &str, id: &str) -> bool {
        self.noticed.get(type_name).map_or(false, |ids| ids.contains(id))
    }

    /// Record a faint thing as found; `false` if it already was.
    pub fn notice(&mut self, type_name: &str, id: &str) -> bool {
        self.noticed.entry(type_name.to_string()).or_default().insert(id.to_string())
    }

    /// How many times the player has entered `room`.
    pub fn visits_to(&self, room: &str) -> u32 {
        self.visits.get(room).copied().unwrap_or(0)
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
    /// Look closely around the room, finding faint things.
    Search,
    /// An object's own named action (`design/STATES.md` › Changing
    /// state): `pull lever` is `verb` "pull", `noun` "lever";
    /// `hit window with stick` adds `with` "stick".
    Use { verb: String, noun: String, with: Option<String> },
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
    /// A thing switched state; `type_name` is `location`, `object` or `character`.
    Became { type_name: String, id: String, state: String },
    EnteredChapter { name: String },
    /// A faint thing was found.
    Noticed { type_name: String, id: String },
    Set { name: String, value: String },
}
