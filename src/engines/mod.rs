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
pub mod scripts;

pub use world_engine::*;
pub use location_view::*;
pub use character_engine::*;
pub use narrative_engine::*;
pub use game_engine::*;
pub use player_commands::{Parsed, DIRECTIONS};
pub use saves::{list_saves, load_named, save_named, save_path, DEFAULT_SAVE};
/// The id for a new door joining two rooms (Bruce, 2026-11-24): every
/// client names doors the same predictable way, `door-bedroom-hallway-west`,
/// and the id is a safe file name. A room's area becomes part of the name
/// (`house/attic` is `house-attic`); anything not a letter or digit
/// becomes one `-` (REQ-ENG-47).
pub fn door_id(from_room: &str, to_room: &str, direction: &str) -> String {
    let mut id = String::from("door");
    for part in [from_room, to_room, direction] {
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
            id.push('-');
            id.push_str(word);
        }
    }
    id
}

#[cfg(test)]
mod tests {
    use super::door_id;

    #[test]
    fn door_ids_are_predictable_file_names() {
        assert_eq!(door_id("bedroom", "hallway", "west"), "door-bedroom-hallway-west");
        assert_eq!(door_id("house/attic", "house/landing", "down"), "door-house-attic-house-landing-down");
        assert_eq!(door_id(" Bedroom ", "Big Hall", "north east"), "door-bedroom-big-hall-north-east");
        assert_eq!(door_id("a//b", "../c", "up"), "door-a-b-c-up");
    }
}
