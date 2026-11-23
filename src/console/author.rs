//! Edit mode's commands that change content one thing at a time:
//! `/new`, `/set`, `/link` and `/unlink` (REQ-CON-5). Each saves through
//! the writer (REQ-ENG-7), reloads the world and reports any problems the
//! change caused.

use super::session::{Console, Place, COMPASS_ALIASES};
use super::world::Kind;
use crate::engines::EntityKind;
use crate::writer::{Edit, NewEntity, WorldWriter};
use serde_yaml::Value;
use std::path::PathBuf;
use std::time::Instant;

/// Exits that get a return exit when `/new room` makes them
/// (`PLAN.md` › Open question 3, proposed): compass directions and
/// up/down. Free-text directions such as `behind` don't.
const OPPOSITES: &[(&str, &str)] = &[
    ("north", "south"),
    ("northeast", "southwest"),
    ("east", "west"),
    ("southeast", "northwest"),
    ("up", "down"),
];

/// Frontmatter keys that hold a list of ids, set with `/set key a, b`.
const LIST_KEYS: &[&str] = &["characters", "objects", "inventory"];

/// What the console is looking at, when it's a content file.
struct Current {
    kind: Kind,
    id: String,
}

impl Console {
    pub(super) fn new_entity(&mut self, args: &[&str]) -> String {
        match args {
            ["room", rest @ ..] => self.new_room(rest),
            ["character", rest @ ..] => self.new_character(rest),
            ["object", rest @ ..] => self.new_object(rest),
            _ => "Usage: /new room|character|object ... (/help has the details)".to_string(),
        }
    }

    fn new_room(&mut self, args: &[&str]) -> String {
        let here = self.current_room();
        let (direction, id, title) = match (&here, args) {
            (Some(_), [direction, id, title @ ..]) => (Some(expand_direction(direction)), *id, title),
            (Some(_), [direction]) => {
                return format!("Name the new room: /new room {} <id> [title]", direction);
            }
            (None, [id, title @ ..]) => (None, *id, title),
            _ => {
                return "Usage: /new room <direction> <id> [title] in a room, /new room <id> [title] elsewhere".to_string()
            }
        };
        let id = self.room_id(id);
        let title = title_or_default(title, &id);

        if let (Some(here), Some(direction)) = (&here, &direction) {
            if let Some(existing) = self.world.engine.get_location(here).and_then(|l| l.connections.get(direction)) {
                return format!(
                    "{} already has a {} exit, to {}. Use /link {} <room> to change it.",
                    here, direction, existing, direction
                );
            }
        }

        let mut edits = Vec::new();
        let back = direction.as_deref().and_then(opposite);
        if let (Some(here), Some(back)) = (&here, back) {
            edits.push(Edit::SetMapEntry {
                key: "connections".into(),
                subkey: back.to_string(),
                value: Value::String(here.clone()),
            });
        }
        let new = NewEntity { kind: EntityKind::Location, id: id.clone(), title: title.clone(), object_type: None };
        let mut written = match self.create(&new, &edits) {
            Ok(path) => vec![path],
            Err(e) => return e,
        };
        let mut summary = format!("Created {}, {}", id, title);
        if let (Some(here), Some(direction)) = (&here, &direction) {
            let link = Edit::SetMapEntry {
                key: "connections".into(),
                subkey: direction.clone(),
                value: Value::String(id.clone()),
            };
            match self.save(Kind::Room, here, &[link]) {
                Ok(path) => written.push(path),
                Err(e) => {
                    let report = self.after_writes(&written, summary);
                    return format!("{}\nBut linking it from {} failed: {}", report, here, e);
                }
            }
            summary.push_str(&format!(", through the {} exit of {}", direction, here));
            if let Some(back) = back {
                summary.push_str(&format!(", with a {} exit back", back));
            }
        }
        summary.push('.');
        let report = self.after_writes(&written, summary);
        match self.world.path_of(Kind::Room, &id) {
            Some(path) => {
                self.place = Place::File(path);
                format!("{}\nNow at {}.", report, self.pwd())
            }
            None => report,
        }
    }

