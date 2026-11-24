//! The console's state and its commands: the `/` commands that work in
//! both modes (REQ-CON-2), edit-mode navigation (REQ-CON-4) and
//! `/reload` (REQ-CON-7). Play mode is in `play.rs`, authoring in
//! `author.rs`.

use super::verbs::{parse, Verb, DIRECTIONS};
use super::view;
use super::world::{Kind, World};
use super::Mode;
use crate::engines::GameEngine;
use crate::models::game::PlayState;
use crate::render::render_location;
use anyhow::Result;
use std::path::Path;
use std::time::Instant;

/// Whether the loop should keep reading input.
#[derive(Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
    /// `/restart`: the terminal loop restarts onto the new build.
    Restart,
    /// `/clear`: the screen empties its output.
    Clear,
}

/// Where edit mode is, as a path relative to the world root: a folder
/// (`""` is the root, `castle` is an area) or a content file
/// (`castle/courtyard`, `npcs/king`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Place {
    Dir(String),
    File(String),
}

pub struct Console {
    pub(super) world: World,
    pub(super) mode: Mode,
    /// Where edit mode is. Separate from the player's room: moving around
    /// in edit mode never moves the player (REQ-CON-12).
    pub(super) place: Place,
    /// The game in progress, if one has started.
    pub(super) game: Option<PlayState>,
    pub(super) game_engine: GameEngine,
    /// Files the console saved itself, relative to the world root, with
    /// when, so the file watcher doesn't announce them a second time.
    pub(super) own_writes: Vec<(String, Instant)>,
    /// The world's warnings before the current command changed anything.
    pub(super) warnings_before: Option<Vec<String>>,
    /// Whether a terminal is attached, so `/edit <kind>` can open the
    /// full-screen editor. Scripts and transcripts have none.
    pub(super) interactive: bool,
    /// The file a `/edit <kind>` asked to open, for the terminal loop.
    pub(super) pending_edit: Option<super::full_edit::EditTarget>,
    /// Where edit mode has been, most recent last, for `go back`.
    pub(super) trail: Vec<Place>,
    /// Where input the console didn't understand is logged, for reviewing
    /// what authors and players reach for. Only a terminal session sets it.
    pub(super) unknown_log: Option<std::path::PathBuf>,
}

/// How many places `go back` remembers.
const TRAIL_LENGTH: usize = 100;

pub(super) const COMPASS_ALIASES: &[(&str, &str)] = &[
    ("n", "north"),
    ("ne", "northeast"),
    ("e", "east"),
    ("se", "southeast"),
    ("s", "south"),
    ("sw", "southwest"),
    ("w", "west"),
    ("nw", "northwest"),
    ("u", "up"),
    ("d", "down"),
];

impl Console {
    pub fn open(world_path: &Path, mode: Mode) -> Result<Console> {
        let world = World::load(world_path)?;
        let mut console = Console {
            world,
            mode,
            place: Place::Dir(String::new()),
            game: None,
            game_engine: GameEngine::new(),
            own_writes: Vec::new(),
            warnings_before: None,
            interactive: false,
            pending_edit: None,
            trail: Vec::new(),
            unknown_log: None,
        };
        if mode == Mode::Play {
            if let Some(room) = console.world.default_room() {
                console.start_game(&room).map_err(anyhow::Error::msg)?;
                console.place_at_room();
            }
        }
        Ok(console)
    }

    pub fn prompt(&self) -> String {
        let at = match self.mode {
            Mode::Edit => self.pwd(),
            Mode::Play => self.game.as_ref().map(|g| g.location.clone()).unwrap_or_else(|| "nowhere".to_string()),
        };
        format!("{} {}> ", self.mode.name(), at)
    }

    /// What the console says when it opens.
    pub fn greeting(&self) -> String {
        let verb = match self.mode {
            Mode::Edit => "Editing",
            Mode::Play => "Playing",
        };
        let mut out = vec![format!(
            "{} {} ({}). Type /help for commands.",
            verb,
            self.world.display_name(),
            self.world.root.display()
        )];
        out.extend(self.problem_summary());
        if self.mode == Mode::Play {
            out.push(String::new());
            out.push(self.play_look());
        }
        out.join("\n")
    }

