//! The quick menu (REQ-CON-63 to 68, Bruce, 2026-11-25): what fits where
//! edit mode is (a room, thing or door), each entry asking its questions
//! at the bottom of the console and then running the commands it stands
//! for, so the author sees the typed form.
//!
//! This holds the entries and what their answers become; `screen.rs` asks
//! the questions. The entries "add a reply" and "add chapter version"
//! aren't here yet: their commands aren't built, so REQ-CON-64 hides them.

use super::author::entity_kind;
use super::session::{suggest, Console, Place};
use super::world::Kind;

/// The visibilities a state can have (`design/STATES.md` › Hidden things).
const VISIBILITIES: &[&str] = &["visible", "faint", "hidden", "absent"];
/// The recipes `/recipe` knows.
const RECIPES: &[&str] = &["fixed-until", "locked-with", "switch", "container"];

/// One question an entry asks.
#[derive(Debug, Clone, PartialEq)]
pub enum Ask {
    /// Something typed, named by its `[console]` text key. A required one
    /// won't take a blank answer.
    Text { key: &'static str, required: bool },
    /// One of `options`, starting at `default`.
    Choose { key: &'static str, options: Vec<String>, default: usize },
}

/// What an entry does with its answers.
#[derive(Debug, Clone, PartialEq)]
enum Does {
    Action,
    State,
    Visibility { has_states: bool },
    Start,
    Link,
    AddThing,
    AddDoor,
    Recipe,
    Play,
}

/// One quick menu entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Its letter, fixed per entry so a hidden one moves nothing.
    pub letter: char,
    /// Its label's `[console]` text key.
    pub label: &'static str,
    /// The command it stands for, as the menu shows it.
    pub stands_for: &'static str,
    pub asks: Vec<Ask>,
    does: Does,
}

impl Entry {
    /// The commands these answers stand for, one per question asked.
    pub fn commands(&self, answers: &[String]) -> Vec<String> {
        let a = |i: usize| answers.get(i).map(|s| s.trim()).unwrap_or("");
        match &self.does {
            Does::Action => vec![format!("/action {} {} --> {}", a(0), a(1), a(2))],
            Does::State if a(1) == "start" => vec![format!("/state {} start", a(0))],
            Does::State => vec![format!("/state {}", a(0))],
            Does::Visibility { has_states } => {
                let set = format!("/set visibility {} in {}", a(1), a(0));
                if *has_states { vec![set] } else { vec![format!("/state {}", a(0)), set] }
            }
            Does::Start => vec![format!("/state {} start", a(0))],
            Does::Link => vec![format!("/link {} {}", a(0), a(1))],
            Does::AddThing => vec![format!("/add {}", a(0))],
            Does::AddDoor => vec![format!("/add door {} {}", a(0), a(1))],
            Does::Recipe => vec![format!("/recipe {} {}", a(0), a(1)).trim_end().to_string()],
            Does::Play => vec!["/play here".to_string()],
        }
    }
}

/// The menu for one place.
#[derive(Debug, Clone, PartialEq)]
pub struct QuickMenu {
    /// What it's for, as the heading names it.
    pub id: String,
    pub kind: Kind,
    pub entries: Vec<Entry>,
}

/// The entries for a kind of place, given the thing's states (sorted) and
/// the one it starts in.
fn entries_for(kind: Kind, has_states_at_all: bool, states: &[String], start: Option<&str>) -> Vec<Entry> {
    let room = kind == Kind::Room;
    let state_ask = |key: &'static str| match states.is_empty() {
        true => Ask::Text { key, required: true },
        false => Ask::Choose { key, options: states.to_vec(), default: 0 },
    };
    let yes_no = vec!["no".to_string(), "start".to_string()];
    let mut out = Vec::new();
    let mut entry = |letter, label, stands_for, asks, does| out.push(Entry { letter, label, stands_for, asks, does });
    if has_states_at_all {
        entry('a', "qm_action", "/action", vec![Ask::Text { key: "qm_ask_verb", required: true }, state_ask("qm_ask_from"), state_ask("qm_ask_to")], Does::Action);
        entry('s', "qm_state", "/state", vec![Ask::Text { key: "qm_ask_state_name", required: true }, Ask::Choose { key: "qm_ask_starts", options: yes_no, default: 0 }], Does::State);
        let visibilities = VISIBILITIES.iter().map(|v| v.to_string()).collect();
        entry('v', "qm_visibility", "/set visibility", vec![state_ask("qm_ask_which_state"), Ask::Choose { key: "qm_ask_visibility", options: visibilities, default: 2 }], Does::Visibility { has_states: !states.is_empty() });
        if !states.is_empty() {
            let default = start.and_then(|s| states.iter().position(|n| n == s)).unwrap_or(0);
            entry('t', "qm_start", "/state {name} start", vec![Ask::Choose { key: "qm_ask_start", options: states.to_vec(), default }], Does::Start);
        }
    }
    if room {
        entry('l', "qm_link", "/link", vec![Ask::Text { key: "qm_ask_direction", required: true }, Ask::Text { key: "qm_ask_room", required: true }], Does::Link);
        entry('n', "qm_add_thing", "/add", vec![Ask::Text { key: "qm_ask_thing", required: true }], Does::AddThing);
        entry('d', "qm_add_door", "/add door", vec![Ask::Text { key: "qm_ask_direction", required: true }, Ask::Text { key: "qm_ask_room", required: true }], Does::AddDoor);
    }
    if has_states_at_all && kind != Kind::Door {
        let recipes = RECIPES.iter().map(|r| r.to_string()).collect();
        entry('p', "qm_recipe", "/recipe", vec![Ask::Choose { key: "qm_ask_recipe", options: recipes, default: 0 }, Ask::Text { key: "qm_ask_recipe_with", required: false }], Does::Recipe);
    }
    if room {
        entry('g', "qm_play", "/play here", Vec::new(), Does::Play);
    }
    out
}