    fn new_character(&mut self, args: &[&str]) -> String {
        let [id, title @ ..] = args else {
            return "Usage: /new character <id> [title]".to_string();
        };
        if let Some(problem) = bare_id_problem(id) {
            return problem;
        }
        let here = self.current_room();
        let mut edits = Vec::new();
        if let Some(here) = &here {
            edits.push(Edit::SetScalar { key: "default_location".into(), value: Value::String(here.clone()) });
        }
        let title = title_or_default(title, id);
        let new = NewEntity { kind: EntityKind::Character, id: id.to_string(), title: title.clone(), object_type: None };
        self.create_and_place(&new, &edits, "characters", here, format!("Created character {}, {}", id, title))
    }

    fn new_object(&mut self, args: &[&str]) -> String {
        // Scenery this room names, given only its id, gets the default card
        // (`object_type: scenery`, REQ-CF-8).
        let scenery_args;
        let args = match args {
            [id] if self.is_scenery_here(id) => {
                scenery_args = [*id, "scenery"];
                &scenery_args[..]
            }
            _ => args,
        };
        let [id, object_type, title @ ..] = args else {
            return "Usage: /new object <id> <type> [title], where type is free-form: tool, weapon, furniture, ...".to_string();
        };
        if let Some(problem) = bare_id_problem(id) {
            return problem;
        }
        let here = self.current_room();
        let title = title_or_default(title, id);
        let new = NewEntity {
            kind: EntityKind::Object,
            id: id.to_string(),
            title: title.clone(),
            object_type: Some(object_type.to_string()),
        };
        self.create_and_place(&new, &[], "objects", here, format!("Created object {}, {}", id, title))
    }

    /// Creates a character or object card and, in a room, adds it to the
    /// room's `characters` or `objects` list. If the room already names it
    /// (scenery, for an object), the list is left as it is and the new
    /// card fills it in (REQ-CF-8).
    fn create_and_place(
        &mut self,
        new: &NewEntity,
        edits: &[Edit],
        list_key: &str,
        here: Option<String>,
        mut summary: String,
    ) -> String {
        let mut written = match self.create(new, edits) {
            Ok(path) => vec![path],
            Err(e) => return e,
        };
        if let Some(here) = here {
            let listed = self
                .world
                .engine
                .get_location(&here)
                .map(|l| if list_key == "characters" { l.characters.clone() } else { l.objects.clone() })
                .unwrap_or_default();
            if listed.contains(&new.id) {
                summary.push_str(&format!("; {} already lists it, and now it has a card", here));
            } else {
                let mut items = listed;
                items.push(new.id.clone());
                match self.save(Kind::Room, &here, &[Edit::SetList { key: list_key.into(), items }]) {
                    Ok(path) => {
                        written.push(path);
                        summary.push_str(&format!(", placed in {}", here));
                    }
                    Err(e) => {
                        let report = self.after_writes(&written, summary);
                        return format!("{}\nBut placing it in {} failed: {}", report, here, e);
                    }
                }
            }
        }
        summary.push('.');
        self.after_writes(&written, summary)
    }

    pub(super) fn set(&mut self, args: &[&str]) -> String {
        let Some(current) = self.current_file() else {
            return "/set changes the room, character or object you're in; cd to one first.".to_string();
        };
        let [key, value @ ..] = args else {
            return "Usage: /set <key> <value>, for example /set title The Old Well".to_string();
        };
        if value.is_empty() {
            return format!("Give `{}` a value: /set {} <value>", key, key);
        }
        let value = value.join(" ");
        let edit = if *key == "body" {
            Edit::SetBody(value)
        } else if let Some((map, entry)) = key.split_once('.') {
            Edit::SetMapEntry { key: map.into(), subkey: entry.into(), value: scalar(&value) }
        } else if LIST_KEYS.contains(key) {
            Edit::SetList { key: key.to_string(), items: split_list(&value) }
        } else {
            Edit::SetScalar { key: key.to_string(), value: scalar(&value) }
        };
        match self.save(current.kind, &current.id, &[edit]) {
            Ok(path) => self.after_writes(&[path], format!("Set {} on {}.", key, current.id)),
            Err(e) => e,
        }
    }

