//! Text as data (`design/ENGINE_TEXT.md`, Bruce, 2026-11-24): every
//! sentence thwarter writes comes from a table of `key = "template"`,
//! with compiled-in defaults, the same shape everywhere.
//!
//! | Table | Defaults | Overridden in |
//! |---|---|---|
//! | `[play]` | `src/builtins/text/play.toml` | `world.toml` |
//! | `[engine]` | `src/builtins/text/engine.toml` | `world.toml` |
//! | `[console]` | `src/builtins/text/console.toml` | the personal `config.toml` |
//!
//! A value is text, a list of texts (picked by turn, `turn % len`, never
//! at random), or true or false for a setting. Text fills `{name}` from
//! the call's arguments; `{Name}` gives the argument with its first
//! letter capitalized. `{list}` and `{or}` are arguments like any other,
//! joined by the caller with `Texts::list` and `Texts::list_or`.
//!
//! ```
//! use thwarter::engines::texts::Texts;
//! let texts = Texts::defaults();
//! assert_eq!(texts.text("play", "inventory", &[("list", "a key")]), "You have a key.");
//! ```

use std::collections::BTreeMap;

/// The compiled-in defaults, by table.
pub const DEFAULTS: &[(&str, &str)] = &[
    ("play", include_str!("../builtins/text/play.toml")),
    ("engine", include_str!("../builtins/text/engine.toml")),
    ("console", include_str!("../builtins/text/console.toml")),
];

/// One value in a table.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Text(String),
    /// Several wordings of one reply, picked by turn.
    Choices(Vec<String>),
    /// A setting, such as `show_room_names`.
    Flag(bool),
}

impl Entry {
    fn from_toml(value: &toml::Value) -> Option<Entry> {
        match value {
            toml::Value::String(s) => Some(Entry::Text(s.clone())),
            toml::Value::Boolean(b) => Some(Entry::Flag(*b)),
            toml::Value::Array(items) if !items.is_empty() => {
                items.iter().map(|i| i.as_str().map(str::to_string)).collect::<Option<Vec<_>>>().map(Entry::Choices)
            }
            _ => None,
        }
    }

    /// The wording for `turn`: a list's entry `turn % len`.
    fn pick(&self, turn: u64) -> Option<&str> {
        match self {
            Entry::Text(s) => Some(s),
            Entry::Choices(list) => list.get((turn as usize) % list.len()).map(String::as_str),
            Entry::Flag(_) => None,
        }
    }
}

/// Every text table, defaults with any overrides over them.
#[derive(Debug, Clone, PartialEq)]
pub struct Texts {
    tables: BTreeMap<String, BTreeMap<String, Entry>>,
}

impl Default for Texts {
    fn default() -> Self {
        Self::defaults()
    }
}

impl Texts {
    /// The compiled-in defaults. They're part of the build, so a default
    /// that doesn't read is a bug, caught by this module's tests.
    pub fn defaults() -> Self {
        let mut tables = BTreeMap::new();
        for (name, text) in DEFAULTS {
            let value: toml::Value = text.parse().unwrap_or_else(|e| panic!("src/builtins/text/{}.toml: {}", name, e));
            let table = value.get(*name).and_then(|t| t.as_table()).cloned().unwrap_or_default();
            let entries = table
                .iter()
                .map(|(k, v)| {
                    let entry = Entry::from_toml(v).unwrap_or_else(|| panic!("src/builtins/text/{}.toml: `{}` is text, a list of texts, or true or false", name, k));
                    (k.clone(), entry)
                })
                .collect();
            tables.insert(name.to_string(), entries);
        }
        Self { tables }
    }

