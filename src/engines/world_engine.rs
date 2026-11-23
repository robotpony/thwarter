use crate::engines::location_view::{direction_rank, EntityRef, ExitView, LocationView};
use crate::models::character::Character;
use crate::models::world::*;
use crate::parsers::{MarkdownParser, ParsedEntity};
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
    /// Every file each `(kind, id)` was loaded from, in load order. More
    /// than one file means a duplicate id: the last one loaded wins in
    /// the tables above, `problems` reports it, and the writer refuses to
    /// edit it (REQ-ENG-7).
    pub sources: HashMap<(EntityKind, String), Vec<PathBuf>>,
    /// The directory `load_world` walked, as it was given.
    pub world_root: Option<PathBuf>,
    /// Each file's parse result, in load order. The tables above are kept
    /// in step with this, so an in-memory change (REQ-ENG-8) resolves
    /// duplicates and errors exactly as a fresh load would.
    files: Vec<FileRecord>,
}

#[derive(Debug, Clone)]
struct FileRecord {
    path: PathBuf,
    /// The entity, or the parse error, without the path prefix.
    parsed: std::result::Result<ParsedEntity, String>,
    warnings: Vec<String>,
}

impl FileRecord {
    fn key(&self) -> Option<(EntityKind, String)> {
        self.parsed.as_ref().ok().map(entity_key)
    }
}

