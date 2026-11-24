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
    /// Something not written yet: an exit's room, a thing's card.
    Missing(String),
    Warning(String),
    /// A row of the map, drawn as it is: cut at the sidebar's edge, never
    /// wrapped.
    Map(String),
    Blank,
}

/// The sidebar: GAME's first rows stay put, everything after them
/// scrolls when the sidebar is short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidebar {
    pub fixed: Vec<Row>,
    pub rest: Vec<Row>,
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
            format!("{} warning{}", warnings, if warnings == 1 { "" } else { "s" }),
        ];
        if let Some(game) = &self.game {
            let chapter = game.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_else(|| "none".to_string());
            left.push(format!("chapter {}", chapter));
            left.push(format!("turn {}", game.turn));
        }
        let mut right = format!("thwart {}", env!("CARGO_PKG_VERSION"));
        if newer_build {
            right = format!("newer build: /restart · {}", right);
        }
        Status { left: left.join(" · "), right }
    }

    /// `/status`: the world, where you are, the game, saves and the build.
    pub(super) fn status_report(&self) -> String {
        let mut out = vec![format!("{}, at {}", self.world.display_name(), self.world.root.display())];
        out.push(format!("Mode: {}; edit mode is at {}", self.mode.name(), self.pwd()));
        out.push(match &self.game {
            Some(game) => {
                let chapter = game.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_else(|| "none".to_string());
                let carrying = game.inventory.len();
                format!(
                    "Game: at {}, chapter {}, turn {}, carrying {} thing{}",
                    game.location,
                    chapter,
                    game.turn,
                    carrying,
                    if carrying == 1 { "" } else { "s" }
                )
            }
            None => "Game: none yet (/play here starts one)".to_string(),
        });
        let saves = crate::engines::list_saves(&self.world.root).unwrap_or_default();
        out.push(if saves.is_empty() { "Saves: none".to_string() } else { format!("Saves: {}", saves.join(", ")) });
        let broken = self.world.files.values().filter(|e| e.kind == Kind::Broken).count();
        let warnings = self.world.warnings().len();
        out.push(format!(
            "Problems: {} file{} that don't load, {} warning{}",
            broken,
            if broken == 1 { "" } else { "s" },
            warnings,
            if warnings == 1 { "" } else { "s" }
        ));
        out.push(format!("Build: thwart {}", env!("CARGO_PKG_VERSION")));
        if let Some(log) = &self.unknown_log {
            let lines = std::fs::read_to_string(log).map(|t| t.lines().count()).unwrap_or(0);
            out.push(format!("Unknown input: {} ({} logged)", log.display(), lines));
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
        }
        Sidebar { fixed, rest }
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
        let mut rows = vec![Row::Heading(format!("MAP  around {}", room))];
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
        match self.map_room() {
            Some(room) => format!("Map around {}:\n\n{}", room, crate::render::render_map(&self.world.engine.map_layout(&room, None, None))),
            None => "There's no room to map from: go to a room, or /play to start a game.".to_string(),
        }
    }

    /// GAME's fixed rows (where, chapter, carrying), and the rest.
    fn game_panel(&self) -> (Vec<Row>, Vec<Row>) {
        let Some(game) = &self.game else {
            return (
                vec![Row::Heading("GAME".to_string()), Row::Dim("no game yet: /play here starts one".to_string())],
                Vec::new(),
            );
        };
        let chapter = game.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_else(|| "none".to_string());
        let carrying: Vec<String> = game.inventory.iter().map(|id| self.object_title(id)).collect();
        let fixed = vec![
            Row::Heading("GAME".to_string()),
            Row::Text(format!("at  {}", game.location)),
            Row::Text(format!("chapter  {}", chapter)),
            Row::Text(format!("carrying  {}", if carrying.is_empty() { "nothing".to_string() } else { carrying.join(", ") })),
        ];
        let mut rest = Vec::new();
        let changed: Vec<String> = game
            .states
            .iter()
            .flat_map(|(_, things)| things.iter().map(|(id, state)| format!("  {}: {}", id, state)))
            .collect();
        if !changed.is_empty() {
            rest.push(Row::Blank);
            rest.push(Row::Heading("changed".to_string()));
            rest.extend(changed.into_iter().map(Row::Text));
        }
        if !game.values.is_empty() {
            rest.push(Row::Blank);
            rest.push(Row::Heading("values".to_string()));
            rest.extend(game.values.iter().map(|(k, v)| Row::Text(format!("  {} = {}", k, v))));
        }
        (fixed, rest)
    }

    /// HERE: a game master's notes for where edit mode is.
    fn here_panel(&self) -> Vec<Row> {
        let mut rows = vec![Row::Heading(format!("HERE  {}", self.pwd()))];
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
                                Kind::Broken => Row::Warning(format!("  {}  doesn't load", name)),
                                kind => Row::Text(format!("  {}  {}", name, kind.label())),
                            });
                        }
                    }
                    if rows.len() == 1 {
                        rows.push(Row::Dim("  nothing here yet".to_string()));
                    }
                }
            }
            return rows;
        };
        let Some(location) = self.world.engine.get_location(&room) else { return rows };

        rows.push(Row::Heading("exits".to_string()));
        if location.connections.is_empty() {
            rows.push(Row::Dim("  none".to_string()));
        }
        let mut exits: Vec<(&String, &String)> = location.connections.iter().collect();
        exits.sort();
        for (direction, to) in exits {
            // Through a door: the room beyond, and the door by name.
            let (room, through) = match self.world.engine.doors.get(to) {
                Some(door) => (door.leads_to(&location.id).unwrap_or("?").to_string(), format!("  through {}", to)),
                None => (to.clone(), String::new()),
            };
            if self.world.engine.get_location(&room).is_some() {
                rows.push(Row::Text(format!("  {}  {}{}", direction, room, through)));
            } else {
                rows.push(Row::Missing(format!("  {}  {}{}  (not written)", direction, room, through)));
            }
        }
        // People, things, then scenery: the things built from the built-in
        // `scenery` template, or listed with no card yet (dimmed). Scope
        // shows only when it isn't global, the default (Bruce, 2026-11-23).
        let named = |id: &String, path: Option<String>| -> Row {
            match path.map(|p| self.scope_of(&p)) {
                Some(scope) if scope == "global" => Row::Text(format!("  {}", id)),
                Some(scope) => Row::Text(format!("  {}  {}", id, scope)),
                None => Row::Dim(format!("  {}", id)),
            }
        };
        if !location.characters.is_empty() {
            rows.push(Row::Heading("people".to_string()));
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
            rows.push(Row::Heading("things".to_string()));
            rows.extend(with_held(things));
        }
        if !scenery.is_empty() {
            rows.push(Row::Heading("scenery".to_string()));
            rows.extend(with_held(scenery));
        }
        if let Some(path) = &self.looked_at {
            rows.push(Row::Heading("looked at".to_string()));
            rows.extend(self.file_rows(path));
        }
        let warnings: Vec<String> = self
            .world
            .warnings()
            .into_iter()
            .filter(|w| w.contains(&format!("`{}`", room)) || w.contains(&format!("{}.md", room)) || w.contains(&room))
            .collect();
        if !warnings.is_empty() {
            rows.push(Row::Heading("warnings".to_string()));
            rows.extend(warnings.into_iter().map(|w| Row::Warning(format!("  {}", w))));
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
        format!("area {}/", area)
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
            Row::Text(t) | Row::Dim(t) | Row::Missing(t) | Row::Warning(t) | Row::Map(t) => Some(t.as_str()),
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
        Console::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/transcripts/worlds/tiny"), Mode::Edit).unwrap()
    }

    fn texts(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Heading(t) => format!("# {}", t),
                Row::Text(t) => t.clone(),
                Row::Dim(t) | Row::Missing(t) | Row::Warning(t) => format!("~ {}", t),
                Row::Map(t) => format!("| {}", t),
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
        let here: Vec<&str> = rest.iter().map(String::as_str).skip_while(|r| !r.starts_with("# HERE")).collect();
        assert_eq!(
            here,
            vec![
                "# HERE  village/square",
                "# exits",
                "~   east  village/inn  (not written)",
                "  north  village/well",
                "# people",
                "  elder",
                "# things",
                "  lantern",
                "# scenery",
                "~   bench",
                "# warnings",
                "~   village/square: connection `east` points at unknown location `village/inn`",
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
        assert!(rest.contains(&"  down  vault  through vault-door".to_string()), "{:?}", rest);
        assert_eq!(console.handle("down").0, "Vault (room)");
        assert_eq!(console.world.files["doors/vault-door"].kind, Kind::Door);
    }

    #[test]
    fn here_in_a_folder_lists_what_it_holds() {
        let mut console = tiny();
        let here: Vec<String> = texts(&console.sidebar().rest).into_iter().skip_while(|r| !r.starts_with("# HERE")).collect();
        assert_eq!(here, vec!["# HERE  /", "  npcs/", "  objects/", "  village/"]);
        console.handle("go village");
        let here: Vec<String> = texts(&console.sidebar().rest).into_iter().skip_while(|r| !r.starts_with("# HERE")).collect();
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
    fn play_mode_has_no_here() {
        let mut console = tiny();
        console.handle("/play");
        assert!(!texts(&console.sidebar().rest).iter().any(|r| r.starts_with("# HERE")));
    }
}
