use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotPoint {
    pub id: String,
    pub title: String,
    pub description: String,
    pub prerequisites: Vec<String>,
    pub triggers: Vec<PlotTrigger>,
    pub consequences: Vec<PlotConsequence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotTrigger {
    pub condition: String,
    pub location: Option<String>,
    pub character: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotConsequence {
    pub action: String,
    pub target: String,
    pub value: String,
}

impl PlotPoint {
    pub fn new(id: String, title: String, description: String) -> Self {
        Self {
            id,
            title,
            description,
            prerequisites: Vec::new(),
            triggers: Vec::new(),
            consequences: Vec::new(),
        }
    }
    
    pub fn can_trigger(&self, completed_points: &[String]) -> bool {
        self.prerequisites.iter().all(|prereq| completed_points.contains(prereq))
    }
}