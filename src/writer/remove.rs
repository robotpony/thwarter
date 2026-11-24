//! Removing a thing (`/remove`, REQ-ENG-50): its file or bundle folder
//! deleted, and every reference to its id taken out, found the way rename
//! finds them (REQ-ENG-30). Planned in memory first, so the console can
//! show what will change and ask before anything is written.
//!
//! - An object or character comes out of every `objects:`, `characters:`
//!   and `inventory` list. Scenery with no card has only its list entries.
//! - A door takes the exits through it, on both sides, with it.
//! - A room takes the exits into it, and every door with it on one side
//!   (and that door's other exit). A character that starts there loses
//!   its `default_location`.
//! - Refused, with the reason: a room that still holds people or things
//!   (move them first), and the world's start room (set another first).
//! - Left for the author and reported: prose links, an action's
//!   `becomes:` or `requires:`, and a door's `key`.

use super::content_file::{ContentFile, Edit};
use super::{ChangeReport, WorldWriter};
use crate::engines::bundles::FileRole;
use crate::engines::{EntityKind, WorldEngine};
use crate::parsers::{link_target, MarkdownParser};
use anyhow::{anyhow, bail, Context, Result};
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// What a removal will do, before anything is written.
#[derive(Debug, Clone)]
pub struct RemovePlan {
    /// Every thing that goes: the one asked for, then doors that go with a
    /// room.
    pub removed: Vec<(EntityKind, String)>,
    /// Files deleted, relative to the world root: a thing's file, or
    /// every file in its bundle.
    pub deleted: Vec<PathBuf>,
    /// Files whose text changes, each with what changes in it
    /// ("objects: drops desk", "connections.east removed").
    pub changed: Vec<(PathBuf, String)>,
    /// References left for the author, each with its file.
    pub by_hand: Vec<String>,
    /// Bundle folders deleted whole.
    folders: Vec<PathBuf>,
    /// The new text of each changed file.
    texts: BTreeMap<PathBuf, ContentFile>,
}

/// What a removal did.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveReport {
    pub removed: Vec<(EntityKind, String)>,
    /// Files whose text changed.
    pub written: Vec<PathBuf>,
    /// Files removed.
    pub deleted: Vec<PathBuf>,
    /// References left for the author.
    pub by_hand: Vec<String>,
    /// The world's problems before and after.
    pub change: ChangeReport,
}