fn entity_key(entity: &ParsedEntity) -> (EntityKind, String) {
    match entity {
        ParsedEntity::Location(l) => (EntityKind::Location, l.id.clone()),
        ParsedEntity::Character(c) => (EntityKind::Character, c.id.clone()),
        ParsedEntity::Object(o) => (EntityKind::Object, o.id.clone()),
    }
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
    /// case from a known one (prefixed with its file), or an id shared
    /// between kinds.
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
            files: Vec::new(),
        }
    }

    /// Walk every `.md` file under `world_path`, parse it per
    /// `design/CONTENT_FORMAT.md`, and load locations and objects into
    /// this engine. Files that fail to parse are collected as errors
    /// rather than aborting the whole load, so `validate` can report every
    /// problem in one pass instead of stopping at the first one.
    pub fn load_world<P: AsRef<Path>>(&mut self, world_path: P) -> Result<WorldLoadReport> {
        let parser = MarkdownParser::new();
        let mut report = WorldLoadReport::default();
        self.world_root = Some(world_path.as_ref().to_path_buf());

        for entry in WalkDir::new(&world_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let record = match parser.parse_file_with_warnings(path) {
                Ok(outcome) => FileRecord { path: path.to_path_buf(), parsed: Ok(outcome.entity), warnings: outcome.warnings },
                Err(e) => FileRecord { path: path.to_path_buf(), parsed: Err(e.to_string()), warnings: Vec::new() },
            };
            for warning in &record.warnings {
                report.warnings.push(format!("{}: {}", path.display(), warning));
            }
            match &record.parsed {
                Ok(entity) => {
                    let key = entity_key(entity);
                    self.sources.entry(key).or_default().push(path.to_path_buf());
                    self.add_parsed(entity.clone(), &mut report);
                }
                Err(e) => report.errors.push(format!("{}: {}", path.display(), e)),
            }
            self.files.push(record);
        }

        self.loaded_locations = self.world_graph.locations.clone();
        report.warnings.extend(self.shared_id_warnings());
        Ok(report)
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

    fn add_parsed(&mut self, entity: ParsedEntity, report: &mut WorldLoadReport) {
        match entity {
            ParsedEntity::Location(location) => {
                self.world_graph.add_location(location);
                report.locations_loaded += 1;
            }
            ParsedEntity::Object(object) => {
                self.objects.insert(object.id.clone(), object);
                report.objects_loaded += 1;
            }
            ParsedEntity::Character(character) => {
                self.characters.insert(character.id.clone(), character.clone());
                report.characters.push(character);
            }
        }
    }

    /// Change the loaded world in memory (REQ-ENG-8): replace whatever
    /// `path` defined with what `text` defines, as if the file had been
    /// loaded with that text. A parse error becomes that file's problem,
    /// as it would on a load. Nothing is read from or written to disk.
    /// `path` is in the form `load_world` uses (its root joined with the
    /// file's relative path), the same form as `sources`.
    pub fn set_file(&mut self, path: &Path, text: &str) {
        let record = match MarkdownParser::new().parse_content_with_warnings(text, path) {
            Ok(outcome) => FileRecord { path: path.to_path_buf(), parsed: Ok(outcome.entity), warnings: outcome.warnings },
            Err(e) => FileRecord { path: path.to_path_buf(), parsed: Err(e.to_string()), warnings: Vec::new() },
        };
        let new_key = record.key();
        let old_key = match self.files.iter_mut().find(|r| r.path == path) {
            Some(existing) => std::mem::replace(existing, record).key(),
            None => {
                self.files.push(record);
                None
            }
        };
        for key in [old_key, new_key].into_iter().flatten() {
            self.refresh_key(&key);
        }
    }

    /// Forget a file and everything it defined, in memory only.
    pub fn remove_file(&mut self, path: &Path) {
        if let Some(i) = self.files.iter().position(|r| r.path == path) {
            let removed = self.files.remove(i);
            if let Some(key) = removed.key() {
                self.refresh_key(&key);
            }
        }
    }

    /// Recompute one `(kind, id)` from the file records: its sources in
    /// load order, and its table entry from the last file defining it.
    fn refresh_key(&mut self, key: &(EntityKind, String)) {
        let defining: Vec<&FileRecord> = self.files.iter().filter(|r| r.key().as_ref() == Some(key)).collect();
        let paths: Vec<PathBuf> = defining.iter().map(|r| r.path.clone()).collect();
        let winner = defining.last().and_then(|r| r.parsed.as_ref().ok()).cloned();
        if paths.is_empty() {
            self.sources.remove(key);
        } else {
            self.sources.insert(key.clone(), paths);
        }
        let id = &key.1;
        match (key.0, winner) {
            (EntityKind::Location, Some(ParsedEntity::Location(l))) => {
                self.loaded_locations.insert(id.clone(), l.clone());
                self.world_graph.add_location(l);
            }
            (EntityKind::Location, _) => {
                self.world_graph.locations.remove(id);
                self.loaded_locations.remove(id);
            }
            (EntityKind::Character, Some(ParsedEntity::Character(c))) => {
                self.characters.insert(id.clone(), c);
            }
            (EntityKind::Character, _) => {
                self.characters.remove(id);
            }
            (EntityKind::Object, Some(ParsedEntity::Object(o))) => {
                self.objects.insert(id.clone(), o);
            }
            (EntityKind::Object, _) => {
                self.objects.remove(id);
            }
        }
    }

    /// `problems` for the world as it is now, after any in-memory changes:
    /// each file's parse error, then the same checks. No load report needed.
    pub fn current_problems(&self, strict: bool) -> Vec<String> {
        let errors = self
            .files
            .iter()
            .filter_map(|r| r.parsed.as_ref().err().map(|e| format!("{}: {}", r.path.display(), e)))
            .collect();
        self.problems_from(errors, strict)
    }

    /// The warnings for the world as it is now: each file's (prefixed with
    /// its path), then ids shared between kinds.
    pub fn current_warnings(&self) -> Vec<String> {
        let mut warnings: Vec<String> = self
            .files
            .iter()
            .flat_map(|r| r.warnings.iter().map(move |w| format!("{}: {}", r.path.display(), w)))
            .collect();
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
    /// objects with their card names, `None` where there's no card.
    pub fn describe_location(&self, id: &str) -> Option<LocationView> {
        let location = self.get_location(id)?;
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
                name: self.characters.get(cid).map(|c| c.name.clone()),
            })
            .collect();
        let objects = location
            .objects
            .iter()
            .map(|oid| EntityRef {
                id: oid.clone(),
                name: self.objects.get(oid).map(|o| o.title.clone()),
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
}
