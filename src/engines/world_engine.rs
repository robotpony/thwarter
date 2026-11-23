use crate::engines::location_view::{direction_rank, EntityRef, ExitView, LocationView};
use crate::models::character::Character;
use crate::models::world::*;
use crate::parsers::{MarkdownParser, ParsedEntity};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct WorldEngine {
    pub world_graph: WorldGraph,
    pub loaded_locations: HashMap<String, Location>,
    pub objects: HashMap<String, GameObject>,
    /// Character cards by id, as loaded. A read-only catalog for lookups
    /// and validation; placement and state belong to `CharacterEngine`.
    pub characters: HashMap<String, Character>,
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
                Ok(ParsedEntity::Location(location)) => {
                    self.world_graph.add_location(location);
                    report.locations_loaded += 1;
                }
                Ok(ParsedEntity::Object(object)) => {
                    self.objects.insert(object.id.clone(), object);
                    report.objects_loaded += 1;
                }
                Ok(ParsedEntity::Character(character)) => {
                    self.characters.insert(character.id.clone(), character.clone());
                    report.characters.push(character);
                }
                Err(e) => {
                    report.errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        self.loaded_locations = self.world_graph.locations.clone();
        Ok(report)
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
        for (from, direction, target) in self.broken_connections() {
            problems.push(format!("{}: connection `{}` points at unknown location `{}`", from, direction, target));
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
            .map(|cid| EntityRef { id: cid.clone(), name: self.characters.get(cid).map(|c| c.name.clone()) })
            .collect();
        let objects = location
            .objects
            .iter()
            .map(|oid| EntityRef { id: oid.clone(), name: self.objects.get(oid).map(|o| o.title.clone()) })
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
        let exits: Vec<(&str, bool)> = view.exits.iter().map(|e| (e.direction.as_str(), e.target_exists)).collect();
        assert_eq!(exits, vec![("north", false), ("west", true), ("behind", true)]);
        assert_eq!(view.characters[0].name.as_deref(), Some("The King"));
        assert_eq!(view.characters[1].name, None);
        assert_eq!(view.objects[0].name.as_deref(), Some("Old Well"));
        assert_eq!(view.objects[1].name, None);
    }

    #[test]
    fn describe_location_unknown_id_is_none() {
        assert!(engine().describe_location("castle/nowhere").is_none());
    }

    #[test]
    fn problems_include_load_errors_broken_exits_and_strict_checks() {
        let engine = engine();
        let report = WorldLoadReport { errors: vec!["bad.md: no type".into()], ..Default::default() };
        let loose = engine.problems(&report, false);
        assert_eq!(loose.len(), 2);
        assert_eq!(loose[0], "bad.md: no type");
        assert!(loose[1].contains("`north` points at unknown location `castle/gone`"));
        let strict = engine.problems(&report, true);
        assert_eq!(strict.len(), 3);
        assert!(strict[2].starts_with("king: default_location `castle/throne`"));
    }
}
