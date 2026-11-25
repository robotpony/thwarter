//! `/inspect <thing>`: everything the console knows about one thing, for
//! chasing a warning (Bruce, 2026-11-24): its file and scope, where it
//! starts, its templates, states, sections and actions, and every problem
//! or warning that names it. The room view stays the player's prose; this
//! and the sidebar hold the author's detail.

use super::author::id_from;
use super::session::{suggest, Console, Place};
use super::world::Kind;
use crate::engines::bundles::Thing;
use crate::engines::EntityKind;

impl Console {
    /// `/inspect [<name>]`: the thing named, or the one edit mode is at.
    pub(super) fn inspect(&self, args: &[&str]) -> String {
        let typed = args.join(" ");
        if let Some(out) = self.inspect_exit(&typed) {
            return out;
        }
        let (kind, id) = match self.thing_named(&typed) {
            Some(found) => found,
            None if typed.is_empty() => return suggest(&self.t("inspect_what", &[]), &["/inspect {thing}"]),
            None => return self.t("nothing_called", &[("name", &typed)]),
        };
        let Some(entity) = super::author::entity_kind(kind) else { return self.t("cant_inspect", &[("id", &id)]) };
        let engine = &self.world.engine;
        let title = self.world.files.values().find(|e| e.kind == kind && e.id == id).map(|e| e.title.clone()).unwrap_or_default();
        let mut out = vec![format!("{} ({}), {}", title, id, kind.label())];
        let row = |out: &mut Vec<String>, label: &str, text: String| out.push(format!("  {:<10} {}", label, text));

        if let Some(path) = self.world.path_of(kind, &id) {
            row(&mut out, "file", format!("{}.md", path));
        }
        if let Some(thing) = engine.thing(entity, &id) {
            self.thing_rows(thing, &mut out, &row);
        }
        if entity != EntityKind::Location {
            let starts: Vec<String> = engine.placement(&id).iter().map(|h| h.to_string()).collect();
            row(&mut out, "starts in", if starts.is_empty() { "nowhere".to_string() } else { starts.join(", ") });
        }
        if let Some(door) = engine.doors.get(&id) {
            let sides: Vec<String> = door.sides.iter().map(|s| format!("{} from {}", s.direction, s.room)).collect();
            row(&mut out, "sides", sides.join("; "));
        }
        if let Some(game) = &self.game {
            if let Some(state) = game.states.values().find_map(|m| m.get(&id)) {
                row(&mut out, "in play", format!("{} now", state));
            }
        }
        let check = engine.validation(false);
        let mentions: Vec<String> = check
            .problems
            .iter()
            .chain(&check.warnings)
            .filter(|w| w.contains(&format!("`{}`", id)) || w.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '/')).any(|word| word == id))
            .cloned()
            .collect();
        if !mentions.is_empty() {
            out.push(String::new());
            out.push(self.t("inspect_warnings", &[("count", &mentions.len().to_string())]));
            out.extend(mentions.iter().map(|w| format!("  {}", w)));
        }
        out.join("\n")
    }

    /// `/inspect east`: an exit from the room edit mode is in, where it
    /// goes, and its door in full, since the sidebar shows only whether a
    /// door is in the way (Bruce, 2026-11-24).
    fn inspect_exit(&self, typed: &str) -> Option<String> {
        let room = self.current_room()?;
        let location = self.world.engine.get_location(&room)?;
        let direction = super::author::expand_direction(typed.trim());
        let to = location.connections.get(&direction)?;
        let engine = &self.world.engine;
        let (beyond, door) = match engine.doors.get(to) {
            Some(door) => (door.leads_to(&room).unwrap_or("?").to_string(), Some(to.clone())),
            None => (to.clone(), None),
        };
        let written = if engine.get_location(&beyond).is_some() { "" } else { " (not written)" };
        let mut out = vec![format!("{} from {}, to {}{}", direction, room, beyond, written)];
        if let Some(door) = door {
            out.push(String::new());
            out.push(self.inspect(&[door.as_str()]));
        }
        Some(out.join("\n"))
    }

    /// The rows from a thing's layers: scope, templates, states, sections, actions.
    fn thing_rows(&self, thing: &Thing, out: &mut Vec<String>, row: &dyn Fn(&mut Vec<String>, &str, String)) {
        let scope = thing.scope.to_string_lossy().replace('\\', "/");
        row(out, "scope", if scope.is_empty() || ["objects", "npcs", "doors"].contains(&scope.as_str()) { "global".to_string() } else { scope });
        if !thing.templates.is_empty() {
            let names: Vec<String> = thing
                .templates
                .iter()
                .map(|l| l.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default())
                .map(|n| if n == "index" { "(bundle)".to_string() } else { n })
                .collect();
            row(out, "templates", names.join(" ← "));
        }
        if !thing.states.is_empty() {
            let states: Vec<String> = thing
                .states
                .keys()
                .map(|s| if thing.starting_state.as_deref() == Some(s) { format!("{} (starting)", s) } else { s.clone() })
                .collect();
            row(out, "states", states.join(", "));
        }
        let layers = std::iter::once(("", &thing.index)).chain(thing.states.iter().map(|(n, l)| (n.as_str(), l)));
        let mut sections = Vec::new();
        let mut actions = Vec::new();
        for (state, layer) in layers {
            for section in &layer.sections {
                let word = if section.unless { "Unless" } else { "When" };
                sections.push(format!("## {}: {}", word, section.tags.join(" ")));
            }
            if let Some(map) = layer.metadata.get("actions").and_then(|a| a.as_mapping()) {
                for verb in map.keys().filter_map(|k| k.as_str()) {
                    actions.push(if state.is_empty() { verb.to_string() } else { format!("{} (in {})", verb, state) });
                }
            }
        }
        if !actions.is_empty() {
            row(out, "actions", actions.join(", "));
        }
        if !sections.is_empty() {
            row(out, "sections", sections.join("; "));
        }
    }

    /// A room, character, object or door by name, or the one edit mode
    /// is at when no name is given.
    fn thing_named(&self, typed: &str) -> Option<(Kind, String)> {
        if typed.trim().is_empty() {
            let Place::File(path) = &self.place else { return None };
            let entry = self.world.files.get(path)?;
            return (entry.kind.is_thing() || entry.kind == Kind::Door).then(|| (entry.kind, entry.id.clone()));
        }
        let (id, _) = id_from(typed);
        for kind in [Kind::Object, Kind::Character, Kind::Door] {
            if self.world.files.values().any(|e| e.kind == kind && e.id == id) {
                return Some((kind, id));
            }
        }
        if let Ok(Place::File(path)) = self.resolve(typed) {
            let entry = self.world.files.get(&path)?;
            return (entry.kind.is_thing() || entry.kind == Kind::Door).then(|| (entry.kind, entry.id.clone()));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::console::session::Console;
    use crate::console::Mode;
    use std::path::Path;

    #[test]
    fn inspect_shows_where_a_thing_is_and_what_names_it() {
        let console = Console::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny"), Mode::Edit).unwrap();
        let out = console.inspect(&["lantern"]);
        assert!(out.starts_with("Brass Lantern (lantern), object"), "{}", out);
        assert!(out.contains("  file       objects/lantern.md"), "{}", out);
        assert!(out.contains("  starts in  room `village/square`"), "{}", out);
        assert!(out.contains("  actions    light"), "{}", out);
        let square = console.inspect(&["village/square"]);
        assert!(square.contains("village/inn"), "its warning is listed: {}", square);
        assert!(console.inspect(&["nothing-at-all"]).starts_with("Nothing called"));
        let mut console = console;
        console.handle("go /village/square");
        assert_eq!(console.inspect(&["e"]), "east from village/square, to village/inn (not written)");
    }
}
