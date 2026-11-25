//! What the full-screen console shows around the output: the status bar
//! and the sidebar's GAME and HERE panels (REQ-CON-22, 23). Worked out
//! here as plain rows from the console's state, so it can be tested
//! without a terminal; `screen.rs` draws them.

use super::session::{Console, Place};
use super::world::Kind;
use super::Mode;

/// One row of a panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Heading(String),
    Text(String),
    /// Something to notice but not act on, such as no game yet.
    Dim(String),
    Warning(String),
    Error(String),
    /// A row of the map, drawn as it is: cut at the sidebar's edge, never
    /// wrapped.
    Map(String),
    /// An exit: one line, cut with an ellipsis at the edge, never wrapped
    /// (Bruce, 2026-11-24). `true` when its room isn't written yet.
    Exit(String, bool),
    Blank,
}

/// The sidebar: GAME's first rows stay put, everything after them
/// scrolls when the sidebar is short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidebar {
    pub fixed: Vec<Row>,
    pub rest: Vec<Row>,
    /// The world's problems and warnings, kept at the sidebar's foot under
    /// a line, as an IDE keeps its problems pane (Bruce, 2026-11-23).
    pub warnings: Vec<Row>,
}

/// The status bar's two ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub left: String,
    pub right: String,
}

impl Console {
    /// `Tiny World · edit · 1 warning · chapter arrival · turn 3`, with
    /// the build on the right.
    pub(super) fn status(&self, newer_build: bool) -> Status {
        let warnings = self.world.warnings().len();
        let mut left = vec![
            self.world.display_name(),
            self.mode.name().to_string(),
            self.counted("warning", warnings),
        ];
        if let Some(game) = &self.game {
            let chapter = game.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_else(|| self.t("no_chapter", &[]));
            left.push(self.t("bar_chapter", &[("chapter", &chapter)]));
            left.push(self.t("bar_turn", &[("turn", &game.turn.to_string())]));
        }
        let mut right = self.t("bar_build", &[("version", env!("CARGO_PKG_VERSION"))]);
        if newer_build {
            right = self.t("bar_newer_build", &[("build", &right)]);
        }
        Status { left: left.join(" · "), right }
    }

    /// Where this world's command history is kept: beside its saves, under
    /// the same key (`<state>/history/<world>-<hash>.txt`, or
    /// `<persona>/<world>.txt` for an installed world).
    pub(super) fn history_file(&self) -> Option<std::path::PathBuf> {
        let state = crate::settings::state_dir()?;
        let saves = crate::engines::saves::save_dir(&self.world.root, &state);
        let key = saves.strip_prefix(state.join("saves")).ok()?.to_path_buf();
        Some(state.join("history").join(key).with_extension("txt"))
    }

    /// `/status`: the world, where you are, the game, saves and the build.
    pub(super) fn status_report(&self) -> String {
        let mut out = vec![self.t("status_world", &[("world", &self.world.display_name()), ("path", &self.world.root.display().to_string())])];
        out.push(self.t("status_mode", &[("mode", self.mode.name()), ("place", &self.pwd())]));
        out.push(match &self.game {
            Some(game) => {
                let chapter = game.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_else(|| self.t("no_chapter", &[]));
                self.t(
                    "status_game",
                    &[
                        ("room", &game.location),
                        ("chapter", &chapter),
                        ("turn", &game.turn.to_string()),
                        ("things", &self.counted("thing", game.inventory.len())),
                    ],
                )
            }
            None => self.t("status_no_game", &[]),
        });
        let saves = crate::engines::list_saves(&self.world.root).unwrap_or_default();
        out.push(if saves.is_empty() { self.t("status_no_saves", &[]) } else { self.t("status_saves", &[("saves", &saves.join(", "))]) });
        let broken = self.world.files.values().filter(|e| e.kind == Kind::Broken).count();
        let warnings = self.world.warnings().len();
        out.push(self.t("status_problems", &[("files", &self.counted("file", broken)), ("warnings", &self.counted("warning", warnings))]));
        out.push(self.t("status_build", &[("version", env!("CARGO_PKG_VERSION"))]));
        if let Some(log) = &self.unknown_log {
            let lines = std::fs::read_to_string(log).map(|t| t.lines().count()).unwrap_or(0);
            out.push(self.t("status_unknown_input", &[("log", &log.display().to_string()), ("count", &lines.to_string())]));
        }
        out.join("\n")
    }

