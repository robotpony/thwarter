use crate::models::world::*;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WorldEngine {
    pub world_graph: WorldGraph,
    pub loaded_locations: HashMap<String, Location>,
}

impl WorldEngine {
    pub fn new() -> Self {
        Self {
            world_graph: WorldGraph::new(),
            loaded_locations: HashMap::new(),
        }
    }
    
    pub fn load_world<P: AsRef<Path>>(&mut self, world_path: P) -> Result<()> {
        // TODO: Implement world loading from filesystem
        Ok(())
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
            location.connections.iter()
                .map(|(direction, target)| (direction.clone(), target.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for WorldEngine {
    fn default() -> Self {
        Self::new()
    }
}