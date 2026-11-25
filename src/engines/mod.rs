pub mod world_engine;
pub mod layers;
pub mod bundles;
pub mod world_def;
pub mod variables;
pub mod builtins;
pub mod persona;
pub mod map;
pub mod search;
pub mod location_view;
pub mod character_engine;
pub mod narrative_engine;
pub mod game_engine;
pub mod player_commands;
pub mod saves;
pub mod turn;
pub mod play_text;
pub mod texts;
pub mod scripts;
pub mod questions;
pub mod verbs;
pub mod clock;

pub use world_engine::*;
pub use location_view::*;
pub use character_engine::*;
pub use narrative_engine::*;
pub use game_engine::*;
pub use player_commands::{Parsed, DIRECTIONS};
pub use saves::{list_saves, load_named, save_named, save_path, DEFAULT_SAVE};
/// The id for a door joining two rooms, the same from either side
/// (REQ-ENG-47; Bruce, 2026-11-24): `door-<room>-<room>`, the rooms sorted,
/// with no direction, so the hallway's east exit doesn't say "west". A
/// room's area becomes part of the name (`house/attic` is `house-attic`);
/// anything not a letter or digit becomes one `-`, so the id is a safe
/// file name. `WorldEngine::new_door_name` adds a suffix when it's taken.
pub fn door_id_between(room: &str, other: &str) -> String {
    let mut rooms = [id_from_parts("", &[room]), id_from_parts("", &[other])];
    rooms.sort();
    id_from_parts("door", &[rooms[0].as_str(), rooms[1].as_str()])
}

fn id_from_parts(start: &str, parts: &[&str]) -> String {
    let mut id = String::from(start);
    for part in parts {
        let mut word = String::new();
        for c in part.trim().to_lowercase().chars() {
            if c.is_alphanumeric() {
                word.push(c);
            } else if !word.is_empty() && !word.ends_with('-') {
                word.push('-');
            }
        }
        let word = word.trim_end_matches('-');
        if !word.is_empty() {
            if !id.is_empty() {
                id.push('-');
            }
            id.push_str(word);
        }
    }
    id
}

#[cfg(test)]
mod tests {
    use super::door_id_between;

    #[test]
    fn a_door_id_is_the_same_from_either_side() {
        assert_eq!(door_id_between("bedroom", "hallway"), "door-bedroom-hallway");
        assert_eq!(door_id_between("hallway", "bedroom"), "door-bedroom-hallway");
        assert_eq!(door_id_between("house/landing", "house/attic"), "door-house-attic-house-landing");
        assert_eq!(door_id_between(" Big Hall ", "../c"), "door-big-hall-c");
        assert_eq!(door_id_between("bedroom", ""), "door-bedroom");
    }

    #[test]
    fn door_ids_are_predictable_file_names() {
        assert_eq!(door_id_between(" Bedroom ", "Big Hall"), "door-bedroom-big-hall");
        assert_eq!(door_id_between("a//b", "../c"), "door-a-b-c");
        assert_eq!(door_id_between("Café", "x y"), "door-café-x-y");
    }
}