    /// GAME, then MAP, then HERE in edit mode.
    pub(super) fn sidebar(&self) -> Sidebar {
        let (fixed, mut rest) = self.game_panel();
        let map = self.map_panel();
        if !map.is_empty() {
            rest.push(Row::Blank);
            rest.extend(map);
        }
        if self.mode == Mode::Edit {
            rest.push(Row::Blank);
            rest.extend(self.here_panel());
        } else if let Some(game) = &self.game {
            // Play's text is the player's; the detail an author needs goes
            // here instead (Bruce, 2026-11-23).
            rest.push(Row::Blank);
            let title = self.world.engine.get_location(&game.location).map(|l| l.name.clone()).unwrap_or_default();
            rest.push(Row::Heading(self.t("side_room", &[("title", &title), ("id", &game.location)])));
            rest.extend(self.room_rows(&game.location));
            if !self.notes.is_empty() {
                rest.push(Row::Heading(self.t("side_author_notes", &[])));
                rest.extend(self.notes.iter().map(|n| Row::Warning(format!("  {}", n))));
            }
        }
        if self.mode == Mode::Edit {
            let to_write = self.to_write_panel();
            if !to_write.is_empty() {
                rest.push(Row::Blank);
                rest.extend(to_write);
            }
        }
        Sidebar { fixed, rest, warnings: self.warnings_panel() }
    }

    /// The foot of the sidebar: every problem, then every warning.
    fn warnings_panel(&self) -> Vec<Row> {
        let check = self.world.engine.validation(false);
        // In edit mode an exit to a room not written yet is a TO WRITE
        // entry, not a problem (Bruce, 2026-11-24); validate and package
        // still fail on it.
        let rooms_to_write: Vec<String> = if self.mode == Mode::Edit {
            check.not_written.iter().filter(|u| u.wants == crate::engines::EntityKind::Location).map(|u| format!("unknown location `{}`", u.name)).collect()
        } else {
            Vec::new()
        };
        let problems: Vec<&String> = check.problems.iter().filter(|p| !rooms_to_write.iter().any(|r| p.ends_with(r.as_str()))).collect();
        let mut rows = vec![Row::Heading(self.t("side_problems", &[("problems", &problems.len().to_string()), ("warnings", &check.warnings.len().to_string())]))];
        rows.extend(problems.iter().map(|p| Row::Error(format!("  {}", p))));
        rows.extend(check.warnings.iter().map(|w| Row::Warning(format!("  {}", w))));
        rows
    }

    /// The room the map is drawn around: edit mode's room, or the player's.
    fn map_room(&self) -> Option<String> {
        let edit_room = match (&self.mode, &self.place) {
            (Mode::Edit, Place::File(path)) => self.world.files.get(path).filter(|e| e.kind == Kind::Room).map(|e| e.id.clone()),
            _ => None,
        };
        edit_room.or_else(|| self.game.as_ref().map(|g| g.location.clone()))
    }

    /// MAP: the rooms one exit out from `map_room`, small enough for the
    /// sidebar. It follows you; `/map` shows the whole map in the output.
    fn map_panel(&self) -> Vec<Row> {
        let Some(room) = self.map_room() else { return Vec::new() };
        let map = self.world.engine.map_layout(&room, Some(1), None);
        if map.cells.is_empty() {
            return Vec::new();
        }
        let mut rows = vec![Row::Heading(self.t("side_map", &[("room", &room)]))];
        rows.extend(mini_map(&map).into_iter().map(Row::Map));
        for b in &map.beside {
            let missing = if b.written { "" } else { " ?" };
            let short = |id: &str| id.rsplit('/').next().unwrap_or(id).to_string();
            rows.push(Row::Map(format!("  {}: {} → {}{}", short(&b.room), b.direction, short(&b.target), missing)));
        }
        rows
    }

