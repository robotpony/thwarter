//! Renaming a thing (`design/STATES.md` › Ids and names, REQ-ENG-29): a
//! new `id`, its file or bundle folder moved to match, and every reference
//! to the old id rewritten. A room's id is its path, so moving a room into
//! an area is a rename (`/move hallway house` makes it `house/hallway`).

use super::content_file::{ContentFile, Edit};
use super::{check_id, new_entity_relative_path, ChangeReport, WorldWriter};
use crate::engines::bundles::FileRole;
use crate::engines::variables::LEVELS;
use crate::engines::{EntityKind, WorldEngine};
use crate::parsers::{link_target, MarkdownParser};
use anyhow::{anyhow, bail, Context, Result};
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// What a rename did.
#[derive(Debug, Clone, PartialEq)]
pub struct RenameReport {
    /// Files moved, from and to, relative to the world root. A bundle
    /// lists each of its files.
    pub moved: Vec<(PathBuf, PathBuf)>,
    /// Files whose text changed, relative to the world root, at their
    /// new paths. Includes the renamed thing's own file.
    pub written: Vec<PathBuf>,
    /// References the writer found but couldn't rewrite (inside an
    /// action's `becomes:` or `requires:`), each with its file. The author
    /// fixes these by hand; `validate` reports them meanwhile.
    pub by_hand: Vec<String>,
    /// The world's problems before and after.
    pub change: ChangeReport,
}