    /// Handles one line of input and returns the text to show.
    pub fn handle(&mut self, line: &str) -> (String, Flow) {
        let line = line.trim();
        if line.is_empty() {
            return (String::new(), Flow::Continue);
        }
        if let Some(command) = line.strip_prefix('/') {
            return self.console_command(command);
        }
        let out = match self.mode {
            Mode::Edit => self.edit_input(line),
            Mode::Play => self.play_input(line),
        };
        (out, Flow::Continue)
    }

    fn console_command(&mut self, command: &str) -> (String, Flow) {
        let mut words = command.split_whitespace();
        let name = words.next().unwrap_or("");
        let rest: Vec<&str> = words.collect();
        let out = match (name, rest.as_slice()) {
            ("quit" | "q", _) => return ("Bye.".to_string(), Flow::Quit),
            ("help" | "?", _) => self.help(),
            ("clear", []) if self.interactive => return (String::new(), Flow::Clear),
            ("clear", []) => "/clear empties the screen's output, which needs a terminal.".to_string(),
            ("status", []) => self.status_report(),
            ("clear" | "status", _) => format!("Usage: /{}", name),
            ("restart", []) if self.interactive => return (String::new(), Flow::Restart),
            ("restart", []) => "/restart restarts the console on the newest build, which needs a terminal.".to_string(),
            ("restart", _) => "Usage: /restart".to_string(),
            ("edit", args) => self.full_edit(args),
            ("play", []) => self.play(),
            ("play", ["here"]) => self.play_here(),
            ("play", _) => "Usage: /play, or /play here".to_string(),
            ("save", args) => self.save_game_as(args),
            ("load", args) => self.load_game_named(args),
            ("reload", _) => self.reload(),
            ("new", args) if self.mode == Mode::Edit => self.new_entity(args),
            ("set", args) if self.mode == Mode::Edit => self.set(args),
            ("link", args) if self.mode == Mode::Edit => self.link(args),
            ("unlink", args) if self.mode == Mode::Edit => self.unlink(args),
            ("move", args) if self.mode == Mode::Edit => self.move_thing(args),
            ("new" | "set" | "link" | "unlink" | "move", _) => {
                format!("/{} changes content, so it only works in edit mode. Type /edit to switch.", name)
            }
            ("", _) => "Type /help for commands.".to_string(),
            _ => {
                self.log_unknown("command", &format!("/{}", command));
                format!("Unknown command `/{}`. Type /help for commands.", name)
            }
        };
        (out, Flow::Continue)
    }

    fn help(&self) -> String {
        let mut lines = vec![
            "Console commands, in either mode:".to_string(),
            "  /edit           edit where you are (from play: the player's room)".to_string(),
            "  /play           switch to play mode, resuming the game".to_string(),
            "  /play here      play from the room edit mode is in".to_string(),
            "  /save [name]    save the game (quicksave if no name)".to_string(),
            "  /load [name]    load a saved game".to_string(),
            "  /reload         reload the world from disk, staying where you are".to_string(),
            "  /status         the world, where you are, the game, saves and the build".to_string(),
            "  /clear          empty the output (the session carries on)".to_string(),
            "  /restart        restart the console on the newest build, keeping your place".to_string(),
            "  /help           this list".to_string(),
            "  /quit           leave the console".to_string(),
        ];
        match self.mode {
            Mode::Edit => lines.extend([
                String::new(),
                "Edit mode moves through the world as play does:".to_string(),
                "  north, n, go up, ...  follow an exit from this room".to_string(),
                "  go <name>             go to a room, area, folder, character or object".to_string(),
                "  go back               back to where you were;  go /  goes to the world root".to_string(),
                "  look                  show where you are: a room, a thing, or a folder".to_string(),
                "  examine <thing>, x    show a person or thing here without going to it".to_string(),
                String::new(),
                "Changing content (each saves the file):".to_string(),
                "  /new room <direction> <id> [title]   a room through a new exit from this one".to_string(),
                "  /new room <id> [title]               a room with no exits, outside a room".to_string(),
                "  /new character <id> [title]          a character, placed here if in a room".to_string(),
                "  /new object <id> <type> [title]      an object, placed here if in a room".to_string(),
                "  /set <key> <value>                   one field; attributes.health 80 sets one entry".to_string(),
                "  /link <direction> <room>             add or change one exit from this room".to_string(),
                "  /unlink <direction>                  remove one exit from this room".to_string(),
                "  /move <thing> <room or character>    put a person or thing somewhere else".to_string(),
                String::new(),
                "Full editing, in a full-screen editor (ctrl-s saves, esc closes):".to_string(),
                "  /edit room [id]                      the room you're in, or the one named".to_string(),
                "  /edit character [id]                 the character you're in, or the one named".to_string(),
                "  /edit object [id]                    an object; scenery here gets its card".to_string(),
            ]),
            Mode::Play => lines.extend([
                String::new(),
                "In the game:".to_string(),
                "  look, l                   look around".to_string(),
                "  go <direction>, north, n  go somewhere (ne, up, u, ...)".to_string(),
                "  take <thing>, get         pick something up".to_string(),
                "  examine <thing>, x        look closely (also: look at)".to_string(),
                "  inventory, i              what you're carrying".to_string(),
            ]),
        }
        lines.join("\n")
    }

