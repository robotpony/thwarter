//! Edit mode's commands that change content one thing at a time:
//! `/new`, `/set`, `/link` and `/unlink` (REQ-CON-5). Each saves through
//! the writer (REQ-ENG-7), reloads the world and reports any problems the
//! change caused.

use super::session::{Console, Place, COMPASS_ALIASES};
use super::world::Kind;
use crate::engines::{EntityKind, Holder};
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
            _ => super::session::suggest(&self.t("usage_details", &[]), &["/new room|character|object {id} [{title}]"]),
        }
    }

    fn new_room(&mut self, args: &[&str]) -> String {
        let here = self.current_room();
        let (direction, id, title) = match (&here, args) {
            (Some(_), [direction, id, title @ ..]) => (Some(expand_direction(direction)), *id, title),
            (Some(_), [direction]) => {
                return super::session::suggest(&self.t("name_new_room", &[]), &[&format!("/new room {} {{id}} [{{title}}]", direction)]);
            }
            (None, [id, title @ ..]) => (None, *id, title),
            _ => {
                return super::session::suggest(
                    &self.t("usage_room_and_elsewhere", &[]),
                    &["/new room {direction} {id} [{title}]", "/new room {id} [{title}]"],
                )
            }
        };
        let id = self.room_id(id);
        let title = title_or_default(title, &id);

        if let (Some(here), Some(direction)) = (&here, &direction) {
            if let Some(existing) = self.world.engine.get_location(here).and_then(|l| l.connections.get(direction)) {
                return super::session::suggest(
                    &self.t("exit_already", &[("room", &here), ("direction", &direction), ("target", &existing)]),
                    &[&format!("/link {} {{room}}", direction)],
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
        let mut summary = self.t("created_room", &[("id", &id), ("title", &title)]);
        if let (Some(here), Some(direction)) = (&here, &direction) {
            let link = Edit::SetMapEntry {
                key: "connections".into(),
                subkey: direction.clone(),
                value: Value::String(id.clone()),
            };
            match self.save(Kind::Room, here, &[link]) {
                Ok(path) => written.push(path),
                Err(e) => {
                    let failed = self.t("created_room_link_failed", &[("id", &id), ("title", &title), ("room", here), ("reason", &e.to_string())]);
                    return self.after_writes(&written, failed);
                }
            }
            let args = [("id", id.as_str()), ("title", title.as_str()), ("direction", direction.as_str()), ("room", here.as_str()), ("back", back.unwrap_or(""))];
            summary = self.t(if back.is_some() { "created_room_linked" } else { "created_room_linked_one_way" }, &args);
        }
        let report = self.after_writes(&written, summary);
        match self.world.path_of(Kind::Room, &id) {
            Some(path) => {
                self.place = Place::File(path);
                self.t("now_at", &[("report", &report), ("place", &self.pwd())])
            }
            None => report,
        }
    }

    fn new_character(&mut self, args: &[&str]) -> String {
        let [typed, title @ ..] = args else {
            return super::session::suggest(&self.t("usage", &[]), &["/new character {id} [{title}]"]);
        };
        let (id, named) = id_from(typed);
        let id = id.as_str();
        let named_title = named.map(|n| vec![n]).unwrap_or_default();
        let title: Vec<&str> = if title.is_empty() { named_title.iter().map(String::as_str).collect() } else { title.to_vec() };
        let title = title.as_slice();
        if let Some(problem) = bare_id_problem(&self.texts, id) {
            return problem;
        }
        let here = self.current_room();
        let mut edits = Vec::new();
        if let Some(here) = &here {
            edits.push(Edit::SetScalar { key: "default_location".into(), value: Value::String(here.clone()) });
        }
        let title = title_or_default(title, id);
        let new = NewEntity { kind: EntityKind::Character, id: id.to_string(), title: title.clone(), object_type: None };
        self.create_and_place(&new, &edits, "characters", here)
    }

    fn new_object(&mut self, args: &[&str]) -> String {
        let Some((typed, rest)) = args.split_first() else {
            return self.t("usage_new_object", &[]);
        };
        let (typed_id, named) = id_from(typed);
        let owned: Vec<String> = std::iter::once(typed_id).chain(rest.iter().map(|s| s.to_string())).collect();
        let args: Vec<&str> = owned.iter().map(String::as_str).collect();
        let args = args.as_slice();
        // Scenery this room names, given only its id, gets the default card
        // (REQ-CF-8): one that inherits the built-in `scenery` template
        // (REQ-CF-21).
        let scenery_args;
        let scenery = matches!(args, [id] if self.is_scenery_here(id));
        let args = match args {
            [id] if scenery => {
                scenery_args = [*id, "scenery"];
                &scenery_args[..]
            }
            _ => args,
        };
        let [id, object_type, title @ ..] = args else {
            return self.t("usage_new_object", &[]);
        };
        if let Some(problem) = bare_id_problem(&self.texts, id) {
            return problem;
        }
        let here = self.current_room();
        let title = match (title, &named) {
            ([], Some(named)) => named.clone(),
            _ => title_or_default(title, id),
        };
        let new = NewEntity {
            kind: EntityKind::Object,
            id: id.to_string(),
            title: title.clone(),
            object_type: Some(object_type.to_string()),
        };
        let edits = if scenery { vec![Edit::ReplaceAll(super::full_edit::scenery_card(id, &title))] } else { Vec::new() };
        self.create_and_place(&new, &edits, "objects", here)
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
    ) -> String {
        // One whole reply per case, so each can be reworded on its own.
        let kind = if list_key == "characters" { "character" } else { "object" };
        let key = |case: &str| format!("created_{}{}", kind, case);
        let args = |room: &str| -> Vec<(&str, String)> { vec![("id", new.id.clone()), ("title", new.title.clone()), ("room", room.to_string())] };
        let say = |console: &Self, case: &str, room: &str, extra: &[(&str, &str)]| {
            let owned = args(room);
            let mut all: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
            all.extend_from_slice(extra);
            console.t(&key(case), &all)
        };
        let mut summary = say(self, "", "", &[]);
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
                summary = say(self, "_listed", &here, &[]);
            } else {
                let mut items = listed;
                items.push(new.id.clone());
                match self.save(Kind::Room, &here, &[Edit::SetList { key: list_key.into(), items }]) {
                    Ok(path) => {
                        written.push(path);
                        summary = say(self, "_placed", &here, &[]);
                    }
                    Err(e) => {
                        let failed = say(self, "_place_failed", &here, &[("reason", &e.to_string())]);
                        return self.after_writes(&written, failed);
                    }
                }
            }
        }
        self.after_writes(&written, summary)
    }

    pub(super) fn set(&mut self, args: &[&str]) -> String {
        let Some(current) = self.current_file() else {
            return "/set changes the room, character or object you're in; go to one first.".to_string();
        };
        let [key, value @ ..] = args else {
            return super::session::suggest(&self.t("usage_for_example", &[]), &["/set {key} {value}", "/set title The Old Well"]);
        };
        if value.is_empty() {
            return super::session::suggest(&self.t("give_a_value", &[("key", key)]), &[&format!("/set {} {{value}}", key)]);
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
            Ok(path) => self.after_writes(&[path], self.t("set_on", &[("key", key), ("id", &current.id)])),
            Err(e) => e,
        }
    }

    pub(super) fn link(&mut self, args: &[&str]) -> String {
        let Some(here) = self.current_room() else {
            return self.t("link_needs_room", &[]);
        };
        let [direction, target] = args else {
            return super::session::suggest(&self.t("usage_for_example", &[]), &["/link {direction} {room}", "/link north castle/great-hall"]);
        };
        let direction = expand_direction(direction);
        let target = self.room_id(target);
        let edit = Edit::SetMapEntry {
            key: "connections".into(),
            subkey: direction.clone(),
            value: Value::String(target.clone()),
        };
        match self.save(Kind::Room, &here, &[edit]) {
            Ok(path) => self.after_writes(&[path], self.t("linked", &[("room", &here), ("direction", &direction), ("target", &target)])),
            Err(e) => e,
        }
    }

    pub(super) fn unlink(&mut self, args: &[&str]) -> String {
        let Some(here) = self.current_room() else {
            return "/unlink removes an exit of the room you're in; go to a room first.".to_string();
        };
        let [direction] = args else {
            return super::session::suggest(&self.t("usage", &[]), &["/unlink {direction}"]);
        };
        let direction = expand_direction(direction);
        let edit = Edit::RemoveMapEntry { key: "connections".into(), subkey: direction.clone() };
        match self.save(Kind::Room, &here, &[edit]) {
            Ok(path) => self.after_writes(&[path], self.t("unlinked", &[("direction", &direction), ("room", &here)])),
            Err(e) => e,
        }
    }

    /// `/add <thing> [room or character]`: puts a person or thing into a
    /// room (this one if none is named), or an object into a character's
    /// inventory, without taking it from anywhere else (Bruce, 2026-11-23).
    /// A name with no card becomes scenery the room lists (REQ-CF-8).
    pub(super) fn add_thing(&mut self, args: &[&str]) -> String {
        // `/add door ...` makes a door; `/add hallway <door>` gives a door
        // its other side (Bruce, 2026-11-23; doors.rs).
        if args.first().is_some_and(|w| Console::is_door_word(w)) {
            return self.add_door(args);
        }
        match args {
            [room, door] if self.world.engine.doors.contains_key(*door) => return self.add_door_side(room, door, None),
            [room, door, direction] if self.world.engine.doors.contains_key(*door) => {
                return self.add_door_side(room, door, Some(direction))
            }
            _ => {}
        }
        let (thing, target) = match args {
            [thing] => match self.current_room() {
                Some(here) => (*thing, here),
                None => return super::session::suggest(&self.t("add_needs_room", &[("thing", thing)]), &[&format!("/add {} {{room}}", thing)]),
            },
            [thing, target] => (*thing, target.to_string()),
            _ => return super::session::suggest(&self.t("usage_for_example", &[]), &["/add {thing} [{room|character|thing}]", "/add chair bedroom"]),
        };
        let (thing_id, named) = id_from(thing);
        let thing = thing_id.as_str();
        if let Some(problem) = bare_id_problem(&self.texts, thing) {
            return problem;
        }
        let is_character = self.world.engine.characters.contains_key(thing);
        let mut written = Vec::new();
        let holder = match self.holder_for(&target, &mut written) {
            Ok(Some(Holder::Room(room))) => Holder::Room(room),
            Ok(Some(_)) if is_character => {
                return self.t("add_character_into_thing", &[("thing", thing)])
            }
            Ok(Some(Holder::Object(id))) if id == thing => return format!("{} can't hold itself.", thing),
            Ok(Some(holder)) => holder,
            Ok(None) => return self.t("add_no_holder", &[("target", &target), ("thing", thing)]),
            Err(e) => return e,
        };
        let key = if is_character { "characters" } else { "objects" };
        // Already somewhere else: a thing starts in one place, so ask to
        // move it rather than add a second (REQ-CF-31, Bruce, 2026-11-24).
        let elsewhere: Vec<Holder> = self.world.engine.placement(thing).into_iter().filter(|h| *h != holder).collect();
        if !elsewhere.is_empty() && !self.held_by(&holder, key).iter().any(|i| i == thing) {
            let places: Vec<String> = elsewhere.iter().map(short_holder).collect();
            let to = match &holder {
                Holder::Room(id) => format!("/{}", id),
                Holder::Character(id) | Holder::Object(id) => id.clone(),
            };
            self.asked = Some(super::doors::Asked::MoveHere { thing: thing.to_string(), to });
            return self.t("move_here", &[("thing", thing), ("places", &places.join(", "))]);
        }
        let mut items = self.held_by(&holder, key);
        let place = match &holder {
            Holder::Character(who) => self.t("inventory_of", &[("who", who)]),
            Holder::Room(id) | Holder::Object(id) => id.clone(),
        };
        if items.iter().any(|i| i == thing) {
            return self.t("already_in", &[("thing", thing), ("place", &place)]);
        }
        items.push(thing.to_string());
        let card = is_character || self.world.engine.objects.contains_key(thing);
        // A name typed with spaces or capitals ("old shirt") would be lost
        // in a bare id, so it gets a scenery card holding it as the title.
        let mut added = "added_to";
        let mut titled = String::new();
        if !card {
            match &named {
                Some(name) => {
                    let new = NewEntity {
                        kind: EntityKind::Object,
                        id: thing.to_string(),
                        title: name.clone(),
                        object_type: Some("scenery".to_string()),
                    };
                    match self.create(&new, &[Edit::ReplaceAll(super::full_edit::scenery_card(thing, name))]) {
                        Ok(path) => written.push(path),
                        Err(e) => return e,
                    }
                    added = "added_with_scenery_card";
                    titled = name.clone();
                }
                None => added = "added_as_scenery",
            }
        }
        match self.save_list(&holder, key, items) {
            Ok(path) => {
                written.push(path);
                self.added = Some((if is_character { Kind::Character } else { Kind::Object }, thing.to_string()));
                self.after_writes(&written, self.t(added, &[("thing", thing), ("place", &place), ("title", &titled)]))
            }
            Err(e) => self.after_writes(&written, e),
        }
    }

    /// `/move <thing> <room or character>`: puts a person or thing somewhere
    /// else (Bruce, 2026-11-23). A character goes to a room, and its
    /// `default_location` follows; an object goes to a room or into a
    /// character's inventory. The thing's own file stays where it is.
    pub(super) fn move_thing(&mut self, args: &[&str]) -> String {
        let [thing, target] = args else {
            return super::session::suggest(&self.t("usage_for_example", &[]), &["/move {thing} {room|character|thing}", "/move lantern village/well"]);
        };
        if matches!(*target, "global" | "area" | "local" | "here") {
            return self.t("move_scope_later", &[]);
        }
        // A room moves into an area folder, which changes its id and its
        // file's path (Bruce, 2026-11-23). That waits on the writer moving
        // files and updating every exit (`PLAN.md` › renaming).
        let room = self.room_id(thing);
        if self.world.engine.get_location(&room).is_some() {
            // The target is an area, or a room standing for its own area.
            let target_room = self.room_id(target);
            let area = if self.world.engine.get_location(&target_room).is_some() {
                target_room.rsplit_once('/').map(|(a, _)| a.to_string()).unwrap_or_default()
            } else {
                target.trim_matches('/').to_string()
            };
            let name = room.rsplit('/').next().unwrap_or(&room);
            let current = room.rsplit_once('/').map(|(a, _)| a).unwrap_or("");
            if current == area {
                let shown = if area.is_empty() { "the world's root".to_string() } else { format!("{}/", area) };
                return format!("{} is already in {}, as {} is.", room, shown, target);
            }
            let to = if area.is_empty() { name.to_string() } else { format!("{}/{}", area, name) };
            return self.move_room(&room, &to);
        }
        let (thing_id, _) = id_from(thing);
        let thing = thing_id.as_str();
        if let Some(problem) = bare_id_problem(&self.texts, thing) {
            return problem;
        }
        let engine = &self.world.engine;
        let is_character = engine.characters.contains_key(thing)
            || engine.loaded_locations.values().any(|l| l.characters.iter().any(|c| c == thing));
        let from = engine.placement(thing);
        if !is_character && from.is_empty() && !engine.objects.contains_key(thing) {
            return self.t("nothing_to_move", &[("thing", thing)]);
        }
        let mut written = Vec::new();
        let to = match self.holder_for(target, &mut written) {
            Ok(Some(Holder::Room(room))) => Holder::Room(room),
            Ok(Some(_)) if is_character => {
                return self.t("move_character_into_thing", &[("thing", thing)])
            }
            Ok(Some(Holder::Object(id))) if id == thing => return format!("{} can't hold itself.", thing),
            Ok(Some(holder)) => holder,
            Ok(None) => return self.t("move_no_holder", &[("target", target), ("thing", thing)]),
            Err(e) => return e,
        };
        let list_key = if is_character { "characters" } else { "objects" };
        let leaving: Vec<Holder> = from.iter().filter(|h| **h != to).cloned().collect();
        let already = from.contains(&to);
        if already && leaving.is_empty() {
            return self.t("already_there", &[("thing", thing), ("place", &where_to(&to))]);
        }

        let fail = |console: &mut Console, written: &Vec<PathBuf>, e: String| {
            let report = console.after_writes(written, console.t("move_stopped", &[("thing", thing)]));
            format!("{}\n{}", report, e)
        };
        for holder in &leaving {
            let items: Vec<String> = self.held_by(holder, list_key).into_iter().filter(|i| i != thing).collect();
            match self.save_list(holder, list_key, items) {
                Ok(path) => written.push(path),
                Err(e) => return fail(self, &written, e),
            }
        }
        if !already {
            let mut items = self.held_by(&to, list_key);
            items.push(thing.to_string());
            match self.save_list(&to, list_key, items) {
                Ok(path) => written.push(path),
                Err(e) => return fail(self, &written, e),
            }
        }
        let mut followed = "";
        if let (true, Holder::Room(room)) = (is_character && self.world.engine.characters.contains_key(thing), &to) {
            let edit = Edit::SetScalar { key: "default_location".into(), value: Value::String(room.clone()) };
            match self.save(Kind::Character, thing, &[edit]) {
                Ok(path) => {
                    written.push(path);
                    followed = " (and its default_location)";
                }
                Err(e) => return fail(self, &written, e),
            }
        }
        let from_text = if leaving.is_empty() { "nowhere".to_string() } else { leaving.iter().map(short_holder).collect::<Vec<_>>().join(", ") };
        let summary = self.t("moved_thing", &[("thing", thing), ("from", &from_text), ("to", &short_holder(&to)), ("followed", &followed)]);
        self.after_writes(&written, summary)
    }

    /// The room, character or object `typed` names, as a place to put
    /// something: a room first (from this folder, as /link takes it), then
    /// a character, then an object (REQ-CF-31).
    fn holder_named(&self, typed: &str) -> Option<Holder> {
        let engine = &self.world.engine;
        let room = self.room_id(typed);
        if engine.get_location(&room).is_some() {
            return Some(Holder::Room(room));
        }
        let (id, _) = id_from(typed);
        if engine.characters.contains_key(&id) {
            return Some(Holder::Character(id));
        }
        if engine.objects.contains_key(&id) {
            return Some(Holder::Object(id));
        }
        None
    }

    /// `holder_named`, and when `typed` is scenery some room lists with no
    /// card yet, its card first (REQ-CF-8), so it can hold things: any
    /// object can, scenery included (Bruce, 2026-11-23). The card written
    /// is added to `written`.
    fn holder_for(&mut self, typed: &str, written: &mut Vec<PathBuf>) -> Result<Option<Holder>, String> {
        if let Some(holder) = self.holder_named(typed) {
            return Ok(Some(holder));
        }
        let (id, named) = id_from(typed);
        let listed = self.world.engine.loaded_locations.values().any(|l| l.objects.iter().any(|o| *o == id));
        if !listed || bare_id_problem(&self.texts, &id).is_some() {
            return Ok(None);
        }
        let title = named.unwrap_or_else(|| title_or_default(&[], &id));
        let new = NewEntity { kind: EntityKind::Object, id: id.clone(), title: title.clone(), object_type: Some("scenery".to_string()) };
        let path = self.create(&new, &[Edit::ReplaceAll(super::full_edit::scenery_card(&id, &title))])?;
        written.push(path);
        Ok(Some(Holder::Object(id)))
    }

    /// What `holder` holds now, in the list a thing of `list_key` goes in.
    fn held_by(&self, holder: &Holder, list_key: &str) -> Vec<String> {
        let engine = &self.world.engine;
        match holder {
            Holder::Room(room) => engine
                .get_location(room)
                .map(|l| if list_key == "characters" { l.characters.clone() } else { l.objects.clone() })
                .unwrap_or_default(),
            Holder::Character(who) => engine.characters.get(who).map(|c| c.inventory.clone()).unwrap_or_default(),
            Holder::Object(id) => engine.objects.get(id).map(|o| o.objects.clone()).unwrap_or_default(),
        }
    }

    /// Writes `items` as `holder`'s list: a room's `characters:` or
    /// `objects:`, a character's `inventory:`, an object's `objects:`.
    fn save_list(&mut self, holder: &Holder, list_key: &str, items: Vec<String>) -> Result<PathBuf, String> {
        let (kind, id, key) = match holder {
            Holder::Room(room) => (Kind::Room, room.clone(), list_key),
            Holder::Character(who) => (Kind::Character, who.clone(), "inventory"),
            Holder::Object(id) => (Kind::Object, id.clone(), "objects"),
        };
        self.save(kind, &id, &[Edit::SetList { key: key.into(), items }])
    }

    /// Moves room `old` to the id `new`: its file (or bundle folder) moves to
    /// match, and every exit, list, start and link to it follows, through
    /// the writer's rename. Edit mode, `go back` and the game follow too.
    fn move_room(&mut self, old: &str, new: &str) -> String {
        let writer = match self.writer() {
            Ok(writer) => writer,
            Err(e) => return e,
        };
        self.note_warnings_before();
        let report = match writer.rename_loaded(&mut self.world.engine, EntityKind::Location, old, new, false) {
            Ok(report) => report,
            Err(e) => return self.t("not_moved", &[("reason", &within(&writer, &e))]),
        };
        self.follow_rename(old, new);

        // Every path touched, so the watcher doesn't announce them again.
        let mut touched: Vec<PathBuf> = report.written.clone();
        for (from, to) in &report.moved {
            touched.push(from.clone());
            touched.push(to.clone());
        }
        let moved_to: Vec<&PathBuf> = report.moved.iter().map(|(_, to)| to).collect();
        let mut updated: Vec<String> = report
            .written
            .iter()
            .filter(|p| !moved_to.contains(p))
            .map(|p| p.with_extension("").to_string_lossy().replace('\\', "/"))
            .collect();
        updated.sort();
        let file = report
            .moved
            .first()
            .map(|(from, to)| {
                let (from, to) = (from.display().to_string(), to.display().to_string());
                match report.moved.len() {
                    1 => self.t("moved_file", &[("from", &from), ("to", &to)]),
                    n => self.t("moved_bundle", &[("from", &from), ("to", &to), ("more", &(n - 1).to_string())]),
                }
            })
            .unwrap_or_else(|| self.t("no_file_to_move", &[]));
        let key = match updated.len() {
            0 => "moved_room",
            1 => "moved_room_updated_one",
            _ => "moved_room_updated",
        };
        let count = updated.len().to_string();
        let files = updated.join(", ");
        let mut summary = self.t(key, &[("old", old), ("new", new), ("file", &file), ("count", &count), ("files", &files)]);
        if !report.by_hand.is_empty() {
            summary.push_str(&self.t("still_naming", &[("id", &old.to_string())]));
            for place in &report.by_hand {
                summary.push_str(&format!("\n  {}", place));
            }
        }
        self.after_writes(&touched, summary)
    }

    /// After room `old` became `new`: where edit mode is, the places `go
    /// back` remembers, and the game in progress follow it.
    fn follow_rename(&mut self, old: &str, new: &str) {
        let swap = |path: &str| -> Option<String> {
            if path == old {
                Some(new.to_string())
            } else {
                path.strip_prefix(&format!("{}/", old)).map(|rest| format!("{}/{}", new, rest))
            }
        };
        let follow = |place: &mut Place| match place {
            Place::File(path) | Place::Dir(path) => {
                if let Some(moved) = swap(path) {
                    *path = moved;
                }
            }
        };
        follow(&mut self.place);
        for place in &mut self.trail {
            follow(place);
        }
        if let Some(game) = self.game.as_mut() {
            if game.location == old {
                game.location = new.to_string();
            }
            if let Some(taken) = game.taken.remove(old) {
                game.taken.insert(new.to_string(), taken);
            }
            if let Some(visits) = game.visits.remove(old) {
                game.visits.insert(new.to_string(), visits);
            }
            if let Some(states) = game.states.get_mut("location") {
                if let Some(state) = states.remove(old) {
                    states.insert(new.to_string(), state);
                }
            }
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
        entry.kind.is_thing().then(|| Current { kind: entry.kind, id: entry.id.clone() })
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
        WorldWriter::new(&self.world.root).map_err(|e| self.t("cant_write_world", &[("reason", &e.to_string())]))
    }

    /// Writes a new entity and adds it to the loaded world (REQ-ENG-7, 8).
    pub(super) fn create(&mut self, new: &NewEntity, edits: &[Edit]) -> Result<PathBuf, String> {
        let writer = self.writer()?;
        self.note_warnings_before();
        let report = writer
            .create_loaded(&mut self.world.engine, new, edits, false)
            .map_err(|e| self.t("not_created", &[("reason", &within(&writer, &e))]))?;
        self.created.push((new.kind, new.id.clone()));
        Ok(report.path)
    }

    /// Writes edits to an entity's file and applies them to the loaded world.
    pub(super) fn save(&mut self, kind: Kind, id: &str, edits: &[Edit]) -> Result<PathBuf, String> {
        let kind = entity_kind(kind).ok_or_else(|| format!("{} can't be changed.", id))?;
        let writer = self.writer()?;
        self.note_warnings_before();
        let report = writer
            .edit_loaded(&mut self.world.engine, kind, id, edits, false)
            .map_err(|e| self.t("not_saved", &[("reason", &within(&writer, &e))]))?;
        Ok(report.path)
    }

    /// Replaces the whole file at `relative` (from the world root, without
    /// `.md`), even one that doesn't parse now. With `expected`, refused if
    /// the file on disk no longer holds that text.
    pub(super) fn replace_file(&mut self, relative: &str, text: &str, expected: Option<&str>) -> Result<PathBuf, String> {
        let writer = self.writer()?;
        self.note_warnings_before();
        let path = PathBuf::from(format!("{}.md", relative));
        let report = writer
            .replace_file_loaded(&mut self.world.engine, &path, text, expected, false)
            .map_err(|e| self.t("not_saved", &[("reason", &within(&writer, &e))]))?;
        Ok(report.path)
    }

    /// Keeps the world's warnings from before a command's first change, so
    /// `after_writes` can say which ones the command added.
    pub(super) fn note_warnings_before(&mut self) {
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
            out.push(self.t("adds_problems", &[("count", &new_problems.len().to_string()), ("problems", &new_problems.join("\n  "))]));
        }
        out.extend(game_warnings.iter().map(|w| self.t("game_note", &[("note", w)])));
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

pub(super) fn entity_kind(kind: Kind) -> Option<EntityKind> {
    match kind {
        Kind::Room => Some(EntityKind::Location),
        Kind::Character => Some(EntityKind::Character),
        Kind::Object => Some(EntityKind::Object),
        Kind::Door => Some(EntityKind::Door),
        _ => None,
    }
}

pub(super) fn expand_direction(typed: &str) -> String {
    COMPASS_ALIASES
        .iter()
        .find(|(short, _)| *short == typed)
        .map(|(_, long)| long.to_string())
        .unwrap_or_else(|| typed.to_string())
}

pub(super) fn opposite(direction: &str) -> Option<&'static str> {
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

/// An id from what was typed, since ids can't hold spaces: `old shirt` or
/// `Old Shirt` becomes `old-shirt`, and the name as typed comes back to
/// use as a title. A typed id that's already plain comes back alone.
pub(super) fn id_from(typed: &str) -> (String, Option<String>) {
    let id = typed.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    if id == typed { (id, None) } else { (id, Some(typed.trim().to_string())) }
}

/// Character and object ids are bare names (`king`, not `npcs/king`).
pub(super) fn bare_id_problem(texts: &crate::engines::texts::Texts, id: &str) -> Option<String> {
    id.contains('/').then(|| {
        texts.text("console", "bare_ids", &[("id", id.rsplit('/').next().unwrap_or(id))])
    })
}

/// A typed value as YAML would read it, so `80` is a number and `true` a
/// bool, but anything that isn't a plain number or bool stays text.
pub(super) fn scalar(typed: &str) -> Value {
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


/// "1 file" or "3 files".

/// "in village/well", "carried by elder" or "in walls".
fn where_to(holder: &Holder) -> String {
    match holder {
        Holder::Character(who) => format!("carried by {}", who),
        Holder::Room(id) | Holder::Object(id) => format!("in {}", id),
    }
}

/// "village/well", "carried by elder" or "walls", for a move's report.
fn short_holder(holder: &Holder) -> String {
    match holder {
        Holder::Character(who) => format!("carried by {}", who),
        Holder::Room(id) | Holder::Object(id) => id.clone(),
    }
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