impl WorldWriter {
    /// Rename `(kind, old)` to `new`: rewrite its `id`, move its file (or
    /// bundle folder) to the path the new id gives, and update every
    /// reference to it: exits, `characters:`, `objects:`, `inventory`,
    /// `default_location`, `world/index.md`'s `start:`, and links and
    /// variables in prose. Everything is checked in memory before anything
    /// is written, and refused if the new path is taken. Then `engine`
    /// takes the same change.
    pub fn rename_loaded(
        &self,
        engine: &mut WorldEngine,
        kind: EntityKind,
        old: &str,
        new: &str,
        strict: bool,
    ) -> Result<RenameReport> {
        check_id(new)?;
        if new.contains('.') || LEVELS.contains(&new) {
            bail!("`{}` can't be an id: ids can't contain a dot or be one of {}", new, LEVELS.join(", "));
        }
        if old == new {
            bail!("`{}` already has that id", old);
        }
        engine.source_path(kind, old)?;
        if engine.thing(kind, new).is_some() {
            bail!("a {} `{}` already exists", kind.type_name(), new);
        }
        let engine_root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        let rel = |p: &Path| p.strip_prefix(&engine_root).unwrap_or(p).to_path_buf();
        let thing = engine.thing(kind, old).expect("checked by source_path").clone();

        // Where the thing's files go.
        let (from, to): (PathBuf, PathBuf) = match &thing.bundle {
            Some(folder) => {
                let target = match kind {
                    EntityKind::Location => PathBuf::from(new),
                    _ => folder.with_file_name(new),
                };
                (folder.clone(), target)
            }
            None => {
                let file = rel(&thing.index.path);
                let target = match kind {
                    EntityKind::Location => new_entity_relative_path(kind, new),
                    _ => file.with_file_name(format!("{}.md", new)),
                };
                (file, target)
            }
        };
        let target_abs = self.root().join(&to);
        if target_abs.exists() {
            bail!("{} already exists", to.display());
        }
        self.check_inside(target_abs.parent().map(nearest_existing).as_deref().unwrap_or(self.root()))?;
        let moves = |p: &Path| -> Option<PathBuf> {
            if thing.bundle.is_some() {
                p.strip_prefix(&from).ok().map(|inner| to.join(inner))
            } else {
                (p == from).then(|| to.clone())
            }
        };

        // Plan every edit in memory first.
        let old_names = names_for(&from, old, thing.bundle.is_some());
        let mut planned: BTreeMap<PathBuf, ContentFile> = BTreeMap::new();
        let mut by_hand = Vec::new();
        let roles: Vec<(PathBuf, FileRole)> = engine.file_roles().map(|(p, r)| (p.to_path_buf(), r.clone())).collect();
        for (engine_path, role) in roles {
            if matches!(role, FileRole::Problem(_) | FileRole::Game) {
                continue;
            }
            let relative = rel(&engine_path);
            let disk = self.root().join(&relative);
            let is_own_index = engine_path == thing.index.path;
            let mut file = if matches!(role, FileRole::Thing { .. }) {
                ContentFile::load(&disk)?
            } else {
                ContentFile::load_layer(&disk)?
            };
            let text = fs::read_to_string(&disk)?;
            let doc = MarkdownParser::new().split_document(&text)?;
            let mut edits = reference_edits(&doc.metadata, kind, &old_names, new, engine, &role);
            let nested = nested_references(&doc.metadata, kind, &old_names, engine);
            let body = rewrite_prose(&doc.body, &old_names, new);
            if body != doc.body {
                edits.push(Edit::SetBody(body));
            }
            if is_own_index {
                file.set_id(new).with_context(|| format!("can't rewrite the id in {}", relative.display()))?;
            }
            // References inside actions: rewritten in place and checked
            // against what the frontmatter should read as; reported for
            // the author if that can't be done safely.
            let mut rewrote_nested = false;
            if !nested.is_empty() {
                let mut expected = file_frontmatter(&file)?;
                rename_in_actions(&mut expected, kind, &old_names, new);
                let mut attempt = file.clone();
                let ok = old_names.iter().all(|name| {
                    let mut wanted = file_frontmatter(&attempt).unwrap_or_default();
                    rename_in_actions(&mut wanted, kind, std::slice::from_ref(name), new);
                    attempt.replace_in_key("actions", name, new, &wanted).is_ok()
                });
                if ok && file_frontmatter(&attempt)? == expected {
                    file = attempt;
                    rewrote_nested = true;
                } else {
                    by_hand.extend(nested.iter().map(|w| format!("{}: {}", relative.display(), w)));
                }
            }
            if edits.is_empty() && !is_own_index && !rewrote_nested && moves(&relative).is_none() {
                continue;
            }
            file.apply_all(&edits).with_context(|| format!("can't update the references in {}", relative.display()))?;
            planned.insert(relative, file);
        }

        // Write: the moved files first, then everything else.
        let before = engine.current_problems(strict);
        let mut moved = Vec::new();
        let mut written = Vec::new();
        if let Some(parent) = target_abs.parent() {
            fs::create_dir_all(parent).with_context(|| format!("can't create {}", parent.display()))?;
        }
        if thing.bundle.is_some() {
            fs::rename(self.root().join(&from), &target_abs)
                .with_context(|| format!("can't move {} to {}", from.display(), to.display()))?;
        }
        let mut texts: Vec<(PathBuf, String)> = Vec::new();
        for (relative, file) in planned {
            match moves(&relative) {
                Some(new_rel) => {
                    let file = file.at(self.root().join(&new_rel));
                    if thing.bundle.is_some() {
                        file.save()?;
                    } else {
                        file.save_new()?;
                        fs::remove_file(self.root().join(&relative))
                            .with_context(|| format!("wrote {} but can't remove {}", new_rel.display(), relative.display()))?;
                    }
                    moved.push((relative.clone(), new_rel.clone()));
                    written.push(new_rel.clone());
                    texts.push((new_rel, file.text()));
                }
                None => {
                    file.save()?;
                    written.push(relative.clone());
                    texts.push((relative, file.text()));
                }
            }
        }

        // The same change in memory: forget the old paths, then set each
        // new or changed file, and any moved file nobody edited.
        if thing.bundle.is_some() {
            let old_files: Vec<PathBuf> = engine
                .file_roles()
                .map(|(p, _)| p.to_path_buf())
                .filter(|p| rel(p).starts_with(&from))
                .collect();
            for path in old_files {
                let new_rel = moves(&rel(&path)).expect("under the bundle");
                if !texts.iter().any(|(p, _)| *p == new_rel) {
                    let text = fs::read_to_string(self.root().join(&new_rel))?;
                    moved.push((rel(&path), new_rel.clone()));
                    texts.push((new_rel, text));
                }
                engine.remove_file(&path);
            }
        } else {
            engine.remove_file(&engine_root.join(&from));
        }
        for (relative, text) in &texts {
            engine.set_file(&engine_root.join(relative), text);
        }
        moved.sort();
        moved.dedup();
        let change = ChangeReport::new(engine_root.join(&to), before, engine.current_problems(strict));
        Ok(RenameReport { moved, written, by_hand, change })
    }
}

