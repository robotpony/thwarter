use crate::models::character::Player;
use crate::models::narrative::GameState;
use crate::models::game::Action;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub id: String,
    pub player: Player,
    pub game_state: GameState,
    pub action_history: Vec<Action>,
    pub session_data: HashMap<String, String>,
    pub created_at: String,
    pub last_updated: String,
}

impl GameSession {
    pub fn new(player: Player) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            player,
            game_state: GameState::new(),
            action_history: Vec::new(),
            session_data: HashMap::new(),
            created_at: now.clone(),
            last_updated: now,
        }
    }
    
    pub fn add_action(&mut self, action: Action) {
        self.action_history.push(action);
        self.last_updated = chrono::Utc::now().to_rfc3339();
    }
    
    pub fn get_current_location(&self) -> &str {
        &self.player.current_location
    }
    
    pub fn move_player(&mut self, location: String) {
        self.player.move_to(location);
        self.last_updated = chrono::Utc::now().to_rfc3339();
    }
}