impl Console {
    /// The quick menu for where edit mode is, or why there's none.
    pub(super) fn quick_menu(&self) -> Result<QuickMenu, String> {
        if self.mode != super::Mode::Edit {
            return Err(self.t("edit_only_command", &[("command", "menu")]));
        }
        let entry = match &self.place {
            Place::File(path) => self.world.files.get(path),
            _ => None,
        };
        let Some(entry) = entry.filter(|e| matches!(e.kind, Kind::Room | Kind::Object | Kind::Character | Kind::Door)) else {
            return Err(self.t("qm_nowhere", &[]));
        };
        let engine_kind = entity_kind(entry.kind);
        let thing = engine_kind.and_then(|k| self.world.engine.thing(k, &entry.id));
        let states = thing.map(|t| t.state_names()).unwrap_or_default();
        let start = thing.and_then(|t| t.starting_state.clone());
        let entries = entries_for(entry.kind, engine_kind.is_some(), &states, start.as_deref());
        Ok(QuickMenu { id: entry.id.clone(), kind: entry.kind, entries })
    }

    /// `/menu`: on a terminal the screen opens the menu; elsewhere, such
    /// as a transcript, it lists what the menu would offer.
    pub(super) fn menu_command(&mut self) -> String {
        match self.quick_menu() {
            Err(why) => why,
            Ok(_) if self.interactive => {
                self.pending_menu = true;
                String::new()
            }
            Ok(menu) => {
                let mut out = vec![self.t("qm_heading", &[("id", &menu.id), ("kind", menu.kind.label())])];
                for e in &menu.entries {
                    out.push(format!("  {}  {} ({})", e.letter, self.t(e.label, &[]), e.stands_for));
                }
                out.join("\n")
            }
        }
    }

    /// Whether `/menu` asked the screen to open the menu.
    pub(super) fn take_pending_menu(&mut self) -> bool {
        std::mem::take(&mut self.pending_menu)
    }

    /// The closing note after an entry ran: the commands it stood for.
    pub(super) fn quick_menu_typed(&self, commands: &[String]) -> String {
        let commands: Vec<&str> = commands.iter().map(String::as_str).collect();
        suggest(&self.t("qm_typed", &[]), &commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(entries: &[Entry], letter: char) -> Option<&Entry> {
        entries.iter().find(|e| e.letter == letter)
    }

    #[test]
    fn a_room_gets_links_doors_and_play_and_a_door_gets_neither() {
        let room = entries_for(Kind::Room, true, &[], None);
        for letter in ['a', 's', 'v', 'l', 'n', 'd', 'p', 'g'] {
            assert!(find(&room, letter).is_some(), "room has {}", letter);
        }
        assert!(find(&room, 't').is_none(), "no states, no starting state");
        let door = entries_for(Kind::Door, true, &[], None);
        assert!(find(&door, 'l').is_none() && find(&door, 'g').is_none() && find(&door, 'p').is_none());
    }

    #[test]
    fn answers_become_the_commands_they_stand_for() {
        let states = vec!["closed".to_string(), "open".to_string()];
        let lamp = entries_for(Kind::Object, true, &states, Some("open"));
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(find(&lamp, 'a').unwrap().commands(&s(&["open", "closed", "open"])), vec!["/action open closed --> open"]);
        assert_eq!(find(&lamp, 's').unwrap().commands(&s(&["lit", "start"])), vec!["/state lit start"]);
        assert_eq!(find(&lamp, 's').unwrap().commands(&s(&["lit", "no"])), vec!["/state lit"]);
        assert_eq!(find(&lamp, 'v').unwrap().commands(&s(&["closed", "hidden"])), vec!["/set visibility hidden in closed"]);
        let Ask::Choose { default, .. } = &find(&lamp, 't').unwrap().asks[0] else { panic!("a choice") };
        assert_eq!(*default, 1, "the current starting state is the default");
        assert_eq!(find(&lamp, 'p').unwrap().commands(&s(&["fixed-until", "pull"])), vec!["/recipe fixed-until pull"]);
        assert_eq!(find(&lamp, 'p').unwrap().commands(&s(&["switch", ""])), vec!["/recipe switch"]);
        // No states yet: hide or reveal makes the state first, as /state does.
        let bare = entries_for(Kind::Object, true, &[], None);
        assert_eq!(find(&bare, 'v').unwrap().commands(&s(&["gone", "absent"])), vec!["/state gone", "/set visibility absent in gone"]);
        let room = entries_for(Kind::Room, true, &[], None);
        assert_eq!(find(&room, 'd').unwrap().commands(&s(&["west", "hallway"])), vec!["/add door west hallway"]);
        assert_eq!(find(&room, 'g').unwrap().commands(&[]), vec!["/play here"]);
    }
}