    pub(super) fn link(&mut self, args: &[&str]) -> String {
        let Some(here) = self.current_room() else {
            return "/link changes an exit of the room you're in; cd to a room first.".to_string();
        };
        let [direction, target] = args else {
            return "Usage: /link <direction> <room>, for example /link north castle/great-hall".to_string();
        };
        let direction = expand_direction(direction);
        let target = self.room_id(target);
        let edit = Edit::SetMapEntry {
            key: "connections".into(),
            subkey: direction.clone(),
            value: Value::String(target.clone()),
        };
        match self.save(Kind::Room, &here, &[edit]) {
            Ok(path) => self.after_writes(&[path], format!("Linked {} {} to {}.", here, direction, target)),
            Err(e) => e,
        }
    }

    pub(super) fn unlink(&mut self, args: &[&str]) -> String {
        let Some(here) = self.current_room() else {
            return "/unlink removes an exit of the room you're in; cd to a room first.".to_string();
        };
        let [direction] = args else {
            return "Usage: /unlink <direction>".to_string();
        };
        let direction = expand_direction(direction);
        let edit = Edit::RemoveMapEntry { key: "connections".into(), subkey: direction.clone() };
        match self.save(Kind::Room, &here, &[edit]) {
            Ok(path) => self.after_writes(&[path], format!("Removed the {} exit from {}.", direction, here)),
            Err(e) => e,
        }
    }

    // Shared steps.

    /// Whether the current room names `id` as an object with no card.
    pub(super) fn is_scenery_here(&self, id: &str) -> bool {
        let Some(here) = self.current_room() else { return false };
        let named = self.world.engine.get_location(&here).is_some_and(|l| l.objects.iter().any(|o| o == id));
        named && !self.world.engine.objects.contains_key(id)
    }

    fn current_file(&self) -> Option<Current> {
        let Place::File(path) = &self.place else { return None };
        let entry = self.world.files.get(path)?;
        (entry.kind != Kind::Broken).then(|| Current { kind: entry.kind, id: entry.id.clone() })
    }

    /// The room edit mode is in, if it's in one.
    pub(super) fn current_room(&self) -> Option<String> {
        self.current_file().filter(|c| c.kind == Kind::Room).map(|c| c.id)
    }

    /// A room id as typed: from the world root if it has a `/`, else in
    /// the current folder (`gatehouse` in `castle/` is `castle/gatehouse`).
    pub(super) fn room_id(&self, typed: &str) -> String {
        let typed = typed.trim_start_matches('/');
        let dir = self.current_dir();
        if typed.contains('/') || dir.is_empty() {
            typed.to_string()
        } else {
            format!("{}/{}", dir, typed)
        }
    }

    fn writer(&self) -> Result<WorldWriter, String> {
        WorldWriter::new(&self.world.root).map_err(|e| format!("Can't write to the world: {}", e))
    }

    /// Writes a new entity and adds it to the loaded world (REQ-ENG-7, 8).
    pub(super) fn create(&mut self, new: &NewEntity, edits: &[Edit]) -> Result<PathBuf, String> {
        let writer = self.writer()?;
        self.note_warnings_before();
        let report = writer
            .create_loaded(&mut self.world.engine, new, edits, false)
            .map_err(|e| format!("Not created: {}", within(&writer, &e)))?;
        Ok(report.path)
    }

    /// Writes edits to an entity's file and applies them to the loaded world.
    pub(super) fn save(&mut self, kind: Kind, id: &str, edits: &[Edit]) -> Result<PathBuf, String> {
        let kind = entity_kind(kind).ok_or_else(|| format!("{} can't be changed.", id))?;
        let writer = self.writer()?;
        self.note_warnings_before();
        let report = writer
            .edit_loaded(&mut self.world.engine, kind, id, edits, false)
            .map_err(|e| format!("Not saved: {}", within(&writer, &e)))?;
        Ok(report.path)
    }

    /// Keeps the world's warnings from before a command's first change, so
    /// `after_writes` can say which ones the command added.
    fn note_warnings_before(&mut self) {
        if self.warnings_before.is_none() {
            self.warnings_before = Some(self.world.warnings());
        }
    }

