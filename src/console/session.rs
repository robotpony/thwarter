//! The console's state and its commands: the `/` commands that work in
//! both modes (REQ-CON-2), edit-mode navigation (REQ-CON-4) and
//! `/reload` (REQ-CON-7). Play mode is in `play.rs`, authoring in
//! `author.rs`.

use crate::engines::{Parsed, DIRECTIONS};
use crate::models::game::PlayAction;
use super::registry;
use super::view;
use super::world::{Kind, World};
use super::Mode;
use crate::engines::GameEngine;
use crate::models::game::PlayState;
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
    /// The virtual `game/` folder, the running game (REQ-CON-24): `""`
    /// for `game/` itself, `things` and `things/<id>` below it.
    Game(String),
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
    /// What `examine` showed last, for the sidebar, until edit mode moves.
    pub(super) looked_at: Option<String>,
    /// The last play action's author notes, for the sidebar on a terminal.
    pub(super) notes: Vec<String>,
    /// The topics the last talk reply offered, picked by number or label
    /// (REQ-PC-31). Any other input drops them.
    pub(super) topics: Vec<crate::models::game::Topic>,
    /// The scope `/new` puts things in (REQ-CON-26).
    pub(super) scope: super::scope::Scope,
    /// The folder the `/new` being run writes into, from its scope.
    pub(super) new_folder: Option<std::path::PathBuf>,
    /// The console's wording: the built-in `[console]` table with the
    /// author's own over it, from their config.toml (REQ-ENG-49).
    pub(super) texts: crate::engines::texts::Texts,
    /// What was wrong with the author's `[console]`, for the greeting.
    pub(super) config_warnings: Vec<String>,
    /// The kinds of the `[console]` replies said since this was last
    /// cleared, for `/recipe` to tell a failed step.
    pub(super) said: std::cell::RefCell<Vec<super::kinds::Kind>>,
    /// Questions `/add` or `/new` left for the screen to ask.
    pub(super) pending_questions: Option<super::asking::Asking>,
    /// Whether `/menu` left the quick menu for the screen to open.
    pub(super) pending_menu: bool,
    /// What `/add` just added, for it to open in the editor.
    pub(super) added: Option<(Kind, String)>,
    /// What the current command created, as (kind, id), for `/new` to open.
    pub(super) created: Vec<(crate::engines::EntityKind, String)>,
    /// A question the console asked, which the next plain line answers.
    pub(super) asked: Option<super::doors::Asked>,
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
        let (texts, config_warnings) = console_texts();
        let world = World::load_with(world_path, texts.clone())?;
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
            looked_at: None,
            topics: Vec::new(),
            scope: Default::default(),
            new_folder: None,
            notes: Vec::new(),
            asked: None,
            created: Vec::new(),
            added: None,
            pending_questions: None,
            pending_menu: false,
            texts,
            config_warnings,
            said: Default::default(),
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

    /// `in village/square > ` in edit mode (Bruce, 2026-11-23); play mode
    /// keeps `play village/square> `. The status bar names the mode.
    pub fn prompt(&self) -> String {
        match self.mode {
            Mode::Edit if self.scope != super::scope::Scope::Global => {
                self.t("prompt_edit_scoped", &[("place", &self.pwd()), ("scope", self.scope.word())])
            }
            Mode::Edit => self.t("prompt_edit", &[("place", &self.pwd())]),
            Mode::Play => self.t(
                "prompt_play",
                &[("place", &self.game.as_ref().map(|g| g.location.clone()).unwrap_or_else(|| self.t("nowhere", &[])))],
            ),
        }
    }

    /// What the console says when it opens.
    pub fn greeting(&self) -> String {
        let key = match self.mode {
            Mode::Edit => "greeting_edit",
            Mode::Play => "greeting_play",
        };
        let mut out = vec![self.t(key, &[("world", &self.world.display_name()), ("path", &self.world.root.display().to_string())])];
        out.extend(self.problem_summary());
        out.extend(self.config_warnings.iter().cloned());
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
        // A plain line answers the question just asked; a command drops it.
        if let Some(asked) = self.asked.take() {
            // A yes-or-no question takes only yes or no; anything else
            // drops it and runs as typed.
            let yes_no = matches!(asked, super::doors::Asked::MoveHere { .. } | super::doors::Asked::Remove { .. } | super::doors::Asked::MakeWorldIndex | super::doors::Asked::StartIn { .. });
            let answers = !line.starts_with('/')
                && (!yes_no || matches!(line.trim().to_lowercase().as_str(), "y" | "yes" | "n" | "no"));
            if answers {
                return (self.answer(asked, line), Flow::Continue);
            }
        }
        if let Some(command) = line.strip_prefix('/') {
            return self.console_command(command);
        }
        if let Some(command) = self.bare_command_line(line) {
            return self.console_command(&command);
        }
        let out = match self.mode {
            Mode::Edit => self.edit_input(line),
            Mode::Play => self.play_input(line),
        };
        (out, Flow::Continue)
    }

    fn console_command(&mut self, command: &str) -> (String, Flow) {
        let words = split_args(command);
        let typed = words.first().map(String::as_str).unwrap_or("");
        let rest: Vec<&str> = words.iter().skip(1).map(String::as_str).collect();
        if typed.is_empty() {
            return (self.t("type_help", &[]), Flow::Continue);
        }
        // Name, alias, then a unique prefix, from the registry (REQ-CON-56).
        let def = match registry::lookup(typed) {
            registry::Lookup::Found(def) => def,
            registry::Lookup::Ambiguous(list) => {
                let names: Vec<String> = list.iter().map(|c| format!("/{}", c.name)).collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                return (suggest(&self.t("command_could_be", &[("command", typed)]), &names), Flow::Continue);
            }
            registry::Lookup::Unknown(near) => {
                self.log_unknown("command", &format!("/{}", command));
                let mut out = self.t("unknown_command", &[("command", typed)]);
                if !near.is_empty() {
                    let names: Vec<String> = near.iter().map(|c| c.usage()).collect();
                    let names: Vec<&str> = names.iter().map(String::as_str).collect();
                    out = suggest(&format!("{} {}", out, self.t("did_you_mean", &[])), &names);
                }
                return (out, Flow::Continue);
            }
        };
        if def.edit_only() && self.mode != Mode::Edit {
            return (self.t("edit_only_command", &[("command", &def.name)]), Flow::Continue);
        }
        let name = def.handler.as_str();
        let out = match (name, rest.as_slice()) {
            ("quit", _) => return (self.t("bye", &[]), Flow::Quit),
            ("help", []) => self.help(),
            ("help", [one]) => self.help_for(one),
            ("clear", []) if self.interactive => return (String::new(), Flow::Clear),
            ("clear", []) => self.t("clear_needs_terminal", &[]),
            ("status", []) => self.status_report(),
            ("validate", []) => self.validate(false),
            ("validate", ["strict"]) => self.validate(true),
            ("map", []) => self.map_command(),
            ("list", [kind]) => self.list_command(kind),
            ("scope", [word]) => self.scope_command(word),
            ("inspect", args) => self.inspect(args),
            ("remove", args) => self.remove(args),
            ("write", args) => self.write_command(args),
            ("start", []) => self.start_command(),
            ("restart", []) if self.interactive => return (String::new(), Flow::Restart),
            ("restart", []) => self.t("restart_needs_terminal", &[]),
            ("edit", args) => self.full_edit(args),
            ("play", []) => self.play(),
            ("play", ["here"]) => self.play_here(),
            ("save", args) => self.save_game_as(args),
            ("load", args) => self.load_game_named(args),
            ("reload", _) => self.reload(),
            ("new", args) => self.new_and_edit(args),
            ("set", args) if matches!(self.place, Place::Game(_)) => {
                let Place::Game(sub) = self.place.clone() else { unreachable!() };
                self.game_set(&sub, args)
            }
            ("set", args) if args.len() >= 4 && args[args.len() - 2] == "in" => self.set_in_state(args),
            ("set", args) => self.set(args),
            ("state", args) => self.state_command(args),
            ("action", args) => self.action_command(args),
            ("recipe", args) => self.recipe_command(args),
            ("link", args) => self.link(args),
            ("unlink", args) => self.unlink(args),
            ("move", args) => self.move_thing(args),
            ("add", args) => self.add_and_edit(args),
            ("menu", []) => self.menu_command(),
            // Arguments the command doesn't take: its usage, from its file.
            _ if HANDLERS.contains(&name) => suggest(&self.t("usage", &[]), &[&def.usage()]),
            _ => self.t("no_handler", &[("command", &def.name)]),
        };
        (out, Flow::Continue)
    }

    /// `/help <command>`: one command in full, from its file.
    fn help_for(&self, typed: &str) -> String {
        let typed = typed.trim_start_matches('/');
        match registry::lookup(typed) {
            registry::Lookup::Found(c) => {
                let mut out = vec![c.usage()];
                if !c.aliases.is_empty() {
                    out.push(self.t("also_aliases", &[("aliases", &c.aliases.iter().map(|a| format!("/{}", a)).collect::<Vec<_>>().join(", "))]));
                }
                if c.edit_only() {
                    out.push(self.t("edit_mode_only", &[]));
                }
                out.push(String::new());
                out.push(c.help.clone());
                out.join("\n")
            }
            _ => self.t("unknown_command", &[("command", typed)]),
        }
    }

    fn help(&self) -> String {
        // Every command from the registry, so the list can't drift from the
        // menu (REQ-CON-50).
        let usable: Vec<&registry::CommandDef> =
            registry::commands().iter().filter(|c| self.mode == Mode::Edit || !c.edit_only()).collect();
        let width = usable.iter().map(|c| c.usage().chars().count()).max().unwrap_or(0).min(40);
        let mut lines = vec![self.t("help_heading", &[])];
        for c in usable {
            lines.push(format!("  {:<width$}  {}", c.usage(), c.about, width = width));
        }
        match self.mode {
            Mode::Edit => lines.extend([
                String::new(),
                self.t("help_edit_heading", &[]),
                self.t("help_edit_rows", &[]),
            ]),
            Mode::Play => lines.extend([
                String::new(),
                self.t("help_play_heading", &[]),
                self.t("help_play_rows", &[]),
            ]),
        }
        // The console has the mouse for sidebar clicks (REQ-CON-70).
        lines.extend([String::new(), self.t("help_mouse", &[])]);
        lines.join("\n")
    }

    /// Reloads from disk, keeping the game and edit mode's place
    /// (REQ-CON-7). Keeps the old world if a file fails to parse or the
    /// player's room is gone.
    pub(crate) fn reload(&mut self) -> String {
        let fresh = match World::load_with(&self.world.root, self.texts.clone()) {
            Ok(world) => world,
            Err(e) => return self.t("reload_failed_because", &[("reason", &e.to_string())]),
        };
        let mut blockers: Vec<String> = fresh.blocking_problems();
        if let Some(game) = &self.game {
            if fresh.engine.get_location(&game.location).is_none() {
                blockers.push(self.t("player_room_gone", &[("room", &game.location)]));
            }
        }
        if !blockers.is_empty() {
            let mut out = vec![self.t("reload_failed", &[])];
            out.extend(blockers.iter().map(|p| format!("  {}", p)));
            return out.join("\n");
        }

        self.world = fresh;
        let mut out = vec![self.t(
            "reloaded",
            &[
                ("rooms", &self.world.report.locations_loaded.to_string()),
                ("characters", &self.world.engine.characters.len().to_string()),
                ("objects", &self.world.report.objects_loaded.to_string()),
            ],
        )];
        if !self.place_exists() {
            let gone = self.pwd();
            self.move_to_existing_folder();
            out.push(self.t("place_gone", &[("place", &gone), ("now", &self.pwd())]));
        }
        for warning in self.reconcile_game() {
            out.push(self.t("game_note", &[("note", &warning)]));
        }
        let warnings = self.world.warnings();
        if !warnings.is_empty() {
            out.push(self.t("reload_warnings", &[("count", &warnings.len().to_string())]));
            out.extend(warnings.iter().map(|w| format!("  {}", w)));
        }
        out.join("\n")
    }

    fn problem_summary(&self) -> Vec<String> {
        let mut out = Vec::new();
        let errors = self.world.blocking_problems();
        if !errors.is_empty() {
            out.push(self.t("files_dont_parse", &[("count", &errors.len().to_string())]));
            out.extend(errors.iter().map(|e| format!("  {}", e)));
        }
        let warnings = self.world.warnings();
        if !warnings.is_empty() {
            out.push(self.t("warnings_summary", &[("count", &warnings.len().to_string())]));
        }
        out
    }

    // Edit-mode navigation (REQ-CON-4).

    /// Edit mode moves with play mode's words (REQ-CON-4): the author
    /// walks the world like a game master, not a file system.
    fn edit_input(&mut self, line: &str) -> String {
        let first = line.split_whitespace().next().unwrap_or("");
        if matches!(first, "cd" | "ls" | "pwd") {
            return self.t("shell_word_gone", &[("word", first)]);
        }
        // The same words as play (`design/PLAYER_COMMANDS.md`), read by the
        // engine's parser with no game, since edit mode only needs them.
        let parsed = self.game_engine.parse_command(&self.world.engine, line, &PlayState::default());
        match parsed {
            Parsed::Action(PlayAction::Look) => self.look(),
            Parsed::Action(PlayAction::Examine(noun)) => self.examine(&noun),
            Parsed::Action(PlayAction::Go(target)) => self.go(&target),
            Parsed::Action(PlayAction::Where) if self.pwd() == "/" => self.t("edit_where_root", &[]),
            Parsed::Action(PlayAction::Where) => self.t("edit_where", &[("place", &self.pwd())]),
            Parsed::Action(PlayAction::LookToward(direction)) => self.look_toward(&direction),
            // A play verb that is also a command's name (`save`): ask
            // which was meant, since play's verb doesn't work here
            // (REQ-CON-56).
            Parsed::Action(_) if bare_command(first, false).is_some() => {
                let name = bare_command(first, false).unwrap_or_default();
                suggest(&self.t("bare_command_ask", &[("word", first)]), &[&format!("/{}", line.replacen(first, &name, 1))])
            }
            Parsed::Action(_) => {
                suggest(&self.t("play_command_in_edit", &[]), &["/new {kind} {id}", "/set {key} {value}", "/play here"])
            }
            Parsed::Reply(question) => question,
            Parsed::Unknown(_) => {
                self.log_unknown("edit", line);
                self.t("edit_unknown", &[("word", first)])
            }
        }
    }

    /// In edit mode, a command's name or alias typed without its slash,
    /// when the play words don't read it, as the command to run
    /// (REQ-CON-56).
    fn bare_command_line(&self, line: &str) -> Option<String> {
        let first = line.split_whitespace().next()?;
        if self.mode != Mode::Edit || matches!(first, "cd" | "ls" | "pwd") {
            return None;
        }
        let name = bare_command(first, true)?;
        let parsed = self.game_engine.parse_command(&self.world.engine, line, &PlayState::default());
        if !matches!(parsed, Parsed::Unknown(_)) {
            return None;
        }
        let rest = line[first.len()..].trim_start();
        Some(if rest.is_empty() { name } else { format!("{} {}", name, rest) })
    }

    /// `look <direction>` in edit mode: where that exit goes, by id, and
    /// whether it's written (REQ-PC-23).
    fn look_toward(&self, direction: &str) -> String {
        let room = match &self.place {
            Place::File(path) => self.world.files.get(path).filter(|e| e.kind == Kind::Room).map(|e| e.id.clone()),
            Place::Dir(_) | Place::Game(_) => None,
        };
        let Some(room) = room else { return self.t("edit_look_toward_no_room", &[]) };
        let way: String = direction.chars().take(1).flat_map(char::to_uppercase).chain(direction.chars().skip(1)).collect();
        match self.world.engine.get_connections(&room).into_iter().find(|(d, _)| d == direction) {
            Some((_, target)) if self.world.engine.get_location(&target).is_some() => {
                self.t("edit_look_toward", &[("direction", &way), ("target", &target)])
            }
            Some((_, target)) => self.t("edit_look_toward_unwritten", &[("direction", &way), ("target", &target)]),
            None => self.t("edit_look_toward_none", &[("direction", direction)]),
        }
    }

    /// `go <direction or name>`, `go back` and `go /`.
    fn go(&mut self, target: &str) -> String {
        let target = target.trim();
        self.looked_at = None;
        if target == "back" {
            return match self.trail.pop() {
                Some(place) => {
                    self.place = place;
                    self.arrival()
                }
                None => self.t("no_way_back", &[]),
            };
        }
        let place = match self.game_place(target) {
            Some(found) => found,
            None => self.resolve(target),
        };
        match place {
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
            Place::Game(sub) => self.game_look(sub),
            Place::File(path) => match self.world.files.get(path) {
                Some(entry) if entry.kind == Kind::Room => self.t("arrival_room", &[("title", &entry.title)]),
                Some(entry) if entry.kind == Kind::Broken => self.t("arrival_broken", &[("path", path)]),
                Some(entry) => format!("{} ({})", entry.title, entry.kind.label()),
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
                    return self.t("folder_empty", &[("place", &self.pwd())]);
                }
                // One whole line per case, so each can be reworded.
                let noun = if folders.is_empty() && files.iter().all(|(_, k)| *k == Kind::Room) {
                    "rooms"
                } else if folders.is_empty() && files.iter().all(|(_, k)| matches!(k, Kind::Object | Kind::Character)) {
                    "things"
                } else {
                    "items"
                };
                let key = format!("folder_{}{}", noun, if names.len() == 1 { "_one" } else { "" });
                self.t(&key, &[("place", &self.pwd()), ("count", &names.len().to_string()), ("names", &names.join(", "))])
            }
        }
    }

    /// `examine <thing>`: a person or thing here, shown without going to it.
    fn examine(&mut self, noun: &str) -> String {
        match self.resolve_here(noun) {
            Some(Ok(Place::File(path))) => {
                let view = self.view_of(&path);
                // The sidebar shows its file until edit mode moves on.
                self.looked_at = Some(path);
                view
            }
            Some(Ok(Place::Dir(_) | Place::Game(_))) | None => {
                self.t("nothing_here_to_examine", &[("name", noun)])
            }
            Some(Err(message)) => message,
        }
    }

    /// `/validate [strict]`: the world checked as `thwart validate` checks
    /// it, edits made in the console included (REQ-ENG-44).
    fn validate(&self, strict: bool) -> String {
        let check = self.world.engine.validation(strict);
        let mut out = Vec::new();
        if !check.problems.is_empty() {
            out.push(self.t("problems_heading", &[("count", &check.problems.len().to_string())]));
            out.extend(check.problems.iter().map(|p| format!("  {}", p)));
        }
        if !check.warnings.is_empty() {
            out.push(self.t("warnings_heading", &[("count", &check.warnings.len().to_string())]));
            out.extend(check.warnings.iter().map(|w| format!("  {}", w)));
        }
        out.push(check.summary());
        out.join("\n")
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
            Place::Game(sub) if sub.is_empty() => "game/".to_string(),
            Place::Game(sub) => format!("game/{}", sub),
        }
    }

    fn list_dir(&self, dir: &str) -> String {
        let prefix = if dir.is_empty() { String::new() } else { format!("{}/", dir) };
        let child = |path: &str| -> Option<String> {
            let rest = path.strip_prefix(&prefix)?;
            (!rest.is_empty() && !rest.contains('/')).then(|| rest.to_string())
        };

        let mut rows: Vec<(String, String)> = Vec::new();
        let folders: Vec<String> = self.world.dirs.iter().filter_map(|d| child(d)).collect();
        // At the root, the rooms come first, then areas, then the world's
        // own folders, labelled for what they hold (Bruce, 2026-11-24).
        if dir.is_empty() {
            return self.list_root(&folders);
        }
        for sub in &folders {
            let full = format!("{}{}", prefix, sub);
            let count = self.world.files.keys().filter(|p| p.starts_with(&format!("{}/", full))).count();
            rows.push((format!("{}/", sub), self.t("folder_files", &[("count", &count.to_string())])));
        }
        for (path, entry) in &self.world.files {
            if let Some(name) = child(path) {
                let note = if entry.kind == Kind::Broken {
                    self.t("doesnt_parse", &[])
                } else {
                    format!("{}  ({})", entry.title, entry.kind.label())
                };
                rows.push((name, note));
            }
        }
        if rows.is_empty() {
            return self.t("nothing_here", &[]);
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
            (Mode::Edit, Place::Game(sub)) => self.game_look(sub),
        }
    }

    /// The world's root: its rooms, then its areas, then its own folders
    /// with what they hold: "World settings", "Global objects: 4".
    fn list_root(&self, folders: &[String]) -> String {
        let in_folder = |folder: &str, kind: Option<Kind>| {
            self.world
                .files
                .iter()
                .filter(|(p, e)| p.starts_with(&format!("{}/", folder)) && kind.is_none_or(|k| e.kind == k))
                .count()
        };
        let mut rows: Vec<(String, String)> = Vec::new();
        for (path, entry) in self.world.files.iter().filter(|(p, _)| !p.contains('/')) {
            let note = if entry.kind == Kind::Broken { self.t("doesnt_parse", &[]) } else { format!("{}  ({})", entry.title, entry.kind.label()) };
            rows.push((path.clone(), note));
        }
        const OWN: &[&str] = &["objects", "npcs", "doors", "world", "questions", "templates", "chapters", "game"];
        for folder in folders.iter().filter(|f| !OWN.contains(&f.as_str())) {
            rows.push((format!("{}/", folder), self.t("root_area", &[("count", &in_folder(folder, Some(Kind::Room)).to_string())])));
        }
        for folder in folders.iter().filter(|f| OWN.contains(&f.as_str())) {
            let label = match folder.as_str() {
                "objects" => self.t("root_objects", &[("count", &in_folder(folder, Some(Kind::Object)).to_string())]),
                "npcs" => self.t("root_characters", &[("count", &in_folder(folder, Some(Kind::Character)).to_string())]),
                "doors" => self.t("root_doors", &[("count", &in_folder(folder, Some(Kind::Door)).to_string())]),
                "world" => self.t("root_world", &[]),
                "questions" => self.t("root_questions", &[("count", &in_folder(folder, None).to_string())]),
                "templates" => self.t("root_templates", &[("count", &in_folder(folder, None).to_string())]),
                "chapters" => self.t("root_chapters", &[("count", &in_folder(folder, None).to_string())]),
                _ => self.t("folder_files", &[("count", &in_folder(folder, None).to_string())]),
            };
            rows.push((format!("{}/", folder), label));
        }
        if rows.is_empty() {
            return self.t("nothing_here", &[]);
        }
        let width = rows.iter().map(|(n, _)| n.chars().count()).max().unwrap_or(0);
        rows.iter().map(|(name, note)| format!("{:<width$}  {}", name, note, width = width)).collect::<Vec<_>>().join("\n")
    }

    /// The edit-mode view of the content file at `path`.
    fn view_of(&self, path: &str) -> String {
        let path = path.to_string();
        let Some(entry) = self.world.files.get(&path) else {
            return self.t("not_in_world", &[("id", &path)]);
        };
        match entry.kind {
            Kind::Room => match self.world.engine.describe_location(&entry.id) {
                Some(view) => crate::render::render_location_with(&view, &self.room_texts()),
                None => self.t("not_in_world", &[("id", &entry.id)]),
            },
            Kind::Character => match self.world.engine.characters.get(&entry.id) {
                Some(c) => view::character(c, &self.texts),
                None => self.t("not_in_world", &[("id", &entry.id)]),
            },
            Kind::Object => match self.world.engine.objects.get(&entry.id) {
                Some(o) => view::object(o, &self.texts),
                None => self.t("not_in_world", &[("id", &entry.id)]),
            },
            Kind::World => view::world(self.world.engine.world(), &path, &self.texts),
            Kind::Broken => {
                let reason = if entry.title.is_empty() { self.t("it_doesnt_parse", &[]) } else { entry.title.clone() };
                self.t("not_part_of_world", &[("path", &path), ("reason", &reason)])
            }
            // A part of a thing or of the world: shown as written, until
            // bundle editing shows it in place (`design/STATES.md` › Console).
            _ => {
                let text = std::fs::read_to_string(self.world.root.join(format!("{}.md", path))).unwrap_or_default();
                self.t("part_as_written", &[("title", &entry.title), ("kind", entry.kind.label()), ("path", &path), ("text", text.trim_end())])
            }
        }
    }

    /// Finds the place `target` names, trying in turn: `/`; an exit, person
    /// or thing here (`resolve_here`); a path from this folder or the world
    /// root; then a character, object, room, area or folder anywhere, by id
    /// or name. A name that fits several places is a question.
    pub(super) fn resolve(&self, target: &str) -> Result<Place, String> {
        let target = target.trim();
        if target == "/" {
            return Ok(Place::Dir(String::new()));
        }
        if let Some(found) = self.resolve_here(target) {
            return found;
        }
        if DIRECTIONS.contains(&target) && !matches!(self.current_kind(), Some(Kind::Room)) {
            return Err(self.t("no_exits_here", &[("place", &self.pwd())]));
        }

        let trimmed = target.trim_end_matches('/');
        let here = self.current_dir();
        let relative = if here.is_empty() { trimmed.to_string() } else { format!("{}/{}", here, trimmed) };
        let absolute = trimmed.trim_start_matches('/').to_string();
        for candidate in [relative, absolute] {
            if self.world.files.contains_key(&candidate) {
                return Ok(Place::File(candidate));
            }
            // A bundle's folder is the thing: going there lands on its index.
            let index = format!("{}/index", candidate);
            if self.world.files.get(&index).is_some_and(|e| e.kind.is_thing()) {
                return Ok(Place::File(index));
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
            0 => Err(self.t("nothing_called", &[("name", target)])),
            1 => Ok(found.remove(0)),
            _ => {
                let names: Vec<String> = found
                    .iter()
                    .map(|p| match p {
                        Place::File(path) => path.clone(),
                        Place::Dir(dir) => format!("{}/", dir),
                        Place::Game(sub) => format!("game/{}", sub),
                    })
                    .collect();
                Err(self.t("which_do_you_mean", &[("names", &names.join(", "))]))
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
                    None => return Some(Err(self.t("door_leads_nowhere", &[("direction", direction), ("door", to)]))),
                },
                None => to.clone(),
            };
            let to = &to;
            return Some(match self.world.path_of(Kind::Room, to) {
                Some(path) => Ok(Place::File(path)),
                None => Err(self.t("exit_not_written", &[("direction", direction), ("room", to)])),
            });
        }
        if location.characters.iter().any(|c| c == target) {
            return Some(
                self.world
                    .path_of(Kind::Character, target)
                    .map(Place::File)
                    .ok_or_else(|| self.t("no_character_card", &[("id", target)])),
            );
        }
        if location.objects.iter().any(|o| o == target) {
            return Some(self.world.path_of(Kind::Object, target).map(Place::File).ok_or_else(|| {
                suggest(&self.t("scenery_no_card", &[("id", target)]), &[&format!("/edit object {}", target)])
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
            Place::Game(_) => String::new(),
        }
    }

    fn parent_dir(&self) -> String {
        match &self.place {
            Place::Dir(dir) => parent_of(dir),
            Place::File(path) => parent_of(path),
            Place::Game(_) => String::new(),
        }
    }

    pub(super) fn place_exists(&self) -> bool {
        match &self.place {
            Place::Dir(dir) => self.world.dirs.contains(dir),
            Place::File(path) => self.world.files.contains_key(path),
            Place::Game(_) => true,
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

/// A command's words, with `"..."` or `'...'` keeping spaces inside one
/// word: `/add "old shirt" bedroom` is `add`, `old shirt`, `bedroom`. An
/// unclosed quote runs to the end of the line.
pub(super) fn split_args(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    let mut in_word = false;
    for c in command.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                in_word = true;
            }
            (None, c) if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            (None, c) => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(word);
    }
    words
}

/// The command `word` names exactly, by name or (with `aliases`) by
/// alias, for a command typed without its slash. Prefixes don't count:
/// a bare word is too easily something else.
fn bare_command(word: &str, aliases: bool) -> Option<String> {
    let word = word.to_lowercase();
    registry::commands()
        .iter()
        .find(|c| c.name == word || (aliases && c.aliases.iter().any(|a| *a == word)))
        .map(|c| c.name.clone())
}

/// A reply that suggests what to type: the sentence, a blank line, then
/// each command on its own indented line (Bruce, 2026-11-24: examples go
/// on a new line with space above).
pub(super) fn suggest(text: &str, commands: &[&str]) -> String {
    let mut out = text.to_string();
    out.push_str("\n");
    for command in commands {
        out.push_str(&format!("\n  {}", command));
    }
    out
}

/// The `[console]` text table: the built-in defaults, with the author's
/// `[console]` from `config.toml` laid over them, and a warning for each
/// problem in it. A problem keeps the default for that key.
pub(super) fn console_texts() -> (crate::engines::texts::Texts, Vec<String>) {
    with_personal(crate::engines::texts::Texts::defaults())
}

/// `texts` with the author's own `[console]` from `config.toml` over it.
fn with_personal(texts: crate::engines::texts::Texts) -> (crate::engines::texts::Texts, Vec<String>) {
    let Some(path) = crate::settings::config_dir().map(|d| d.join("config.toml")) else { return (texts, Vec::new()) };
    let Ok(text) = std::fs::read_to_string(&path) else { return (texts, Vec::new()) };
    personal_over(texts, &text, &path.display().to_string())
}

/// `config` (a config.toml's text) laid over `texts`. `[console.commands]`
/// is the command layer's (REQ-CON-52), so it isn't a text key.
fn personal_over(mut texts: crate::engines::texts::Texts, config: &str, source: &str) -> (crate::engines::texts::Texts, Vec<String>) {
    let mut value = match config.parse::<toml::Value>() {
        Ok(value) => value,
        // settings.rs reports a config.toml that doesn't parse.
        Err(_) => return (texts, Vec::new()),
    };
    let mut warnings = Vec::new();
    if let Some(table) = value.get_mut("console").and_then(|v| v.as_table_mut()) {
        table.remove("commands");
    }
    if value.get("console").is_some() {
        texts.apply("console", value.get("console"), source, &mut warnings);
    }
    (texts, warnings)
}

impl Console {
    /// The texts edit mode's room view uses: the world's `[play]` and
    /// `[engine]`, with the author's `[console]` over them, so the world's
    /// wording shows while it's being written (Bruce, 2026-11-24).
    pub(super) fn room_texts(&self) -> crate::engines::texts::Texts {
        with_personal(self.world.engine.texts().clone()).0
    }

    /// A line of the console's own text, from `[console]` (REQ-ENG-49).
    pub(super) fn t(&self, key: &str, args: &[(&str, &str)]) -> String {
        if let Some(kind) = super::kinds::kind_of(key) {
            self.said.borrow_mut().push(kind);
        }
        self.texts.text("console", key, args)
    }
}

fn parent_of(path: &str) -> String {
    path.rsplit_once('/').map(|(parent, _)| parent.to_string()).unwrap_or_default()
}


/// Every handler `console_command` dispatches to. Each registry entry's
/// `handler` MUST be here and each name here MUST have an entry (REQ-CON-53);
/// a test fails on either gap.
const HANDLERS: &[&str] = &[
    "quit", "help", "clear", "status", "validate", "map", "list", "scope", "inspect", "remove", "write", "start", "restart", "edit", "play", "save",
    "load", "reload", "new", "set", "state", "action", "recipe", "link", "unlink", "move", "add", "menu",
];

#[cfg(test)]
mod tests {
    //! `/reload` cases a transcript can't reach, because they change
    //! files while the console runs (REQ-CON-7).
    use super::*;

    #[test]
    fn the_authors_console_table_rewords_and_warns_about_unknown_keys() {
        let config = "[console]\ntype_help = \"Try /help.\"\ncolour = \"red\"\n[console.commands.quit]\naliases = [\"q\"]\n";
        let (texts, warnings) = personal_over(crate::engines::texts::Texts::defaults(), config, "config.toml");
        assert_eq!(texts.text("console", "type_help", &[]), "Try /help.");
        assert_eq!(warnings.len(), 1, "{:?}", warnings);
        assert!(warnings[0].contains("`[console] colour` isn't a console setting"), "{:?}", warnings);
        let (_, none) = personal_over(crate::engines::texts::Texts::defaults(), "[world]\npath = \"x\"\n", "config.toml");
        assert!(none.is_empty(), "no [console], nothing to say: {:?}", none);
    }

    #[test]
    fn every_registry_command_has_a_handler_and_every_handler_a_command() {
        for c in registry::commands() {
            assert!(HANDLERS.contains(&c.handler.as_str()), "commands/{}.md names handler `{}`, which isn't dispatched", c.name, c.handler);
        }
        for h in HANDLERS {
            assert!(registry::commands().iter().any(|c| c.handler == *h), "handler `{}` has no command file", h);
        }
    }
    use std::fs;
    use walkdir::WalkDir;

    #[test]
    fn quotes_keep_spaces_inside_one_word() {
        assert_eq!(split_args(r#"add "old shirt" bedroom"#), vec!["add", "old shirt", "bedroom"]);
        assert_eq!(split_args("set title 'The Old Well'"), vec!["set", "title", "The Old Well"]);
        assert_eq!(split_args(r#"add "unclosed thing"#), vec!["add", "unclosed thing"]);
        assert_eq!(split_args("  move   lantern  well "), vec!["move", "lantern", "well"]);
        assert_eq!(split_args(r#"set body """#), vec!["set", "body", ""]);
    }

    #[test]
    fn unknown_input_is_logged_with_where_it_was_typed() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("state/thwarter/unknown-commands.log");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        let mut console = Console::open(&root, Mode::Edit).unwrap();
        console.unknown_log = Some(log.clone());
        console.handle("go /village/square");
        console.handle("/bogus thing");
        console.handle("dance");
        console.handle("/play");
        // An object's action is understood, so it isn't logged.
        assert!(!console.handle("light the lantern").0.starts_with("I don't understand"));
        console.handle("dance wildly");
        console.handle("look");
        let text = fs::read_to_string(&log).unwrap();
        let rows: Vec<Vec<&str>> = text.lines().map(|l| l.split('\t').skip(2).collect()).collect();
        assert_eq!(
            rows,
            vec![
                vec!["edit", "village/square", "command", "/bogus thing"],
                vec!["edit", "village/square", "edit", "dance"],
                vec!["play", "village/square", "play", "dance wildly"],
            ]
        );
    }

    const FIXTURE: &str = "tests/worlds/tiny";

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
        assert_eq!(console.world.files["village/well"].title, "The Old Well", "the new title is loaded");
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
        assert!(console.handle("look").0.starts_with("A mossy stone well."), "old world still loaded");
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
