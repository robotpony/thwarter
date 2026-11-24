use super::object::ObjectAction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A door (REQ-CF-19, `design/STATES.md` › Doors): its own type of thing,
/// joining rooms with one state seen from every side. A room's exit names
/// the door, and the door says where each side leads. A portal joins more
/// than two rooms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Door {
    pub id: String,
    pub title: String,
    /// The door's current text, which the room view shows by its exit.
    pub description: String,
    /// Each room it joins, in the order written.
    pub sides: Vec<DoorSide>,
    /// A portal may join more than two rooms.
    pub portal: bool,
    /// Whether the exit can be used in the door's current state.
    pub passable: bool,
    /// Whether the other side can be seen through it (a window, a grate).
    pub transparent: bool,
    pub actions: HashMap<String, ObjectAction>,
    pub responses: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DoorSide {
    /// The room on this side.
    pub room: String,
    /// The direction of the exit from that room.
    pub direction: String,
    /// Where this side leads, for a portal; a two-sided door leads to its
    /// other side.
    pub to: Option<String>,
}

impl Door {
    pub fn side(&self, room: &str) -> Option<&DoorSide> {
        self.sides.iter().find(|s| s.room == room)
    }

    /// The room the door leads to from `room`: the side's `to:`, else, for
    /// a door with two sides, the other one.
    pub fn leads_to(&self, room: &str) -> Option<&str> {
        let side = self.side(room)?;
        if let Some(to) = &side.to {
            return Some(to);
        }
        match self.sides.as_slice() {
            [a, b] if a.room == room => Some(&b.room),
            [a, b] if b.room == room => Some(&a.room),
            _ => None,
        }
    }
}
