use crate::models::character::Character;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub character: Character,
    pub inventory: Vec<String>,
    pub current_location: String,
    pub progress: HashMap<String, bool>,
}

impl Player {
    pub fn new(character: Character, starting_location: String) -> Self {
        Self {
            character,
            inventory: Vec::new(),
            current_location: starting_location,
            progress: HashMap::new(),
        }
    }
    
    pub fn add_to_inventory(&mut self, item: String) {
        self.inventory.push(item);
    }
    
    pub fn move_to(&mut self, location: String) {
        self.current_location = location;
    }
}