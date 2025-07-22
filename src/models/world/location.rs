use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub description: String,
    pub coordinates: Option<(i32, i32, i32)>,
    pub connections: HashMap<String, String>,
    pub characters: Vec<String>,
    pub objects: Vec<String>,
}

impl Location {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            coordinates: None,
            connections: HashMap::new(),
            characters: Vec::new(),
            objects: Vec::new(),
        }
    }
}