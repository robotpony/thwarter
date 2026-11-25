//! The virtual `game/` folder: the running game, shown and changed like
//! a folder of its own, never on disk (REQ-CON-24, REQ-CON-25).

use std::path::Path;

use super::author::entity_kind;
use super::session::{Console, Place};
use super::world::Kind;

impl Console {
    /// Where `target` goes in `game/`, if it names a place there: `/game`
    /// from anywhere, `game` from the root, and from inside `game/` a
    /// path below it. `None` leaves `target` to the world's places.
    pub(super) fn game_place(&self, target: &str) -> Option<Result<Place, String>> {
        let target = target.trim().trim_end_matches('/');
        let at_root = matches!(&self.place, Place::Dir(dir) if dir.is_empty());
        let sub = if let Some(rest) = target.strip_prefix("/game").filter(|r| r.is_empty() || r.starts_with('/')) {
            rest.trim_start_matches('/').to_string()
        } else if let Some(rest) = target.strip_prefix("game").filter(|r| at_root && (r.is_empty() || r.starts_with('/'))) {
            rest.trim_start_matches('/').to_string()
        } else if let Place::Game(here) = &self.place {
            let joined = if here.is_empty() { target.to_string() } else { format!("{}/{}", here, target) };
            // Only a path that exists below `game/`; anything else is a
            // name in the world.
            if self.game_sub_exists(&joined) {
                return Some(Ok(Place::Game(joined)));
            }
            if self.game_sub_exists(target) && !target.is_empty() {
                return Some(Ok(Place::Game(target.to_string())));
            }
            return None;
        } else {
            return None;
        };
        if self.game_sub_exists(&sub) {
            Some(Ok(Place::Game(sub)))
        } else {
            Some(Err(self.t("nothing_called", &[("name", target)])))
        }
    }

    fn game_sub_exists(&self, sub: &str) -> bool {
        match sub.split_once('/') {
            None => matches!(sub, "" | "things"),
            Some(("things", id)) => self.thing_kind(id).is_some(),
            Some(_) => false,
        }
    }

    /// The kind of the thing with this id, among those that have states.
    fn thing_kind(&self, id: &str) -> Option<crate::engines::world_engine::EntityKind> {
        self.world.files.values().filter(|e| e.id == id).find_map(|e| entity_kind(e.kind))
    }

