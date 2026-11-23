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

/// Compass and vertical directions in the order an author reads them.
pub const DIRECTION_ORDER: &[&str] = &[
    "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest", "up", "down",
    "in", "out",
];

/// Sort key putting known directions first, in `DIRECTION_ORDER`.
pub fn direction_rank(direction: &str) -> (usize, String) {
    let rank = DIRECTION_ORDER.iter().position(|d| *d == direction).unwrap_or(DIRECTION_ORDER.len());
    (rank, direction.to_string())
}