    /// `/map`: the whole map around you, as `thwart map` draws it.
    pub(super) fn map_command(&self) -> String {
        if self.mode == Mode::Edit && matches!(&self.place, Place::Dir(dir) if dir.is_empty()) {
            return self.whole_map();
        }
        match self.map_room() {
            Some(room) => format!("{}\n\n{}", self.t("map_around", &[("room", &room)]), crate::render::render_map(&self.world.engine.map_layout(&room, None, None))),
            None => self.t("no_room_to_map", &[]),
        }
    }

    /// `/map` at the world's root: every room, starting from where play
    /// starts, then each group of rooms no exit joins to those, by path.
    fn whole_map(&self) -> String {
        let mut rooms: Vec<String> = self.world.files.values().filter(|e| e.kind == Kind::Room).map(|e| e.id.clone()).collect();
        if let Some(start) = self.world.default_room() {
            rooms.retain(|r| *r != start);
            rooms.insert(0, start);
        }
        if rooms.is_empty() {
            return self.t("no_room_to_map", &[]);
        }
        let mut drawn = std::collections::BTreeSet::new();
        let mut parts = Vec::new();
        for room in rooms {
            if drawn.contains(&room) {
                continue;
            }
            let map = self.world.engine.map_layout(&room, None, None);
            drawn.extend(map.cells.iter().map(|c| c.room.clone()));
            drawn.insert(room.clone());
            let heading = if parts.is_empty() { "map_whole" } else { "map_apart" };
            parts.push(format!("{}\n\n{}", self.t(heading, &[("room", &room)]), crate::render::render_map(&map)));
        }
        parts.join("\n\n")
    }

    /// `/list <kind>`: every room, character, object or door, by id.
    pub(super) fn list_command(&self, kind: &str) -> String {
        let engine = &self.world.engine;
        let (key, mut found): (&str, Vec<(String, String)>) = match kind.to_lowercase().as_str() {
            "rooms" | "room" => ("list_rooms", engine.loaded_locations.iter().map(|(id, l)| (id.clone(), l.name.clone())).collect()),
            "characters" | "character" | "people" => ("list_characters", engine.characters.iter().map(|(id, c)| (id.clone(), c.name.clone())).collect()),
            "objects" | "object" | "things" => ("list_objects", engine.objects.iter().map(|(id, o)| (id.clone(), o.title.clone())).collect()),
            "doors" | "door" => ("list_doors", engine.doors.iter().map(|(id, d)| (id.clone(), d.title.clone())).collect()),
            _ => return super::session::suggest(&self.t("list_what", &[("kind", kind)]), &["/list rooms", "/list characters", "/list objects", "/list doors"]),
        };
        if found.is_empty() {
            return self.t(&format!("{}_none", key), &[]);
        }
        found.sort();
        let width = found.iter().map(|(id, _)| id.chars().count()).max().unwrap_or(0);
        let mut out = self.t(key, &[("count", &found.len().to_string())]);
        out.push('\n');
        for (id, title) in found {
            out.push_str(&format!("\n{}", self.t("list_row", &[("id", &format!("{:width$}", id, width = width)), ("title", &title)])));
        }
        out
    }

    /// GAME's fixed rows (where, chapter, carrying), and the rest.
    fn game_panel(&self) -> (Vec<Row>, Vec<Row>) {
        let Some(game) = &self.game else {
            return (
                vec![Row::Heading(self.t("side_game", &[])), Row::Dim(self.t("side_no_game", &[]))],
                Vec::new(),
            );
        };
        let chapter = game.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_else(|| self.t("no_chapter", &[]));
        let carrying: Vec<String> = game.inventory.iter().map(|id| self.object_title(id)).collect();
        let fixed = vec![
            Row::Heading(self.t("side_game", &[])),
            Row::Text(self.t("side_at", &[("room", &game.location)])),
            Row::Text(self.t("side_chapter", &[("chapter", &chapter)])),
            Row::Text(self.t("side_carrying", &[("things", &if carrying.is_empty() { self.t("side_carrying_nothing", &[]) } else { carrying.join(", ") })])),
        ];
        let mut rest = Vec::new();
        let changed: Vec<String> = game
            .states
            .iter()
            .flat_map(|(_, things)| things.iter().map(|(id, state)| format!("  {}: {}", id, state)))
            .collect();
        if !changed.is_empty() {
            rest.push(Row::Blank);
            rest.push(Row::Heading(self.t("side_changed", &[])));
            rest.extend(changed.into_iter().map(Row::Text));
        }
        if !game.values.is_empty() {
            rest.push(Row::Blank);
            rest.push(Row::Heading(self.t("side_values", &[])));
            rest.extend(game.values.iter().map(|(k, v)| Row::Text(format!("  {} = {}", k, v))));
        }
        (fixed, rest)
    }

