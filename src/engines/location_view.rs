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
    /// What the room's objects and doors hold, as play sees it (REQ-CF-31):
    /// each with its holder, in the order they're shown. Empty outside play.
    pub held: Vec<Held>,
    /// The objects here that are scenery, part of the room's prose (no
    /// card, or `object_type: scenery`): play doesn't list them.
    pub scenery: Vec<String>,
}

/// A thing another thing holds: a photo on a wall, a key in a lock.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    pub thing: EntityRef,
    /// The holder's id and its name as play shows it.
    pub holder: String,
    pub holder_name: String,
    /// How the holder holds it: `on` (the default), `in`, or the
    /// holder's own word (`holds:`).
    pub word: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExitView {
    pub direction: String,
    /// The room the exit leads to: the target named, or for an exit
    /// through a door, the room on the door's other side.
    pub target: String,
    pub target_exists: bool,
    /// The door the exit goes through, if it names one.
    pub door: Option<DoorView>,
}

/// A door on an exit (REQ-CF-19), in its current version.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorView {
    pub id: String,
    pub title: String,
    /// The door's text, which the room view shows by its exits.
    pub description: String,
    pub passable: bool,
    pub transparent: bool,
    /// For a door that can be seen through, the title of the room beyond.
    pub beyond: Option<String>,
    /// Its state in a game (`open`, `locked`); `None` outside play.
    pub state: Option<String>,
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