    /// Reloads from disk, keeping the game and edit mode's place
    /// (REQ-CON-7). Keeps the old world if a file fails to parse or the
    /// player's room is gone.
    pub(crate) fn reload(&mut self) -> String {
        let fresh = match World::load(&self.world.root) {
            Ok(world) => world,
            Err(e) => return format!("Reload failed, keeping the previous world: {}", e),
        };
        let mut blockers: Vec<String> = fresh.blocking_problems();
        if let Some(game) = &self.game {
            if fresh.engine.get_location(&game.location).is_none() {
                blockers.push(format!(
                    "the player's room `{}` no longer exists (/play here from another room moves the player)",
                    game.location
                ));
            }
        }
        if !blockers.is_empty() {
            let mut out = vec!["Reload failed, keeping the previous world:".to_string()];
            out.extend(blockers.iter().map(|p| format!("  {}", p)));
            return out.join("\n");
        }

        self.world = fresh;
        let mut out = vec![format!(
            "Reloaded: {} room(s), {} character(s), {} object(s).",
            self.world.report.locations_loaded,
            self.world.engine.characters.len(),
            self.world.report.objects_loaded
        )];
        if !self.place_exists() {
            let gone = self.pwd();
            self.move_to_existing_folder();
            out.push(format!("{} is gone; now at {}.", gone, self.pwd()));
        }
        for warning in self.reconcile_game() {
            out.push(format!("Game: {}", warning));
        }
        let warnings = self.world.warnings();
        if !warnings.is_empty() {
            out.push(format!("{} warning(s):", warnings.len()));
            out.extend(warnings.iter().map(|w| format!("  {}", w)));
        }
        out.join("\n")
    }

    fn problem_summary(&self) -> Vec<String> {
        let mut out = Vec::new();
        let errors = self.world.blocking_problems();
        if !errors.is_empty() {
            out.push(format!("{} file(s) don't parse:", errors.len()));
            out.extend(errors.iter().map(|e| format!("  {}", e)));
        }
        let warnings = self.world.warnings();
        if !warnings.is_empty() {
            out.push(format!("{} warning(s), such as exits to rooms not written yet. `thwart validate` lists them.", warnings.len()));
        }
        out
    }

    // Edit-mode navigation (REQ-CON-4).

    /// Edit mode moves with play mode's words (REQ-CON-4): the author
    /// walks the world like a game master, not a file system.
    fn edit_input(&mut self, line: &str) -> String {
        let first = line.split_whitespace().next().unwrap_or("");
        if matches!(first, "cd" | "ls" | "pwd") {
            return format!(
                "Edit mode moves as play does now: go <name>, a direction, go back, look. (`{}` is gone.)",
                first
            );
        }
        match parse(line) {
            Verb::Look => self.look(),
            Verb::Examine(noun) => self.examine(&noun),
            Verb::Go(target) => self.go(&target),
            Verb::Incomplete(question) => question,
            Verb::Take(_) | Verb::Inventory => {
                "That's a play-mode verb. Here, /new and /set change what's in a room; /play here tries it.".to_string()
            }
            Verb::Unknown(_) => {
                self.log_unknown("edit", line);
                format!("Edit mode doesn't know `{}`. Type /help for commands.", first)
            }
        }
    }