/// Every way frontmatter or prose can name the thing: its id, and its
/// file's path from the root without `.md` (a bundle by its folder).
fn names_for(from: &Path, id: &str, is_bundle: bool) -> Vec<String> {
    let path = if is_bundle { from.to_path_buf() } else { from.with_extension("") };
    let mut names = vec![id.to_string(), path.to_string_lossy().replace('\\', "/")];
    names.dedup();
    names
}

fn names_old(value: &str, old_names: &[String]) -> bool {
    old_names.iter().any(|n| *n == link_target(value))
}

/// The same reference, renamed, in the style it was written: a link stays
/// a link.
fn renamed(value: &str, new: &str) -> String {
    if value.trim().starts_with("[[") {
        format!("[[{}]]", new)
    } else {
        new.to_string()
    }
}

/// Edits for the top-level references a file makes to the renamed thing.
fn reference_edits(meta: &Value, kind: EntityKind, old: &[String], new: &str, engine: &WorldEngine, role: &FileRole) -> Vec<Edit> {
    let mut edits = Vec::new();
    let str_of = |v: &Value| crate::parsers::id_value(v).map(|_| v.as_str().map(str::to_string).unwrap_or_else(|| format!("[[{}]]", crate::parsers::id_value(v).unwrap())));
    if kind == EntityKind::Location {
        if let Some(map) = meta.get("connections").and_then(Value::as_mapping) {
            for (dir, target) in map {
                if let (Some(dir), Some(t)) = (dir.as_str(), str_of(target)) {
                    if names_old(&t, old) {
                        edits.push(Edit::SetMapEntry { key: "connections".into(), subkey: dir.into(), value: Value::String(renamed(&t, new)) });
                    }
                }
            }
        }
        let scalar = if *role == FileRole::World { "start" } else { "default_location" };
        if let Some(t) = meta.get(scalar).and_then(|v| str_of(v)) {
            if names_old(&t, old) {
                edits.push(Edit::SetScalar { key: scalar.into(), value: Value::String(renamed(&t, new)) });
            }
        }
    }
    let lists: &[&str] = match kind {
        EntityKind::Character => &["characters"],
        EntityKind::Object => &["objects", "inventory"],
        EntityKind::Location | EntityKind::Door => &[],
    };
    for key in lists {
        if let Some(items) = meta.get(*key).and_then(Value::as_sequence) {
            let texts: Vec<String> = items.iter().filter_map(|v| str_of(v)).collect();
            if texts.iter().any(|t| names_old(t, old)) {
                let items = texts.iter().map(|t| if names_old(t, old) { renamed(t, new) } else { t.clone() }).collect();
                edits.push(Edit::SetList { key: (*key).into(), items });
            }
        }
    }
    let _ = engine;
    edits
}

/// A file's frontmatter as a map, as it reads now.
fn file_frontmatter(file: &ContentFile) -> Result<serde_yaml::Mapping> {
    let doc = MarkdownParser::new().split_document(&file.text())?;
    Ok(match doc.metadata {
        Value::Mapping(m) => m,
        _ => serde_yaml::Mapping::new(),
    })
}

