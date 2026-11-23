use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A physical or environmental item: weapons, containers, furniture,
/// scenery, and similar world dressing. Object content is far more varied
/// than location or character content (a sword has damage/durability, a
/// stream has none of that), so beyond the handful of fields every object
/// shares, the rest is kept as an open bag of YAML values rather than
/// forced into a fixed struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameObject {
    pub id: String,
    pub title: String,
    pub description: String,
    /// Free-form kind, e.g. "weapon", "container", "environmental".
    /// Not a closed enum: the sample world alone uses a dozen distinct
    /// values, and authors should be able to invent new ones.
    pub object_type: String,
    pub category: Option<String>,
    /// Qualitative by convention ("light", "heavy", "immovable") rather
    /// than a numeric unit. Kept as a string; an engine that needs real
    /// physics can define its own numeric scale later.
    pub weight: Option<String>,
    /// Stored as text because sample content mixes numbers ("1000") with
    /// descriptive values ("priceless", "unknown").
    pub value: Option<String>,
    /// Named action -> response text, e.g. "wield" -> "The sword feels
    /// like an extension of your arm".
    pub actions: HashMap<String, String>,
    /// Everything else an author attaches: properties, effects, stats,
    /// contents, security, or any future section. Validated loosely (must
    /// be a YAML mapping); interpreted by whatever engine code cares.
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl GameObject {
    pub fn new(id: String, title: String, description: String, object_type: String) -> Self {
        Self {
            id,
            title,
            description,
            object_type,
            category: None,
            weight: None,
            value: None,
            actions: HashMap::new(),
            extra: HashMap::new(),
        }
    }
}