    /// HERE: a game master's notes for where edit mode is.
    fn here_panel(&self) -> Vec<Row> {
        // A room's title leads its notes, since the room view shows only
        // what the player reads (Bruce, 2026-11-24).
        let heading = match &self.place {
            Place::File(path) => match self.world.files.get(path) {
                Some(entry) if entry.kind == Kind::Room => self.t("side_here_room", &[("title", &entry.title), ("id", &entry.id)]),
                _ => self.t("side_here", &[("place", &self.pwd())]),
            },
            Place::Dir(_) => self.t("side_here", &[("place", &self.pwd())]),
        };
        let mut rows = vec![Row::Heading(heading)];
        let room = match &self.place {
            Place::File(path) => self.world.files.get(path).filter(|e| e.kind == Kind::Room).map(|e| e.id.clone()),
            Place::Dir(_) => None,
        };
        let Some(room) = room else {
            match &self.place {
                Place::File(path) => rows.extend(self.file_rows(path)),
                // A folder: what it holds, folders first.
                Place::Dir(dir) => {
                    let prefix = if dir.is_empty() { String::new() } else { format!("{}/", dir) };
                    let child = |path: &str| -> Option<String> {
                        let rest = path.strip_prefix(&prefix)?;
                        (!rest.is_empty() && !rest.contains('/')).then(|| rest.to_string())
                    };
                    for sub in self.world.dirs.iter().filter_map(|d| child(d)) {
                        rows.push(Row::Text(format!("  {}/", sub)));
                    }
                    for (path, entry) in &self.world.files {
                        if let Some(name) = child(path) {
                            rows.push(match entry.kind {
                                Kind::Broken => Row::Warning(self.t("side_doesnt_load", &[("name", &name)])),
                                kind => Row::Text(format!("  {}  {}", name, kind.label())),
                            });
                        }
                    }
                    if rows.len() == 1 {
                        rows.push(Row::Dim(self.t("side_nothing_here_yet", &[])));
                    }
                }
            }
            return rows;
        };
        rows.extend(self.room_rows(&room));
        rows
    }

    /// A room as a game master's notes: its exits and where they lead,
    /// its people, things and scenery by id, held things under their
    /// holders, and its warnings. HERE shows it for edit mode's room, and
    /// A door's state when it's in the way (closed, locked), in the game
    /// if one is running, else where it starts. An open door is no note.
    fn door_in_the_way(&self, id: &str) -> Option<String> {
        let state = self
            .game
            .as_ref()
            .and_then(|g| g.states.values().find_map(|m| m.get(id)).cloned())
            .or_else(|| self.world.engine.thing(crate::engines::EntityKind::Door, id).and_then(|t| t.starting_state.clone()))?;
        (!matches!(state.as_str(), "open" | "opened" | "ajar")).then_some(state)
    }