    /// Put `value` (a table such as `world.toml`'s `[engine]`) over the
    /// defaults of `table`. A key the defaults don't have, or a value of
    /// the wrong kind, is a warning naming `source`, and is left out.
    pub fn apply(&mut self, table: &str, value: Option<&toml::Value>, source: &str, warnings: &mut Vec<String>) {
        let Some(value) = value else { return };
        let Some(defaults) = self.tables.get_mut(table) else {
            warnings.push(format!("{}: there's no `[{}]` text table", source, table));
            return;
        };
        let Some(given) = value.as_table() else {
            warnings.push(format!("{}: `[{}]` must be a table of settings", source, table));
            return;
        };
        for (key, v) in given {
            let Some(default) = defaults.get(key) else {
                let keys: Vec<&str> = defaults.keys().map(String::as_str).collect();
                let known = if keys.len() <= 20 { format!("; they are {}", keys.join(", ")) } else { String::new() };
                warnings.push(format!("{}: `[{}] {}` isn't a {} setting{}", source, table, key, table, known));
                continue;
            };
            match (default, Entry::from_toml(v)) {
                (Entry::Flag(_), Some(e @ Entry::Flag(_))) => {
                    defaults.insert(key.clone(), e);
                }
                (Entry::Flag(_), _) => warnings.push(format!("{}: `[{}] {}` is true or false", source, table, key)),
                (_, Some(e @ (Entry::Text(_) | Entry::Choices(_)))) => {
                    defaults.insert(key.clone(), e);
                }
                (_, _) => warnings.push(format!("{}: `[{}] {}` is text, or a list of texts", source, table, key)),
            }
        }
    }

    /// The entry `key` in `table`, if there is one.
    pub fn entry(&self, table: &str, key: &str) -> Option<&Entry> {
        self.tables.get(table)?.get(key)
    }

    /// Every key in `table`, sorted.
    pub fn keys(&self, table: &str) -> Vec<&str> {
        self.tables.get(table).map(|t| t.keys().map(String::as_str).collect()).unwrap_or_default()
    }

    /// A setting's value; `false` if `key` isn't a setting.
    pub fn flag(&self, table: &str, key: &str) -> bool {
        matches!(self.entry(table, key), Some(Entry::Flag(true)))
    }

    /// `key`'s unfilled template for `turn`, if it's text.
    pub fn template(&self, table: &str, key: &str, turn: u64) -> Option<&str> {
        self.entry(table, key).and_then(|e| e.pick(turn))
    }

    /// `key`'s text with `args` filled, its first wording if it's a list.
    pub fn text(&self, table: &str, key: &str, args: &[(&str, &str)]) -> String {
        self.text_at(table, key, args, 0)
    }

    /// `key`'s text for `turn` (a list's entry `turn % len`), with `args`
    /// filled. An unknown key gives `[table.key]`, so a missing default
    /// shows in tests rather than as a blank.
    pub fn text_at(&self, table: &str, key: &str, args: &[(&str, &str)], turn: u64) -> String {
        match self.entry(table, key).and_then(|e| e.pick(turn)) {
            Some(template) => fill(template, args),
            None => format!("[{}.{}]", table, key),
        }
    }

    /// `items` joined for a sentence: "a", "a and b", "a, b, and c", with
    /// `[play] serial_comma` deciding the last comma.
    pub fn list(&self, items: &[String]) -> String {
        join(items, "and", self.flag("play", "serial_comma"))
    }

    /// `items` joined with `or`: "a, b, or c".
    pub fn list_or(&self, items: &[String]) -> String {
        join(items, "or", self.flag("play", "serial_comma"))
    }
}

/// Join for a sentence with `word` before the last item.
pub fn join(items: &[String], word: &str, serial_comma: bool) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{} {} {}", a, word, b),
        [init @ .., last] => format!("{}{} {} {}", init.join(", "), if serial_comma { "," } else { "" }, word, last),
    }
}