    /// `go <direction or name>`, `go back` and `go /`.
    fn go(&mut self, target: &str) -> String {
        let target = target.trim();
        if target == "back" {
            return match self.trail.pop() {
                Some(place) => {
                    self.place = place;
                    self.arrival()
                }
                None => "There's nowhere to go back to yet.".to_string(),
            };
        }
        match self.resolve(target) {
            Ok(place) => {
                if place != self.place {
                    self.trail.push(std::mem::replace(&mut self.place, place));
                    if self.trail.len() > TRAIL_LENGTH {
                        self.trail.remove(0);
                    }
                }
                self.arrival()
            }
            Err(message) => message,
        }
    }

    /// What edit mode says on arriving somewhere: a room's or thing's
    /// title, or what a folder holds.
    fn arrival(&self) -> String {
        match &self.place {
            Place::File(path) => match self.world.files.get(path) {
                Some(entry) if entry.kind == Kind::Room => format!("{} (room)", entry.title),
                Some(entry) if entry.kind == Kind::Broken => format!("{}.md (doesn't parse)", path),
                Some(entry) => format!("{} ({}), {}.md", entry.title, entry.kind.label(), path),
                None => self.pwd(),
            },
            Place::Dir(dir) => {
                let prefix = if dir.is_empty() { String::new() } else { format!("{}/", dir) };
                let child = |path: &str| -> Option<String> {
                    let rest = path.strip_prefix(&prefix)?;
                    (!rest.is_empty() && !rest.contains('/')).then(|| rest.to_string())
                };
                let folders: Vec<String> = self.world.dirs.iter().filter_map(|d| child(d)).collect();
                let files: Vec<(String, Kind)> =
                    self.world.files.iter().filter_map(|(p, e)| child(p).map(|n| (n, e.kind))).collect();
                let mut names: Vec<String> = folders.iter().map(|f| format!("{}/", f)).collect();
                names.extend(files.iter().map(|(n, _)| n.clone()));
                if names.is_empty() {
                    return format!("{}  nothing here yet", self.pwd());
                }
                let noun = if folders.is_empty() && files.iter().all(|(_, k)| *k == Kind::Room) {
                    "room"
                } else if folders.is_empty() && files.iter().all(|(_, k)| matches!(k, Kind::Object | Kind::Character)) {
                    "thing"
                } else {
                    "item"
                };
                let plural = if names.len() == 1 { "" } else { "s" };
                format!("{}  {} {}{}: {}", self.pwd(), names.len(), noun, plural, names.join(", "))
            }
        }
    }

    /// `examine <thing>`: a person or thing here, shown without going to it.
    fn examine(&self, noun: &str) -> String {
        match self.resolve_here(noun) {
            Some(Ok(Place::File(path))) => self.view_of(&path),
            Some(Ok(Place::Dir(_))) | None => {
                format!("There's no `{}` here. go {} looks for it anywhere in the world.", noun, noun)
            }
            Some(Err(message)) => message,
        }
    }

    /// Appends one line to the unknown-input log: when, the world, the
    /// mode, where, what kind of input (`command`, `edit`, `play`) and what
    /// was typed, tab-separated. A log that can't be written is skipped.
    pub(super) fn log_unknown(&self, kind: &str, typed: &str) {
        let Some(path) = &self.unknown_log else { return };
        let at = match self.mode {
            Mode::Play => self.game.as_ref().map(|g| g.location.clone()).unwrap_or_default(),
            Mode::Edit => self.pwd(),
        };
        let line = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
            self.world.root.display(),
            self.mode.name(),
            at,
            kind,
            typed.replace(['\t', '\n'], " ")
        );
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub(super) fn pwd(&self) -> String {
        match &self.place {
            Place::Dir(dir) if dir.is_empty() => "/".to_string(),
            Place::Dir(dir) => format!("{}/", dir),
            Place::File(path) => path.clone(),
        }
    }