    /// ROOM for the player's, so play's own text can stay plain.
    fn room_rows(&self, room: &str) -> Vec<Row> {
        let mut rows = Vec::new();
        let room = room.to_string();
        let Some(location) = self.world.engine.get_location(&room) else { return rows };

        rows.push(Row::Heading(self.t("side_exits", &[])));
        if location.connections.is_empty() {
            rows.push(Row::Dim(self.t("side_no_exits", &[])));
        }
        let mut exits: Vec<(&String, &String)> = location.connections.iter().collect();
        exits.sort();
        // `east -----> bedroom    closed door`: an arrow out to a common
        // column, the room, and a door only when it's in the way (Bruce,
        // 2026-11-24). The door's name and id are /inspect's.
        let column = exits.iter().map(|(d, _)| d.chars().count()).max().unwrap_or(0) + 4;
        let lines: Vec<(String, String, bool)> = exits
            .iter()
            .map(|(direction, to)| {
                let (room, blocked) = match self.world.engine.doors.get(*to) {
                    Some(door) => (door.leads_to(&location.id).unwrap_or("?").to_string(), self.door_in_the_way(to)),
                    None => (to.to_string(), None),
                };
                let written = self.world.engine.get_location(&room).is_some();
                let note = match (written, blocked) {
                    (false, _) => self.t("side_not_written", &[]),
                    (true, Some(state)) => self.t("side_door_state", &[("state", &state)]),
                    (true, None) => String::new(),
                };
                let arrow = format!("{}>", "-".repeat(column - direction.chars().count()));
                (format!("  {} {} {}", direction, arrow, room), note, written)
            })
            .collect();
        let width = lines.iter().map(|(l, _, _)| l.chars().count()).max().unwrap_or(0) + 4;
        for (line, note, written) in lines {
            let text = if note.is_empty() { line } else { format!("{:<width$}{}", line, note, width = width) };
            rows.push(Row::Exit(text, !written));
        }
        // People, things, then scenery: the things built from the built-in
        // `scenery` template, or listed with no card yet (dimmed). Scope
        // shows only when it isn't global, the default (Bruce, 2026-11-23).
        // Each with its state, the game's if one is running (the
        // engine's state_at, 627f899), and its scope when not global.
        let at = self.game.as_ref().map(crate::engines::At::play).unwrap_or_default();
        let named = |id: &String, path: Option<String>| -> Row {
            let kind = if self.world.engine.characters.contains_key(id) { crate::engines::EntityKind::Character } else { crate::engines::EntityKind::Object };
            let state = self.world.engine.state_at(kind, id, at).map(|s| format!("  {}", s)).unwrap_or_default();
            match path.map(|p| self.scope_of(&p)) {
                Some(scope) if scope == "global" => Row::Text(format!("  {}{}", id, state)),
                Some(scope) => Row::Text(format!("  {}{}  {}", id, state, scope)),
                None => Row::Dim(format!("  {}", id)),
            }
        };
        if !location.characters.is_empty() {
            rows.push(Row::Heading(self.t("side_people", &[])));
            for id in &location.characters {
                rows.push(named(id, self.world.path_of(Kind::Character, id)));
            }
        }
        let is_scenery = |id: &String| self.world.engine.objects.get(id).is_none_or(|o| o.object_type == "scenery");
        let (scenery, things): (Vec<&String>, Vec<&String>) = location.objects.iter().partition(|id| is_scenery(id));
        // What an object holds sits under it, indented (REQ-CF-31).
        let with_held = |ids: Vec<&String>| -> Vec<Row> {
            let mut out = Vec::new();
            for id in ids {
                out.push(named(id, self.world.path_of(Kind::Object, id)));
                self.held_rows(id, 2, &mut out, &mut vec![id.clone()]);
            }
            out
        };
        if !things.is_empty() {
            rows.push(Row::Heading(self.t("side_things", &[])));
            rows.extend(with_held(things));
        }
        if !scenery.is_empty() {
            rows.push(Row::Heading(self.t("side_scenery", &[])));
            rows.extend(with_held(scenery));
        }
        if let Some(path) = &self.looked_at {
            rows.push(Row::Heading(self.t("side_looked_at", &[])));
            rows.extend(self.file_rows(path));
        }

        rows
    }

    /// The things `id` holds, each indented under it, and theirs in turn.
    /// `seen` stops a loop of holders, which `validate` reports.
    fn held_rows(&self, id: &str, depth: usize, out: &mut Vec<Row>, seen: &mut Vec<String>) {
        let Some(object) = self.world.engine.objects.get(id) else { return };
        for held in &object.objects {
            if seen.contains(held) {
                continue;
            }
            let pad = "  ".repeat(depth);
            out.push(match self.world.path_of(Kind::Object, held) {
                Some(_) => Row::Text(format!("{}{}", pad, held)),
                None => Row::Dim(format!("{}{}", pad, held)),
            });
            seen.push(held.clone());
            self.held_rows(held, depth + 1, out, seen);
        }
    }

