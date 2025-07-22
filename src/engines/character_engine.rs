use crate::models::character::*;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CharacterEngine {
    pub characters: HashMap<String, Character>,
    pub character_states: HashMap<String, CharacterState>,
}

impl CharacterEngine {
    pub fn new() -> Self {
        Self {
            characters: HashMap::new(),
            character_states: HashMap::new(),
        }
    }
    
    pub fn add_character(&mut self, character: Character) {
        let character_id = character.id.clone();
        let default_location = character.default_location.clone().unwrap_or_else(|| "start".to_string());
        
        let state = CharacterState::new(character_id.clone(), default_location);
        self.character_states.insert(character_id.clone(), state);
        self.characters.insert(character_id, character);
    }
    
    pub fn get_character(&self, character_id: &str) -> Option<&Character> {
        self.characters.get(character_id)
    }
    
    pub fn get_character_state(&self, character_id: &str) -> Option<&CharacterState> {
        self.character_states.get(character_id)
    }
    
    pub fn move_character(&mut self, character_id: &str, location: String) -> Result<()> {
        if let Some(state) = self.character_states.get_mut(character_id) {
            state.move_to(location);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Character not found: {}", character_id))
        }
    }
    
    pub fn get_characters_at_location(&self, location_id: &str) -> Vec<&Character> {
        self.character_states.iter()
            .filter(|(_, state)| state.current_location == location_id)
            .filter_map(|(id, _)| self.characters.get(id))
            .collect()
    }
}

impl Default for CharacterEngine {
    fn default() -> Self {
        Self::new()
    }
}