impl WorldWriter {
    /// What removing `(kind, id)` would delete and change, written
    /// nowhere. Refused for a room that still holds people or things, and
    /// for the world's start room.
    pub fn plan_remove(&self, engine: &WorldEngine, kind: EntityKind, id: &str) -> Result<RemovePlan> {
        let root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        let rel = |p: &Path| p.strip_prefix(&root).unwrap_or(p).to_path_buf();

        // What goes: the thing, and for a room, every door with it on a side.
        let mut removed = vec![(kind, id.to_string())];
        let scenery = kind == EntityKind::Object && engine.thing(kind, id).is_none();
        if scenery {
            if engine.placement(id).is_empty() {
                bail!("there's no object `{}`", id);
            }
        } else {
            engine.source_path(kind, id)?;
        }
        if kind == EntityKind::Location {
            let room = engine.get_location(id).ok_or_else(|| anyhow!("there's no room `{}`", id))?;
            let mut here: Vec<String> = room.characters.iter().chain(room.objects.iter()).cloned().collect();
            here.dedup();
            if !here.is_empty() {
                bail!("{} still holds {}; move or remove them first", id, here.iter().map(|h| format!("`{}`", h)).collect::<Vec<_>>().join(", "));
            }
            // Only a start the world sets; a world without one starts in
            // whichever room the console picks, and that isn't refused.
            if engine.world().start.as_deref() == Some(id) {
                let set_in = if root.join("world/index.md").exists() { "`start:` in world/index.md" } else { "`starting_location` in world.toml" };
                bail!("{} is where the game starts; set another {} first", id, set_in);
            }
            let mut doors: Vec<&String> = engine.doors.iter().filter(|(_, d)| d.sides.iter().any(|s| s.room == id)).map(|(k, _)| k).collect();
            doors.sort();
            removed.extend(doors.into_iter().map(|d| (EntityKind::Door, d.clone())));
        }

        // The files that go, and every name each removed thing goes by.
        let mut deleted = Vec::new();
        let mut folders = Vec::new();
        let mut names: Vec<(EntityKind, Vec<String>)> = Vec::new();
        for (k, thing_id) in &removed {
            let Some(thing) = engine.thing(*k, thing_id) else {
                names.push((*k, vec![thing_id.clone()]));
                continue;
            };
            let mut these = vec![thing_id.clone()];
            match &thing.bundle {
                Some(folder) => {
                    let folder = rel(folder);
                    these.push(folder.to_string_lossy().replace('\\', "/"));
                    let mut files: Vec<PathBuf> = engine.file_roles().map(|(p, _)| rel(p)).filter(|p| p.starts_with(&folder)).collect();
                    files.extend(engine_scripts(engine, &root, &folder));
                    files.sort();
                    files.dedup();
                    deleted.extend(files);
                    folders.push(folder);
                }
                None => {
                    let file = rel(&thing.index.path);
                    these.push(file.with_extension("").to_string_lossy().replace('\\', "/"));
                    deleted.push(file);
                }
            }
            these.dedup();
            names.push((*k, these));
        }
        for folder in &folders {
            self.check_inside(&self.root().join(folder))?;
        }

        // Every other file's references, planned in memory.
        let mut changed = Vec::new();
        let mut by_hand = Vec::new();
        let mut texts = BTreeMap::new();
        let roles: Vec<(PathBuf, FileRole)> = engine.file_roles().map(|(p, r)| (p.to_path_buf(), r.clone())).collect();
        for (engine_path, role) in roles {
            if matches!(role, FileRole::Problem(_) | FileRole::Game) {
                continue;
            }
            let relative = rel(&engine_path);
            if deleted.contains(&relative) {
                continue;
            }
            let disk = self.root().join(&relative);
            let text = fs::read_to_string(&disk).with_context(|| format!("can't read {}", relative.display()))?;
            let doc = MarkdownParser::new().split_document(&text)?;
            let mut edits = Vec::new();
            for (k, these) in &names {
                let (mut e, said) = reference_removals(&doc.metadata, *k, these);
                edits.append(&mut e);
                changed.extend(said.into_iter().map(|s| (relative.clone(), s)));
                by_hand.extend(left_for_author(&doc.metadata, &doc.body, *k, these).into_iter().map(|w| format!("{}: {}", relative.display(), w)));
            }
            if edits.is_empty() {
                continue;
            }
            let mut file = if matches!(role, FileRole::Thing { .. }) { ContentFile::load(&disk)? } else { ContentFile::load_layer(&disk)? };
            file.apply_all(&edits).with_context(|| format!("can't take the references out of {}", relative.display()))?;
            texts.insert(relative, file);
        }
        Ok(RemovePlan { removed, deleted, changed, by_hand, folders, texts })
    }

    /// Remove `(kind, id)` exactly as `plan_remove` says: write each
    /// changed file, then delete the thing's files. Then `engine` takes
    /// the same change.
    pub fn remove_loaded(&self, engine: &mut WorldEngine, kind: EntityKind, id: &str, strict: bool) -> Result<RemoveReport> {
        let plan = self.plan_remove(engine, kind, id)?;
        let root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        let before = engine.current_problems(strict);
        let mut written = Vec::new();
        for (relative, file) in &plan.texts {
            file.save().with_context(|| format!("can't write {}", relative.display()))?;
            written.push(relative.clone());
        }
        for folder in &plan.folders {
            fs::remove_dir_all(self.root().join(folder)).with_context(|| format!("can't delete {}", folder.display()))?;
        }
        for file in &plan.deleted {
            let disk = self.root().join(file);
            if disk.exists() {
                fs::remove_file(&disk).with_context(|| format!("can't delete {}", file.display()))?;
            }
        }
        for file in &plan.deleted {
            engine.remove_file(&root.join(file));
        }
        for (relative, file) in &plan.texts {
            engine.set_file(&root.join(relative), &file.text());
        }
        let change = ChangeReport::new(root.join(plan.deleted.first().cloned().unwrap_or_default()), before, engine.current_problems(strict));
        Ok(RemoveReport { removed: plan.removed, written, deleted: plan.deleted, by_hand: plan.by_hand, change })
    }
}

