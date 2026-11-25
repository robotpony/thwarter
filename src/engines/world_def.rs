//! The world's own definition, from its `world/` folder (`design/STATES.md`
//! › Chapters, REQ-CF-20): `index.md` (title, author, version, starting
//! room; the intro is its prose), `story.md` (`story.` values) and
//! `player.md` (the player, a character, with `player.` values).
//! `world/styles/` belongs to the writing helpers and isn't read here.

use super::layers::Layer;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldDef {
    pub title: Option<String>,
    /// `authors:`, one id or a list: machine user names, also persona
    /// ids (REQ-WS-5, 7). An old `author:` is read as a one-item list.
    pub authors: Vec<String>,
    /// The first of `authors`, for clients not yet reading the list.
    pub author: Option<String>,
    pub version: Option<String>,
    /// The room a new game starts in.
    pub start: Option<String>,
    /// What a player reads when a new game starts.
    pub intro: String,
    /// The file the fields above came from: `world/index.md`, or
    /// `world.toml` for a world not yet moved to `world/`.
    pub source: Option<PathBuf>,
    /// `story.` values, declared with their starting values.
    pub story: BTreeMap<String, String>,
    pub player: Option<PlayerDef>,
    /// Game time (`design/STATES.md` › Clocks): `start_time: "08:00"` as
    /// minutes past midnight, and `minutes_per_turn`. Both, or no time.
    pub start_time: Option<u64>,
    pub minutes_per_turn: Option<u64>,
}

/// The player, from `world/player.md`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerDef {
    /// The player's default name, which `{{ player.title }}` shows.
    pub title: String,
    /// What the player sees when they examine themselves.
    pub description: String,
    /// `player.` values, declared with their starting values.
    pub vars: BTreeMap<String, String>,
    pub path: PathBuf,
}

/// Which of the world's files a path is, relative to the world root.
pub(crate) enum WorldFile {
    Index,
    Story,
    Player,
    /// `world/styles/`, read by the writing helpers, not the engine.
    Styles,
    Other,
}

pub(crate) fn world_file(parts: &[String]) -> Option<WorldFile> {
    if parts.first().map(String::as_str) != Some("world") || parts.len() < 2 {
        return None;
    }
    Some(match parts[1..].iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["index.md"] => WorldFile::Index,
        ["story.md"] => WorldFile::Story,
        ["player.md"] => WorldFile::Player,
        ["styles", ..] => WorldFile::Styles,
        _ => WorldFile::Other,
    })
}

