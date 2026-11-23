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
}

impl WorldEngine {
    pub fn new() -> Self {
        Self {
            world_graph: WorldGraph::new(),
            loaded_locations: HashMap::new(),
            objects: HashMap::new(),
            characters: HashMap::new(),
            sources: HashMap::new(),
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

        for entry in WalkDir::new(&world_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            match parser.parse_file(path) {
                Ok(entity) => {
                    let key = match &entity {
                        ParsedEntity::Location(l) => (EntityKind::Location, l.id.clone()),
                        ParsedEntity::Character(c) => (EntityKind::Character, c.id.clone()),
                        ParsedEntity::Object(o) => (EntityKind::Object, o.id.clone()),
                    };
                    self.sources.entry(key).or_default().push(path.to_path_buf());
                    self.add_parsed(entity, &mut report);
                }
                Err(e) => {
                    report.errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        self.loaded_locations = self.world_graph.locations.clone();
        Ok(report)
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
        let mut problems = report.errors.clone();
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
