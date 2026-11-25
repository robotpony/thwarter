//! Moving a thing's file, or its bundle folder, to another folder while
//! its id stays the same (REQ-CON-29): an object into an area's
//! `objects/`, say. References name the id, so they're left as they are.
//! A room's id is its path, so moving a room is a rename
//! (`rename_loaded`), not this.

use super::rename::nearest_existing;
use super::{check_folder, ChangeReport, WorldWriter};
use crate::engines::{EntityKind, WorldEngine};
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

impl WorldWriter {
    /// Move `(kind, id)`'s file, or its bundle folder whole, into
    /// `to_folder` (relative to the root), keeping its name and id.
    /// Refused for a room, for a target that's taken, and for a folder
    /// outside the world. Then `engine` takes the same change.
    pub fn move_loaded(&self, engine: &mut WorldEngine, kind: EntityKind, id: &str, to_folder: &Path, strict: bool) -> Result<ChangeReport> {
        if kind == EntityKind::Location {
            bail!("a room's id is its path, so moving room `{}` changes its id; rename it instead", id);
        }
        check_folder(to_folder)?;
        engine.source_path(kind, id)?;
        let engine_root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        let rel = |p: &Path| p.strip_prefix(&engine_root).unwrap_or(p).to_path_buf();
        let thing = engine.thing(kind, id).expect("checked by source_path").clone();
        let from = match &thing.bundle {
            Some(folder) => folder.clone(),
            None => rel(&thing.index.path),
        };
        let name = from.file_name().ok_or_else(|| anyhow!("{} has no name to move", from.display()))?;
        let to = to_folder.join(name);
        if to == from {
            bail!("{} {} is already in {}", kind.type_name(), id, display_folder(to_folder));
        }
        let target = self.root().join(&to);
        if target.exists() {
            bail!("{} already exists", to.display());
        }
        self.check_inside(&nearest_existing(target.parent().unwrap_or(self.root())))?;

        // The files that go, from the engine's own list, before any move.
        let files: Vec<PathBuf> = engine.file_roles().map(|(p, _)| rel(p)).filter(|p| if thing.bundle.is_some() { p.starts_with(&from) } else { *p == from }).collect();
        let before = engine.current_problems(strict);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("can't create {}", parent.display()))?;
        }
        fs::rename(self.root().join(&from), &target).with_context(|| format!("can't move {} to {}", from.display(), to.display()))?;
        for old in &files {
            let new = match thing.bundle {
                Some(_) => to.join(old.strip_prefix(&from).expect("under the bundle")),
                None => to.clone(),
            };
            let text = fs::read_to_string(self.root().join(&new)).with_context(|| format!("moved, but can't read {}", new.display()))?;
            engine.remove_file(&engine_root.join(old));
            engine.set_file(&engine_root.join(&new), &text);
        }
        let index = match thing.bundle {
            Some(_) => engine_root.join(&to).join("index.md"),
            None => engine_root.join(&to),
        };
        Ok(ChangeReport::new(index, before, engine.current_problems(strict)))
    }
}

fn display_folder(folder: &Path) -> String {
    if folder.as_os_str().is_empty() { "the world's root".to_string() } else { format!("{}/", folder.display()) }
}

#[cfg(test)]
mod tests {
    use crate::engines::{EntityKind, WorldEngine};
    use crate::writer::{NewEntity, WorldWriter};
    use std::fs;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, text: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    #[test]
    fn things_load_from_area_folders_are_made_there_and_move_keeping_their_id() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\nobjects: [torch, lamp, desk]\ncharacters: [cook]\n---\n");
        write(d, "castle/objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: light\n---\n");
        write(d, "castle/npcs/cook.md", "---\nid: cook\ntitle: Cook\ntype: character\n---\n");
        write(d, "castle/rope.md", "---\nid: rope\ntitle: Rope\ntype: object\nobject_type: tool\n---\n");
        write(d, "objects/desk/index.md", "---\nid: desk\ntitle: Desk\ntype: object\nobject_type: furniture\nstate: shut\n---\n");
        write(d, "objects/desk/shut.md", "---\ninherits: index\n---\nShut.\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        // The loader finds things in an area's objects/ and npcs/, and loose in it.
        assert!(engine.objects.contains_key("lamp") && engine.objects.contains_key("rope") && engine.characters.contains_key("cook"), "{:?}", engine.objects.keys().collect::<Vec<_>>());

        let root = engine.world_root.clone().unwrap();
        let writer = WorldWriter::new(&root).unwrap();
        let torch = NewEntity { kind: EntityKind::Object, id: "torch".into(), title: "Torch".into(), object_type: Some("light".into()) };
        let report = writer.create_loaded_in(&mut engine, &torch, Some(Path::new("castle/objects")), &[], false).unwrap();
        assert_eq!(report.path, root.join("castle/objects/torch.md"));
        assert!(d.join("castle/objects/torch.md").is_file() && engine.objects.contains_key("torch"));
        assert!(writer.create_loaded_in(&mut engine, &torch, Some(Path::new("../out")), &[], false).is_err());

        // A one-file thing moves, keeping its id; the room still names it.
        writer.move_loaded(&mut engine, EntityKind::Object, "rope", Path::new("castle/objects"), false).unwrap();
        assert!(d.join("castle/objects/rope.md").is_file() && !d.join("castle/rope.md").exists());
        assert_eq!(engine.source_path(EntityKind::Object, "rope").unwrap(), root.join("castle/objects/rope.md"));
        // A bundle moves whole.
        let report = writer.move_loaded(&mut engine, EntityKind::Object, "desk", Path::new("castle/objects"), false).unwrap();
        assert!(d.join("castle/objects/desk/shut.md").is_file() && !d.join("objects/desk").exists());
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(engine.thing(EntityKind::Object, "desk").unwrap().state_names(), ["shut"]);
        // Never onto something there, never a room.
        write(d, "objects/lamp.md", "x");
        assert!(writer.move_loaded(&mut engine, EntityKind::Object, "lamp", Path::new("objects"), false).unwrap_err().to_string().contains("already exists"));
        assert!(writer.move_loaded(&mut engine, EntityKind::Location, "castle/hall", Path::new("keep"), false).is_err());
    }
}