/// A bundle's `actions.rhai`, which the engine keeps apart from its files.
fn engine_scripts(engine: &WorldEngine, root: &Path, folder: &Path) -> Vec<PathBuf> {
    engine.script_paths().filter_map(|p| p.strip_prefix(root).ok().map(Path::to_path_buf)).filter(|p| p.starts_with(folder)).collect()
}

fn names_one(value: &str, names: &[String]) -> bool {
    names.iter().any(|n| *n == link_target(value))
}

fn text_of(v: &Value) -> Option<String> {
    crate::parsers::id_value(v).map(|id| v.as_str().map(str::to_string).unwrap_or(id))
}

/// Edits taking a removed thing's references out of one file's
/// frontmatter, with what each does.
fn reference_removals(meta: &Value, kind: EntityKind, names: &[String]) -> (Vec<Edit>, Vec<String>) {
    let mut edits = Vec::new();
    let mut said = Vec::new();
    // Exits into a removed room, or through a removed door.
    if matches!(kind, EntityKind::Location | EntityKind::Door) {
        if let Some(map) = meta.get("connections").and_then(Value::as_mapping) {
            let gone: Vec<&str> = map
                .iter()
                .filter_map(|(dir, target)| Some((dir.as_str()?, text_of(target)?)))
                .filter(|(_, t)| names_one(t, names))
                .map(|(dir, _)| dir)
                .collect();
            if !gone.is_empty() && gone.len() == map.len() {
                edits.push(Edit::RemoveKey { key: "connections".into() });
            } else {
                edits.extend(gone.iter().map(|dir| Edit::RemoveMapEntry { key: "connections".into(), subkey: dir.to_string() }));
            }
            said.extend(gone.iter().map(|dir| format!("connections.{} removed", dir)));
        }
    }
    if kind == EntityKind::Location {
        if let Some(t) = meta.get("default_location").and_then(text_of) {
            if names_one(&t, names) {
                edits.push(Edit::RemoveKey { key: "default_location".into() });
                said.push("default_location removed".into());
            }
        }
    }
    let lists: &[&str] = match kind {
        EntityKind::Character => &["characters"],
        EntityKind::Object => &["objects", "inventory"],
        _ => &[],
    };
    for key in lists {
        let Some(items) = meta.get(*key).and_then(Value::as_sequence) else { continue };
        let texts: Vec<String> = items.iter().filter_map(text_of).collect();
        let kept: Vec<String> = texts.iter().filter(|t| !names_one(t, names)).cloned().collect();
        if kept.len() == texts.len() {
            continue;
        }
        edits.push(if kept.is_empty() { Edit::RemoveKey { key: (*key).into() } } else { Edit::SetList { key: (*key).into(), items: kept } });
        said.push(format!("{}: drops {}", key, names[0]));
    }
    (edits, said)
}

