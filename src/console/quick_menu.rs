//! The quick menu (REQ-CON-63 to 68, Bruce, 2026-11-25): what fits where
//! edit mode is (a room, thing or door), each entry asking its questions
//! at the bottom of the console and then running the commands it stands
//! for, so the author sees the typed form.
//!
//! This holds the entries and what their answers become; `screen.rs` asks
//! the questions. "Add a reply" isn't here yet: it waits on
//! response hints, so REQ-CON-64 hides it.

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
    /// Something typed, named by its `[console]` text key, with a hint
    /// under it. A required one won't take a blank answer.
    Text { key: &'static str, hint: &'static str, required: bool },
    /// One of `options`, starting at `default`. `new` adds a last option,
    /// "a new state…", that asks for a name instead. `describe` is the
    /// text key prefix for a note beside each option (`<prefix>_<option>`).
    Choose { key: &'static str, hint: &'static str, options: Vec<String>, default: usize, new: bool, describe: Option<&'static str> },
}

impl Ask {
    pub fn key(&self) -> &'static str {
        match self {
            Ask::Text { key, .. } | Ask::Choose { key, .. } => key,
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Ask::Text { hint, .. } | Ask::Choose { hint, .. } => hint,
        }
    }
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
    Chapter,
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
            Does::Chapter if a(1) == "file" => vec![format!("/chapter {} file", a(0))],
            Does::Chapter => vec![format!("/chapter {}", a(0))],
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
    /// What the hints name from the world: `states` and `exits`.
    pub facts: Vec<(&'static str, String)>,
}

/// The entries for a kind of place, given the thing's states (sorted) and
/// the one it starts in.
fn entries_for(kind: Kind, has_states_at_all: bool, states: &[String], start: Option<&str>, chapters: &[String]) -> Vec<Entry> {
    let room = kind == Kind::Room;
    let text = |key, hint| Ask::Text { key, hint, required: true };
    let choose = |key, hint, options: Vec<String>, default| Ask::Choose { key, hint, options, default, new: false, describe: None };
    // A state question: a choice of the states there are, or typed when
    // there are none. `any` offers "any state" first; `new` offers a new
    // one last (Bruce, 2026-11-25).
    let state_ask = |key, hint, any: bool, new: bool| {
        let mut options: Vec<String> = if any { vec!["any".to_string()] } else { Vec::new() };
        options.extend(states.iter().cloned());
        if options.is_empty() {
            Ask::Text { key, hint, required: true }
        } else {
            Ask::Choose { key, hint, options, default: 0, new, describe: None }
        }
    };
    let strings = |list: &[&str]| list.iter().map(|v| v.to_string()).collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut entry = |letter, label, stands_for, asks, does| out.push(Entry { letter, label, stands_for, asks, does });
    if has_states_at_all {
        let from = state_ask("qm_ask_from", "qm_hint_from", true, true);
        let to = state_ask("qm_ask_to", "qm_hint_to", false, true);
        entry('a', "qm_action", "/action", vec![text("qm_ask_verb", "qm_hint_verb"), from, to], Does::Action);
        let starts = choose("qm_ask_starts", "qm_hint_starts", strings(&["no", "start"]), 0);
        entry('s', "qm_state", "/state", vec![text("qm_ask_state_name", "qm_hint_state_name"), starts], Does::State);
        let visibility = Ask::Choose { key: "qm_ask_visibility", hint: "qm_hint_visibility", options: strings(VISIBILITIES), default: 2, new: false, describe: Some("qm_vis") };
        entry('v', "qm_visibility", "/set visibility", vec![state_ask("qm_ask_which_state", "qm_hint_which_state", false, true), visibility], Does::Visibility { has_states: !states.is_empty() });
        if !states.is_empty() {
            let default = start.and_then(|s| states.iter().position(|n| n == s)).unwrap_or(0);
            entry('t', "qm_start", "/state {name} start", vec![choose("qm_ask_start", "qm_hint_start", states.to_vec(), default)], Does::Start);
        }
    }
    if room {
        entry('l', "qm_link", "/link", vec![text("qm_ask_direction", "qm_hint_direction"), text("qm_ask_room", "qm_hint_room")], Does::Link);
        entry('n', "qm_add_thing", "/add", vec![text("qm_ask_thing", "qm_hint_thing")], Does::AddThing);
        entry('d', "qm_add_door", "/add door", vec![text("qm_ask_direction", "qm_hint_direction"), text("qm_ask_room", "qm_hint_room")], Does::AddDoor);
    }
    // A chapter version: a section here, or its own file (Bruce, 2026-11-25).
    if has_states_at_all && !chapters.is_empty() {
        let which = choose("qm_ask_chapter", "qm_hint_chapter", chapters.to_vec(), 0);
        let form = Ask::Choose { key: "qm_ask_chapter_form", hint: "qm_hint_chapter_form", options: strings(&["section", "file"]), default: 0, new: false, describe: Some("qm_form") };
        entry('c', "qm_chapter", "/chapter", vec![which, form], Does::Chapter);
    }
    if has_states_at_all && kind != Kind::Door {
        let recipe = Ask::Choose { key: "qm_ask_recipe", hint: "qm_hint_recipe", options: strings(RECIPES), default: 0, new: false, describe: Some("qm_recipe") };
        entry('p', "qm_recipe", "/recipe", vec![recipe, Ask::Text { key: "qm_ask_recipe_with", hint: "qm_hint_recipe_with", required: false }], Does::Recipe);
    }
    if room {
        entry('g', "qm_play", "/play here", Vec::new(), Does::Play);
    }
    out
}

