//! The console's commands as a list, and completing what's being typed
//! from it and from the world (REQ-CON-33). Typing `/` offers the commands
//! that work right now; after a command or a verb such as `go`, the
//! exits, rooms, people and things that fit. The screen draws the list;
//! this works out what goes in it, so it tests without a terminal.

use crate::engines::list_saves;
use super::session::{Console, Place, COMPASS_ALIASES};
use super::world::Kind;
use super::Mode;

/// One console command, as the `/` menu lists it.
pub struct Command {
    pub name: &'static str,
    /// Its arguments, as `/help` writes them.
    pub args: &'static str,
    pub about: &'static str,
    /// Whether it only works in edit mode.
    pub edit_only: bool,
}

const fn command(name: &'static str, args: &'static str, about: &'static str, edit_only: bool) -> Command {
    Command { name, args, about, edit_only }
}

/// Every console command, in the order the menu shows them.
pub const COMMANDS: &[Command] = &[
    command("edit", "[room|object|character] [id] or <name>", "edit where you are, or what you name", false),
    command("play", "[here]", "play, resuming the game, or from the room edit mode is in", false),
    command("new", "room|character|object ...", "make a room, character or object", true),
    command("set", "<key> <value>", "change one field of where you are", true),
    command("link", "<direction> <room>", "add or change an exit from this room", true),
    command("unlink", "<direction>", "remove an exit from this room", true),
    command("add", "<thing> [room or character]", "put a person or thing here too; scenery if it has no card", true),
    command("move", "<thing> <room or character>", "put something somewhere else, or a room in an area", true),
    command("save", "[name]", "save the game", false),
    command("load", "[name]", "load a saved game", false),
    command("reload", "", "reload the world from disk, staying where you are", false),
    command("restart", "", "restart the console on the newest build", false),
    command("status", "", "the world, where you are, the game, saves and the build", false),
    command("map", "", "the map around you; the sidebar shows the nearest rooms", false),
    command("clear", "", "empty the output", false),
    command("help", "", "every command", false),
    command("quit", "", "leave the console", false),
];

/// One suggestion: what replaces the word being typed, and how it's shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// The text that replaces the word being typed.
    pub text: String,
    /// What the menu shows on the left, usually `text`.
    pub label: String,
    /// What the menu shows beside it.
    pub detail: String,
    /// Whether more is typed after it, so picking it adds a space.
    pub more: bool,
}

/// What the menu offers for a line: the suggestions, where the word they
/// replace starts, and a hint for the command's arguments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Completion {
    pub from: usize,
    pub suggestions: Vec<Suggestion>,
    pub hint: Option<String>,
}

impl Completion {
    /// `line` with suggestion `pick` in place of the word being typed.
    pub fn apply(&self, line: &str, pick: usize) -> String {
        let Some(s) = self.suggestions.get(pick) else { return line.to_string() };
        format!("{}{}{}", &line[..self.from.min(line.len())], s.text, if s.more { " " } else { "" })
    }
}

impl Console {
    /// What the menu offers for `line` as typed so far.
    pub(super) fn complete(&self, line: &str) -> Completion {
        let from = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[from..];
        let before: Vec<&str> = line[..from].split_whitespace().collect();

        if let Some(name) = line.strip_prefix('/').filter(|_| before.is_empty()) {
            return Completion { from, suggestions: self.commands_like(name), hint: None };
        }
        let mut done = Completion { from, ..Completion::default() };
        let options: Vec<Suggestion> = match before.as_slice() {
            [] => return done,
            [first, args @ ..] if first.starts_with('/') => {
                let name = &first[1..];
                done.hint = COMMANDS.iter().find(|c| c.name == name).map(|c| format!("/{} {}", c.name, c.args));
                self.command_args(name, args)
            }
            ["go" | "walk" | "move"] => self.places_to_go(),
            ["examine" | "x" | "inspect"] | ["look", "at"] => self.things_here(),
            ["take" | "get"] | ["pick", "up"] if self.mode == Mode::Play => self.things_here(),
            _ => Vec::new(),
        };
        done.suggestions = narrow(options, word);
        done
    }

    /// The commands that work in this mode and start with `typed`.
    fn commands_like(&self, typed: &str) -> Vec<Suggestion> {
        COMMANDS
            .iter()
            .filter(|c| self.mode == Mode::Edit || !c.edit_only)
            .filter(|c| c.name.starts_with(&typed.to_lowercase()))
            .map(|c| Suggestion {
                text: format!("/{}", c.name),
                label: format!("/{}", c.name),
                detail: c.about.to_string(),
                more: !c.args.is_empty() && !c.args.starts_with('['),
            })
            .collect()
    }

