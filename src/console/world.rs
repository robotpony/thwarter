//! A loaded world as the console sees it: the engine's entities, plus an
//! index of the files on disk, so edit mode can navigate the world as
//! folders (REQ-CON-4).

use crate::engines::bundles::FileRole;
use crate::engines::{EntityKind, WorldEngine, WorldLoadReport};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// What a content file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Room,
    Character,
    Object,
    /// The file is in the world but doesn't parse.
    Broken,
    /// A file in `world/`: the world's own settings (title, start, intro,
    /// story and player), not a thing.
    World,
    /// One of a bundle's state files (`design/STATES.md` › Bundles).
    State,
    /// A chapter overlay, on one thing or on everything in an area.
    Overlay,
    /// A chapter's definition.
    Chapter,
    /// An area's `_index.md`.
    Area,
    /// A template, kept out of the world itself.
    Template,
    /// A file in `game/`, which sets the starting state.
    Game,
    /// A door joining two rooms (`design/STATES.md` › Doors). Shown, not
    /// yet editable from the console.
    Door,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Room => "room",
            Kind::Character => "character",
            Kind::Object => "object",
            Kind::Broken => "doesn't parse",
            Kind::World => "world settings",
            Kind::State => "state",
            Kind::Overlay => "chapter overlay",
            Kind::Chapter => "chapter",
            Kind::Area => "area",
            Kind::Template => "template",
            Kind::Game => "game start",
            Kind::Door => "door",
        }
    }

    /// A room, character or object: something the world holds, as opposed
    /// to a part of one (a state, an overlay) or a file with a problem.
    pub fn is_thing(self) -> bool {
        matches!(self, Kind::Room | Kind::Character | Kind::Object)
    }
}

/// One content file, keyed in `World::files` by its path relative to the
/// world root without `.md` (`castle/courtyard`, `npcs/king`). With a
/// duplicate id, each file has an entry, and the title is the one the
/// engine kept.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub kind: Kind,
    /// The entity's `id`, which for characters and objects is bare
    /// (`king`) rather than its path (`npcs/king`).
    pub id: String,
    pub title: String,
}

pub struct World {
    pub root: PathBuf,
    pub engine: WorldEngine,
    pub report: WorldLoadReport,
    pub title: Option<String>,
    pub starting_location: Option<String>,
    pub files: BTreeMap<String, FileEntry>,
    /// Every folder that holds content, relative to the root, without a
    /// trailing slash. The root itself is `""`.
    pub dirs: BTreeSet<String>,
}

impl World {
    pub fn load(root: &Path) -> Result<World> {
        if !root.is_dir() {
            bail!("World directory `{}` not found", root.display());
        }
        let mut engine = WorldEngine::new();
        let report = engine.load_world(root)?;
        // world/index.md, or world.toml for a world not yet moved (the
        // engine falls back by itself).
        let title = engine.world().title.clone();
        let starting_location = engine.world().start.clone();
        let (files, dirs) = index_files(root, &engine);
        Ok(World { root: root.to_path_buf(), engine, report, title, starting_location, files, dirs })
    }

    /// Problems that make a reload unsafe: files that fail to parse, whose
    /// entities would silently drop out of the world (REQ-CON-7). A file
    /// fixed in the console since the load no longer counts.
    pub fn blocking_problems(&self) -> Vec<String> {
        let current = self.engine.current_problems(false);
        self.report.errors.iter().filter(|e| current.contains(e)).cloned().collect()
    }

    /// Problems that are normal while a world is being written, such as
    /// exits to rooms not written yet. Shown, never blocking (REQ-CON-7).
    /// Includes the engine's warnings, such as keys that differ only in
    /// case from a known key. Reflects changes made in memory since the load.
    pub fn warnings(&self) -> Vec<String> {
        self.engine
            .current_problems(false)
            .into_iter()
            .filter(|p| !self.report.errors.contains(p))
            .chain(self.engine.current_warnings())
            .collect()
    }

    /// Rebuilds the folder index after files were written.
    pub fn refresh_index(&mut self) {
        let (files, dirs) = index_files(&self.root, &self.engine);
        self.files = files;
        self.dirs = dirs;
        // A saved world/ file can change these too.
        self.title = self.engine.world().title.clone();
        self.starting_location = self.engine.world().start.clone();
    }

    pub fn path_of(&self, kind: Kind, id: &str) -> Option<String> {
        self.files.iter().find(|(_, e)| e.kind == kind && e.id == id).map(|(path, _)| path.clone())
    }

    /// Where play and edit mode start when nothing else says: the world's
    /// `starting_location`, else the first room by path.
    pub fn default_room(&self) -> Option<String> {
        if let Some(start) = &self.starting_location {
            if self.engine.get_location(start).is_some() {
                return Some(start.clone());
            }
        }
        self.files.values().find(|e| e.kind == Kind::Room).map(|e| e.id.clone())
    }

    pub fn display_name(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.root.display().to_string())
    }
}