    /// What a file is, for the sidebar rather than the output: its title
    /// and kind, its file, and an object's type.
    fn file_rows(&self, path: &str) -> Vec<Row> {
        let Some(entry) = self.world.files.get(path) else { return Vec::new() };
        let mut rows = vec![Row::Text(format!("  {} ({})", entry.title, entry.kind.label()))];
        rows.push(Row::Dim(format!("  {}.md", path)));
        if entry.kind == Kind::Object {
            if let Some(object) = self.world.engine.objects.get(&entry.id) {
                let mut kind = object.object_type.clone();
                if let Some(category) = &object.category {
                    kind = format!("{}, {}", kind, category);
                }
                rows.push(Row::Dim(format!("  {}", kind)));
            }
        }
        rows
    }

    /// A thing's scope from where its file is (`design/STATES.md` › Scope):
    /// the world's root folders are global, a bundle's folder is local, and
    /// anything else belongs to its area.
    fn scope_of(&self, path: &str) -> String {
        let first = path.split('/').next().unwrap_or("");
        if matches!(first, "objects" | "npcs" | "doors") && path.matches('/').count() == 1 {
            return "global".to_string();
        }
        if matches!(first, "objects" | "npcs" | "doors") && path.ends_with("/index") && path.matches('/').count() == 2 {
            return "global".to_string();
        }
        let dir = path.trim_end_matches("/index");
        let parent = dir.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        if self.world.files.get(&format!("{}/index", parent)).is_some_and(|e| e.kind.is_thing()) {
            return "local".to_string();
        }
        let area = parent.trim_end_matches("/objects").trim_end_matches("/npcs").trim_end_matches("/doors");
        self.t("side_area_scope", &[("area", area)])
    }

    /// "1 warning" or "3 warnings": `noun`'s `[console]` counted forms.
    fn counted(&self, noun: &str, n: usize) -> String {
        let key = if n == 1 { format!("count_{}_one", noun) } else { format!("count_{}_many", noun) };
        self.t(&key, &[("count", &n.to_string())])
    }

    fn object_title(&self, id: &str) -> String {
        self.world.engine.objects.get(id).map(|o| o.title.to_lowercase()).unwrap_or_else(|| id.to_string())
    }
}

/// The map drawn small: each room a short label in brackets (`@` where
/// you are, `?` not written yet), joined by `-`, `|`, `/` and `\\`, with a
/// door as `+` (passable) or `#` (not).
fn mini_map(map: &crate::engines::map::Map) -> Vec<String> {
    const LABEL: usize = 7;
    let label = |c: &crate::engines::map::MapCell| -> String {
        let name = c.room.rsplit('/').next().unwrap_or(&c.room);
        let mark = if c.here { "@" } else if !c.written { "?" } else { "" };
        let mut text: String = format!("{}{}", mark, name);
        if text.chars().count() > LABEL {
            text = text.chars().take(LABEL - 1).collect::<String>() + "…";
        }
        text
    };
    let width = LABEL + 2;
    let pitch = width + 2;
    let (min_x, max_x) = (map.cells.iter().map(|c| c.x).min().unwrap_or(0), map.cells.iter().map(|c| c.x).max().unwrap_or(0));
    let (min_y, max_y) = (map.cells.iter().map(|c| c.y).min().unwrap_or(0), map.cells.iter().map(|c| c.y).max().unwrap_or(0));
    let cols = (max_x - min_x) as usize * pitch + width;
    let mut canvas = vec![vec![' '; cols]; (max_y - min_y) as usize * 2 + 1];
    let at = |x: i32, y: i32| ((x - min_x) as usize * pitch, (y - min_y) as usize * 2);
    let mut put = |col: usize, row: usize, text: &str| {
        for (i, ch) in text.chars().enumerate() {
            if let Some(cell) = canvas.get_mut(row).and_then(|r| r.get_mut(col + i)) {
                *cell = ch;
            }
        }
    };
    for c in &map.cells {
        let (col, row) = at(c.x, c.y);
        let text = label(c);
        put(col, row, &format!("[{:^w$}]", text, w = LABEL));
    }
    for link in &map.links {
        let (a, b) = if (link.from.1, link.from.0) <= (link.to.1, link.to.0) { (link.from, link.to) } else { (link.to, link.from) };
        let door = link.door.as_ref().map(|d| if d.passable { '+' } else { '#' });
        let (col, row) = at(a.0, a.1);
        match (b.0 - a.0, b.1 - a.1) {
            (1, 0) => put(col + width, row, &door.map(|d| format!("{}-", d)).unwrap_or_else(|| "--".to_string())),
            (0, 1) => put(col + width / 2, row + 1, &door.unwrap_or('|').to_string()),
            (1, 1) => put(col + width, row + 1, &door.unwrap_or('\\').to_string()),
            (-1, 1) if col >= 1 => put(col - 1, row + 1, &door.unwrap_or('/').to_string()),
            _ => {}
        }
    }
    canvas.into_iter().map(|r| r.into_iter().collect::<String>().trim_end().to_string()).collect()
}

