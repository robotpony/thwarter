use crate::models::world::Coordinate;

/// A location as the console and `thwart show` describe it (REQ-ENG-9):
/// the room plus, for each reference, whether its target exists.
/// Built by `WorldEngine::describe_location`, rendered by
/// `crate::render::render_location`.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationView {
    pub id: String,
    pub title: String,
    pub coordinates: Option<Coordinate>,
    pub body: String,
    /// In `DIRECTION_ORDER`, then any other direction alphabetically.
    pub exits: Vec<ExitView>,
    pub characters: Vec<EntityRef>,
    pub objects: Vec<EntityRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExitView {
    pub direction: String,
    pub target: String,
    pub target_exists: bool,
}

/// A character or object named by a location. `name` is `None` when no
/// card (file) defines it: for an object that's scenery (REQ-CF-8); for a
/// character it's a dangling reference.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityRef {
    pub id: String,
    pub name: Option<String>,
}

impl EntityRef {
    /// The card's name, or for something without a card, a name made from
    /// its id (`title_from_id`).
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| title_from_id(&self.id))
    }
}

/// A title made from an id, for scenery without a card: the last path
/// segment, hyphens and underscores as spaces, in sentence case
/// (`stone-bench` → "Stone bench"). The scenery-card default in
/// `design/CONTENT_FORMAT.md`.
pub fn title_from_id(id: &str) -> String {
    let words = id.rsplit('/').next().unwrap_or(id).replace(['-', '_'], " ");
    let mut chars = words.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Compass and vertical directions in the order an author reads them.
pub const DIRECTION_ORDER: &[&str] = &[
    "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest", "up", "down",
    "in", "out",
];

/// The full direction for an abbreviation (`n` → `north`, `u` → `up`);
/// anything else as given, lowercased.
pub fn expand_direction(direction: &str) -> String {
    let d = direction.trim().to_lowercase();
    match d.as_str() {
        "n" => "north", "ne" => "northeast", "e" => "east", "se" => "southeast",
        "s" => "south", "sw" => "southwest", "w" => "west", "nw" => "northwest",
        "u" => "up", "d" => "down",
        other => return other.to_string(),
    }
    .to_string()
}

/// Sort key putting known directions first, in `DIRECTION_ORDER`.
pub fn direction_rank(direction: &str) -> (usize, String) {
    let rank = DIRECTION_ORDER.iter().position(|d| *d == direction).unwrap_or(DIRECTION_ORDER.len());
    (rank, direction.to_string())
}
