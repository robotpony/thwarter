use crate::models::narrative::*;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NarrativeEngine {
    pub game_state: GameState,
    pub plot_points: HashMap<String, PlotPoint>,
    pub narrative_arcs: HashMap<String, NarrativeArc>,
}

impl NarrativeEngine {
    pub fn new() -> Self {
        Self {
            game_state: GameState::new(),
            plot_points: HashMap::new(),
            narrative_arcs: HashMap::new(),
        }
    }
    
    pub fn add_plot_point(&mut self, plot_point: PlotPoint) {
        self.plot_points.insert(plot_point.id.clone(), plot_point);
    }
    
    pub fn add_narrative_arc(&mut self, arc: NarrativeArc) {
        self.narrative_arcs.insert(arc.id.clone(), arc);
    }
    
    pub fn set_variable(&mut self, variable: StoryVariable) {
        self.game_state.set_variable(variable);
    }
    
    pub fn get_variable(&self, name: &str) -> Option<&StoryVariable> {
        self.game_state.get_variable(name)
    }
    
    pub fn trigger_plot_point(&mut self, plot_point_id: &str) -> Result<()> {
        if let Some(plot_point) = self.plot_points.get(plot_point_id) {
            if plot_point.can_trigger(&self.game_state.completed_plot_points) {
                self.game_state.complete_plot_point(plot_point_id.to_string());
                // TODO: Execute plot point consequences
                Ok(())
            } else {
                Err(anyhow::anyhow!("Plot point prerequisites not met: {}", plot_point_id))
            }
        } else {
            Err(anyhow::anyhow!("Plot point not found: {}", plot_point_id))
        }
    }
    
    pub fn check_narrative_progress(&mut self) {
        // TODO: Check and update narrative arc progress based on completed plot points
    }
    
    pub fn evaluate_alternative_text(&self, conditions: &str) -> bool {
        // TODO: Implement condition evaluation for alternative text
        true
    }
}

impl Default for NarrativeEngine {
    fn default() -> Self {
        Self::new()
    }
}