/// `meta` with the renamed thing's references inside `actions` renamed:
/// `becomes:` keys (any kind) and, for an object, `requires: { has: }`,
/// each in the style it was written.
fn rename_in_actions(meta: &mut serde_yaml::Mapping, kind: EntityKind, old: &[String], new: &str) {
    let Some(Value::Mapping(actions)) = meta.get_mut("actions") else { return };
    for (_, action) in actions.iter_mut() {
        let Value::Mapping(action) = action else { continue };
        if let Some(Value::Mapping(becomes)) = action.get_mut("becomes") {
            let renamed_map: serde_yaml::Mapping = std::mem::take(becomes)
                .into_iter()
                .map(|(k, v)| match k.as_str() {
                    Some(name) if names_old(name, old) => (Value::String(swap(name, old, new)), v),
                    _ => (k, v),
                })
                .collect();
            *becomes = renamed_map;
        }
        if kind != EntityKind::Object {
            continue;
        }
        if let Some(Value::Mapping(req)) = action.get_mut("requires") {
            match req.get_mut("has") {
                Some(Value::String(s)) if names_old(s, old) => *s = swap(s, old, new),
                Some(Value::Sequence(items)) => {
                    for item in items.iter_mut() {
                        if let Value::String(s) = item {
                            if names_old(s, old) {
                                *s = swap(s, old, new);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// `value` with the old name in it replaced, keeping any link brackets.
fn swap(value: &str, old: &[String], new: &str) -> String {
    let name = link_target(value);
    match old.iter().find(|n| **n == name) {
        Some(n) => value.replacen(n.as_str(), new, 1),
        None => value.to_string(),
    }
}

/// References too deep for the writer to rewrite: an action's
/// `becomes:` keys and `requires: { has: }`.
fn nested_references(meta: &Value, kind: EntityKind, old: &[String], _engine: &WorldEngine) -> Vec<String> {
    let mut out = Vec::new();
    let Some(actions) = meta.get("actions").and_then(Value::as_mapping) else { return out };
    for (name, action) in actions {
        let name = name.as_str().unwrap_or("?");
        if let Some(becomes) = action.get("becomes").and_then(Value::as_mapping) {
            if becomes.keys().filter_map(Value::as_str).any(|k| names_old(k, old)) {
                out.push(format!("action `{}`'s `becomes:` names `{}`; change it by hand", name, old[0]));
            }
        }
        if kind == EntityKind::Object {
            let has = action.get("requires").and_then(|r| r.get("has"));
            let named = match has {
                Some(Value::String(s)) => names_old(s, old),
                Some(Value::Sequence(items)) => items.iter().filter_map(Value::as_str).any(|s| names_old(s, old)),
                _ => false,
            };
            if named {
                out.push(format!("action `{}`'s `requires: {{ has: }}` names `{}`; change it by hand", name, old[0]));
            }
        }
    }
    out
}

/// Prose with links and variables naming the old thing renamed:
/// `[[old]]`, `[[old|text]]`, `[[old#part]]`, `[text](old.md)` and
/// `{{ old.value }}`.
fn rewrite_prose(body: &str, old: &[String], new: &str) -> String {
    let mut out = body.to_string();
    for name in old {
        for (from, to) in [
            (format!("[[{}]]", name), format!("[[{}]]", new)),
            (format!("[[{}|", name), format!("[[{}|", new)),
            (format!("[[{}#", name), format!("[[{}#", new)),
            (format!("]({}.md)", name), format!("]({}.md)", new)),
            (format!("{{{{ {}.", name), format!("{{{{ {}.", new)),
            (format!("{{{{{}.", name), format!("{{{{{}.", new)),
        ] {
            out = out.replace(&from, &to);
        }
    }
    out
}

/// The nearest folder at or above `path` that exists, for the inside-the-
/// world check before anything is created.
pub(crate) fn nearest_existing(path: &Path) -> PathBuf {
    let mut p = path.to_path_buf();
    while !p.exists() {
        match p.parent() {
            Some(parent) => p = parent.to_path_buf(),
            None => break,
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::WorldEngine;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn world() -> (tempfile::TempDir, WorldEngine, WorldWriter) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "world/index.md", "---\ntitle: T\ntype: world\nstart: hallway\n---\n");
        write(d, "hallway.md", "---\nid: hallway\ntitle: Hallway\ntype: location\nconnections:\n  east: kitchen\nobjects: [lamp]\n---\nA long hallway.\n");
        write(d, "kitchen.md", "---\nid: kitchen\ntitle: Kitchen\ntype: location\nconnections:\n  west: hallway   # back the way you came\n  north: \"[[hallway]]\"\n---\nThe [[hallway]] is west; it's {{ hallway.title }}. See [it](hallway.md).\n");
        write(d, "castle/vault/index.md", "---\nid: castle/vault\ntitle: Vault\ntype: location\nstate: shut\nconnections: { up: kitchen }\n---\n");
        write(d, "castle/vault/shut.md", "---\ninherits: index\n---\nShut.\n");
        write(d, "castle/vault/chapters/after.md", "---\n---\nBroken.\n");
        write(d, "chapters/10-after.md", "---\n---\n");
        write(d, "objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: tool\n---\n");
        write(d, "objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { becomes: { castle/vault: shut }, requires: { has: lamp } }\n---\n<!-- thwarter:action pull -->\nClunk.\n<!-- thwarter:action pull refused -->\nDark.\n");
        write(d, "npcs/cook.md", "---\nid: cook\ntitle: Cook\ntype: character\ndefault_location: hallway\ninventory: [lamp]\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        assert!(engine.current_problems(true).is_empty(), "{:?}", engine.current_problems(true));
        let writer = WorldWriter::new(d).unwrap();
        (dir, engine, writer)
    }

    #[test]
    fn moving_a_room_into_an_area_renames_it_and_every_reference() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        let report = writer.rename_loaded(&mut engine, EntityKind::Location, "hallway", "house/hallway", true).unwrap();
        assert_eq!(report.moved, vec![(PathBuf::from("hallway.md"), PathBuf::from("house/hallway.md"))]);
        for f in ["house/hallway.md", "kitchen.md", "npcs/cook.md", "world/index.md"] {
            assert!(report.written.contains(&PathBuf::from(f)), "{} not in {:?}", f, report.written);
        }
        assert!(report.by_hand.is_empty(), "{:?}", report.by_hand);
        assert!(!d.join("hallway.md").exists());
        assert!(fs::read_to_string(d.join("house/hallway.md")).unwrap().contains("id: house/hallway\n"));
        let kitchen = fs::read_to_string(d.join("kitchen.md")).unwrap();
        assert!(kitchen.contains("west: house/hallway   # back the way you came\n"), "{}", kitchen);
        assert!(kitchen.contains("north: \"[[house/hallway]]\""), "{}", kitchen);
        assert!(kitchen.contains("The [[house/hallway]] is west; it's {{ house/hallway.title }}. See [it](house/hallway.md)."), "{}", kitchen);
        assert!(fs::read_to_string(d.join("npcs/cook.md")).unwrap().contains("default_location: house/hallway"));
        assert!(fs::read_to_string(d.join("world/index.md")).unwrap().contains("start: house/hallway"));

        // The engine took the same change, and a fresh load agrees.
        assert!(engine.get_location("hallway").is_none());
        assert_eq!(engine.get_location("kitchen").unwrap().connections["west"], "house/hallway");
        assert_eq!(engine.world().start.as_deref(), Some("house/hallway"));
        assert!(report.change.problems.is_empty(), "{:?}", report.change.problems);
        let mut fresh = WorldEngine::new();
        fresh.load_world(d).unwrap();
        assert!(fresh.current_problems(true).is_empty(), "{:?}", fresh.current_problems(true));
    }

    #[test]
    fn a_bundle_moves_whole_with_its_states_and_chapters() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        let report = writer.rename_loaded(&mut engine, EntityKind::Location, "castle/vault", "castle/strongroom", true).unwrap();
        assert!(!d.join("castle/vault").exists());
        for f in ["index.md", "shut.md", "chapters/after.md"] {
            assert!(d.join("castle/strongroom").join(f).exists(), "{}", f);
            assert!(report.moved.iter().any(|(_, to)| *to == Path::new("castle/strongroom").join(f)), "{:?}", report.moved);
        }
        let thing = engine.thing(EntityKind::Location, "castle/strongroom").unwrap();
        assert_eq!(thing.starting_state.as_deref(), Some("shut"));
        assert_eq!(thing.overlays.len(), 1);
        // The lever's becomes: is rewritten in place too.
        assert!(report.by_hand.is_empty(), "{:?}", report.by_hand);
        assert!(fs::read_to_string(d.join("objects/lever.md")).unwrap().contains("becomes: { castle/strongroom: shut }"));
        assert!(report.change.problems.is_empty(), "{:?}", report.change);
    }

    #[test]
    fn renaming_an_object_updates_lists_and_keeps_its_folder() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        let report = writer.rename_loaded(&mut engine, EntityKind::Object, "lamp", "lantern", true).unwrap();
        assert!(d.join("objects/lantern.md").exists() && !d.join("objects/lamp.md").exists());
        assert_eq!(engine.get_location("hallway").unwrap().objects, vec!["lantern"]);
        assert_eq!(engine.characters["cook"].inventory, vec!["lantern"]);
        assert!(report.by_hand.is_empty(), "{:?}", report.by_hand);
        assert!(fs::read_to_string(d.join("objects/lever.md")).unwrap().contains("requires: { has: lantern }"));
        assert!(report.change.problems.is_empty(), "{:?}", report.change);
    }

    #[test]
    fn a_rename_is_refused_before_anything_is_written() {
        let (dir, _, writer) = world();
        let d = dir.path();
        write(d, "house/hallway.md", "not a thing\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        let before = fs::read_to_string(d.join("kitchen.md")).unwrap();
        for (old, new, why) in [
            ("hallway", "house/hallway", "already exists"),
            ("hallway", "kitchen", "already exists"),
            ("hallway", "game", "can't be an id"),
            ("hallway", "hall.way", "can't be an id"),
            ("hallway", "../out", "can't be used as an id"),
            ("nowhere", "elsewhere", "no location `nowhere`"),
        ] {
            let err = writer.rename_loaded(&mut engine, EntityKind::Location, old, new, false).unwrap_err().to_string();
            assert!(err.contains(why), "{} -> {}: {}", old, new, err);
        }
        assert_eq!(fs::read_to_string(d.join("kitchen.md")).unwrap(), before);
        assert!(d.join("hallway.md").exists());
    }

    #[test]
    fn an_action_that_mentions_the_old_id_elsewhere_is_left_for_the_author() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        // `lamp` is also a value nobody should rename: the read-back check refuses.
        engine.set_file(&d.join("objects/lever.md"), "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { requires: { has: lamp }, sets: { story.clue: lamp } }\n---\n<!-- thwarter:action pull -->\nClunk.\n<!-- thwarter:action pull refused -->\nDark.\n");
        fs::write(d.join("objects/lever.md"), "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { requires: { has: lamp }, sets: { story.clue: lamp } }\n---\n<!-- thwarter:action pull -->\nClunk.\n<!-- thwarter:action pull refused -->\nDark.\n").unwrap();
        let before = fs::read_to_string(d.join("objects/lever.md")).unwrap();
        let report = writer.rename_loaded(&mut engine, EntityKind::Object, "lamp", "lantern", false).unwrap();
        assert_eq!(fs::read_to_string(d.join("objects/lever.md")).unwrap(), before);
        assert!(report.by_hand.iter().any(|w| w.starts_with("objects/lever.md: action `pull`'s `requires: { has: }` names `lamp`")), "{:?}", report.by_hand);
    }
}
