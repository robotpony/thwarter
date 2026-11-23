use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub action_type: ActionType,
    pub target: Option<String>,
    pub location: Option<String>,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Move,
    Take,
    Drop,
    Use,
    Examine,
    Talk,
    Look,
    Custom(String),
}

impl Action {
    pub fn new(action_type: ActionType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action_type,
            target: None,
            location: None,
            parameters: Vec::new(),
        }
    }
    
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }
    
    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }
}