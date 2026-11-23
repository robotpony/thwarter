//! The console's state and its commands: the `/` commands that work in
//! both modes (REQ-CON-2), edit-mode navigation (REQ-CON-4) and
//! `/reload` (REQ-CON-7).

use super::view;
use super::world::{Kind, World};
use super::Mode;
use crate::render::render_location;
use anyhow::Result;
use std::path::Path;

/// Whether the loop should keep reading input.
#[derive(Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

/// Where edit mode is, as a path relative to the world root: a folder
/// (`""` is the root, `castle` is an area) or a content file
/// (`castle/courtyard`, `npcs/king`).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Place {
    Dir(String),
    File(String),
}

pub struct Console {
    world: World,
    mode: Mode,
    place: Place,
    /// The room the player is in, shared by both modes, so switching mode
    /// or reloading keeps it (REQ-CON-7).
    room: Option<String>,
}

const COMPASS_ALIASES: &[(&str, &str)] = &[
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
        let room = world.default_room();
        let mut console = Console { world, mode, place: Place::Dir(String::new()), room };
        if mode == Mode::Play {
            console.place_at_room();
        }
        Ok(console)
    }

    pub fn prompt(&self) -> String {
        let at = match self.mode {
            Mode::Edit => self.pwd(),
            Mode::Play => self.room.clone().unwrap_or_else(|| "nowhere".to_string()),
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
            out.push(self.look());
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
            Mode::Play => format!(
                "Play mode doesn't understand `{}` yet: in-game verbs come in PLAN.md › Phase 4.",
                line
            ),
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
            ("edit", []) => self.switch(Mode::Edit),
            ("edit", [kind, ..]) => self.full_edit(kind),
            ("play", _) => self.switch(Mode::Play),
            ("reload", _) => self.reload(),
            ("new" | "set" | "link", _) if self.mode == Mode::Edit => {
                format!("/{} isn't built yet (PLAN.md › Phase 3).", name)
            }
            ("new" | "set" | "link", _) => {
                format!("/{} changes content, so it only works in edit mode. Type /edit to switch.", name)
            }
            ("", _) => "Type /help for commands.".to_string(),
            _ => format!("Unknown command `/{}`. Type /help for commands.", name),
        };
        (out, Flow::Continue)
    }

    fn help(&self) -> String {
        let mut lines = vec![
            "Console commands, in either mode:".to_string(),
            "  /edit           switch to edit mode".to_string(),
            "  /play           switch to play mode".to_string(),
            "  /reload         reload the world from disk, staying where you are".to_string(),
            "  /help           this list".to_string(),
            "  /quit           leave the console".to_string(),
        ];
        match self.mode {
            Mode::Edit => lines.extend([
                String::new(),
                "Edit mode moves through the world's folders:".to_string(),
                "  ls              list this folder, or show this file".to_string(),
                "  cd <name>       go into a folder, room, character or object".to_string(),
                "  cd <direction>  follow an exit from this room (north, n, up, ...)".to_string(),
                "  cd ..           go up one folder;  cd /  goes to the world root".to_string(),
                "  pwd             where you are".to_string(),
                "  look            show the room, character or object you're in".to_string(),
                String::new(),
                "Not built yet: /new, /set, /link, /edit room (PLAN.md › Phase 3).".to_string(),
            ]),
            Mode::Play => lines.extend([
                String::new(),
                "In-game verbs (look, go, take, ...) come in PLAN.md › Phase 4.".to_string(),
            ]),
        }
        lines.join("\n")
    }

    fn switch(&mut self, mode: Mode) -> String {
        if self.mode == mode {
            return format!("Already in {} mode.", mode.name());
        }
        self.mode = mode;
        match mode {
            Mode::Edit => {
                self.place_at_room();
                format!("Edit mode, at {}.", self.pwd())
            }
            Mode::Play => {
                if let Place::File(path) = &self.place {
                    if let Some(entry) = self.world.files.get(path) {
                        if entry.kind == Kind::Room {
                            self.room = Some(entry.id.clone());
                        }
                    }
                }
                if self.room.is_none() {
                    self.room = self.world.default_room();
                }
                format!("Play mode.\n\n{}", self.look())
            }
        }
    }

    /// `/edit room`, `/edit object`, `/edit character` (REQ-CON-6). From
    /// play mode, switches to edit mode at the current room first
    /// (REQ-CON-2). The editing itself is Phase 3.
    fn full_edit(&mut self, kind: &str) -> String {
        let mut out = Vec::new();
        if !matches!(kind, "room" | "object" | "character") {
            return format!("`/edit {}`: the kinds are room, object and character.", kind);
        }
        if self.mode == Mode::Play {
            self.mode = Mode::Edit;
            self.place_at_room();
            out.push(format!("Edit mode, at {}.", self.pwd()));
        }
        out.push(format!("/edit {} isn't built yet (PLAN.md › Phase 3).", kind));
        out.join("\n")
    }

    /// Reloads from disk, keeping the room and place (REQ-CON-7). Keeps the
    /// old world if a file fails to parse or the current room is gone.
    pub(crate) fn reload(&mut self) -> String {
        let fresh = match World::load(&self.world.root) {
            Ok(world) => world,
            Err(e) => return format!("Reload failed, keeping the previous world: {}", e),
        };
        let mut blockers: Vec<String> = fresh.blocking_problems().to_vec();
        if let Some(room) = &self.room {
            if fresh.engine.get_location(room).is_none() {
                blockers.push(format!("the current room `{}` no longer exists", room));
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
            self.place = Place::Dir(String::new());
            self.place_at_room();
            out.push(format!("{} is gone; now at {}.", gone, self.pwd()));
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

    fn edit_input(&mut self, line: &str) -> String {
        let mut words = line.split_whitespace();
        let verb = words.next().unwrap_or("");
        let arg: Vec<&str> = words.collect();
        let arg = arg.join(" ");
        match (verb, arg.as_str()) {
            ("pwd", _) => self.pwd(),
            ("ls", "") => self.ls(),
            ("look" | "l", "") => self.look(),
            ("cd", "") => {
                self.place = Place::Dir(String::new());
                self.pwd()
            }
            ("cd", target) => self.cd(target),
            ("ls" | "look" | "l", _) => format!("`{}` takes no name here; cd to it first.", verb),
            _ => format!("Edit mode doesn't know `{}`. Type /help for commands.", verb),
        }
    }

    fn pwd(&self) -> String {
        match &self.place {
            Place::Dir(dir) if dir.is_empty() => "/".to_string(),
            Place::Dir(dir) => format!("{}/", dir),
            Place::File(path) => path.clone(),
        }
    }

    fn ls(&self) -> String {
        match &self.place {
            Place::Dir(dir) => self.list_dir(dir),
            Place::File(_) => self.look(),
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
        let path = match (&self.mode, &self.place) {
            (Mode::Play, _) => match self.room.as_ref().and_then(|r| self.world.path_of(Kind::Room, r)) {
                Some(path) => path,
                None => return "You are nowhere: this world has no rooms.".to_string(),
            },
            (Mode::Edit, Place::Dir(dir)) => return self.list_dir(dir),
            (Mode::Edit, Place::File(path)) => path.clone(),
        };
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
            Kind::Broken => {
                let reason = self
                    .world
                    .report
                    .errors
                    .iter()
                    .find(|e| e.contains(&format!("{}.md", path)))
                    .cloned()
                    .unwrap_or_else(|| "it doesn't parse".to_string());
                format!("{}.md doesn't parse: {}", path, reason)
            }
        }
    }

    fn cd(&mut self, target: &str) -> String {
        match self.resolve(target) {
            Ok(place) => {
                self.place = place;
                if let Place::File(path) = &self.place {
                    if let Some(entry) = self.world.files.get(path) {
                        if entry.kind == Kind::Room {
                            self.room = Some(entry.id.clone());
                        }
                        if entry.kind != Kind::Broken {
                            return format!("{} ({})", entry.title, entry.kind.label());
                        }
                    }
                }
                self.pwd()
            }
            Err(message) => message,
        }
    }

    /// Finds the place `target` names, trying in turn: `/` and `..`; an
    /// exit from this room; a character or object this room holds; a path
    /// relative to this folder; a path from the world root.
    fn resolve(&self, target: &str) -> Result<Place, String> {
        let target = target.trim();
        if target == "/" {
            return Ok(Place::Dir(String::new()));
        }
        if target == ".." {
            return Ok(Place::Dir(self.parent_dir()));
        }

        if let Place::File(path) = &self.place {
            if let Some(entry) = self.world.files.get(path).filter(|e| e.kind == Kind::Room) {
                if let Some(location) = self.world.engine.get_location(&entry.id) {
                    let direction = COMPASS_ALIASES
                        .iter()
                        .find(|(short, _)| *short == target)
                        .map(|(_, long)| *long)
                        .unwrap_or(target);
                    if let Some(to) = location.connections.get(direction) {
                        return match self.world.path_of(Kind::Room, to) {
                            Some(path) => Ok(Place::File(path)),
                            None => Err(format!("The {} exit leads to `{}`, which isn't written yet.", direction, to)),
                        };
                    }
                    if location.characters.iter().any(|c| c == target) {
                        return self
                            .world
                            .path_of(Kind::Character, target)
                            .map(Place::File)
                            .ok_or_else(|| format!("`{}` is here but has no character card.", target));
                    }
                    if location.objects.iter().any(|o| o == target) {
                        return self.world.path_of(Kind::Object, target).map(Place::File).ok_or_else(|| {
                            format!("`{}` is scenery, with no card yet. /edit object will create one (PLAN.md › Phase 3).", target)
                        });
                    }
                }
            }
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
        Err(format!("No folder, room, exit, character or object called `{}` here.", target))
    }

    /// The folder this place is in: itself for a folder, its parent for a file.
    fn current_dir(&self) -> String {
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

    fn place_exists(&self) -> bool {
        match &self.place {
            Place::Dir(dir) => self.world.dirs.contains(dir),
            Place::File(path) => self.world.files.contains_key(path),
        }
    }

    /// Moves edit mode's place to the current room's file, if there is one.
    fn place_at_room(&mut self) {
        if let Some(path) = self.room.as_ref().and_then(|r| self.world.path_of(Kind::Room, r)) {
            self.place = Place::File(path);
        }
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
        console.handle("cd village/well");
        assert_eq!(console.room.as_deref(), Some("village/well"));
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
        assert!(console.handle("cd s").0.starts_with("Village Square"), "old world still loaded");
    }

    #[test]
    fn reload_keeps_the_old_world_when_the_current_room_is_gone() {
        let dir = copy_fixture();
        let mut console = console_at_well(&dir);
        fs::remove_file(dir.path().join("village/well.md")).unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.contains("the current room `village/well` no longer exists"), "{}", out);
        assert_eq!(console.pwd(), "village/well");
    }

    #[test]
    fn reload_moves_to_the_room_when_the_current_card_is_gone() {
        let dir = copy_fixture();
        let mut console = console_at_well(&dir);
        console.handle("cd /npcs/elder");
        fs::remove_file(dir.path().join("npcs/elder.md")).unwrap();

        let (out, _) = console.handle("/reload");
        assert!(out.contains("npcs/elder is gone; now at village/well."), "{}", out);
    }
}
