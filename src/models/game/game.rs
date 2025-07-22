use crate::models::character::Character;
use crate::models::game::GameSession;
use crate::engines::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub title: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub world_path: String,
    pub starting_location: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug)]
pub struct GameInstance {
    pub game: Game,
    pub world_engine: WorldEngine,
    pub character_engine: CharacterEngine,
    pub narrative_engine: NarrativeEngine,
    pub game_engine: GameEngine,
    pub sessions: HashMap<String, GameSession>,
}

impl Game {
    pub fn new(title: String, author: String, world_path: String) -> Self {
        Self {
            title,
            author,
            version: "1.0.0".to_string(),
            description: String::new(),
            world_path,
            starting_location: "start".to_string(),
            metadata: HashMap::new(),
        }
    }
}

impl GameInstance {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            world_engine: WorldEngine::new(),
            character_engine: CharacterEngine::new(),
            narrative_engine: NarrativeEngine::new(),
            game_engine: GameEngine::new(),
            sessions: HashMap::new(),
        }
    }
    
    pub fn create_session(&mut self, player_name: String) -> String {
        let character = Character::new(
            format!("player_{}", player_name),
            player_name,
            "Player character".to_string(),
        );
        let player = crate::models::character::Player::new(
            character,
            self.game.starting_location.clone(),
        );
        let session = GameSession::new(player);
        let session_id = session.id.clone();
        self.sessions.insert(session_id.clone(), session);
        session_id
    }
    
    pub fn get_session(&self, session_id: &str) -> Option<&GameSession> {
        self.sessions.get(session_id)
    }
    
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut GameSession> {
        self.sessions.get_mut(session_id)
    }
}