/// Every content file and folder under `root`. Each file's kind and id
/// come from the engine's record of where it loaded them
/// (`WorldEngine::sources`). A `.md` file the engine has no record of
/// failed to parse.
fn index_files(root: &Path, engine: &WorldEngine) -> (BTreeMap<String, FileEntry>, BTreeSet<String>) {
    let mut files = BTreeMap::new();
    let mut dirs = BTreeSet::new();
    dirs.insert(String::new());

    // What the loader skips (dot-names, Hugo's folders, world.toml's
    // `ignore`) isn't the world's, so edit mode doesn't show it either.
    let walk = WalkDir::new(root).into_iter().filter_entry(|e| {
        let rel = e.path().strip_prefix(root).unwrap_or(e.path());
        rel.as_os_str().is_empty() || !engine.is_ignored(rel)
    });
    for entry in walk.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else { continue };
        let key = rel.with_extension("").to_string_lossy().replace('\\', "/");

        let mut parent = Path::new(&key).parent();
        while let Some(dir) = parent {
            dirs.insert(dir.to_string_lossy().replace('\\', "/"));
            parent = dir.parent();
        }
        files.insert(key.clone(), entry_for_role(engine, engine.file_role(rel), &key));
    }
    (files, dirs)
}

/// What edit mode shows for a file, from the role the engine gave it.
fn entry_for_role(engine: &WorldEngine, role: Option<&FileRole>, key: &str) -> FileEntry {
    let part = |kind: Kind, id: &str, title: String| FileEntry { kind, id: id.to_string(), title };
    match role {
        Some(FileRole::Thing { kind: EntityKind::Location, id }) => {
            entry_for(Kind::Room, id, engine.get_location(id).map(|l| &l.name))
        }
        Some(FileRole::Thing { kind: EntityKind::Character, id }) => {
            entry_for(Kind::Character, id, engine.characters.get(id).map(|c| &c.name))
        }
        Some(FileRole::Thing { kind: EntityKind::Object, id }) => {
            entry_for(Kind::Object, id, engine.objects.get(id).map(|o| &o.title))
        }
        Some(FileRole::Thing { kind: EntityKind::Door, id }) => {
            entry_for(Kind::Door, id, engine.doors.get(id).map(|d| &d.title))
        }
        Some(FileRole::State { id, name, .. }) => part(Kind::State, id, format!("State `{}` of {}", name, id)),
        Some(FileRole::Overlay { id, chapter, .. }) => {
            part(Kind::Overlay, id, format!("{} in chapter {}", id, chapter.name()))
        }
        Some(FileRole::AreaOverlay { area, chapter }) => part(
            Kind::Overlay,
            key,
            format!("Everything in {}/ in chapter {}", area.to_string_lossy().replace('\\', "/"), chapter.name()),
        ),
        Some(FileRole::Chapter(chapter)) => part(Kind::Chapter, key, format!("Chapter {}", chapter.name())),
        Some(FileRole::Area) => part(Kind::Area, key, "Notes for this area".to_string()),
        Some(FileRole::World) => {
            // `world/index.md` is the world itself; other files are named
            // for what they hold (`world/player.md`).
            let stem = key.rsplit('/').next().unwrap_or(key);
            let title = if stem == "index" { "World".to_string() } else { title_case(stem) };
            part(Kind::World, key, title)
        }
        Some(FileRole::Template) => part(Kind::Template, key, title_case(key.rsplit('/').next().unwrap_or(key))),
        Some(FileRole::Game) => part(Kind::Game, key, "How a new game starts".to_string()),
        // The title holds the reason, for `look`.
        Some(FileRole::Problem(why)) => part(Kind::Broken, key, why.clone()),
        None => part(Kind::Broken, key, String::new()),
    }
}

fn entry_for(kind: Kind, id: &str, title: Option<&String>) -> FileEntry {
    FileEntry { kind, id: id.to_string(), title: title.cloned().unwrap_or_default() }
}

/// `player` becomes `Player`.
fn title_case(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_bundles_files_show_as_its_parts_not_as_problems() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("objects/bread")).unwrap();
        fs::create_dir_all(root.join(".obsidian")).unwrap();
        fs::write(root.join("hall.md"), "---\nid: hall\ntitle: Hall\ntype: location\nobjects: [bread]\n---\n").unwrap();
        fs::write(
            root.join("objects/bread/index.md"),
            "---\nid: bread\ntitle: Loaf of bread\ntype: object\nobject_type: food\nstate: fresh\n---\nA loaf.\n",
        )
        .unwrap();
        fs::write(root.join("objects/bread/fresh.md"), "---\ninherits: index\n---\nStill warm.\n").unwrap();
        fs::write(root.join("objects/bread/stale.md"), "---\ninherits: index\n---\nGone hard.\n").unwrap();
        fs::write(root.join(".obsidian/notes.md"), "not the world's").unwrap();

        let world = World::load(root).unwrap();
        assert_eq!(world.files["objects/bread/index"].kind, Kind::Object);
        assert_eq!(world.files["objects/bread/stale"].kind, Kind::State, "{:?}", world.files["objects/bread/stale"]);
        assert_eq!(world.files["objects/bread/stale"].title, "State `stale` of bread");
        assert!(!world.files.keys().any(|k| k.starts_with(".obsidian")), "ignored folders stay out");
    }
}
