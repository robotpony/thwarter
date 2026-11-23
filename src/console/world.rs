//! A loaded world as the console sees it: the engine's entities, plus an
//! index of the files on disk, so edit mode can navigate the world as
//! folders (REQ-CON-4).

use crate::engines::{EntityKind, WorldEngine, WorldLoadReport};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Room => "room",
            Kind::Character => "character",
            Kind::Object => "object",
            Kind::Broken => "doesn't parse",
        }
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
        let (title, starting_location) = read_world_toml(root);
        let (files, dirs) = index_files(root, &engine);
        Ok(World { root: root.to_path_buf(), engine, report, title, starting_location, files, dirs })
    }

    /// Problems that make a reload unsafe: files that fail to parse, whose
    /// entities would silently drop out of the world (REQ-CON-7).
    pub fn blocking_problems(&self) -> &[String] {
        &self.report.errors
    }

    /// Problems that are normal while a world is being written, such as
    /// exits to rooms not written yet. Shown, never blocking (REQ-CON-7).
    /// Includes keys that differ only in case from a known key.
    pub fn warnings(&self) -> Vec<String> {
        self.engine
            .problems(&self.report, false)
            .into_iter()
            .filter(|p| !self.report.errors.contains(p))
            .chain(self.report.warnings.iter().cloned())
            .collect()
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

fn read_world_toml(root: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(root.join("world.toml")) else {
        return (None, None);
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return (None, None);
    };
    let get = |table: &str, key: &str| {
        value.get(table).and_then(|t| t.get(key)).and_then(|v| v.as_str()).map(str::to_string)
    };
    (get("metadata", "title"), get("settings", "starting_location"))
}

/// Every content file and folder under `root`. Each file's kind and id
/// come from the engine's record of where it loaded them
/// (`WorldEngine::sources`). A `.md` file the engine has no record of
/// failed to parse.
fn index_files(root: &Path, engine: &WorldEngine) -> (BTreeMap<String, FileEntry>, BTreeSet<String>) {
    let mut loaded: HashMap<&Path, (EntityKind, &str)> = HashMap::new();
    for ((kind, id), paths) in &engine.sources {
        for path in paths {
            loaded.insert(path.as_path(), (*kind, id.as_str()));
        }
    }

    let mut files = BTreeMap::new();
    let mut dirs = BTreeSet::new();
    dirs.insert(String::new());

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
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

        let file = match loaded.get(path) {
            Some((EntityKind::Location, id)) => entry_for(Kind::Room, id, engine.get_location(id).map(|l| &l.name)),
            Some((EntityKind::Character, id)) => {
                entry_for(Kind::Character, id, engine.characters.get(*id).map(|c| &c.name))
            }
            Some((EntityKind::Object, id)) => entry_for(Kind::Object, id, engine.objects.get(*id).map(|o| &o.title)),
            None => FileEntry { kind: Kind::Broken, id: key.clone(), title: String::new() },
        };
        files.insert(key, file);
    }
    (files, dirs)
}

fn entry_for(kind: Kind, id: &str, title: Option<&String>) -> FileEntry {
    FileEntry { kind, id: id.to_string(), title: title.cloned().unwrap_or_default() }
}
