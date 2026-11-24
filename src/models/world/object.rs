use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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
    /// Named actions (`design/STATES.md` › Changing state), e.g. `pull`
    /// and what it changes.
    pub actions: HashMap<String, ObjectAction>,
    /// The objects it holds (REQ-CF-31): photos on a wall, a key in a
    /// lock. In play, a thing keeps more (`PlayState.held`).
    pub objects: Vec<String>,
    /// How it holds them: `in` or `on` (the default), or any word
    /// (`under`), for "Coins (in the chest)".
    pub holds: Option<String>,
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
            objects: Vec::new(),
            holds: None,
            extra: HashMap::new(),
        }
    }
}

/// What one of an object's actions does: `actions: { pull: { becomes: {
/// castle/vault: open } } }`. Its reply is the prose of the state it
/// reaches (`design/SECTIONS.md` › Replies).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectAction {
    /// Thing id to the state it switches to.
    pub becomes: BTreeMap<String, String>,
    /// Values to set, by full name (`story.king`, `knife.sharp`, or
    /// `this.lit` for the object's own). Values are strings.
    pub sets: BTreeMap<String, String>,
    /// The chapter the story moves to, seen from the object's folder.
    pub chapter: Option<String>,
    /// Object ids the player must carry first (`requires: { has: key }`).
    pub requires_has: Vec<String>,
}