    /// Suggestions for a command's next argument, given those typed already.
    fn command_args(&self, name: &str, args: &[&str]) -> Vec<Suggestion> {
        let kinds = || {
            ["room", "object", "character"]
                .iter()
                .map(|k| Suggestion { text: k.to_string(), label: k.to_string(), detail: String::new(), more: true })
                .collect()
        };
        match (name, args) {
            ("edit", []) => {
                let mut options: Vec<Suggestion> = kinds();
                options.iter_mut().for_each(|s| s.more = true);
                options.extend(self.places_to_go().into_iter().filter(|s| s.text != "back" && !s.text.ends_with('/')));
                options
            }
            ("edit", ["room"]) => self.rooms(),
            ("edit", ["object"]) => self.things_of(Kind::Object),
            ("edit", ["character"]) => self.things_of(Kind::Character),
            ("new", []) => kinds(),
            ("play", []) => vec![plain("here", "the room edit mode is in", false)],
            ("load" | "save", []) => list_saves(&self.world.root)
                .unwrap_or_default()
                .into_iter()
                .map(|name| plain(&name, "a saved game", false))
                .collect(),
            ("link", []) => directions(),
            ("link", [_]) => self.rooms(),
            ("unlink", []) => self.exits_here().into_iter().map(|(dir, to)| plain(&dir, &to, false)).collect(),
            ("move", []) => {
                let mut things = self.things_here();
                things.extend(self.things_of(Kind::Character));
                things.extend(self.things_of(Kind::Object));
                things.iter_mut().for_each(|s| s.more = true);
                dedupe(things)
            }
            ("add", []) => {
                let mut things = self.things_of(Kind::Character);
                things.extend(self.things_of(Kind::Object));
                things.iter_mut().for_each(|s| s.more = false);
                things
            }
            ("move" | "add", [_]) => {
                let mut places = self.rooms();
                places.extend(self.things_of(Kind::Character));
                // Any object can hold things, scenery included (REQ-CF-31).
                places.extend(self.things_here());
                places.extend(self.things_of(Kind::Object));
                dedupe(places)
            }
            _ => Vec::new(),
        }
    }

    /// For `go`: exits first, then the people and things here, then every
    /// room, character, object and folder.
    fn places_to_go(&self) -> Vec<Suggestion> {
        let mut out: Vec<Suggestion> = self.exits_here().into_iter().map(|(dir, to)| plain(&dir, &to, false)).collect();
        if self.mode == Mode::Play {
            return out;
        }
        out.push(plain("back", "where you were before", false));
        out.extend(self.things_here());
        let here = self.room_here();
        out.extend(self.rooms().into_iter().filter(|s| Some(&s.text) != here.as_ref()));
        out.extend(self.things_of(Kind::Character));
        out.extend(self.things_of(Kind::Object));
        out.extend(self.world.dirs.iter().filter(|d| !d.is_empty()).map(|d| plain(&format!("{}/", d), "folder", false)));
        dedupe(out)
    }

    /// The people and things in the room: edit mode's, or the player's.
    fn things_here(&self) -> Vec<Suggestion> {
        let room = self.room_here();
        let Some(location) = room.and_then(|r| self.world.engine.get_location(&r)) else { return Vec::new() };
        let mut out: Vec<Suggestion> = location
            .characters
            .iter()
            .map(|id| plain(id, &self.title_of(Kind::Character, id).unwrap_or_else(|| "no card".to_string()), false))
            .collect();
        out.extend(location.objects.iter().map(|id| {
            plain(id, &self.title_of(Kind::Object, id).unwrap_or_else(|| "scenery, no card".to_string()), false)
        }));
        out
    }

    /// This room's exits, as (direction, where to).
    fn exits_here(&self) -> Vec<(String, String)> {
        let room = self.room_here();
        let Some(location) = room.and_then(|r| self.world.engine.get_location(&r)) else { return Vec::new() };
        // Where each exit leads: through a door to the room beyond, and
        // marked when that room isn't written yet.
        let engine = &self.world.engine;
        let mut exits: Vec<(String, String)> = location
            .connections
            .iter()
            .map(|(d, t)| {
                let room = engine.doors.get(t).and_then(|door| door.leads_to(&location.id)).unwrap_or(t).to_string();
                let shown = if engine.get_location(&room).is_some() { room } else { format!("{} (not written)", room) };
                (d.clone(), shown)
            })
            .collect();
        exits.sort();
        exits
    }

    /// Every room, nearest first from where you are (the map's distances),
    /// then the ones no exit reaches, A to Z.
    fn rooms(&self) -> Vec<Suggestion> {
        let mut rooms = self.things_of(Kind::Room);
        if let Some(here) = self.room_here() {
            let steps: std::collections::HashMap<String, usize> = self.world.engine.distances(&here).into_iter().collect();
            rooms.sort_by_key(|s| (steps.get(&s.text).copied().unwrap_or(usize::MAX), s.text.clone()));
        }
        rooms
    }