    /// After a command saved `written`, which the engine has already
    /// applied in memory: refreshes the folder index and reports `summary`
    /// with any problems or warnings the command added.
    pub(super) fn after_writes(&mut self, written: &[PathBuf], summary: String) -> String {
        let now = Instant::now();
        for path in written {
            let relative = path.strip_prefix(&self.world.root).unwrap_or(path).to_string_lossy().replace('\\', "/");
            self.own_writes.push((relative, now));
        }
        self.world.refresh_index();
        let game_warnings = self.reconcile_game();
        let before = self.warnings_before.take().unwrap_or_default();
        let new_problems: Vec<String> = self.world.warnings().into_iter().filter(|w| !before.contains(w)).collect();
        let mut out = vec![summary];
        if !new_problems.is_empty() {
            out.push(format!("This adds {} problem(s):\n  {}", new_problems.len(), new_problems.join("\n  ")));
        }
        out.extend(game_warnings.iter().map(|w| format!("Game: {}", w)));
        out.join("\n")
    }

    /// Drops the changed paths the console wrote itself in the last few
    /// seconds, so the file watcher doesn't reload and announce them again.
    pub(super) fn not_own_writes(&mut self, changed: Vec<String>) -> Vec<String> {
        let window = std::time::Duration::from_secs(3);
        self.own_writes.retain(|(_, when)| when.elapsed() < window);
        changed.into_iter().filter(|path| !self.own_writes.iter().any(|(own, _)| own == path)).collect()
    }
}

/// A writer error with file paths shown from the world root, as the rest
/// of the console shows them.
fn within(writer: &WorldWriter, error: &anyhow::Error) -> String {
    let root = format!("{}/", writer.root().display());
    format!("{:#}", error).replace(&root, "")
}

fn entity_kind(kind: Kind) -> Option<EntityKind> {
    match kind {
        Kind::Room => Some(EntityKind::Location),
        Kind::Character => Some(EntityKind::Character),
        Kind::Object => Some(EntityKind::Object),
        Kind::Broken => None,
    }
}

fn expand_direction(typed: &str) -> String {
    COMPASS_ALIASES
        .iter()
        .find(|(short, _)| *short == typed)
        .map(|(_, long)| long.to_string())
        .unwrap_or_else(|| typed.to_string())
}

fn opposite(direction: &str) -> Option<&'static str> {
    OPPOSITES.iter().find_map(|(a, b)| {
        if *a == direction {
            Some(*b)
        } else if *b == direction {
            Some(*a)
        } else {
            None
        }
    })
}

/// The typed title, or one made from the id's last part:
/// `castle/great-hall` becomes `Great hall`.
pub(super) fn title_or_default(typed: &[&str], id: &str) -> String {
    if !typed.is_empty() {
        return typed.join(" ");
    }
    let words = id.rsplit('/').next().unwrap_or(id).replace(['-', '_'], " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => id.to_string(),
    }
}

/// Character and object ids are bare names (`king`, not `npcs/king`).
pub(super) fn bare_id_problem(id: &str) -> Option<String> {
    id.contains('/').then(|| {
        format!("Character and object ids are bare names, without folders: `{}`.", id.rsplit('/').next().unwrap_or(id))
    })
}

/// A typed value as YAML would read it, so `80` is a number and `true` a
/// bool, but anything that isn't a plain number or bool stays text.
fn scalar(typed: &str) -> Value {
    match serde_yaml::from_str::<Value>(typed) {
        Ok(v @ (Value::Number(_) | Value::Bool(_))) => v,
        _ => Value::String(typed.to_string()),
    }
}

/// `a, b, c` or `a b c`.
fn split_list(typed: &str) -> Vec<String> {
    let parts: Vec<&str> = if typed.contains(',') { typed.split(',').collect() } else { typed.split_whitespace().collect() };
    parts.into_iter().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_and_titles() {
        assert_eq!(expand_direction("n"), "north");
        assert_eq!(expand_direction("behind"), "behind");
        assert_eq!(opposite("west"), Some("east"));
        assert_eq!(opposite("down"), Some("up"));
        assert_eq!(opposite("behind"), None);
        assert_eq!(title_or_default(&[], "castle/great-hall"), "Great hall");
        assert_eq!(title_or_default(&["The", "Hall"], "x"), "The Hall");
    }

    #[test]
    fn typed_values() {
        assert_eq!(scalar("80"), Value::from(80));
        assert_eq!(scalar("true"), Value::Bool(true));
        assert_eq!(scalar("The Old Well"), Value::String("The Old Well".into()));
        assert_eq!(scalar("[a, b]"), Value::String("[a, b]".into()));
        assert_eq!(split_list("a, b ,c"), vec!["a", "b", "c"]);
        assert_eq!(split_list("a b"), vec!["a", "b"]);
    }
}
