//! How play words the room and the inventory (Bruce, 2026-11-23): plain
//! sentences for a player, with every line a world can change in
//! `world.toml`'s `[play]` table. What an author needs (exits as
//! directions, ids, notes) belongs in the console's sidebar, not here.
//!
//! ```toml
//! [play]
//! show_room_names = false
//! objects = "You see {list}."
//! ```
//!
//! `{list}` is the things, people or directions joined with commas and
//! `and` ("a chair, a bed, and an old shirt"); `{or}` joins them with `or`
//! ("north, east, or down").

use std::collections::BTreeMap;

/// The play text settings and their defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayText {
    /// Whether looking shows the room's name above its description.
    pub show_room_names: bool,
    /// The things in the room: "You see a chair and a bed."
    pub objects: String,
    /// One person here: "Captain Marcus is here."
    pub character: String,
    /// More than one: "Captain Marcus and the king are here."
    pub characters: String,
    /// What the room's things hold: "There's an old photo on the walls."
    /// Also follows a holder's description when it's examined.
    pub held: String,
    /// The ways out: "You can go north or west."
    pub exits: String,
    /// One way out through a door, in `exits`' list: "down through an
    /// iron door". `{door}` is the door's name with an article, and "open"
    /// when it stands open.
    pub door_exit: String,
    /// No way out.
    pub no_exits: String,
    /// What the player carries: "You have a key."
    pub inventory: String,
    /// Carrying nothing.
    pub empty_inventory: String,
    /// A comma before the last `and` ("a, b, and c").
    pub serial_comma: bool,
}

impl Default for PlayText {
    fn default() -> Self {
        Self {
            show_room_names: false,
            objects: "You see {list}.".into(),
            character: "{list} is here.".into(),
            characters: "{list} are here.".into(),
            held: "There's {list}.".into(),
            exits: "You can go {or}.".into(),
            door_exit: "{direction} through {door}".into(),
            no_exits: "There's no way out.".into(),
            inventory: "You have {list}.".into(),
            empty_inventory: "You aren't carrying anything.".into(),
            serial_comma: true,
        }
    }
}

/// The settings by name, as `world.toml` writes them, for the reference
/// copy and for checking keys.
pub const KEYS: &[&str] = &["show_room_names", "objects", "character", "characters", "held", "exits", "door_exit", "no_exits", "inventory", "empty_inventory", "serial_comma"];

impl PlayText {
    /// The defaults with `[play]`'s values over them. A key it doesn't
    /// know, or a value of the wrong kind, is a warning.
    pub fn from_toml(table: Option<&toml::Value>, warnings: &mut Vec<String>) -> Self {
        let mut text = Self::default();
        let Some(table) = table else { return text };
        let Some(table) = table.as_table() else {
            warnings.push("world.toml: `[play]` must be a table of settings".into());
            return text;
        };
        let mut strings: BTreeMap<&str, &mut String> = BTreeMap::new();
        strings.insert("objects", &mut text.objects);
        strings.insert("character", &mut text.character);
        strings.insert("characters", &mut text.characters);
        strings.insert("held", &mut text.held);
        strings.insert("exits", &mut text.exits);
        strings.insert("door_exit", &mut text.door_exit);
        strings.insert("no_exits", &mut text.no_exits);
        strings.insert("inventory", &mut text.inventory);
        strings.insert("empty_inventory", &mut text.empty_inventory);
        let mut flags: Vec<(&str, bool)> = Vec::new();
        for (key, value) in table {
            match (key.as_str(), value) {
                ("show_room_names" | "serial_comma", toml::Value::Boolean(b)) => flags.push((key.as_str(), *b)),
                ("show_room_names" | "serial_comma", _) => warnings.push(format!("world.toml: `[play] {}` is true or false", key)),
                (k, toml::Value::String(s)) if strings.contains_key(k) => **strings.get_mut(k).unwrap() = s.clone(),
                (k, _) if strings.contains_key(k) => warnings.push(format!("world.toml: `[play] {}` is text", k)),
                (k, _) => warnings.push(format!("world.toml: `[play] {}` isn't a play setting; they are {}", k, KEYS.join(", "))),
            }
        }
        for (key, b) in flags {
            match key {
                "show_room_names" => text.show_room_names = b,
                _ => text.serial_comma = b,
            }
        }
        text
    }

