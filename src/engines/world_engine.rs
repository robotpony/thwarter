use crate::engines::location_view::{direction_rank, EntityRef, ExitView, LocationView};
use crate::models::character::Character;
use crate::models::world::*;
use crate::engines::bundles::{self, Chapter, ChapterKey, FileRecord, Thing};
use crate::engines::layers::{Layer, Merged};
use crate::engines::world_def::WorldDef;
use crate::models::game::{PlayState, SavedChapter};
use crate::parsers::ParsedEntity;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// The three kinds of content file. An `id` is unique within a kind; a
/// location and an object may share one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntityKind {
    Location,
    Character,
    Object,
}

impl EntityKind {
    /// The frontmatter `type` value for this kind.
    pub fn type_name(self) -> &'static str {
        match self {
            EntityKind::Location => "location",
            EntityKind::Character => "character",
            EntityKind::Object => "object",
        }
    }

    pub fn from_type_name(name: &str) -> Option<Self> {
        match name {
            "location" => Some(EntityKind::Location),
            "character" => Some(EntityKind::Character),
            "object" => Some(EntityKind::Object),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldEngine {
    pub world_graph: WorldGraph,
    pub loaded_locations: HashMap<String, Location>,
    pub objects: HashMap<String, GameObject>,
    /// Character cards by id, as loaded. A read-only catalog for lookups
    /// and validation; placement and state belong to `CharacterEngine`.
    pub characters: HashMap<String, Character>,
    /// Every file each `(kind, id)` was loaded from, in load order (a
    /// bundle's `index.md`). More than one file means a duplicate id: the
    /// last one loaded wins in the tables above, `problems` reports it,
    /// and the writer refuses to edit it (REQ-ENG-7).
    pub sources: HashMap<(EntityKind, String), Vec<PathBuf>>,
    /// The directory `load_world` walked, as it was given.
    pub world_root: Option<PathBuf>,
    /// Each thing with every file that can apply to it, for building any
    /// version on request (`design/STATES.md`). The tables above hold
    /// each thing's starting version.
    things: HashMap<(EntityKind, String), Thing>,
    /// Every chapter, root ones first.
    chapters: Vec<Chapter>,
    /// Each file's split, in load order. Everything above is rebuilt from
    /// these, so an in-memory change (REQ-ENG-8) resolves duplicates,
    /// bundles and errors exactly as a fresh load would.
    files: Vec<FileRecord>,
    /// Problems and warnings from the last rebuild, each prefixed with its file.
    file_problems: Vec<String>,
    file_warnings: Vec<String>,
    /// The world's definition: `world/`, or `world.toml` for a world not
    /// yet moved.
    world: WorldDef,
    /// What `world.toml` says, read at load: the old definition keys and
    /// the extra folders to ignore.
    world_toml: WorldDef,
    ignore: Vec<PathBuf>,
    world_toml_warning: Option<String>,
    /// A thing's file path from the world root without `.md` (a bundle's
    /// folder), and its file name, to the things it may name, so an
    /// Obsidian link to a file finds the thing (REQ-CF-25).
    links: HashMap<String, Vec<(EntityKind, String)>>,
}

/// Which version of each thing to see: the starting versions (the
/// default), or those of a game in progress.
#[derive(Debug, Clone, Copy, Default)]
pub struct At<'a> {
    pub play: Option<&'a PlayState>,
}

impl<'a> At<'a> {
    pub fn play(state: &'a PlayState) -> Self {
        Self { play: Some(state) }
    }
}

/// One version of a thing: its typed entity and the merged layers it came
/// from, with which file each part came from.
#[derive(Debug, Clone)]
pub struct Version {
    pub entity: ParsedEntity,
    pub merged: Merged,
    pub state: Option<String>,
    pub chapter: Option<ChapterKey>,
}

/// Summary of a `load_world` pass: what was loaded, and every file that
/// failed to parse. A partial load is not a hard error; callers (the
/// `validate` and `show` commands in particular) need to see both what
/// loaded and what didn't in the same report.
#[derive(Debug, Clone, Default)]
pub struct WorldLoadReport {
    pub locations_loaded: usize,
    pub objects_loaded: usize,
    /// The characters loaded, also kept in `WorldEngine::characters`.
    /// Placement and state belong to `CharacterEngine`, per
    /// `design/ARCHITECTURE.md`.
    pub characters: Vec<Character>,
    pub errors: Vec<String>,
    /// Problems that don't fail validation: a key that differs only in
    /// case from a known one, a list a layer replaces, an overlay that
    /// applies by where it sits (each prefixed with its file), or an id
    /// shared between kinds.
    pub warnings: Vec<String>,
}

impl WorldEngine {
    pub fn new() -> Self {
        Self {
            world_graph: WorldGraph::new(),
            loaded_locations: HashMap::new(),
            objects: HashMap::new(),
            characters: HashMap::new(),
            sources: HashMap::new(),
            world_root: None,
            things: HashMap::new(),
            chapters: Vec::new(),
            files: Vec::new(),
            file_problems: Vec::new(),
            file_warnings: Vec::new(),
            world: WorldDef::default(),
            world_toml: WorldDef::default(),
            ignore: Vec::new(),
            world_toml_warning: None,
            links: HashMap::new(),
        }
    }

    /// The world's own definition: title, author, starting room, intro,
    /// story values and the player.
    pub fn world(&self) -> &WorldDef {
        &self.world
    }

    /// Walk every `.md` file under `world_path`, sort each into its role
    /// (a thing, a state, a chapter, an overlay, an area) by where it sits,
    /// and load each thing's starting version. Files that fail are
    /// collected as errors rather than aborting the whole load, so
    /// `validate` can report every problem in one pass.
    pub fn load_world<P: AsRef<Path>>(&mut self, world_path: P) -> Result<WorldLoadReport> {
        let root = world_path.as_ref().to_path_buf();
        self.world_root = Some(root.clone());
        self.files.clear();
        self.read_world_toml(&root);
        let ignore = self.ignore.clone();
        let mut paths: Vec<PathBuf> = WalkDir::new(&world_path)
            .into_iter()
            .filter_entry(|e| {
                // Don't descend into other tools' folders (.git, .obsidian, public/).
                let rel = e.path().strip_prefix(&root).unwrap_or(e.path());
                let parts: Vec<String> = rel.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                parts.is_empty() || !bundles::ignored(&parts, &ignore) || e.file_type().is_file()
            })
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect();
        paths.sort();
        for path in paths {
            let layer = match std::fs::read_to_string(&path) {
                Ok(text) => Layer::parse(&path, &text),
                Err(e) => Err(format!("Failed to read file {:?}: {}", path, e)),
            };
            self.files.push(FileRecord { path, layer });
        }
        let built = self.rebuild();

        let mut report = WorldLoadReport::default();
        for (_, entity) in &built {
            match entity {
                ParsedEntity::Location(_) => report.locations_loaded += 1,
                ParsedEntity::Object(_) => report.objects_loaded += 1,
                ParsedEntity::Character(c) => report.characters.push(c.clone()),
            }
        }
        report.errors = self.file_problems.clone();
        report.warnings = self.current_warnings();
        Ok(report)
    }

    /// Recompute everything from the file records, returning each thing
    /// whose starting version typed, in load order.
    fn rebuild(&mut self) -> Vec<(Thing, ParsedEntity)> {
        let root = self.world_root.clone().unwrap_or_default();
        let built = bundles::build(&root, &self.files, &self.ignore);
        self.world_graph.locations.clear();
        self.objects.clear();
        self.characters.clear();
        self.sources.clear();
        self.things.clear();
        self.links.clear();
        for (thing, _) in &built.things {
            let key = (thing.kind, thing.id.clone());
            self.things.insert(key.clone(), thing.clone());
            let rel = thing.index.path.strip_prefix(&root).unwrap_or(&thing.index.path);
            let path = match &thing.bundle {
                Some(folder) => folder.clone(),
                None => rel.with_extension(""),
            };
            let path = path.to_string_lossy().replace('\\', "/");
            let stem = path.rsplit('/').next().unwrap_or(&path).to_string();
            // A bundle is also named by its index file (`castle/vault/index`).
            let file = rel.with_extension("").to_string_lossy().replace('\\', "/");
            for name in [path, stem, file] {
                let entry = self.links.entry(name).or_default();
                if !entry.contains(&key) {
                    entry.push(key.clone());
                }
            }
        }
        for (thing, entity) in &built.things {
            let mut entity = entity.clone();
            self.resolve_links(&mut entity);
            let key = (thing.kind, thing.id.clone());
            self.sources.entry(key).or_default().push(thing.index.path.clone());
            match entity {
                ParsedEntity::Location(l) => self.world_graph.add_location(l),
                ParsedEntity::Object(o) => {
                    self.objects.insert(o.id.clone(), o);
                }
                ParsedEntity::Character(c) => {
                    self.characters.insert(c.id.clone(), c);
                }
            }
        }
        self.loaded_locations = self.world_graph.locations.clone();
        self.chapters = built.chapters;
        self.world = if built.has_world_index {
            built.world
        } else {
            WorldDef { story: built.world.story, player: built.world.player, ..self.world_toml.clone() }
        };
        if let Some(start) = self.world.start.take() {
            self.world.start = Some(self.resolve_name(&start, Some(EntityKind::Location)));
        }
        self.file_problems = built.problems;
        self.file_warnings = built.warnings;
        if let Some(w) = &self.world_toml_warning {
            self.file_warnings.insert(0, w.clone());
        }
        built.things
    }

    /// The id `name` means for a reference of `kind` (any kind if `None`):
    /// itself if a thing has that id, else the one thing whose file path
    /// or file name it is (an Obsidian link to the file), else unchanged,
    /// so the usual "unknown" checks report it.
    pub fn resolve_name(&self, name: &str, kind: Option<EntityKind>) -> String {
        let has_id = match kind {
            Some(k) => self.things.contains_key(&(k, name.to_string())),
            None => self.things.keys().any(|(_, id)| id == name),
        };
        if has_id {
            return name.to_string();
        }
        let matches: Vec<&(EntityKind, String)> = self
            .links
            .get(name)
            .map(|v| v.iter().filter(|(k, _)| kind.map_or(true, |want| *k == want)).collect())
            .unwrap_or_default();
        match matches.as_slice() {
            [(_, id)] => id.clone(),
            _ => name.to_string(),
        }
    }

    /// Turn every reference in `entity` that names a file into the id of
    /// the thing in it.
    fn resolve_links(&self, entity: &mut ParsedEntity) {
        use EntityKind::*;
        let r = |name: &mut String, kind: Option<EntityKind>| *name = self.resolve_name(name, kind);
        match entity {
            ParsedEntity::Location(l) => {
                l.connections.values_mut().for_each(|t| r(t, Some(Location)));
                l.characters.iter_mut().for_each(|c| r(c, Some(Character)));
                l.objects.iter_mut().for_each(|o| r(o, Some(Object)));
            }
            ParsedEntity::Character(c) => {
                if let Some(at) = c.default_location.as_mut() {
                    r(at, Some(Location));
                }
                c.inventory.iter_mut().for_each(|o| r(o, Some(Object)));
            }
            ParsedEntity::Object(o) => {
                for action in o.actions.values_mut() {
                    action.requires_has.iter_mut().for_each(|h| r(h, Some(Object)));
                    let becomes = std::mem::take(&mut action.becomes);
                    action.becomes = becomes.into_iter().map(|(k, v)| (self.resolve_name(&k, None), v)).collect();
                }
            }
        }
    }

    /// Read `world.toml`: the extra folders to ignore (`ignore = [...]`,
    /// paths from the root) and, for a world not yet moved to `world/`,
    /// its old `[metadata]` and `[settings] starting_location`.
    fn read_world_toml(&mut self, root: &Path) {
        self.ignore.clear();
        self.world_toml = WorldDef::default();
        self.world_toml_warning = None;
        let path = root.join("world.toml");
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let value = match text.parse::<toml::Value>() {
            Ok(v) => v,
            Err(e) => {
                self.world_toml_warning = Some(format!("{}: doesn't parse, so it is ignored: {}", path.display(), e.message()));
                return;
            }
        };
        if let Some(list) = value.get("ignore").and_then(|v| v.as_array()) {
            self.ignore = list.iter().filter_map(|v| v.as_str()).map(PathBuf::from).collect();
        }
        self.world_toml = WorldDef::from_world_toml(root, &value);
    }

    /// Every id used by more than one kind, with those kinds, sorted. Ids
    /// are scoped by kind, so this isn't an error, but the console's
    /// `cd <id>` can't tell such entities apart.
    pub fn shared_ids(&self) -> Vec<(String, Vec<EntityKind>)> {
        let mut by_id: std::collections::BTreeMap<&str, Vec<EntityKind>> = std::collections::BTreeMap::new();
        for (kind, id) in self.sources.keys() {
            by_id.entry(id.as_str()).or_default().push(*kind);
        }
        by_id
            .into_iter()
            .filter(|(_, kinds)| kinds.len() > 1)
            .map(|(id, mut kinds)| {
                kinds.sort();
                (id.to_string(), kinds)
            })
            .collect()
    }

    fn shared_id_warnings(&self) -> Vec<String> {
        self.shared_ids()
            .into_iter()
            .map(|(id, kinds)| {
                let names: Vec<&str> = kinds.iter().map(|k| k.type_name()).collect();
                format!("id `{}` is used by more than one kind ({}); give each a different id", id, names.join(", "))
            })
            .collect()
    }

    /// Change the loaded world in memory (REQ-ENG-8): replace whatever
    /// `path` held with `text`, as if the file had been loaded with it. A
    /// parse error becomes that file's problem, as it would on a load.
    /// Nothing is read from or written to disk. `path` is in the form
    /// `load_world` uses (its root joined with the file's relative path),
    /// the same form as `sources`. A state or overlay file changes its
    /// thing; an `index.md` can turn its folder into a bundle.
    pub fn set_file(&mut self, path: &Path, text: &str) {
        let record = FileRecord { path: path.to_path_buf(), layer: Layer::parse(path, text) };
        match self.files.iter_mut().find(|r| r.path == path) {
            Some(existing) => *existing = record,
            None => self.files.push(record),
        }
        self.rebuild();
    }

    /// Forget a file and everything it defined, in memory only.
    pub fn remove_file(&mut self, path: &Path) {
        if let Some(i) = self.files.iter().position(|r| r.path == path) {
            self.files.remove(i);
            self.rebuild();
        }
    }

    /// A thing with all its files: index, states and overlays.
    pub fn thing(&self, kind: EntityKind, id: &str) -> Option<&Thing> {
        self.things.get(&(kind, id.to_string()))
    }

    /// The kinds that have a thing with this id, in kind order. More than
    /// one makes a bare reference such as `becomes:` ambiguous.
    pub fn kinds_with_id(&self, id: &str) -> Vec<EntityKind> {
        let mut kinds: Vec<EntityKind> = self.things.keys().filter(|(_, i)| i == id).map(|(k, _)| *k).collect();
        kinds.sort();
        kinds
    }

    /// The one thing a bare id names, for an action's `becomes:`.
    pub fn thing_by_id(&self, id: &str) -> Result<&Thing> {
        match self.kinds_with_id(id).as_slice() {
            [] => anyhow::bail!("nothing has the id `{}`", id),
            [kind] => Ok(&self.things[&(*kind, id.to_string())]),
            kinds => {
                let names: Vec<&str> = kinds.iter().map(|k| k.type_name()).collect();
                anyhow::bail!("`{}` is the id of more than one kind ({}), so it's ambiguous", id, names.join(", "))
            }
        }
    }

    /// Problems with what each object's actions name: a `becomes:` that
    /// names no thing or no state of it, a `chapter:` that can't be seen
    /// from the object, a `requires: { has: }` naming no object.
    fn action_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut ids: Vec<&String> = self.objects.keys().collect();
        ids.sort();
        for id in ids {
            let object = &self.objects[id];
            let scope = self.thing(EntityKind::Object, id).map(|t| t.scope.clone()).unwrap_or_default();
            let mut names: Vec<&String> = object.actions.keys().collect();
            names.sort();
            for name in names {
                let action = &object.actions[name];
                let at = format!("object `{}`, action `{}`", id, name);
                for (target, state) in &action.becomes {
                    match self.thing_by_id(target) {
                        Err(e) => problems.push(format!("{}: `becomes:` {}", at, e)),
                        Ok(thing) if !thing.states.contains_key(state) => problems.push(format!(
                            "{}: `becomes:` names state `{}`, but {} `{}` has no such state file",
                            at,
                            state,
                            thing.kind.type_name(),
                            target
                        )),
                        Ok(_) => {}
                    }
                }
                if let Some(chapter) = &action.chapter {
                    if self.find_chapter(&scope, chapter).is_none() {
                        problems.push(format!("{}: `chapter: {}` names no chapter that can be seen from the object", at, chapter));
                    }
                }
                for needed in &action.requires_has {
                    if !self.objects.contains_key(needed) {
                        problems.push(format!("{}: `requires: {{ has: {} }}` names no object", at, needed));
                    }
                }
            }
        }
        problems
    }

    /// Authoring warnings for responses (`design/STATES.md` › Responses):
    /// an action with mechanics but no response, a `requires:` with no
    /// refusal, and a response with no mechanics. All three still play.
    fn response_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut ids: Vec<&String> = self.objects.keys().collect();
        ids.sort();
        for id in ids {
            let object = &self.objects[id];
            let mut names: Vec<&String> = object.actions.keys().collect();
            names.sort();
            for name in names {
                let action = &object.actions[name];
                let at = format!("object `{}`, action `{}`", id, name);
                if !action.mechanics {
                    warnings.push(format!("{}: has a response but no mechanics in frontmatter", at));
                } else if !object.responses.contains_key(name) {
                    warnings.push(format!(
                        "{}: has no response, so play makes one up; add it after `<!-- thwarter:action {} -->`",
                        at, name
                    ));
                }
                if !action.requires_has.is_empty() && !object.responses.contains_key(&format!("{} refused", name)) {
                    warnings.push(format!(
                        "{}: has `requires:` but no refusal; add one after `<!-- thwarter:action {} refused -->`",
                        at, name
                    ));
                }
            }
        }
        warnings
    }

    /// Every chapter, root ones first, each scope in chapter order.
    pub fn chapters(&self) -> &[Chapter] {
        &self.chapters
    }

    /// One version of a thing (`design/STATES.md` › Current version):
    /// `state` or its starting state, in `chapter` (a name, resolved from
    /// the thing's folder outward) or in no chapter.
    pub fn version(&self, kind: EntityKind, id: &str, state: Option<&str>, chapter: Option<&str>) -> Result<Version> {
        let thing = self
            .thing(kind, id)
            .ok_or_else(|| anyhow::anyhow!("no {} `{}` is loaded", kind.type_name(), id))?;
        let state = match state {
            Some(name) if thing.states.contains_key(name) => Some(name.to_string()),
            Some(name) => {
                let known: Vec<&str> = thing.states.keys().map(String::as_str).collect();
                if known.is_empty() {
                    anyhow::bail!("{} `{}` has no states, so it has no state `{}`", kind.type_name(), id, name);
                }
                anyhow::bail!("{} `{}` has no state `{}` (it has {})", kind.type_name(), id, name, known.join(", "));
            }
            None => thing.starting_state.clone(),
        };
        let chapter = match chapter {
            None => None,
            Some(name) => {
                let path: Vec<String> = name.split('/').map(String::from).collect();
                let found = bundles::resolve_chapter(&self.chapters, &thing.scope, &path)
                    .or_else(|| {
                        bundles::visible_chapters(&self.chapters, &thing.scope)
                            .into_iter()
                            .find(|c| c.name() == name)
                    })
                    .ok_or_else(|| anyhow::anyhow!("no chapter `{}` can be seen from {} `{}`", name, kind.type_name(), id))?;
                Some(found.key.clone())
            }
        };
        let (mut entity, merged) = thing
            .build(state.as_deref(), chapter.as_ref())
            .map_err(|e| anyhow::anyhow!(e))?;
        self.resolve_links(&mut entity);
        Ok(Version { entity, merged, state, chapter })
    }

    /// `problems` for the world as it is now, after any in-memory changes:
    /// each file's parse error, then the same checks. No load report needed.
    pub fn current_problems(&self, strict: bool) -> Vec<String> {
        self.problems_from(self.file_problems.clone(), strict)
    }

    /// The warnings for the world as it is now: each file's (prefixed with
    /// its path), then ids shared between kinds.
    pub fn current_warnings(&self) -> Vec<String> {
        let mut warnings = self.file_warnings.clone();
        warnings.extend(self.response_warnings());
        warnings.extend(self.shared_id_warnings());
        warnings
    }

    /// The one file `(kind, id)` was loaded from. An error if it wasn't
    /// loaded, or if two files define it.
    pub fn source_path(&self, kind: EntityKind, id: &str) -> Result<&Path> {
        match self.sources.get(&(kind, id.to_string())).map(|v| v.as_slice()) {
            None | Some([]) => Err(anyhow::anyhow!("no {} `{}` is loaded", kind.type_name(), id)),
            Some([one]) => Ok(one.as_path()),
            Some(many) => Err(anyhow::anyhow!(
                "{} `{}` is defined in {} files ({}); fix the duplicate first",
                kind.type_name(),
                id,
                many.len(),
                join_paths(many)
            )),
        }
    }

    /// Every `(kind, id)` defined in more than one file, sorted.
    pub fn duplicate_ids(&self) -> Vec<(EntityKind, String, Vec<PathBuf>)> {
        let mut dups: Vec<_> = self
            .sources
            .iter()
            .filter(|(_, paths)| paths.len() > 1)
            .map(|((kind, id), paths)| {
                let mut paths = paths.clone();
                paths.sort();
                (*kind, id.clone(), paths)
            })
            .collect();
        dups.sort();
        dups
    }

    pub fn get_location(&self, location_id: &str) -> Option<&Location> {
        self.world_graph.get_location(location_id)
    }

    pub fn add_location(&mut self, location: Location) {
        self.world_graph.add_location(location);
    }

    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        self.world_graph.find_path(from, to)
    }

    pub fn get_connections(&self, location_id: &str) -> Vec<(String, String)> {
        if let Some(location) = self.get_location(location_id) {
            location
                .connections
                .iter()
                .map(|(direction, target)| (direction.clone(), target.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Locations whose connections point at a location ID that was never
    /// loaded. This is the core check behind `thwart validate`.
    pub fn broken_connections(&self) -> Vec<(String, String, String)> {
        let mut broken = Vec::new();
        for (location_id, location) in &self.world_graph.locations {
            for (direction, target) in &location.connections {
                if !self.world_graph.locations.contains_key(target) {
                    broken.push((location_id.clone(), direction.clone(), target.clone()));
                }
            }
        }
        broken.sort();
        broken
    }

    /// Every problem `validate` reports for this world: the load's parse
    /// errors, broken connections, and with `strict`, character
    /// `default_location`s that aren't known locations. Shared by
    /// `thwart validate` and the console's `/reload` (REQ-CON-7), so "fails
    /// validation" means the same thing in both.
    pub fn problems(&self, report: &WorldLoadReport, strict: bool) -> Vec<String> {
        self.problems_from(report.errors.clone(), strict)
    }

    fn problems_from(&self, errors: Vec<String>, strict: bool) -> Vec<String> {
        let mut problems = errors;
        for (kind, id, paths) in self.duplicate_ids() {
            problems.push(format!(
                "{} `{}` is defined in {} files: {}",
                kind.type_name(),
                id,
                paths.len(),
                join_paths(&paths)
            ));
        }
        problems.extend(self.action_problems());
        if let (Some(start), Some(source)) = (&self.world.start, &self.world.source) {
            if self.get_location(start).is_none() {
                problems.push(format!("{}: the starting room `{}` is not a known location", source.display(), start));
            }
        }
        for (from, direction, target) in self.broken_connections() {
            problems.push(format!(
                "{}: connection `{}` points at unknown location `{}`",
                from, direction, target
            ));
        }
        if strict {
            let mut characters: Vec<&Character> = self.characters.values().collect();
            characters.sort_by(|a, b| a.id.cmp(&b.id));
            for character in characters {
                if let Some(default_location) = &character.default_location {
                    if self.get_location(default_location).is_none() {
                        problems.push(format!(
                            "{}: default_location `{}` is not a known location",
                            character.id, default_location
                        ));
                    }
                }
            }
        }
        problems
    }

    /// A location with every reference resolved (REQ-ENG-9): exits in
    /// compass order with whether each target exists, and characters and
    /// objects with their card names, `None` where there's no card. The
    /// starting version of each; `describe_location_at` shows a game's.
    pub fn describe_location(&self, id: &str) -> Option<LocationView> {
        self.describe_location_at(id, At::default())
    }

    /// `describe_location`, with each thing in the version `at` sees.
    pub fn describe_location_at(&self, id: &str, at: At) -> Option<LocationView> {
        let location = self.location_at(id, at)?;
        let mut exits: Vec<ExitView> = location
            .connections
            .iter()
            .map(|(direction, target)| ExitView {
                direction: direction.clone(),
                target: target.clone(),
                target_exists: self.get_location(target).is_some(),
            })
            .collect();
        exits.sort_by_key(|e| direction_rank(&e.direction));

        let characters = location
            .characters
            .iter()
            .map(|cid| EntityRef {
                id: cid.clone(),
                name: self.character_at(cid, at).map(|c| c.name),
            })
            .collect();
        let objects = location
            .objects
            .iter()
            .map(|oid| EntityRef {
                id: oid.clone(),
                name: self.object_at(oid, at).map(|o| o.title),
            })
            .collect();

        Some(LocationView {
            id: location.id.clone(),
            title: location.name.clone(),
            coordinates: location.coordinates,
            body: location.description.clone(),
            exits,
            characters,
            objects,
        })
    }

    /// A thing as `at` sees it: in a game, its current state in the
    /// game's chapter; otherwise its starting version. A version that
    /// doesn't type (a problem `validate` reports) falls back to the
    /// starting one, so play never loses a thing that loaded.
    pub fn current(&self, kind: EntityKind, id: &str, at: At) -> Option<ParsedEntity> {
        let starting = || -> Option<ParsedEntity> {
            match kind {
                EntityKind::Location => self.get_location(id).cloned().map(ParsedEntity::Location),
                EntityKind::Object => self.objects.get(id).cloned().map(ParsedEntity::Object),
                EntityKind::Character => self.characters.get(id).cloned().map(ParsedEntity::Character),
            }
        };
        let Some(play) = at.play else { return starting() };
        let Some(thing) = self.thing(kind, id) else { return starting() };
        let state = play
            .state_of(kind.type_name(), id)
            .filter(|s| thing.states.contains_key(*s))
            .map(String::from)
            .or_else(|| thing.starting_state.clone());
        let chapter = play.chapter.as_ref().and_then(|c| self.chapter_key(c));
        if state == thing.starting_state && chapter.is_none() {
            return starting();
        }
        match thing.build(state.as_deref(), chapter.as_ref()) {
            Ok((mut entity, _)) => {
                self.resolve_links(&mut entity);
                Some(entity)
            }
            Err(_) => starting(),
        }
    }

    pub fn location_at(&self, id: &str, at: At) -> Option<Location> {
        match self.current(EntityKind::Location, id, at)? {
            ParsedEntity::Location(l) => Some(l),
            _ => None,
        }
    }

    pub fn object_at(&self, id: &str, at: At) -> Option<GameObject> {
        match self.current(EntityKind::Object, id, at)? {
            ParsedEntity::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn character_at(&self, id: &str, at: At) -> Option<Character> {
        match self.current(EntityKind::Character, id, at)? {
            ParsedEntity::Character(c) => Some(c),
            _ => None,
        }
    }

    /// The loaded chapter a save names, if it still exists.
    pub fn chapter_key(&self, saved: &SavedChapter) -> Option<ChapterKey> {
        let key = ChapterKey { scope: PathBuf::from(&saved.scope), path: saved.path.clone() };
        self.chapters.iter().any(|c| c.key == key).then_some(key)
    }

    /// How a save names a loaded chapter.
    pub fn saved_chapter(&self, chapter: &Chapter) -> SavedChapter {
        SavedChapter {
            scope: chapter.key.scope.to_string_lossy().replace('\\', "/"),
            path: chapter.key.path.clone(),
            order: chapter.order.clone(),
        }
    }

    /// The chapter `name` names as seen from `scope` (a folder relative to
    /// the world root; empty for the root): a sub-chapter path
    /// (`after-the-king/the-funeral`) or a bare name.
    pub fn find_chapter(&self, scope: &Path, name: &str) -> Option<&Chapter> {
        let path: Vec<String> = name.split('/').map(String::from).collect();
        bundles::resolve_chapter(&self.chapters, scope, &path)
            .or_else(|| bundles::visible_chapters(&self.chapters, scope).into_iter().find(|c| c.name() == name))
    }
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

impl Default for WorldEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> WorldEngine {
        let mut engine = WorldEngine::new();
        let mut yard = Location::new("castle/yard".into(), "Yard".into(), "Grass.".into());
        yard.connections.insert("west".into(), "castle/hall".into());
        yard.connections.insert("north".into(), "castle/gone".into());
        yard.connections.insert("behind".into(), "castle/hall".into());
        yard.characters = vec!["king".into(), "ghost".into()];
        yard.objects = vec!["well".into(), "grass".into()];
        engine.add_location(yard);
        engine.add_location(Location::new("castle/hall".into(), "Hall".into(), "Stone.".into()));
        let mut king = Character::new("king".into(), "The King".into(), "Old.".into());
        king.default_location = Some("castle/throne".into());
        engine.characters.insert("king".into(), king);
        engine.objects.insert(
            "well".into(),
            GameObject::new("well".into(), "Old Well".into(), "Deep.".into(), "environmental".into()),
        );
        engine
    }

    #[test]
    fn describe_location_resolves_references() {
        let view = engine().describe_location("castle/yard").unwrap();
        assert_eq!(view.title, "Yard");
        let exits: Vec<(&str, bool)> = view
            .exits
            .iter()
            .map(|e| (e.direction.as_str(), e.target_exists))
            .collect();
        assert_eq!(exits, vec![("north", false), ("west", true), ("behind", true)]);
        assert_eq!(view.characters[0].name.as_deref(), Some("The King"));
        assert_eq!(view.characters[1].name, None);
        assert_eq!(view.objects[0].name.as_deref(), Some("Old Well"));
        assert_eq!(view.objects[1].name, None);
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn load_records_sources_and_reports_duplicate_ids() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "castle/yard.md",
            "---\nid: castle/yard\ntitle: Yard\ntype: location\n---\n",
        );
        write(
            dir.path(),
            "npcs/king.md",
            "---\nid: king\ntitle: King\ntype: character\n---\n",
        );
        write(
            dir.path(),
            "npcs/king-copy.md",
            "---\nid: king\ntitle: King 2\ntype: character\n---\n",
        );
        // Same id, different kind: not a duplicate.
        write(
            dir.path(),
            "objects/yard.md",
            "---\nid: castle/yard\ntitle: Yard\ntype: object\nobject_type: x\n---\n",
        );

        let mut engine = WorldEngine::new();
        let report = engine.load_world(dir.path()).unwrap();

        let yard = engine.source_path(EntityKind::Location, "castle/yard").unwrap();
        assert!(yard.ends_with("castle/yard.md"));
        assert!(engine
            .source_path(EntityKind::Object, "castle/yard")
            .unwrap()
            .ends_with("objects/yard.md"));
        assert!(engine
            .source_path(EntityKind::Location, "nowhere")
            .unwrap_err()
            .to_string()
            .contains("no location `nowhere`"));
        let dup_err = engine
            .source_path(EntityKind::Character, "king")
            .unwrap_err()
            .to_string();
        assert!(dup_err.contains("defined in 2 files"), "{}", dup_err);

        let dups = engine.duplicate_ids();
        assert_eq!(dups.len(), 1);
        assert_eq!((dups[0].0, dups[0].1.as_str()), (EntityKind::Character, "king"));

        let problems = engine.problems(&report, false);
        assert_eq!(problems.len(), 1, "{:?}", problems);
        assert!(problems[0].starts_with("character `king` is defined in 2 files: "));
    }

    fn small_world() -> (tempfile::TempDir, WorldEngine, WorldLoadReport) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "castle/yard.md", "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  north: castle/hall\n---\nGrass.\n");
        write(dir.path(), "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\n---\n");
        write(dir.path(), "npcs/king.md", "---\nid: king\ntitle: King\ntype: character\ndefault_location: castle/hall\n---\n");
        write(dir.path(), "broken.md", "no frontmatter\n");
        let mut engine = WorldEngine::new();
        let report = engine.load_world(dir.path()).unwrap();
        (dir, engine, report)
    }

    #[test]
    fn current_problems_match_a_fresh_load() {
        let (_dir, engine, report) = small_world();
        assert_eq!(engine.current_problems(true), engine.problems(&report, true));
        assert_eq!(engine.current_problems(false).len(), 1); // broken.md
        assert_eq!(engine.current_warnings(), report.warnings);
    }

    #[test]
    fn set_file_changes_an_entity_and_its_problems_in_memory() {
        let (dir, mut engine, _) = small_world();
        let yard = dir.path().join("castle/yard.md");
        engine.set_file(
            &yard,
            "---\nid: castle/yard\ntitle: New Yard\ntype: location\nconnections:\n  north: castle/hall\n  west: castle/gone\n---\n",
        );
        assert_eq!(engine.get_location("castle/yard").unwrap().name, "New Yard");
        assert_eq!(engine.loaded_locations["castle/yard"].name, "New Yard");
        let problems = engine.current_problems(false);
        assert!(problems.iter().any(|p| p.contains("`west` points at unknown location `castle/gone`")), "{:?}", problems);
        // Nothing was written.
        assert!(std::fs::read_to_string(&yard).unwrap().contains("title: Yard\n"));

        // A new file adds the missing room and fixes the exit.
        engine.set_file(&dir.path().join("castle/gone.md"), "---\nid: castle/gone\ntitle: Gone\ntype: location\n---\n");
        assert!(engine.current_problems(false).iter().all(|p| !p.contains("castle/gone")));
        assert!(engine.source_path(EntityKind::Location, "castle/gone").is_ok());
    }

    #[test]
    fn a_file_that_stops_parsing_drops_its_entity_and_reports_why() {
        let (dir, mut engine, _) = small_world();
        let hall = dir.path().join("castle/hall.md");
        engine.set_file(&hall, "---\nid: castle/hall\ntype: location\n---\n");
        assert!(engine.get_location("castle/hall").is_none());
        assert!(engine.source_path(EntityKind::Location, "castle/hall").is_err());
        let problems = engine.current_problems(true);
        assert!(problems.iter().any(|p| p.contains("hall.md") && p.contains("`title`")), "{:?}", problems);
        assert!(problems.iter().any(|p| p.contains("`north` points at unknown location `castle/hall`")));
        assert!(problems.iter().any(|p| p.starts_with("king: default_location `castle/hall`")));

        // Fixing it brings everything back.
        engine.set_file(&hall, "---\nid: castle/hall\ntitle: Hall\ntype: location\n---\n");
        assert_eq!(engine.current_problems(true).len(), 1); // only broken.md
    }

    #[test]
    fn duplicates_and_removal_resolve_like_a_load() {
        let (dir, mut engine, _) = small_world();
        let copy = dir.path().join("castle/yard-copy.md");
        engine.set_file(&copy, "---\nid: castle/yard\ntitle: Copy\ntype: location\n---\n");
        assert_eq!(engine.get_location("castle/yard").unwrap().name, "Copy"); // last one wins
        assert!(engine.current_problems(false).iter().any(|p| p.contains("defined in 2 files")));
        assert!(engine.source_path(EntityKind::Location, "castle/yard").is_err());

        engine.remove_file(&copy);
        assert_eq!(engine.get_location("castle/yard").unwrap().name, "Yard");
        assert!(engine.source_path(EntityKind::Location, "castle/yard").is_ok());

        engine.remove_file(&dir.path().join("npcs/king.md"));
        assert!(!engine.characters.contains_key("king"));
        assert!(!engine.sources.contains_key(&(EntityKind::Character, "king".to_string())));
    }

    #[test]
    fn a_file_whose_id_changes_moves_its_entity() {
        let (dir, mut engine, _) = small_world();
        engine.set_file(&dir.path().join("castle/hall.md"), "---\nid: castle/great-hall\ntitle: Great Hall\ntype: location\n---\n");
        assert!(engine.get_location("castle/hall").is_none());
        assert_eq!(engine.get_location("castle/great-hall").unwrap().name, "Great Hall");
    }

    #[test]
    fn ids_shared_between_kinds_are_warnings_not_problems() {
        let (dir, mut engine, report) = small_world();
        assert!(engine.shared_ids().is_empty());
        assert!(report.warnings.is_empty());
        engine.set_file(&dir.path().join("objects/hall.md"), "---\nid: castle/hall\ntitle: Hall Bell\ntype: object\nobject_type: x\n---\n");
        assert_eq!(engine.shared_ids(), vec![("castle/hall".to_string(), vec![EntityKind::Location, EntityKind::Object])]);
        let warnings = engine.current_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("id `castle/hall` is used by more than one kind (location, object)"), "{}", warnings[0]);
        assert_eq!(engine.current_problems(false).len(), 1); // still only broken.md

        // On a load, it's in the report's warnings too.
        let mut fresh = WorldEngine::new();
        write(dir.path(), "objects/hall.md", "---\nid: castle/hall\ntitle: Hall Bell\ntype: object\nobject_type: x\n---\n");
        let report = fresh.load_world(dir.path()).unwrap();
        assert_eq!(report.warnings, fresh.current_warnings());
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn describe_location_unknown_id_is_none() {
        assert!(engine().describe_location("castle/nowhere").is_none());
    }

    #[test]
    fn problems_include_load_errors_broken_exits_and_strict_checks() {
        let engine = engine();
        let report = WorldLoadReport {
            errors: vec!["bad.md: no type".into()],
            ..Default::default()
        };
        let loose = engine.problems(&report, false);
        assert_eq!(loose.len(), 2);
        assert_eq!(loose[0], "bad.md: no type");
        assert!(loose[1].contains("`north` points at unknown location `castle/gone`"));
        let strict = engine.problems(&report, true);
        assert_eq!(strict.len(), 3);
        assert!(strict[2].starts_with("king: default_location `castle/throne`"));
    }

    #[test]
    fn obsidian_links_name_things_by_id_or_by_file() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\nconnections:\n  south: \"[[castle/yard]]\"\n  down: \"[[castle/vault/index|the vault]]\"\n  up: [[castle/tower.md]]\nobjects: [\"[[objects/lever]]\", \"[[lamp]]\"]\ncharacters: [\"[[npcs/king]]\"]\n---\n");
        write(d, "castle/yard.md", "---\nid: castle/yard\ntitle: Yard\ntype: location\n---\n");
        write(d, "castle/tower.md", "---\nid: castle/tower\ntitle: Tower\ntype: location\n---\n");
        write(d, "castle/vault/index.md", "---\nid: castle/vault\ntitle: Vault\ntype: location\nstate: shut\n---\n");
        write(d, "castle/vault/shut.md", "---\ninherits: index\n---\n");
        write(d, "castle/vault/open.md", "---\ninherits: index\n---\n");
        write(d, "objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull:\n    becomes: { \"[[castle/vault]]\": open }\n    requires: { has: \"[[objects/lamp]]\" }\n---\n<!-- thwarter:action pull -->\nClunk.\n<!-- thwarter:action pull refused -->\nDark.\n");
        write(d, "objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: x\n---\n");
        write(d, "npcs/king.md", "---\nid: king\ntitle: King\ntype: character\ndefault_location: \"[[castle/hall]]\"\ninventory: [\"[[objects/lamp.md]]\"]\n---\n");
        write(d, "world/index.md", "---\ntitle: T\ntype: world\nstart: \"[[castle/hall]]\"\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        assert!(engine.current_problems(true).is_empty(), "{:?}", engine.current_problems(true));

        let hall = engine.get_location("castle/hall").unwrap();
        assert_eq!(hall.connections["south"], "castle/yard");
        assert_eq!(hall.connections["down"], "castle/vault");
        assert_eq!(hall.connections["up"], "castle/tower");
        assert_eq!(hall.objects, vec!["lever", "lamp"]);
        assert_eq!(hall.characters, vec!["king"]);
        let king = &engine.characters["king"];
        assert_eq!(king.default_location.as_deref(), Some("castle/hall"));
        assert_eq!(king.inventory, vec!["lamp"]);
        let pull = &engine.objects["lever"].actions["pull"];
        assert_eq!(pull.becomes["castle/vault"], "open");
        assert_eq!(pull.requires_has, vec!["lamp"]);
        assert_eq!(engine.world().start.as_deref(), Some("castle/hall"));

        // A link to nothing is reported like any unknown id.
        engine.set_file(&d.join("castle/yard.md"), "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  east: \"[[castle/nowhere]]\"\n---\n");
        assert!(engine.current_problems(false).iter().any(|p| p.contains("`east` points at unknown location `castle/nowhere`")));
    }
}