    fn list_dir(&self, dir: &str) -> String {
        let prefix = if dir.is_empty() { String::new() } else { format!("{}/", dir) };
        let child = |path: &str| -> Option<String> {
            let rest = path.strip_prefix(&prefix)?;
            (!rest.is_empty() && !rest.contains('/')).then(|| rest.to_string())
        };

        let mut rows: Vec<(String, String)> = Vec::new();
        for sub in self.world.dirs.iter().filter_map(|d| child(d)) {
            let full = format!("{}{}", prefix, sub);
            let count = self.world.files.keys().filter(|p| p.starts_with(&format!("{}/", full))).count();
            rows.push((format!("{}/", sub), format!("{} file(s)", count)));
        }
        for (path, entry) in &self.world.files {
            if let Some(name) = child(path) {
                let note = if entry.kind == Kind::Broken {
                    "(doesn't parse)".to_string()
                } else {
                    format!("{}  ({})", entry.title, entry.kind.label())
                };
                rows.push((name, note));
            }
        }
        if rows.is_empty() {
            return "Nothing here.".to_string();
        }
        let width = rows.iter().map(|(n, _)| n.chars().count()).max().unwrap_or(0);
        rows.iter()
            .map(|(name, note)| format!("{:<width$}  {}", name, note, width = width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn look(&self) -> String {
        match (&self.mode, &self.place) {
            (Mode::Play, _) => self.play_look(),
            (Mode::Edit, Place::Dir(dir)) => self.list_dir(dir),
            (Mode::Edit, Place::File(path)) => self.view_of(path),
        }
    }

    /// The edit-mode view of the content file at `path`.
    fn view_of(&self, path: &str) -> String {
        let path = path.to_string();
        let Some(entry) = self.world.files.get(&path) else {
            return format!("{} is not in the world.", path);
        };
        match entry.kind {
            Kind::Room => match self.world.engine.describe_location(&entry.id) {
                Some(view) => render_location(&view),
                None => format!("{} is not in the world.", entry.id),
            },
            Kind::Character => match self.world.engine.characters.get(&entry.id) {
                Some(c) => view::character(c, &path),
                None => format!("{} is not in the world.", entry.id),
            },
            Kind::Object => match self.world.engine.objects.get(&entry.id) {
                Some(o) => view::object(o, &path),
                None => format!("{} is not in the world.", entry.id),
            },
            Kind::World => view::world(self.world.engine.world(), &path),
            Kind::Broken => {
                let reason = if entry.title.is_empty() { "it doesn't parse" } else { entry.title.as_str() };
                format!("{}.md isn't part of the world: {}", path, reason)
            }
            // A part of a thing or of the world: shown as written, until
            // bundle editing shows it in place (`design/STATES.md` › Console).
            _ => {
                let text = std::fs::read_to_string(self.world.root.join(format!("{}.md", path))).unwrap_or_default();
                format!("{} ({}), {}.md\n\n{}", entry.title, entry.kind.label(), path, text.trim_end())
            }
        }
    }

    /// Finds the place `target` names, trying in turn: `/`; an exit, person
    /// or thing here (`resolve_here`); a path from this folder or the world
    /// root; then a character, object, room, area or folder anywhere, by id
    /// or name. A name that fits several places is a question.
    fn resolve(&self, target: &str) -> Result<Place, String> {
        let target = target.trim();
        if target == "/" {
            return Ok(Place::Dir(String::new()));
        }
        if let Some(found) = self.resolve_here(target) {
            return found;
        }
        if DIRECTIONS.contains(&target) && !matches!(self.current_kind(), Some(Kind::Room)) {
            return Err(format!("There are no exits here: {} isn't a room. go <name> goes anywhere.", self.pwd()));
        }

        let trimmed = target.trim_end_matches('/');
        let here = self.current_dir();
        let relative = if here.is_empty() { trimmed.to_string() } else { format!("{}/{}", here, trimmed) };
        let absolute = trimmed.trim_start_matches('/').to_string();
        for candidate in [relative, absolute] {
            if self.world.files.contains_key(&candidate) {
                return Ok(Place::File(candidate));
            }
            if self.world.dirs.contains(&candidate) {
                return Ok(Place::Dir(candidate));
            }
        }

        for kind in [Kind::Character, Kind::Object] {
            if let Some(path) = self.world.path_of(kind, trimmed) {
                return Ok(Place::File(path));
            }
        }
        let wanted = trimmed.to_lowercase();
        let last = |path: &str| path.rsplit('/').next().unwrap_or(path).to_lowercase();
        let mut found: Vec<Place> = self
            .world
            .files
            .iter()
            .filter(|(path, e)| e.kind != Kind::Broken && (last(path) == wanted || e.title.to_lowercase() == wanted))
            .map(|(path, _)| Place::File(path.clone()))
            .collect();
        found.extend(self.world.dirs.iter().filter(|d| !d.is_empty() && last(d) == wanted).map(|d| Place::Dir(d.clone())));
        match found.len() {
            0 => Err(format!("Nothing called `{}` here or anywhere in the world.", target)),
            1 => Ok(found.remove(0)),
            _ => {
                let names: Vec<String> = found
                    .iter()
                    .map(|p| match p {
                        Place::File(path) => path.clone(),
                        Place::Dir(dir) => format!("{}/", dir),
                    })
                    .collect();
                Err(format!("Which do you mean: {}?", names.join(", ")))
            }
        }
    }

    /// An exit from this room, or a person or thing in it, if `target`
    /// names one. `None` when it names nothing here.
    fn resolve_here(&self, target: &str) -> Option<Result<Place, String>> {
        let Place::File(path) = &self.place else { return None };
        let entry = self.world.files.get(path).filter(|e| e.kind == Kind::Room)?;
        let location = self.world.engine.get_location(&entry.id)?;
        let direction = COMPASS_ALIASES.iter().find(|(short, _)| *short == target).map(|(_, long)| *long).unwrap_or(target);
        if let Some(to) = location.connections.get(direction) {
            // An exit through a door names the door; it leads to the room
            // on the door's far side.
            let to = match self.world.engine.doors.get(to) {
                Some(door) => match door.leads_to(&entry.id) {
                    Some(room) => room.to_string(),
                    None => return Some(Err(format!("The {} exit goes through `{}`, which doesn't say where it leads from here.", direction, to))),
                },
                None => to.clone(),
            };
            let to = &to;
            return Some(match self.world.path_of(Kind::Room, to) {
                Some(path) => Ok(Place::File(path)),
                None => Err(format!("The {} exit leads to `{}`, which isn't written yet.", direction, to)),
            });
        }
        if location.characters.iter().any(|c| c == target) {
            return Some(
                self.world
                    .path_of(Kind::Character, target)
                    .map(Place::File)
                    .ok_or_else(|| format!("`{}` is here but has no character card.", target)),
            );
        }
        if location.objects.iter().any(|o| o == target) {
            return Some(self.world.path_of(Kind::Object, target).map(Place::File).ok_or_else(|| {
                format!("`{}` is scenery, with no card yet. /new object {} gives it one.", target, target)
            }));
        }
        None
    }

    /// The kind of file edit mode is in, if it's in one.
    fn current_kind(&self) -> Option<Kind> {
        let Place::File(path) = &self.place else { return None };
        self.world.files.get(path).map(|e| e.kind)
    }

    /// The folder this place is in: itself for a folder, its parent for a file.
    pub(super) fn current_dir(&self) -> String {
        match &self.place {
            Place::Dir(dir) => dir.clone(),
            Place::File(path) => parent_of(path),
        }
    }

    fn parent_dir(&self) -> String {
        match &self.place {
            Place::Dir(dir) => parent_of(dir),
            Place::File(path) => parent_of(path),
        }
    }

    pub(super) fn place_exists(&self) -> bool {
        match &self.place {
            Place::Dir(dir) => self.world.dirs.contains(dir),
            Place::File(path) => self.world.files.contains_key(path),
        }
    }

    /// Moves edit mode's place to the player's room, if there is one.
    pub(super) fn place_at_room(&mut self) {
        let room = self.game.as_ref().map(|g| g.location.clone());
        if let Some(path) = room.and_then(|r| self.world.path_of(Kind::Room, &r)) {
            self.place = Place::File(path);
        }
    }

    /// After the place's file or folder vanished: up to the nearest
    /// folder that still exists.
    pub(super) fn move_to_existing_folder(&mut self) {
        let mut dir = self.parent_dir();
        while !dir.is_empty() && !self.world.dirs.contains(&dir) {
            dir = parent_of(&dir);
        }
        self.place = Place::Dir(dir);
    }
}

fn parent_of(path: &str) -> String {
    path.rsplit_once('/').map(|(parent, _)| parent.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    //! `/reload` cases a transcript can't reach, because they change
    //! files while the console runs (REQ-CON-7).
    use super::*;
    use std::fs;
    use walkdir::WalkDir;

    #[test]
    fn unknown_input_is_logged_with_where_it_was_typed() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("state/thwarter/unknown-commands.log");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/transcripts/worlds/tiny");
        let mut console = Console::open(&root, Mode::Edit).unwrap();
        console.unknown_log = Some(log.clone());
        console.handle("go /village/square");
        console.handle("/bogus thing");
        console.handle("dance");
        console.handle("/play");
        console.handle("light the lantern");
        console.handle("look");
        let text = fs::read_to_string(&log).unwrap();
        let rows: Vec<Vec<&str>> = text.lines().map(|l| l.split('\t').skip(2).collect()).collect();
        assert_eq!(
            rows,
            vec![
                vec!["edit", "village/square", "command", "/bogus thing"],
                vec!["edit", "village/square", "edit", "dance"],
                vec!["play", "village/square", "play", "light the lantern"],
            ]
        );
    }

    const FIXTURE: &str = "tests/transcripts/worlds/tiny";

    fn copy_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for entry in WalkDir::new(FIXTURE).into_iter().filter_map(|e| e.ok()) {
            let rel = entry.path().strip_prefix(FIXTURE).unwrap();
            let to = dir.path().join(rel);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&to).unwrap();
            } else {
                fs::copy(entry.path(), &to).unwrap();
            }
        }
        dir
    }