    /// The room edit mode is in, or the player's in play mode.
    fn room_here(&self) -> Option<String> {
        match self.mode {
            Mode::Play => self.game.as_ref().map(|g| g.location.clone()),
            Mode::Edit => match &self.place {
                Place::File(path) => self.world.files.get(path).filter(|e| e.kind == Kind::Room).map(|e| e.id.clone()),
                Place::Dir(_) => None,
            },
        }
    }

    /// Every thing of `kind`, by id, A to Z.
    fn things_of(&self, kind: Kind) -> Vec<Suggestion> {
        let mut out: Vec<Suggestion> = self
            .world
            .files
            .values()
            .filter(|e| e.kind == kind)
            .map(|e| plain(&e.id, &format!("{} ({})", e.title, kind.label()), false))
            .collect();
        out.sort_by(|a, b| a.text.cmp(&b.text));
        out
    }

    fn title_of(&self, kind: Kind, id: &str) -> Option<String> {
        self.world.files.values().find(|e| e.kind == kind && e.id == id).map(|e| e.title.clone())
    }
}

fn plain(text: &str, detail: &str, more: bool) -> Suggestion {
    Suggestion { text: text.to_string(), label: text.to_string(), detail: detail.to_string(), more }
}

fn directions() -> Vec<Suggestion> {
    COMPASS_ALIASES.iter().map(|(_, long)| plain(long, "", true)).collect()
}

/// The suggestions that fit `typed`: those starting with it first, then
/// those containing it, in their order.
fn narrow(options: Vec<Suggestion>, typed: &str) -> Vec<Suggestion> {
    let typed = typed.to_lowercase();
    if typed.is_empty() {
        return options;
    }
    let (starts, rest): (Vec<_>, Vec<_>) =
        options.into_iter().partition(|s| s.text.to_lowercase().starts_with(&typed));
    let contains = rest.into_iter().filter(|s| s.text.to_lowercase().contains(&typed));
    starts.into_iter().chain(contains).collect()
}

/// Each suggestion once, the first time it appears.
fn dedupe(options: Vec<Suggestion>) -> Vec<Suggestion> {
    let mut seen = std::collections::BTreeSet::new();
    options.into_iter().filter(|s| seen.insert(s.text.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tiny() -> Console {
        Console::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/transcripts/worlds/tiny"), Mode::Edit).unwrap()
    }

    fn texts(c: &Completion) -> Vec<&str> {
        c.suggestions.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn a_slash_offers_the_commands_that_work_here() {
        let mut console = tiny();
        let all = console.complete("/");
        assert_eq!(all.suggestions.len(), COMMANDS.len());
        assert_eq!(texts(&console.complete("/re")), vec!["/reload", "/restart"]);
        console.handle("/play");
        assert!(!texts(&console.complete("/")).contains(&"/set"), "edit-only commands leave play mode's list");
    }

    #[test]
    fn picking_a_command_that_takes_arguments_adds_a_space() {
        let console = tiny();
        let c = console.complete("/li");
        assert_eq!(c.apply("/li", 0), "/link ");
        let c = console.complete("/sta");
        assert_eq!(c.apply("/sta", 0), "/status");
    }

    #[test]
    fn arguments_come_from_the_world() {
        let mut console = tiny();
        console.handle("go /village/square");
        let c = console.complete("/edit object ");
        assert_eq!(texts(&c), vec!["lantern"]);
        assert_eq!(c.hint.as_deref(), Some("/edit [room|object|character] [id] or <name>"));
        assert_eq!(texts(&console.complete("/unlink ")), vec!["east", "north"]);
        assert_eq!(texts(&console.complete("/link north ")), vec!["village/square", "village/well"]);
        let c = console.complete("/move l");
        assert_eq!(c.apply("/move l", 0), "/move lantern ");
    }

    #[test]
    fn go_offers_exits_first_then_what_is_here_then_anywhere() {
        let mut console = tiny();
        console.handle("go /village/square");
        let go = console.complete("go ");
        assert_eq!(&texts(&go)[..5], &["east", "north", "back", "elder", "lantern"]);
        assert!(texts(&go).contains(&"village/"));
        assert_eq!(texts(&console.complete("go we")), vec!["village/well"]);
        assert_eq!(texts(&console.complete("x b")), vec!["bench"]);
    }

    #[test]
    fn play_mode_offers_exits_and_what_is_here() {
        let mut console = tiny();
        console.handle("/play");
        assert_eq!(texts(&console.complete("go ")), vec!["east", "north"]);
        assert_eq!(texts(&console.complete("take l")), vec!["lantern", "elder"], "starting with it, then containing it");
    }
}
