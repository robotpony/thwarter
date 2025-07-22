use crate::models::world::Location;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WorldGraph {
    pub locations: HashMap<String, Location>,
}

impl WorldGraph {
    pub fn new() -> Self {
        Self {
            locations: HashMap::new(),
        }
    }
    
    pub fn add_location(&mut self, location: Location) {
        self.locations.insert(location.id.clone(), location);
    }
    
    pub fn get_location(&self, id: &str) -> Option<&Location> {
        self.locations.get(id)
    }
    
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        // TODO: Implement pathfinding algorithm
        None
    }
}

impl Default for WorldGraph {
    fn default() -> Self {
        Self::new()
    }
}