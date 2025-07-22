use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterState {
    pub character_id: String,
    pub current_location: String,
    pub mood: String,
    pub health: i32,
    pub dynamic_attributes: HashMap<String, String>,
}

impl CharacterState {
    pub fn new(character_id: String, location: String) -> Self {
        Self {
            character_id,
            current_location: location,
            mood: "neutral".to_string(),
            health: 100,
            dynamic_attributes: HashMap::new(),
        }
    }
    
    pub fn set_mood(&mut self, mood: String) {
        self.mood = mood;
    }
    
    pub fn move_to(&mut self, location: String) {
        self.current_location = location;
    }
}