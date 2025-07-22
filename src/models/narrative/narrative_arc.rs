use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeArc {
    pub id: String,
    pub title: String,
    pub description: String,
    pub theme: String,
    pub plot_points: Vec<String>,
    pub current_stage: usize,
    pub completion_percentage: f32,
}

impl NarrativeArc {
    pub fn new(id: String, title: String, description: String, theme: String) -> Self {
        Self {
            id,
            title,
            description,
            theme,
            plot_points: Vec::new(),
            current_stage: 0,
            completion_percentage: 0.0,
        }
    }
    
    pub fn add_plot_point(&mut self, plot_point_id: String) {
        self.plot_points.push(plot_point_id);
    }
    
    pub fn advance_stage(&mut self) {
        if self.current_stage < self.plot_points.len() {
            self.current_stage += 1;
            self.completion_percentage = (self.current_stage as f32 / self.plot_points.len() as f32) * 100.0;
        }
    }
    
    pub fn is_complete(&self) -> bool {
        self.current_stage >= self.plot_points.len()
    }
}