    /// `items` joined for a sentence: "a", "a and b", "a, b, and c".
    pub fn list(&self, items: &[String]) -> String {
        match items {
            [] => String::new(),
            [one] => one.clone(),
            [a, b] => format!("{} and {}", a, b),
            [init @ .., last] => format!("{}{} and {}", init.join(", "), if self.serial_comma { "," } else { "" }, last),
        }
    }

    /// `items` joined with `or`: "a", "a or b", "a, b, or c".
    pub fn list_or(&self, items: &[String]) -> String {
        match items {
            [] => String::new(),
            [one] => one.clone(),
            [a, b] => format!("{} or {}", a, b),
            [init @ .., last] => format!("{}{} or {}", init.join(", "), if self.serial_comma { "," } else { "" }, last),
        }
    }

    /// `template` with `{list}` and `{or}` filled.
    pub fn fill(&self, template: &str, items: &[String]) -> String {
        template.replace("{list}", &self.list(items)).replace("{or}", &self.list_or(items))
    }

    /// `door_exit` for one door: "down through an open iron door".
    pub fn door_exit(&self, direction: &str, door: &str, open: bool) -> String {
        let name = if open { format!("open {}", door) } else { door.to_string() };
        self.door_exit.replace("{direction}", direction).replace("{door}", &with_article(&name))
    }
}

/// A thing's name mid-sentence with an article: "a chair", "an old
/// shirt", and "some walls" for a name that reads as plural. Titles are
/// lowercased, since object titles are often in title case.
pub fn with_article(title: &str) -> String {
    let name = title.trim().to_lowercase();
    let first = name.split_whitespace().last().unwrap_or("");
    let plural = first.ends_with('s') && !(first.ends_with("ss") || first.ends_with("us") || first.ends_with("is"));
    if plural {
        return format!("some {}", name);
    }
    let vowel = name.chars().next().map_or(false, |c| "aeiou".contains(c));
    format!("{} {}", if vowel { "an" } else { "a" }, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_read_as_sentences() {
        let t = PlayText::default();
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(t.list(&s(&["a chair"])), "a chair");
        assert_eq!(t.list(&s(&["a chair", "a bed"])), "a chair and a bed");
        assert_eq!(t.list(&s(&["a chair", "a bed", "an old shirt"])), "a chair, a bed, and an old shirt");
        let no_serial = PlayText { serial_comma: false, ..PlayText::default() };
        assert_eq!(no_serial.list(&s(&["a", "b", "c"])), "a, b and c");
        assert_eq!(t.fill(&t.exits, &s(&["north", "east", "down"])), "You can go north, east, or down.");
        assert_eq!(t.door_exit("down", "Iron door", true), "down through an open iron door");
        assert_eq!(with_article("Chair"), "a chair");
        assert_eq!(with_article("old shirt"), "an old shirt");
        assert_eq!(with_article("Walls"), "some walls");
        assert_eq!(with_article("Glass"), "a glass");
    }

    #[test]
    fn a_world_overrides_what_it_names_and_unknown_keys_warn() {
        let value: toml::Value = "[play]\nshow_room_names = true\nobjects = \"Here lies {list}.\"\nexits = 3\ncolour = \"red\"\n".parse().unwrap();
        let mut warnings = Vec::new();
        let t = PlayText::from_toml(value.get("play"), &mut warnings);
        assert!(t.show_room_names);
        assert_eq!(t.fill(&t.objects, &["a key".into()]), "Here lies a key.");
        assert_eq!(t.exits, PlayText::default().exits);
        assert_eq!(warnings.len(), 2, "{:?}", warnings);
    }

    /// The test world's `world.toml` is the reference copy of every
    /// setting, each at its default.
    #[test]
    fn the_test_worlds_reference_copy_lists_every_default() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/authors/mx/test-world/world.toml");
        let value: toml::Value = std::fs::read_to_string(path).unwrap().parse().unwrap();
        let table = value.get("play").and_then(|v| v.as_table()).expect("a [play] table");
        let mut names: Vec<&str> = table.keys().map(String::as_str).collect();
        let mut keys = KEYS.to_vec();
        names.sort();
        keys.sort();
        assert_eq!(names, keys, "the reference copy names every setting");
        let mut warnings = Vec::new();
        assert_eq!(PlayText::from_toml(value.get("play"), &mut warnings), PlayText::default());
        assert!(warnings.is_empty(), "{:?}", warnings);
    }
}