/// The sidebar folded into one line, for a terminal too narrow for it.
pub fn folded(sidebar: &Sidebar) -> String {
    let texts: Vec<&str> = sidebar
        .fixed
        .iter()
        .filter_map(|r| match r {
            Row::Text(t) | Row::Dim(t) | Row::Warning(t) | Row::Error(t) | Row::Map(t) | Row::Exit(t, _) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    format!("GAME  {}", texts.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tiny() -> Console {
        Console::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny"), Mode::Edit).unwrap()
    }

    fn texts(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Heading(t) => format!("# {}", t),
                Row::Text(t) => t.clone(),
                Row::Dim(t) | Row::Warning(t) | Row::Error(t) => format!("~ {}", t),
                Row::Map(t) => format!("| {}", t),
                Row::Exit(t, true) => format!("~ {}", t),
                Row::Exit(t, false) => t.clone(),
                Row::Blank => String::new(),
            })
            .collect()
    }

    #[test]
    fn with_no_game_game_says_how_to_start_one() {
        let console = tiny();
        let sidebar = console.sidebar();
        assert_eq!(texts(&sidebar.fixed), vec!["# GAME", "~ no game yet: /play here starts one"]);
        assert_eq!(console.status(false).left, "Tiny World · edit · 1 warning");
    }

    #[test]
    fn here_is_a_game_masters_notes_for_the_room() {
        let mut console = tiny();
        console.handle("go /village/square");
        console.handle("/play here");
        console.handle("take lantern");
        console.handle("/edit");
        let sidebar = console.sidebar();
        assert_eq!(
            texts(&sidebar.fixed),
            vec!["# GAME", "at  village/square", "chapter  none", "carrying  brass lantern"]
        );
        let rest = texts(&sidebar.rest);
        let here: Vec<&str> = rest.iter().map(String::as_str).skip_while(|r| !r.starts_with("# HERE")).take_while(|r| !r.is_empty()).collect();
        assert_eq!(
            here,
            vec![
                "# HERE  Village Square (village/square)",
                "# exits",
                "~   east -----> village/inn     (not written)",
                "  north ----> village/well",
                "# people",
                "  elder",
                "# things",
                "  lantern",
                "# scenery",
                "~   bench",
            ]
        );
        let status = console.status(true);
        assert!(status.left.ends_with("chapter none · turn 1"), "{}", status.left);
        assert!(status.right.starts_with("newer build: /restart"));
    }

    #[test]
    fn an_exit_through_a_door_shows_the_room_beyond_and_the_door() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("doors")).unwrap();
        std::fs::write(root.join("hall.md"), "---\nid: hall\ntitle: Hall\ntype: location\nconnections: { down: vault-door }\n---\n").unwrap();
        std::fs::write(root.join("vault.md"), "---\nid: vault\ntitle: Vault\ntype: location\nconnections: { up: vault-door }\n---\n").unwrap();
        std::fs::write(
            root.join("doors/vault-door.md"),
            "---\nid: vault-door\ntitle: Iron door\ntype: door\ninherits: door\nsides:\n  hall: { direction: down }\n  vault: { direction: up }\n---\n",
        )
        .unwrap();
        let mut console = Console::open(root, Mode::Edit).unwrap();
        console.handle("go hall");
        let rest = texts(&console.sidebar().rest);
        assert!(rest.contains(&"  down ----> vault    closed door".to_string()), "{:?}", rest);
        assert_eq!(console.handle("down").0, "Vault (room)");
        assert_eq!(console.world.files["doors/vault-door"].kind, Kind::Door);
    }

    #[test]
    fn here_in_a_folder_lists_what_it_holds() {
        let mut console = tiny();
        let here: Vec<String> = texts(&console.sidebar().rest).into_iter().skip_while(|r| !r.starts_with("# HERE")).take_while(|r| !r.is_empty()).collect();
        assert_eq!(here, vec!["# HERE  /", "  npcs/", "  objects/", "  village/"]);
        console.handle("go village");
        let here: Vec<String> = texts(&console.sidebar().rest).into_iter().skip_while(|r| !r.starts_with("# HERE")).take_while(|r| !r.is_empty()).collect();
        assert_eq!(here, vec!["# HERE  village/", "  square  room", "  well  room"]);
    }

    #[test]
    fn the_map_follows_you_and_marks_where_you_are() {
        let mut console = tiny();
        assert!(!texts(&console.sidebar().rest).iter().any(|r| r.starts_with("# MAP")), "no room, no map");
        console.handle("go /village/square");
        let map: Vec<String> = texts(&console.sidebar().rest)
            .into_iter()
            .skip_while(|r| !r.starts_with("# MAP"))
            .take_while(|r| !r.is_empty())
            .collect();
        assert_eq!(map[0], "# MAP  around village/square");
        let drawn = map[1..].join("\n");
        assert!(drawn.contains("[ well  ]"), "{}", drawn);
        assert!(drawn.contains("[@square]--[ ?inn  ]"), "{}", drawn);
        assert!(console.handle("/map").0.starts_with("Map around village/square:"));
    }

    #[test]
    fn examining_a_thing_puts_its_file_in_the_sidebar_not_the_output() {
        let mut console = tiny();
        console.handle("go /village/square");
        let (out, _) = console.handle("x lantern");
        assert!(!out.contains("objects/lantern.md"), "{}", out);
        let rest = texts(&console.sidebar().rest);
        let looked: Vec<&str> = rest.iter().map(String::as_str).skip_while(|r| *r != "# looked at").collect();
        assert_eq!(looked[..4], ["# looked at", "  Brass Lantern (object)", "~   objects/lantern.md", "~   tool"]);
        console.handle("n");
        assert!(!texts(&console.sidebar().rest).contains(&"# looked at".to_string()), "gone once edit mode moves");
    }

    #[test]
    fn problems_and_warnings_sit_at_the_foot() {
        let mut console = tiny();
        let foot = texts(&console.sidebar().warnings);
        // The exit to village/inn is a TO WRITE entry in edit mode, not a
        // problem; /validate still fails on it, and play shows it.
        assert_eq!(foot, vec!["# PROBLEMS 0 · WARNINGS 0"]);
        assert!(texts(&console.sidebar().rest).iter().any(|r| r.contains("village/inn")));
        let (out, _) = console.handle("/validate");
        assert!(out.ends_with("Validation failed: 1 problem(s), 0 warning(s)"), "{}", out);
        console.handle("/play");
        assert_eq!(texts(&console.sidebar().warnings)[0], "# PROBLEMS 1 · WARNINGS 0");
    }

    #[test]
    fn here_shows_each_things_state() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("objects/lantern")).unwrap();
        std::fs::write(root.join("hall.md"), "---\nid: hall\ntitle: Hall\ntype: location\nobjects: [lantern]\n---\n").unwrap();
        std::fs::write(root.join("objects/lantern/index.md"), "---\nid: lantern\ntitle: Lantern\ntype: object\nobject_type: tool\nstate: hanging\n---\n").unwrap();
        std::fs::write(root.join("objects/lantern/hanging.md"), "---\ninherits: index\n---\n").unwrap();
        let mut console = Console::open(root, Mode::Edit).unwrap();
        console.handle("go hall");
        let rest = texts(&console.sidebar().rest);
        assert!(rest.contains(&"  lantern  hanging".to_string()), "{:?}", rest);
    }

    #[test]
    fn play_mode_has_no_here() {
        let mut console = tiny();
        console.handle("/play");
        assert!(!texts(&console.sidebar().rest).iter().any(|r| r.starts_with("# HERE")));
        let rest = texts(&console.sidebar().rest);
        assert!(rest.contains(&"# ROOM  Village Square (village/square)".to_string()), "play's detail is in the sidebar: {:?}", rest);
        assert!(rest.contains(&"  north ----> village/well".to_string()), "{:?}", rest);
    }
}