/// `template` with each `{name}` filled from `args`, and `{Name}` with
/// the argument capitalized. A placeholder with no argument is left as
/// written.
pub fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        let lower = lowercase_first(name);
        match args.iter().find(|(k, _)| *k == name).or_else(|| args.iter().find(|(k, _)| *k == lower)) {
            Some((k, v)) if *k == name => out.push_str(v),
            Some((_, v)) => out.push_str(&capitalize(v)),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The `{placeholders}` a template uses, for checks.
pub fn placeholders(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        out.push(after[..close].to_string());
        rest = &after[close + 1..];
    }
    out
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|c| c.to_lowercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_table_reads() {
        let texts = Texts::defaults();
        for (name, _) in DEFAULTS {
            assert!(texts.tables.contains_key(*name), "{}", name);
        }
        assert!(!texts.keys("play").is_empty());
    }

    #[test]
    fn text_fills_slots_and_capitalizes_and_lists_pick_by_turn() {
        assert_eq!(fill("{Verb} what?", &[("verb", "take")]), "Take what?");
        assert_eq!(fill("You can't go {direction}.", &[("direction", "west")]), "You can't go west.");
        assert_eq!(fill("{missing} stays", &[]), "{missing} stays");
        assert_eq!(placeholders("{a} and {B}"), vec!["a", "B"]);
        let mut texts = Texts::defaults();
        let value: toml::Value = "[play]\nempty_inventory = [\"Empty.\", \"Nothing.\"]\n".parse().unwrap();
        let mut warnings = Vec::new();
        texts.apply("play", value.get("play"), "world.toml", &mut warnings);
        assert!(warnings.is_empty(), "{:?}", warnings);
        assert_eq!(texts.text_at("play", "empty_inventory", &[], 0), "Empty.");
        assert_eq!(texts.text_at("play", "empty_inventory", &[], 3), "Nothing.");
        assert_eq!(texts.text("play", "nope", &[]), "[play.nope]");
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(texts.list(&s(&["a", "b", "c"])), "a, b, and c");
        assert_eq!(texts.list_or(&s(&["a", "b"])), "a or b");
    }

    /// Each `[engine]` key is used by the engine, so the defaults file is
    /// the list of every reply and none is left behind.
    #[test]
    fn every_engine_key_is_used() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/engines");
        let source: String = ["game_engine.rs", "player_commands.rs", "turn.rs"]
            .iter()
            .map(|f| std::fs::read_to_string(format!("{}/{}", dir, f)).unwrap())
            .collect();
        for key in Texts::defaults().keys("engine") {
            assert!(source.contains(&format!("\"{}\"", key)), "`[engine] {}` isn't used", key);
        }
    }

    #[test]
    fn a_world_rewords_a_built_in_reply() {
        use crate::engines::{GameEngine, Parsed, WorldEngine};
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hall.md"), "---\nid: hall\ntitle: Hall\ntype: location\n---\nA hall.\n").unwrap();
        std::fs::write(dir.path().join("world.toml"), "[settings]\nstarting_location = \"hall\"\n\n[engine]\nnot_here = \"No {noun} in sight.\"\nno_exit = [\"No way {direction}.\", \"Not {direction}, then.\"]\nnot_understood = \"Eh? {line}\"\n").unwrap();
        let mut world = WorldEngine::new();
        world.load_world(dir.path()).unwrap();
        assert!(world.current_warnings().is_empty(), "{:?}", world.current_warnings());
        let game = GameEngine::new();
        let mut state = game.start(&world, "hall").unwrap();
        let mut say = |line: &str| match game.parse_command(&world, line, &state) {
            Parsed::Action(a) => game.act(&world, &mut state, &a).text,
            Parsed::Reply(t) | Parsed::Unknown(t) => t,
        };
        assert_eq!(say("take lamp"), "No lamp in sight.");
        let first = say("north");
        let second = say("north");
        assert!(first != second && [first.as_str(), second.as_str()].contains(&"No way north."), "{} / {}", first, second);
        assert_eq!(say("xyzzy"), "Eh? xyzzy");
    }

    #[test]
    fn overrides_warn_about_unknown_keys_and_wrong_kinds() {
        let mut texts = Texts::defaults();
        let value: toml::Value = "[play]\nshow_room_names = \"yes\"\nexits = 3\ncolour = \"red\"\n[nowhere]\na = \"b\"\n".parse().unwrap();
        let mut warnings = Vec::new();
        texts.apply("play", value.get("play"), "world.toml", &mut warnings);
        texts.apply("nowhere", value.get("nowhere"), "world.toml", &mut warnings);
        assert_eq!(warnings.len(), 4, "{:?}", warnings);
        assert!(warnings[0].contains("`[play] colour` isn't a play setting; they are"), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("`[play] exits` is text, or a list of texts")), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("`[play] show_room_names` is true or false")), "{:?}", warnings);
        assert!(warnings[3].contains("there's no `[nowhere]` text table"), "{:?}", warnings);
        assert_eq!(texts, Texts::defaults(), "nothing wrong was applied");
    }
}