    /// `look` in `game/`.
    pub(super) fn game_look(&self, sub: &str) -> String {
        let Some(game) = &self.game else { return self.t("game_folder_none", &[]) };
        let engine = &self.world.engine;
        match sub.split_once('/') {
            None if sub.is_empty() => {
                let changed: usize = game.states.values().map(|m| m.len()).sum();
                let chapter = game.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_else(|| self.t("no_chapter", &[]));
                let carrying = if game.inventory.is_empty() { self.t("side_carrying_nothing", &[]) } else { game.inventory.join(", ") };
                let values = if game.values.is_empty() {
                    self.t("game_folder_no_values", &[])
                } else {
                    game.values.iter().map(|(k, v)| format!("{} = {}", k, v)).collect::<Vec<_>>().join(", ")
                };
                let rows = [
                    (self.t("game_folder_location", &[]), game.location.clone()),
                    (self.t("game_folder_chapter", &[]), chapter),
                    (self.t("game_folder_carrying", &[]), carrying),
                    (self.t("game_folder_things", &[]), self.t("game_folder_changed", &[("count", &changed.to_string())])),
                    (self.t("game_folder_values", &[]), values),
                ];
                let width = rows.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0);
                rows.iter().map(|(l, v)| format!("{:width$}  {}", l, v, width = width)).collect::<Vec<_>>().join("\n")
            }
            None => {
                // `things`: each thing out of its starting state.
                let mut rows: Vec<String> = game
                    .states
                    .values()
                    .flat_map(|m| m.iter())
                    .map(|(id, state)| self.t("game_folder_thing_row", &[("id", id), ("state", state)]))
                    .collect();
                if rows.is_empty() {
                    return self.t("game_folder_things_none", &[]);
                }
                rows.sort();
                format!("{}\n\n{}", self.t("game_folder_things_heading", &[]), rows.join("\n"))
            }
            Some((_, id)) => {
                let Some(kind) = self.thing_kind(id) else { return self.t("nothing_called", &[("name", id)]) };
                let Some(thing) = engine.thing(kind, id) else { return self.t("game_thing_no_states", &[("id", id)]) };
                if thing.state_names().is_empty() {
                    return self.t("game_thing_no_states", &[("id", id)]);
                }
                let now = engine.state_at(kind, id, crate::engines::world_engine::At::play(game));
                let now = now.unwrap_or_default();
                let key = if Some(&now) == thing.starting_state.as_ref() { "game_thing_starting" } else { "game_thing_now" };
                format!(
                    "{}\n{}",
                    self.t(key, &[("id", id), ("state", &now)]),
                    self.t("game_thing_states", &[("states", &thing.state_names().join(", "))])
                )
            }
        }
    }

    /// `/set` inside `game/`: changes only the running game (REQ-CON-25).
    pub(super) fn game_set(&mut self, sub: &str, args: &[&str]) -> String {
        if self.game.is_none() {
            return self.t("game_folder_none", &[]);
        }
        let thing = sub.strip_prefix("things/").map(str::to_string);
        match (thing, args) {
            (Some(id), ["state", state]) => {
                let Some(kind) = self.thing_kind(&id) else { return self.t("nothing_called", &[("name", &id)]) };
                let has = self.world.engine.thing(kind, &id).map(|t| (t.has_state(state), t.state_names()));
                match has {
                    Some((true, _)) => {
                        let engine = &self.world.engine;
                        if let Some(game) = self.game.as_mut() {
                            let _ = game.switch_state(engine, kind, &id, state);
                        }
                        self.t("game_set_state", &[("id", &id), ("state", state)])
                    }
                    Some((false, names)) if !names.is_empty() => {
                        self.t("game_no_such_state", &[("id", &id), ("state", state), ("states", &names.join(", "))])
                    }
                    _ => self.t("game_thing_no_states", &[("id", &id)]),
                }
            }
            (_, ["chapter", name]) => {
                let engine = &self.world.engine;
                let entered = self.game.as_mut().map(|g| g.enter_chapter(engine, Path::new(""), name).is_ok()).unwrap_or(false);
                if entered {
                    self.t("game_set_chapter", &[("chapter", name)])
                } else {
                    self.t("game_no_such_chapter", &[("chapter", name)])
                }
            }
            (_, ["location", room]) => {
                let found = self.world.files.values().find(|e| e.kind == Kind::Room && (e.id == *room || e.id.rsplit('/').next() == Some(room))).map(|e| e.id.clone());
                match found {
                    Some(id) => {
                        if let Some(game) = self.game.as_mut() {
                            game.location = id.clone();
                            *game.visits.entry(id.clone()).or_insert(0) += 1;
                        }
                        self.t("game_set_location", &[("room", &id)])
                    }
                    None => self.t("game_no_such_room", &[("room", room)]),
                }
            }
            (_, ["carrying", things @ ..]) => {
                let ids: Vec<String> = things.join(" ").split([',', ' ']).filter(|s| !s.is_empty()).map(str::to_string).collect();
                if let Some(missing) = ids.iter().find(|id| !self.world.engine.objects.contains_key(id.as_str())) {
                    return self.t("game_no_such_object", &[("id", missing)]);
                }
                let shown = if ids.is_empty() { self.t("side_carrying_nothing", &[]) } else { ids.join(", ") };
                if let Some(game) = self.game.as_mut() {
                    game.inventory = ids;
                }
                self.t("game_set_carrying", &[("things", &shown)])
            }
            (_, [key, value @ ..]) if key.contains('.') && !value.is_empty() => {
                let value = value.join(" ");
                if let Some(game) = self.game.as_mut() {
                    game.values.insert(key.to_string(), value.clone());
                }
                self.t("game_set_value", &[("key", key), ("value", &value)])
            }
            _ => super::session::suggest(
                &self.t("game_set_usage", &[]),
                &["/set state {state}", "/set chapter {chapter}", "/set location {room}", "/set carrying {things}", "/set {name.value} {value}"],
            ),
        }
    }
}
