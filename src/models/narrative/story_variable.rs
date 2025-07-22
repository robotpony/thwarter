use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryVariable {
    pub name: String,
    pub value: VariableValue,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<VariableValue>),
}

impl StoryVariable {
    pub fn new_bool(name: String, value: bool) -> Self {
        Self {
            name,
            value: VariableValue::Boolean(value),
            description: None,
        }
    }
    
    pub fn new_string(name: String, value: String) -> Self {
        Self {
            name,
            value: VariableValue::String(value),
            description: None,
        }
    }
}