    fn console_at_well(dir: &tempfile::TempDir) -> Console {
        let mut console = Console::open(dir.path(), Mode::Edit).unwrap();
        console.handle("go village/well");
        assert_eq!(console.pwd(), "village/well");
        console
    }

    #[test]
    fn reload_picks_up_edits_and_keeps_the_room() {
        let dir = copy_fixture();
        let mut console = console_at_well(&dir);
        let well = dir.path().join("village/well.md");
        fs::write(&well, fs::read_to_string(&well).unwrap().replace("The Well", "The Old Well")).unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.starts_with("Reloaded:"), "{}", out);
        assert_eq!(console.pwd(), "village/well");
        assert!(console.handle("look").0.starts_with("The Old Well (village/well)"));
    }

    #[test]
    fn reload_keeps_the_old_world_when_a_file_fails_to_parse() {
        let dir = copy_fixture();
        let mut console = console_at_well(&dir);
        fs::write(dir.path().join("village/square.md"), "---\nid: [unclosed\n---\n").unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.starts_with("Reload failed, keeping the previous world:"), "{}", out);
        assert!(out.contains("square.md"), "{}", out);
        assert!(console.handle("go s").0.starts_with("Village Square"), "old world still loaded");
    }

    #[test]
    fn reload_keeps_the_old_world_when_the_players_room_is_gone() {
        let dir = copy_fixture();
        let mut console = console_at_well(&dir);
        console.handle("/play here");
        fs::remove_file(dir.path().join("village/well.md")).unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.contains("the player's room `village/well` no longer exists"), "{}", out);
        assert!(console.handle("look").0.starts_with("The Well"), "old world still loaded");
    }

    #[test]
    fn reload_without_a_game_moves_edit_mode_up_when_its_room_is_gone() {
        let dir = copy_fixture();
        let mut console = console_at_well(&dir);
        fs::remove_file(dir.path().join("village/well.md")).unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.contains("village/well is gone; now at village/."), "{}", out);
    }

    #[test]
    fn reload_drops_carried_objects_whose_cards_are_gone() {
        let dir = copy_fixture();
        let mut console = Console::open(dir.path(), Mode::Play).unwrap();
        assert!(console.handle("take lantern").0.starts_with("You take"));
        fs::remove_file(dir.path().join("objects/lantern.md")).unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.contains("Game: "), "{}", out);
        assert_eq!(console.handle("i").0, "You aren't carrying anything.");
    }

    #[test]
    fn reload_moves_up_a_folder_when_the_current_card_is_gone() {
        let dir = copy_fixture();
        let mut console = console_at_well(&dir);
        console.handle("go /npcs/elder");
        fs::remove_file(dir.path().join("npcs/elder.md")).unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.contains("npcs/elder is gone; now at /."), "{}", out);
    }
}
