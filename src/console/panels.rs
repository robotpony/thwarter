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
        let saves = super::saves::list_saves(&self.world.root).unwrap_or_default();
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
        out.join("\n")
    }

    /// GAME, then HERE in edit mode.
    pub(super) fn sidebar(&self) -> Sidebar {
        let (fixed, mut rest) = self.game_panel();
        if self.mode == Mode::Edit {
            rest.push(Row::Blank);
            rest.extend(self.here_panel());
        }
        Sidebar { fixed, rest }
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
            if let Place::File(path) = &self.place {
                if let Some(entry) = self.world.files.get(path) {
                    rows.push(Row::Text(format!("{} ({})", entry.title, entry.kind.label())));
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
        if !location.characters.is_empty() {
            rows.push(Row::Heading("people".to_string()));
            for id in &location.characters {
                rows.push(match self.world.path_of(Kind::Character, id) {
                    Some(path) => Row::Text(format!("  {}  {}", id, self.scope_of(&path))),
                    None => Row::Missing(format!("  {}  no card", id)),
                });
            }
        }
        if !location.objects.is_empty() {
            rows.push(Row::Heading("things".to_string()));
            for id in &location.objects {
                rows.push(match self.world.path_of(Kind::Object, id) {
                    Some(path) => Row::Text(format!("  {}  {}", id, self.scope_of(&path))),
                    None => Row::Missing(format!("  {}  scenery, no card", id)),
                });
            }
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

/// The sidebar folded into one line, for a terminal too narrow for it.
pub fn folded(sidebar: &Sidebar) -> String {
    let texts: Vec<&str> = sidebar
        .fixed
        .iter()
        .filter_map(|r| match r {
            Row::Text(t) | Row::Dim(t) | Row::Missing(t) | Row::Warning(t) => Some(t.as_str()),
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
                "  elder  global",
                "# things",
                "  lantern  global",
                "~   bench  scenery, no card",
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
    fn play_mode_has_no_here() {
        let mut console = tiny();
        console.handle("/play");
        assert!(!texts(&console.sidebar().rest).iter().any(|r| r.starts_with("# HERE")));
    }
}
