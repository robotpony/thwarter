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
}

/// Summary of a `load_world` pass: what was loaded, and every file that
/// failed to parse. A partial load is not a hard error; callers (the
/// `validate` and `show` commands in particular) need to see both what
/// loaded and what didn't in the same report.
#[derive(Debug, Clone, Default)]
pub struct WorldLoadReport {
    pub locations_loaded: usize,
    pub objects_loaded: usize,
    /// Characters are returned rather than stored here: character
    /// placement and state belong to `CharacterEngine`, per
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
}

impl Default for WorldEngine {
    fn default() -> Self {
        Self::new()
    }
}
