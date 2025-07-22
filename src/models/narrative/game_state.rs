use crate::models::narrative::StoryVariable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub variables: HashMap<String, StoryVariable>,
    pub completed_plot_points: Vec<String>,
    pub active_narrative_arcs: Vec<String>,
    pub player_progress: HashMap<String, bool>,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            completed_plot_points: Vec::new(),
            active_narrative_arcs: Vec::new(),
            player_progress: HashMap::new(),
        }
    }
    
    pub fn set_variable(&mut self, variable: StoryVariable) {
        self.variables.insert(variable.name.clone(), variable);
    }
    
    pub fn get_variable(&self, name: &str) -> Option<&StoryVariable> {
        self.variables.get(name)
    }
    
    pub fn complete_plot_point(&mut self, plot_point_id: String) {
        if !self.completed_plot_points.contains(&plot_point_id) {
            self.completed_plot_points.push(plot_point_id);
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}