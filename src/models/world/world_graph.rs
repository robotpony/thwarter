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
    
    /// The shortest way between two rooms by their direct exits, as room
    /// ids; `None` if there's none. `WorldEngine::find_path` also goes
    /// through doors.
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if !self.locations.contains_key(from) || !self.locations.contains_key(to) {
            return None;
        }
        let mut came_from: HashMap<&str, &str> = HashMap::new();
        let mut queue = std::collections::VecDeque::from([from]);
        let mut seen = std::collections::HashSet::from([from]);
        while let Some(room) = queue.pop_front() {
            if room == to {
                let mut path = vec![room.to_string()];
                let mut at = room;
                while let Some(prev) = came_from.get(at) {
                    path.push(prev.to_string());
                    at = prev;
                }
                path.reverse();
                return Some(path);
            }
            let Some(location) = self.locations.get(room) else { continue };
            for next in location.connections.values() {
                if self.locations.contains_key(next) && seen.insert(next.as_str()) {
                    came_from.insert(next.as_str(), room);
                    queue.push_back(next.as_str());
                }
            }
        }
        None
    }
}

impl Default for WorldGraph {
    fn default() -> Self {
        Self::new()
    }
}