/// References a removal leaves for the author: prose links and
/// variables, an action's `becomes:` or `requires: { has: }`, and a
/// door's `key`.
fn left_for_author(meta: &Value, body: &str, kind: EntityKind, names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let id = &names[0];
    if names.iter().any(|n| {
        [format!("[[{}]]", n), format!("[[{}|", n), format!("[[{}#", n), format!("]({}.md)", n), format!("{{{{ {}.", n), format!("{{{{{}.", n)]
            .iter()
            .any(|p| body.contains(p.as_str()))
    }) {
        out.push(format!("its prose links to `{}`; change it by hand", id));
    }
    if let Some(actions) = meta.get("actions").and_then(Value::as_mapping) {
        for (name, action) in actions {
            let name = name.as_str().unwrap_or("?");
            if action.get("becomes").and_then(Value::as_mapping).map_or(false, |b| b.keys().filter_map(Value::as_str).any(|k| names_one(k, names))) {
                out.push(format!("action `{}`'s `becomes:` names `{}`; change it by hand", name, id));
            }
            let has = action.get("requires").and_then(|r| r.get("has"));
            let named = match has {
                Some(Value::String(s)) => names_one(s, names),
                Some(Value::Sequence(items)) => items.iter().filter_map(Value::as_str).any(|s| names_one(s, names)),
                _ => false,
            };
            if named {
                out.push(format!("action `{}`'s `requires: {{ has: }}` names `{}`; change it by hand", name, id));
            }
        }
    }
    if kind == EntityKind::Object {
        if let Some(key) = meta.get("vars").and_then(|v| v.get("key")).and_then(Value::as_str) {
            if names_one(key, names) {
                out.push(format!("`vars: {{ key: {} }}` names it; choose another key by hand", id));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn world() -> (tempfile::TempDir, WorldEngine, WorldWriter) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "world/index.md", "---\ntitle: Tiny\ntype: world\nstart: hall\n---\n");
        write(d, "hall.md", "---\nid: hall\ntitle: Hall\ntype: location\nconnections:\n  north: kitchen\n  down: trapdoor\n  west: shed\nobjects: [lamp, desk, bench]\ncharacters: [cook]\n---\nA hall. See [[desk]].\n");
        write(d, "kitchen.md", "---\nid: kitchen\ntitle: Kitchen\ntype: location\nconnections:\n  south: hall\n---\n");
        write(d, "cellar.md", "---\nid: cellar\ntitle: Cellar\ntype: location\nconnections:\n  up: trapdoor\n---\n");
        write(d, "shed.md", "---\nid: shed\ntitle: Shed\ntype: location\nconnections:\n  east: hall\n---\n");
        write(d, "objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: light\n---\n");
        write(d, "objects/desk/index.md", "---\nid: desk\ntitle: Desk\ntype: object\nobject_type: furniture\nstate: shut\nobjects: [lamp]\n---\n");
        write(d, "objects/desk/shut.md", "---\ninherits: index\n---\n");
        write(d, "objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { requires: { has: lamp } }\n---\n");
        write(d, "npcs/cook.md", "---\nid: cook\ntitle: Cook\ntype: character\ndefault_location: hall\ninventory: [lamp]\n---\n");
        write(d, "doors/trapdoor.md", "---\nid: trapdoor\ntitle: Trapdoor\ntype: door\ninherits: locked-door\nvars: { key: lamp }\nsides:\n  hall: { direction: down }\n  cellar: { direction: up }\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        let writer = WorldWriter::new(d).unwrap();
        (dir, engine, writer)
    }

    #[test]
    fn removing_an_object_takes_it_out_of_every_list_and_reports_the_rest() {
        let (dir, mut engine, writer) = world();
        let plan = writer.plan_remove(&engine, EntityKind::Object, "lamp").unwrap();
        assert_eq!(plan.deleted, [PathBuf::from("objects/lamp.md")]);
        let changed: Vec<String> = plan.changed.iter().map(|(p, s)| format!("{}: {}", p.display(), s)).collect();
        assert!(changed.contains(&"hall.md: objects: drops lamp".to_string()), "{:?}", changed);
        assert!(changed.contains(&"npcs/cook.md: inventory: drops lamp".to_string()), "{:?}", changed);
        assert!(changed.contains(&"objects/desk/index.md: objects: drops lamp".to_string()), "{:?}", changed);
        assert!(plan.by_hand.iter().any(|w| w.starts_with("objects/lever.md: action `pull`'s `requires: { has: }`")), "{:?}", plan.by_hand);
        assert!(plan.by_hand.iter().any(|w| w.starts_with("doors/trapdoor.md: `vars: { key: lamp }`")), "{:?}", plan.by_hand);
        assert!(dir.path().join("objects/lamp.md").exists(), "a plan writes nothing");

        let report = writer.remove_loaded(&mut engine, EntityKind::Object, "lamp", false).unwrap();
        assert!(!dir.path().join("objects/lamp.md").exists());
        let hall = fs::read_to_string(dir.path().join("hall.md")).unwrap();
        assert!(hall.contains("objects: [desk, bench]\n"), "{}", hall);
        assert!(!fs::read_to_string(dir.path().join("npcs/cook.md")).unwrap().contains("inventory"));
        assert!(engine.thing(EntityKind::Object, "lamp").is_none());
        assert_eq!(report.written.len(), 3, "{:?}", report.written);
    }

    #[test]
    fn a_bundle_goes_whole_and_scenery_only_leaves_its_lists() {
        let (dir, mut engine, writer) = world();
        let report = writer.remove_loaded(&mut engine, EntityKind::Object, "desk", false).unwrap();
        assert_eq!(report.deleted, [PathBuf::from("objects/desk/index.md"), PathBuf::from("objects/desk/shut.md")]);
        assert!(!dir.path().join("objects/desk").exists());
        assert!(report.by_hand.iter().any(|w| w.starts_with("hall.md: its prose links to `desk`")), "{:?}", report.by_hand);
        // `bench` has no card.
        let report = writer.remove_loaded(&mut engine, EntityKind::Object, "bench", false).unwrap();
        assert!(report.deleted.is_empty());
        assert!(fs::read_to_string(dir.path().join("hall.md")).unwrap().contains("objects: [lamp]\n"));
        assert!(writer.plan_remove(&engine, EntityKind::Object, "ghost").is_err());
    }

    #[test]
    fn a_door_takes_its_exits_and_a_room_takes_its_doors() {
        let (dir, mut engine, writer) = world();
        // A room that still holds things, or where the game starts, is refused.
        let err = writer.plan_remove(&engine, EntityKind::Location, "hall").unwrap_err().to_string();
        assert!(err.contains("still holds `cook`, `lamp`, `desk`, `bench`"), "{}", err);
        let report = writer.remove_loaded(&mut engine, EntityKind::Location, "cellar", false).unwrap();
        assert_eq!(report.removed, [(EntityKind::Location, "cellar".to_string()), (EntityKind::Door, "trapdoor".to_string())]);
        assert_eq!(report.deleted, [PathBuf::from("cellar.md"), PathBuf::from("doors/trapdoor.md")]);
        let hall = fs::read_to_string(dir.path().join("hall.md")).unwrap();
        assert!(!hall.contains("trapdoor") && hall.contains("north: kitchen"), "{}", hall);
        assert!(engine.current_problems(false).is_empty(), "{:?}", engine.current_problems(false));
        // Removing a room takes the exits into it.
        writer.remove_loaded(&mut engine, EntityKind::Location, "shed", false).unwrap();
        assert!(!fs::read_to_string(dir.path().join("hall.md")).unwrap().contains("west:"));
        // The start room is refused even when empty.
        write(dir.path(), "world/index.md", "---\ntitle: Tiny\ntype: world\nstart: kitchen\n---\n");
        engine.load_world(dir.path()).unwrap();
        let err = writer.plan_remove(&engine, EntityKind::Location, "kitchen").unwrap_err().to_string();
        assert!(err.contains("is where the game starts; set another `start:` in world/index.md first"), "{}", err);
        // A start from world.toml names world.toml; with no start at all,
        // nothing is refused for being the start.
        fs::remove_file(dir.path().join("world/index.md")).unwrap();
        write(dir.path(), "world.toml", "[settings]\nstarting_location = \"kitchen\"\n");
        engine.load_world(dir.path()).unwrap();
        let err = writer.plan_remove(&engine, EntityKind::Location, "kitchen").unwrap_err().to_string();
        assert!(err.contains("set another `starting_location` in world.toml first"), "{}", err);
        fs::remove_file(dir.path().join("world.toml")).unwrap();
        engine.load_world(dir.path()).unwrap();
        assert!(writer.plan_remove(&engine, EntityKind::Location, "kitchen").is_ok());
    }
}