impl Console {
    /// The quick menu for where edit mode is, or why there's none.
    pub(super) fn quick_menu(&self) -> Result<QuickMenu, String> {
        match &self.place {
            Place::File(path) => self.quick_menu_for(Some(path)),
            _ => self.quick_menu_for(None),
        }
    }

    /// The quick menu for one file (`World::files`' key), such as the one
    /// open in the editor.
    pub(super) fn quick_menu_for(&self, path: Option<&str>) -> Result<QuickMenu, String> {
        if self.mode != super::Mode::Edit {
            return Err(self.t("edit_only_command", &[("command", "menu")]));
        }
        let entry = path.and_then(|p| self.world.files.get(p));
        let Some(entry) = entry.filter(|e| matches!(e.kind, Kind::Room | Kind::Object | Kind::Character | Kind::Door)) else {
            return Err(self.t("qm_nowhere", &[]));
        };
        let engine_kind = entity_kind(entry.kind);
        let thing = engine_kind.and_then(|k| self.world.engine.thing(k, &entry.id));
        let states = thing.map(|t| t.state_names()).unwrap_or_default();
        let start = thing.and_then(|t| t.starting_state.clone());
        let entries = entries_for(entry.kind, engine_kind.is_some(), &states, start.as_deref(), &self.chapter_names());
        let listed = |names: Vec<String>| if names.is_empty() { self.t("qm_none_yet", &[]) } else { names.join(", ") };
        let exits = match entry.kind {
            Kind::Room => self.world.engine.get_location(&entry.id).map(|l| {
                let mut names: Vec<String> = l.connections.keys().cloned().collect();
                names.sort();
                names
            }).unwrap_or_default(),
            _ => Vec::new(),
        };
        let facts = vec![("states", listed(states)), ("exits", listed(exits))];
        Ok(QuickMenu { id: entry.id.clone(), kind: entry.kind, entries, facts })
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
        let room = entries_for(Kind::Room, true, &[], None, &[]);
        for letter in ['a', 's', 'v', 'l', 'n', 'd', 'p', 'g'] {
            assert!(find(&room, letter).is_some(), "room has {}", letter);
        }
        assert!(find(&room, 't').is_none(), "no states, no starting state");
        let door = entries_for(Kind::Door, true, &[], None, &[]);
        assert!(find(&door, 'l').is_none() && find(&door, 'g').is_none() && find(&door, 'p').is_none());
    }

    #[test]
    fn answers_become_the_commands_they_stand_for() {
        let states = vec!["closed".to_string(), "open".to_string()];
        let lamp = entries_for(Kind::Object, true, &states, Some("open"), &["later".to_string()]);
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(find(&lamp, 'a').unwrap().commands(&s(&["open", "closed", "open"])), vec!["/action open closed --> open"]);
        assert_eq!(find(&lamp, 's').unwrap().commands(&s(&["lit", "start"])), vec!["/state lit start"]);
        assert_eq!(find(&lamp, 's').unwrap().commands(&s(&["lit", "no"])), vec!["/state lit"]);
        assert_eq!(find(&lamp, 'v').unwrap().commands(&s(&["closed", "hidden"])), vec!["/set visibility hidden in closed"]);
        let Ask::Choose { default, .. } = &find(&lamp, 't').unwrap().asks[0] else { panic!("a choice") };
        assert_eq!(*default, 1, "the current starting state is the default");
        assert_eq!(find(&lamp, 'p').unwrap().commands(&s(&["fixed-until", "pull"])), vec!["/recipe fixed-until pull"]);
        assert_eq!(find(&lamp, 'p').unwrap().commands(&s(&["switch", ""])), vec!["/recipe switch"]);
        assert_eq!(find(&lamp, 'c').unwrap().commands(&s(&["later", "section"])), vec!["/chapter later"]);
        assert_eq!(find(&lamp, 'c').unwrap().commands(&s(&["later", "file"])), vec!["/chapter later file"]);
        assert!(find(&entries_for(Kind::Object, true, &[], None, &[]), 'c').is_none(), "no chapters, no entry");
        // No states yet: hide or reveal makes the state first, as /state does.
        let bare = entries_for(Kind::Object, true, &[], None, &[]);
        assert_eq!(find(&bare, 'v').unwrap().commands(&s(&["gone", "absent"])), vec!["/state gone", "/set visibility absent in gone"]);
        let room = entries_for(Kind::Room, true, &[], None, &[]);
        assert_eq!(find(&room, 'd').unwrap().commands(&s(&["west", "hallway"])), vec!["/add door west hallway"]);
        assert_eq!(find(&room, 'g').unwrap().commands(&[]), vec!["/play here"]);
    }
}
