//! `/add door ...`: making a door from the console rather than by hand
//! (Bruce, 2026-11-23; `design/STATES.md` › Adding a door from the console).
//!
//! ```text
//! /add door west hallway          a door west of here, east of the hallway
//! /add door --(west)--> hallway   the same, drawn as an arrow
//! /add door west                  one side so far; it leads nowhere yet
//! /add door                       asks which way
//! /add door(locked) west hallway  a door from the `locked-door` template
//! /add hallway door-bedroom-west  the hallway as a one-sided door's other side
//! ```
//!
//! The far side faces the opposite way unless it's given. A far room that
//! doesn't exist yet is made. The id is the engine's `door_id`, so every
//! client names a door the same way.

use super::author::{id_from, opposite, title_or_default};
use super::session::Console;
use super::world::Kind;
use crate::engines::{door_id, EntityKind};
use crate::writer::{Edit, NewEntity};
use serde_yaml::Value;
use std::path::PathBuf;

/// A question the console asked, which the next line answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// Which way a door goes from this room, for `/add door`.
    DoorDirection { template: String, to: Option<String> },
}

impl Console {
    /// Whether `args` (after `/add`) start with a door word: `door`,
    /// `door(locked)`, `portal` and the like.
    pub(super) fn is_door_word(word: &str) -> bool {
        let base = word.split('(').next().unwrap_or(word);
        matches!(base, "door" | "portal")
    }

    /// `/add door ...`, from the room edit mode is in.
    pub(super) fn add_door(&mut self, args: &[&str]) -> String {
        let Some((word, rest)) = args.split_first() else { return usage() };
        let template = template_for(word);
        let Some(here) = self.current_room() else {
            return "A door starts in a room: go to one, then /add door <direction> [room].".to_string();
        };
        // `--(west)-->` is the direction drawn as an arrow; a bare `-->`
        // leaves it to be asked.
        let mut direction: Option<String> = None;
        let mut rest: Vec<&str> = rest.to_vec();
        if let Some(first) = rest.first().copied() {
            if let Some(inner) = first.strip_prefix("--(").and_then(|r| r.strip_suffix(")-->")) {
                direction = Some(inner.to_string());
                rest.remove(0);
            } else if first == "-->" {
                rest.remove(0);
            } else if !self.is_room(first) {
                direction = Some(first.to_string());
                rest.remove(0);
            }
        }
        let (to, far_direction) = match rest.as_slice() {
            [] => (None, None),
            [to] => (Some(to.to_string()), None),
            [to, back] => (Some(to.to_string()), Some(back.to_string())),
            _ => return usage(),
        };
        let Some(direction) = direction.map(|d| super::author::expand_direction(&d)) else {
            self.asked = Some(Asked::DoorDirection { template, to });
            return format!("Which way is the door from {}? north, east, south, west, up, down, or any word.", here);
        };
        self.make_door(&here, &direction, to.as_deref(), far_direction.as_deref(), &template)
    }

    /// The answer to "Which way is the door?".
    pub(super) fn answer(&mut self, asked: Asked, line: &str) -> String {
        match asked {
            Asked::DoorDirection { template, to } => {
                let Some(here) = self.current_room() else { return "Edit mode left the room; /add door again from one.".to_string() };
                let direction = super::author::expand_direction(line.trim());
                self.make_door(&here, &direction, to.as_deref(), None, &template)
            }
        }
    }

    /// `/add <room> <door>`: `room` becomes a one-sided door's other side,
    /// facing the opposite way.
    pub(super) fn add_door_side(&mut self, room_typed: &str, door: &str, direction: Option<&str>) -> String {
        let room = self.room_id(room_typed);
        let Some(existing) = self.world.engine.doors.get(door) else { return format!("There's no door called `{}`.", door) };
        if existing.sides.iter().any(|s| s.room == room) {
            return format!("{} is already a side of {}.", room, door);
        }
        if existing.sides.len() >= 2 {
            return format!("{} already joins {} and {}.", door, existing.sides[0].room, existing.sides[1].room);
        }
        let near = existing.sides.first().map(|s| s.direction.clone());
        let back = match direction.map(str::to_string).or_else(|| near.as_deref().and_then(opposite).map(str::to_string)) {
            Some(back) => back,
            None => return format!("Which way is {} from {}? /add {} {} <direction>", door, room, room_typed, door),
        };
        let mut written = Vec::new();
        if let Err(e) = self.ensure_room(&room, &mut written) {
            return e;
        }
        if let Err(e) = self.link_side(door, &room, &back, &mut written) {
            return self.after_writes(&written, e);
        }
        self.after_writes(&written, format!("{} now has a side {} too.", door, side_of(&back, &room)))
    }