/// `authors:` as ids: one id, or a list of them, each a plain name with
/// no spaces. Duplicates are dropped. `None` when it's neither.
fn author_ids(value: &Value) -> Option<Vec<String>> {
    let items: Vec<&Value> = match value {
        Value::Sequence(list) => list.iter().collect(),
        one => vec![one],
    };
    let mut ids: Vec<String> = Vec::new();
    for item in items {
        let id = value_text(item)?.trim().to_string();
        if id.is_empty() || id.chars().any(char::is_whitespace) {
            return None;
        }
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Some(ids)
}

/// A scalar value as text: values are strings (`design/STATES.md` ›
/// Values), so `yes`, `100` and `true` are all kept as written.
pub fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// A `vars:` map, each value a single scalar. The error names the first
/// value that isn't.
pub fn read_vars(metadata: &Mapping) -> Result<BTreeMap<String, String>, String> {
    let mut vars = BTreeMap::new();
    match metadata.get("vars") {
        None | Some(Value::Null) => {}
        Some(Value::Mapping(map)) => {
            for (key, value) in map {
                let key = key.as_str().ok_or("`vars` names must be text")?;
                if key.contains('.') {
                    return Err(format!("`vars.{}`: a value's name can't contain a dot", key));
                }
                let value = value_text(value)
                    .ok_or_else(|| format!("`vars.{}` must be a single value (text, a number, yes or no)", key))?;
                vars.insert(key.to_string(), value);
            }
        }
        Some(_) => return Err("`vars` must be a map of names to values".to_string()),
    }
    Ok(vars)
}

/// Check a `world/` file's `type`, and warn about keys it doesn't take.
fn check_keys(layer: &Layer, expected_type: &str, allowed: &[&str], problems: &mut Vec<String>, warnings: &mut Vec<String>) -> bool {
    let shown = layer.path.display();
    match layer.str_key("type") {
        Some(t) if t == expected_type => {}
        Some(t) => {
            problems.push(format!("{}: `type: {}` should be `type: {}` here", shown, t, expected_type));
            return false;
        }
        None => {
            problems.push(format!("{}: needs `type: {}`", shown, expected_type));
            return false;
        }
    }
    for key in layer.metadata.keys().filter_map(Value::as_str) {
        if !allowed.contains(&key) {
            warnings.push(format!("{}: `{}` isn't read here, so it is ignored", shown, key));
        }
    }
    true
}

impl WorldDef {
    pub(crate) fn read_index(&mut self, layer: &Layer, problems: &mut Vec<String>, warnings: &mut Vec<String>) {
        if !check_keys(layer, "world", &["type", "title", "authors", "author", "version", "start", "start_time", "minutes_per_turn"], problems, warnings) {
            return;
        }
        let text = |key: &str| layer.metadata.get(key).and_then(value_text);
        self.title = text("title");
        let shown = layer.path.display();
        match (layer.metadata.get("authors"), text("author")) {
            (Some(value), old) => {
                match author_ids(value) {
                    Some(ids) => self.set_authors(ids),
                    None => problems.push(format!("{}: `authors:` is one id or a list of ids, like `authors: [mx, alice]`", shown)),
                }
                if old.is_some() {
                    warnings.push(format!("{}: `author:` is ignored beside `authors:`; remove it", shown));
                }
            }
            (None, Some(old)) => {
                warnings.push(format!("{}: `author: {}` is the old name; rename it `authors: {}`", shown, old, old));
                self.set_authors(vec![old]);
            }
            (None, None) => self.set_authors(Vec::new()),
        }
        self.version = text("version");
        self.start = layer.metadata.get("start").and_then(crate::parsers::id_value);
        self.intro = layer.body.clone();
        self.source = Some(layer.path.clone());
        let start_time = text("start_time");
        let per_turn = layer.metadata.get("minutes_per_turn");
        self.start_time = start_time.as_deref().and_then(crate::engines::clock::minutes_of);
        self.minutes_per_turn = per_turn.and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))).filter(|n| *n > 0);
        if start_time.is_some() && self.start_time.is_none() {
            problems.push(format!("{}: `start_time:` is a time of day, like \"08:00\"", shown));
        }
        if per_turn.is_some() && self.minutes_per_turn.is_none() {
            problems.push(format!("{}: `minutes_per_turn:` is a whole number of minutes, like 5", shown));
        }
        if self.start_time.is_some() != self.minutes_per_turn.is_some() {
            warnings.push(format!("{}: game time needs both `start_time:` and `minutes_per_turn:`, so there's none", shown));
            self.start_time = None;
            self.minutes_per_turn = None;
        }
    }

    fn set_authors(&mut self, ids: Vec<String>) {
        self.author = ids.first().cloned();
        self.authors = ids;
    }

    pub(crate) fn read_story(&mut self, layer: &Layer, problems: &mut Vec<String>, warnings: &mut Vec<String>) {
        if !check_keys(layer, "story", &["type", "title", "vars"], problems, warnings) {
            return;
        }
        match read_vars(&layer.metadata) {
            Ok(vars) => self.story = vars,
            Err(e) => problems.push(format!("{}: {}", layer.path.display(), e)),
        }
    }

    pub(crate) fn read_player(&mut self, layer: &Layer, problems: &mut Vec<String>, warnings: &mut Vec<String>) {
        if !check_keys(layer, "character", &["type", "title", "character_type", "vars"], problems, warnings) {
            return;
        }
        if layer.str_key("character_type") != Some("player") {
            problems.push(format!("{}: the player needs `character_type: player`", layer.path.display()));
        }
        let vars = match read_vars(&layer.metadata) {
            Ok(vars) => vars,
            Err(e) => {
                problems.push(format!("{}: {}", layer.path.display(), e));
                BTreeMap::new()
            }
        };
        self.player = Some(PlayerDef {
            title: layer.str_key("title").unwrap_or("you").to_string(),
            description: layer.body.clone(),
            vars,
            path: layer.path.clone(),
        });
    }

    /// A world not yet moved to `world/`: its title, author, version and
    /// description from `world.toml`'s `[metadata]`, and its start from
    /// `[settings] starting_location`. Read only when there's no
    /// `world/index.md`.
    pub(crate) fn from_world_toml(root: &Path, toml: &toml::Value) -> WorldDef {
        let get = |table: &str, key: &str| toml.get(table).and_then(|t| t.get(key)).and_then(|v| v.as_str()).map(str::to_string);
        if toml.get("metadata").is_none() && get("settings", "starting_location").is_none() {
            return WorldDef::default();
        }
        let authors: Vec<String> = get("metadata", "author").into_iter().collect();
        WorldDef {
            title: get("metadata", "title"),
            author: authors.first().cloned(),
            authors,
            version: get("metadata", "version"),
            start: get("settings", "starting_location"),
            intro: get("metadata", "description").unwrap_or_default(),
            source: Some(root.join("world.toml")),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engines::WorldEngine;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn base() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "castle/yard.md", "---\nid: castle/yard\ntitle: Yard\ntype: location\n---\nGrass.\n");
        dir
    }

    #[test]
    fn world_folder_defines_the_world_story_and_player() {
        let dir = base();
        let d = dir.path();
        write(d, "world/index.md", "---\ntitle: Castle\ntype: world\nauthor: Bruce\nversion: 2.0.0\nstart: castle/yard\n---\nYou wake in a yard.\n");
        write(d, "world/story.md", "---\ntype: story\nvars: { king: alive, day: 1 }\n---\n<!-- notes -->\n");
        write(d, "world/player.md", "---\ntitle: Traveller\ntype: character\ncharacter_type: player\nvars: { health: 100, brave: yes }\n---\nTired, but upright.\n");
        write(d, "world/styles/terse.md", "anything at all\n");
        write(d, "world.toml", "[metadata]\ntitle = \"Old title\"\n");
        let mut engine = WorldEngine::new();
        let report = engine.load_world(d).unwrap();
        // The old `author:` still reads, with a rename hint. Bruce has no
        // persona file, which is fine: the id is the name.
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(report.warnings[0].contains("`author: Bruce` is the old name; rename it `authors: Bruce`"), "{:?}", report.warnings);
        let world = engine.world();
        assert_eq!(world.authors, ["Bruce"]);
        assert_eq!(world.title.as_deref(), Some("Castle")); // world/ wins over world.toml
        assert_eq!(world.author.as_deref(), Some("Bruce"));
        assert_eq!(world.version.as_deref(), Some("2.0.0"));
        assert_eq!(world.start.as_deref(), Some("castle/yard"));
        assert_eq!(world.intro, "You wake in a yard.");
        assert_eq!(world.story["king"], "alive");
        assert_eq!(world.story["day"], "1");
        let player = world.player.as_ref().unwrap();
        assert_eq!(player.title, "Traveller");
        assert_eq!(player.vars["brave"], "yes");
        assert_eq!(player.description, "Tired, but upright.");
        // None of it is a thing in the world.
        assert_eq!(engine.loaded_locations.len(), 1);
        assert!(engine.characters.is_empty());
    }

    #[test]
    fn world_files_are_checked() {
        let dir = base();
        let d = dir.path();
        write(d, "world/index.md", "---\ntitle: Castle\ntype: world\nstart: castle/nowhere\nmood: grim\n---\n");
        write(d, "world/story.md", "---\ntype: world\n---\n");
        write(d, "world/player.md", "---\ntitle: P\ntype: character\nvars: { pack: [rope] }\n---\n");
        write(d, "world/notes.md", "---\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        let problems = engine.current_problems(false);
        let warnings = engine.current_warnings();
        for expected in [
            "world/index.md: the starting room `castle/nowhere` is not a known location",
            "world/story.md: `type: world` should be `type: story` here",
            "world/player.md: the player needs `character_type: player`",
            "world/player.md: `vars.pack` must be a single value",
        ] {
            assert!(problems.iter().any(|p| p.contains(expected)), "{}: {:?}", expected, problems);
        }
        assert!(warnings.iter().any(|w| w.contains("world/index.md: `mood` isn't read here")), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("world/notes.md: world/ holds index.md")), "{:?}", warnings);
    }

    #[test]
    fn a_world_not_yet_moved_reads_world_toml() {
        let dir = base();
        write(dir.path(), "world.toml", "[metadata]\ntitle = \"Tiny\"\ndescription = \"A test.\"\n\n[settings]\nstarting_location = \"castle/yard\"\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        assert_eq!(engine.world().title.as_deref(), Some("Tiny"));
        assert_eq!(engine.world().start.as_deref(), Some("castle/yard"));
        assert_eq!(engine.world().intro, "A test.");
        assert!(engine.world().source.as_ref().unwrap().ends_with("world.toml"));
    }

    #[test]
    fn other_tools_files_are_ignored() {
        let dir = base();
        let d = dir.path();
        let broken = "not a content file\n";
        for rel in [".obsidian/workspace.md", ".git/notes.md", "castle/.draft.md", "layouts/page.md", "public/index.md", "drafts/old.md", "notes/2026/today.md"] {
            write(d, rel, broken);
        }
        write(d, "world.toml", "ignore = [\"drafts\", \"notes/2026\"]\n");
        let mut engine = WorldEngine::new();
        let report = engine.load_world(d).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        // A Hugo folder name below the root is an ordinary area.
        write(d, "castle/static/hall.md", "---\nid: castle/static/hall\ntitle: Hall\ntype: location\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        assert!(engine.get_location("castle/static/hall").is_some());
    }

    #[test]
    fn authors_is_one_id_or_a_list() {
        let load = |index: &str| {
            let dir = base();
            write(dir.path(), "world/index.md", index);
            let mut engine = WorldEngine::new();
            engine.load_world(dir.path()).unwrap();
            (dir, engine)
        };
        let (_d, one) = load("---\ntype: world\nauthors: mx\n---\n");
        assert_eq!(one.world().authors, ["mx"]);
        assert_eq!(one.world().author.as_deref(), Some("mx"));
        assert!(one.current_warnings().is_empty(), "{:?}", one.current_warnings());
        let (_d, two) = load("---\ntype: world\nauthors: [mx, alice, mx]\n---\n");
        assert_eq!(two.world().authors, ["mx", "alice"]);
        assert!(two.is_author("alice") && two.is_author("mx") && !two.is_author("bob"));
        let (_d, both) = load("---\ntype: world\nauthors: [mx]\nauthor: old\n---\n");
        assert_eq!(both.world().authors, ["mx"]);
        assert!(both.current_warnings().iter().any(|w| w.contains("`author:` is ignored beside `authors:`")), "{:?}", both.current_warnings());
        let (_d, bad) = load("---\ntype: world\nauthors: [B. Writer]\n---\n");
        assert!(bad.current_problems(false).iter().any(|p| p.contains("`authors:` is one id or a list of ids")), "{:?}", bad.current_problems(false));
        // No `authors:` means no one is an author, and validate says so.
        let (_d, none) = load("---\ntype: world\ntitle: Castle\n---\n");
        assert!(none.world().authors.is_empty() && !none.is_author("mx"));
        assert!(none.current_warnings().iter().any(|w| w.contains("names no `authors:`")), "{:?}", none.current_warnings());
    }
}
