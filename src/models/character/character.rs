use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub description: String,
    pub character_type: CharacterType,
    pub default_location: Option<String>,
    pub attributes: HashMap<String, i32>,
    pub dialogue_tree: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterType {
    Npc,
    Player,
    Villain,
    Hero,
}

impl Character {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            character_type: CharacterType::Npc,
            default_location: None,
            attributes: HashMap::new(),
            dialogue_tree: None,
        }
    }
}