    /// Writes the door and the exits to it.
    fn make_door(&mut self, here: &str, direction: &str, to: Option<&str>, back: Option<&str>, template: &str) -> String {
        let to = to.map(|t| self.room_id(t));
        let back = match (&to, back) {
            (Some(_), Some(back)) => Some(super::author::expand_direction(back)),
            (Some(to), None) => match opposite(direction) {
                Some(b) => Some(b.to_string()),
                None => {
                    return format!(
                        "`{}` has no opposite, so say which way the door is from {}: /add door {} {} <direction>",
                        direction, to, direction, to
                    )
                }
            },
            (None, _) => None,
        };
        let id = door_id(here, to.as_deref().unwrap_or(""), direction);
        if self.world.engine.doors.contains_key(&id) {
            return format!("There's a door called {} already: /edit {} changes it.", id, id);
        }
        let mut written: Vec<PathBuf> = Vec::new();
        if let Some(to) = &to {
            if let Err(e) = self.ensure_room(to, &mut written) {
                return e;
            }
        }
        let mut sides = format!("  {}: {{ direction: {} }}\n", here, direction);
        if let (Some(to), Some(back)) = (&to, &back) {
            sides.push_str(&format!("  {}: {{ direction: {} }}\n", to, back));
        }
        let text = format!("---\nid: {}\ntitle: Door\ntype: door\ninherits: {}\nsides:\n{}---\n", id, template, sides);
        let new = NewEntity { kind: EntityKind::Door, id: id.clone(), title: "Door".to_string(), object_type: None };
        match self.create(&new, &[Edit::ReplaceAll(text)]) {
            Ok(path) => written.push(path),
            Err(e) => return self.after_writes(&written, e),
        }
        let exits = [(here.to_string(), direction.to_string())]
            .into_iter()
            .chain(to.clone().zip(back.clone()))
            .collect::<Vec<_>>();
        for (room, dir) in exits {
            let edit = Edit::SetMapEntry { key: "connections".into(), subkey: dir, value: Value::String(id.clone()) };
            match self.save(Kind::Room, &room, &[edit]) {
                Ok(path) => written.push(path),
                Err(e) => return self.after_writes(&written, e),
            }
        }
        let summary = match (&to, &back) {
            (Some(to), Some(back)) => format!("Added {}: {}, {}.", id, side_of(direction, here), side_of(back, to)),
            _ => format!("Added {}, {} (one side so far: /add <room> {} gives it another).", id, side_of(direction, here), id),
        };
        self.after_writes(&written, summary)
    }

    /// Adds `room` as one of `door`'s sides, and its exit through it.
    fn link_side(&mut self, door: &str, room: &str, direction: &str, written: &mut Vec<PathBuf>) -> Result<(), String> {
        let mut side = serde_yaml::Mapping::new();
        side.insert(Value::String("direction".into()), Value::String(direction.to_string()));
        let edit = Edit::SetMapEntry { key: "sides".into(), subkey: room.to_string(), value: Value::Mapping(side) };
        written.push(self.save(Kind::Door, door, &[edit])?);
        let edit = Edit::SetMapEntry { key: "connections".into(), subkey: direction.to_string(), value: Value::String(door.to_string()) };
        written.push(self.save(Kind::Room, room, &[edit])?);
        Ok(())
    }

    /// Makes `room` if it isn't written yet, as `/new room` would.
    fn ensure_room(&mut self, room: &str, written: &mut Vec<PathBuf>) -> Result<(), String> {
        if self.world.engine.get_location(room).is_some() {
            return Ok(());
        }
        let new = NewEntity {
            kind: EntityKind::Location,
            id: room.to_string(),
            title: title_or_default(&[], room),
            object_type: None,
        };
        written.push(self.create(&new, &[])?);
        Ok(())
    }

    fn is_room(&self, typed: &str) -> bool {
        self.world.engine.get_location(&self.room_id(typed)).is_some() || self.world.engine.get_location(&id_from(typed).0).is_some()
    }
}

/// The template a door word names: `door`, `door(locked)` (`locked-door`),
/// `portal`.
fn template_for(word: &str) -> String {
    let (base, hint) = match word.split_once('(') {
        Some((base, hint)) => (base, Some(hint.trim_end_matches(')'))),
        None => (word, None),
    };
    match hint {
        Some(hint) if !hint.is_empty() => format!("{}-{}", hint, base),
        _ => base.to_string(),
    }
}

/// "west of village/square", or "up from village/loft" where "of" reads badly.
fn side_of(direction: &str, room: &str) -> String {
    let compass = ["north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest"];
    if compass.contains(&direction) {
        format!("{} of {}", direction, room)
    } else {
        format!("{} from {}", direction, room)
    }
}

fn usage() -> String {
    "Usage: /add door <direction> [room] [its direction], or /add door --(west)--> hallway".to_string()
}
