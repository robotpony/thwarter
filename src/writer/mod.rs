//! Writing entities back to their files (REQ-ENG-7).
//!
//! `ContentFile` edits one file's text and checks each edit by re-parsing.
//! `WorldWriter` decides which file: an existing entity's source file from
//! the loaded world, or for a new entity the path `design/CONTENT_FORMAT.md`
//! gives its kind, refusing anything that would land outside the world.

pub mod content_file;

pub use content_file::{ContentFile, Edit};

use crate::engines::{EntityKind, WorldEngine};
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Where a new entity's file goes, relative to the world root
/// (`design/CONTENT_FORMAT.md` › Keys, order and file locations). The one
/// place this rule lives; Bruce may change it.
pub fn new_entity_relative_path(kind: EntityKind, id: &str) -> PathBuf {
    match kind {
        EntityKind::Location => PathBuf::from(format!("{}.md", id)),
        EntityKind::Character => Path::new("npcs").join(format!("{}.md", id)),
        EntityKind::Object => Path::new("objects").join(format!("{}.md", id)),
    }
}

/// A new entity: the fields every file of its kind must have.
#[derive(Debug, Clone)]
pub struct NewEntity {
    pub kind: EntityKind,
    pub id: String,
    pub title: String,
    /// Required for objects, ignored otherwise.
    pub object_type: Option<String>,
}

/// What a write did to the loaded world's problems (REQ-CON-5).
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeReport {
    /// The file written, in the engine's path form (as in `sources`).
    pub path: PathBuf,
    /// Every problem after the change, as `WorldEngine::current_problems`.
    pub problems: Vec<String>,
    /// Problems the change introduced.
    pub added: Vec<String>,
    /// Problems the change fixed.
    pub resolved: Vec<String>,
}

impl ChangeReport {
    fn new(path: PathBuf, before: Vec<String>, after: Vec<String>) -> Self {
        let added = after.iter().filter(|p| !before.contains(p)).cloned().collect();
        let resolved = before.iter().filter(|p| !after.contains(p)).cloned().collect();
        Self { path, problems: after, added, resolved }
    }
}

/// Writes entities inside one world directory.
#[derive(Debug, Clone)]
pub struct WorldWriter {
    root: PathBuf,
}

impl WorldWriter {
    pub fn new(world_root: impl AsRef<Path>) -> Result<Self> {
        let root = world_root.as_ref();
        let root = fs::canonicalize(root).with_context(|| format!("world directory {} not found", root.display()))?;
        if !root.is_dir() {
            bail!("{} is not a directory", root.display());
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Apply `edits` to the file `(kind, id)` was loaded from, and save it.
    /// Refused if the id is defined in two files, if the file is outside
    /// the world, or if it no longer holds that entity (changed on disk
    /// since the load). All or nothing; returns the path written.
    pub fn edit(&self, engine: &WorldEngine, kind: EntityKind, id: &str, edits: &[Edit]) -> Result<PathBuf> {
        Ok(self.edit_text(engine, kind, id, edits)?.0)
    }

    /// `edit`, returning the path and the exact text written.
    fn edit_text(&self, engine: &WorldEngine, kind: EntityKind, id: &str, edits: &[Edit]) -> Result<(PathBuf, String)> {
        let path = engine.source_path(kind, id)?.to_path_buf();
        self.check_inside(&path)?;
        let mut file = ContentFile::load(&path)?;
        if file.type_name() != kind.type_name() || file.id() != id {
            bail!(
                "{} now holds {} `{}`, not {} `{}`; reload the world first",
                path.display(),
                file.type_name(),
                file.id(),
                kind.type_name(),
                id
            );
        }
        file.apply_all(edits)?;
        file.save()?;
        Ok((path, file.text()))
    }

    /// Create a new entity's file at the path its kind gives, then apply
    /// `edits` to it before writing. Refuses an existing file and any
    /// path outside the world. Returns the path written.
    pub fn create(&self, new: &NewEntity, edits: &[Edit]) -> Result<PathBuf> {
        Ok(self.create_text(new, edits)?.0)
    }

    /// `create`, returning the path and the exact text written.
    fn create_text(&self, new: &NewEntity, edits: &[Edit]) -> Result<(PathBuf, String)> {
        let path = self.new_entity_path(new.kind, &new.id)?;
        if path.exists() {
            bail!("{} already exists", path.display());
        }
        let mut text = String::from("---\n");
        text.push_str(&format!("id: {}\n", yaml_text(&new.id)?));
        text.push_str(&format!("title: {}\n", yaml_text(&new.title)?));
        text.push_str(&format!("type: {}\n", new.kind.type_name()));
        if new.kind == EntityKind::Object {
            let object_type = new
                .object_type
                .as_deref()
                .ok_or_else(|| anyhow!("a new object needs an `object_type`"))?;
            text.push_str(&format!("object_type: {}\n", yaml_text(object_type)?));
        }
        text.push_str("---\n");

        let mut file = ContentFile::from_text(&path, &text)?;
        file.apply_all(edits)?;

        let parent = path.parent().expect("an entity path has a parent");
        fs::create_dir_all(parent).with_context(|| format!("can't create {}", parent.display()))?;
        self.check_inside(parent)?;
        file.save_new()?;
        Ok((path, file.text()))
    }

    /// `edit`, then the same change in the loaded `engine` (REQ-ENG-8),
    /// from the exact text written, without re-reading the world. Reports
    /// the problems after the change and which ones it added or fixed. If
    /// the write fails, `engine` is untouched.
    pub fn edit_loaded(
        &self,
        engine: &mut WorldEngine,
        kind: EntityKind,
        id: &str,
        edits: &[Edit],
        strict: bool,
    ) -> Result<ChangeReport> {
        let before = engine.current_problems(strict);
        let (path, text) = self.edit_text(engine, kind, id, edits)?;
        engine.set_file(&path, &text);
        Ok(ChangeReport::new(path, before, engine.current_problems(strict)))
    }

    /// `create`, then the same change in the loaded `engine`. The path
    /// reported (and recorded in `engine.sources`) is in the engine's own
    /// form: its `world_root` joined with the new file's relative path.
    pub fn create_loaded(
        &self,
        engine: &mut WorldEngine,
        new: &NewEntity,
        edits: &[Edit],
        strict: bool,
    ) -> Result<ChangeReport> {
        let engine_root = engine
            .world_root
            .clone()
            .ok_or_else(|| anyhow!("the engine has no loaded world to add `{}` to", new.id))?;
        let before = engine.current_problems(strict);
        let (_, text) = self.create_text(new, edits)?;
        let path = engine_root.join(new_entity_relative_path(new.kind, &new.id));
        engine.set_file(&path, &text);
        Ok(ChangeReport::new(path, before, engine.current_problems(strict)))
    }

    /// The absolute path a new `(kind, id)` would be written to, after
    /// checking the id can't escape the world directory.
    pub fn new_entity_path(&self, kind: EntityKind, id: &str) -> Result<PathBuf> {
        check_id(id)?;
        let path = self.root.join(new_entity_relative_path(kind, id));
        // Resolve symlinks in the part of the path that already exists.
        let mut existing = path.parent().expect("an entity path has a parent").to_path_buf();
        while !existing.exists() {
            existing = existing
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| anyhow!("bad path {}", path.display()))?;
        }
        self.check_inside(&existing)?;
        Ok(path)
    }

    fn check_inside(&self, path: &Path) -> Result<()> {
        let resolved = fs::canonicalize(path).with_context(|| format!("can't resolve {}", path.display()))?;
        if !resolved.starts_with(&self.root) {
            bail!(
                "{} is outside the world directory {}",
                path.display(),
                self.root.display()
            );
        }
        Ok(())
    }
}

/// An id usable as a path: `/`-separated segments, none empty, `.`, `..`
/// or hidden, no backslashes, colons or control characters.
fn check_id(id: &str) -> Result<()> {
    let bad = || {
        anyhow!(
            "`{}` can't be used as an id: it must be `/`-separated names, without `..`",
            id
        )
    };
    if id.is_empty() || id.starts_with('/') || id.contains(['\\', ':']) || id.chars().any(char::is_control) {
        return Err(bad());
    }
    for segment in id.split('/') {
        if segment.is_empty() || segment.starts_with('.') || segment.trim() != segment {
            return Err(bad());
        }
    }
    if Path::new(id).components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(bad());
    }
    Ok(())
}

fn yaml_text(s: &str) -> Result<String> {
    content_file::emit_scalar(&serde_yaml::Value::String(s.to_string()), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        };
        write(
            "castle/yard.md",
            "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  north: castle/hall\n---\n\nGrass.\n",
        );
        write(
            "castle/hall.md",
            "---\nid: castle/hall\ntitle: Hall\ntype: location\n---\n",
        );
        write("npcs/king.md", "---\nid: king\ntitle: King\ntype: character\n---\n");
        dir
    }

    fn loaded(dir: &Path) -> WorldEngine {
        let mut engine = WorldEngine::new();
        engine.load_world(dir).unwrap();
        engine
    }

    #[test]
    fn new_entity_paths_follow_the_content_format() {
        assert_eq!(
            new_entity_relative_path(EntityKind::Location, "castle/gate"),
            PathBuf::from("castle/gate.md")
        );
        assert_eq!(
            new_entity_relative_path(EntityKind::Character, "smith"),
            PathBuf::from("npcs/smith.md")
        );
        assert_eq!(
            new_entity_relative_path(EntityKind::Object, "rope"),
            PathBuf::from("objects/rope.md")
        );
    }

    #[test]
    fn edit_writes_the_source_file_and_reloads_cleanly() {
        let dir = world();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let engine = loaded(dir.path());
        let path = writer
            .edit(
                &engine,
                EntityKind::Location,
                "castle/yard",
                &[Edit::SetMapEntry {
                    key: "connections".into(),
                    subkey: "south".into(),
                    value: serde_yaml::Value::String("castle/hall".into()),
                }],
            )
            .unwrap();
        assert!(path.ends_with("castle/yard.md"));
        let reloaded = loaded(dir.path());
        assert_eq!(
            reloaded.get_location("castle/yard").unwrap().connections["south"],
            "castle/hall"
        );
        assert_eq!(reloaded.get_location("castle/yard").unwrap().description, "Grass.");
    }

    #[test]
    fn edit_refuses_duplicate_ids() {
        let dir = world();
        fs::write(
            dir.path().join("castle/yard-copy.md"),
            "---\nid: castle/yard\ntitle: Y2\ntype: location\n---\n",
        )
        .unwrap();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let e = writer
            .edit(
                &loaded(dir.path()),
                EntityKind::Location,
                "castle/yard",
                &[Edit::SetBody("x".into())],
            )
            .unwrap_err();
        assert!(e.to_string().contains("defined in 2 files"), "{}", e);
    }

    #[test]
    fn edit_refuses_a_file_changed_since_load() {
        let dir = world();
        let engine = loaded(dir.path());
        fs::write(
            dir.path().join("castle/yard.md"),
            "---\nid: castle/moved\ntitle: Y\ntype: location\n---\n",
        )
        .unwrap();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let e = writer
            .edit(
                &engine,
                EntityKind::Location,
                "castle/yard",
                &[Edit::SetBody("x".into())],
            )
            .unwrap_err();
        assert!(e.to_string().contains("reload the world first"), "{}", e);
    }

    #[cfg(unix)]
    #[test]
    fn save_keeps_permissions_and_leaves_no_temp_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = world();
        let path = dir.path().join("castle/yard.md");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let writer = WorldWriter::new(dir.path()).unwrap();
        writer
            .edit(
                &loaded(dir.path()),
                EntityKind::Location,
                "castle/yard",
                &[Edit::SetBody("x".into())],
            )
            .unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o640);
        let leftovers: Vec<_> = fs::read_dir(dir.path().join("castle"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn create_writes_each_kind_where_the_format_says() {
        let dir = world();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let gate = writer
            .create(
                &NewEntity {
                    kind: EntityKind::Location,
                    id: "tower/top".into(),
                    title: "Tower Top".into(),
                    object_type: None,
                },
                &[
                    Edit::SetMapEntry {
                        key: "connections".into(),
                        subkey: "down".into(),
                        value: serde_yaml::Value::String("castle/yard".into()),
                    },
                    Edit::SetBody("Wind.".into()),
                ],
            )
            .unwrap();
        assert!(gate.ends_with("tower/top.md"));
        assert_eq!(
            fs::read_to_string(&gate).unwrap(),
            "---\nid: tower/top\ntitle: Tower Top\ntype: location\nconnections:\n  down: castle/yard\n---\n\nWind.\n"
        );
        let rope = writer
            .create(
                &NewEntity {
                    kind: EntityKind::Object,
                    id: "rope".into(),
                    title: "Rope".into(),
                    object_type: Some("tool".into()),
                },
                &[],
            )
            .unwrap();
        assert!(rope.ends_with("objects/rope.md"));
        let smith = writer
            .create(
                &NewEntity {
                    kind: EntityKind::Character,
                    id: "smith".into(),
                    title: "Smith".into(),
                    object_type: None,
                },
                &[],
            )
            .unwrap();
        assert!(smith.ends_with("npcs/smith.md"));

        let engine = loaded(dir.path());
        assert!(engine.get_location("tower/top").is_some());
        assert!(engine.objects.contains_key("rope"));
        assert!(engine.characters.contains_key("smith"));
    }

    #[test]
    fn create_refuses_existing_files_and_objects_without_a_type() {
        let dir = world();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let hall = NewEntity {
            kind: EntityKind::Location,
            id: "castle/hall".into(),
            title: "H".into(),
            object_type: None,
        };
        assert!(writer
            .create(&hall, &[])
            .unwrap_err()
            .to_string()
            .contains("already exists"));
        let rope = NewEntity {
            kind: EntityKind::Object,
            id: "rope".into(),
            title: "R".into(),
            object_type: None,
        };
        assert!(writer
            .create(&rope, &[])
            .unwrap_err()
            .to_string()
            .contains("object_type"));
    }

    #[test]
    fn ids_that_could_escape_the_world_are_refused() {
        let dir = world();
        let writer = WorldWriter::new(dir.path()).unwrap();
        for id in [
            "../outside",
            "castle/../../x",
            "/etc/passwd",
            "a//b",
            "a/./b",
            ".hidden",
            "a\\b",
            "c:x",
            "",
            "a/ b",
        ] {
            assert!(
                writer.new_entity_path(EntityKind::Location, id).is_err(),
                "`{}` was allowed",
                id
            );
        }
        assert!(writer.new_entity_path(EntityKind::Location, "castle/gate").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_world_is_refused() {
        let dir = world();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("portal")).unwrap();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let e = writer.new_entity_path(EntityKind::Location, "portal/room").unwrap_err();
        assert!(e.to_string().contains("outside the world"), "{}", e);
    }

    fn copy_dir(from: &Path, to: &Path) {
        for entry in walkdir::WalkDir::new(from).into_iter().filter_map(|e| e.ok()) {
            let dest = to.join(entry.path().strip_prefix(from).unwrap());
            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest).unwrap();
            } else {
                fs::copy(entry.path(), &dest).unwrap();
            }
        }
    }

    /// Every entity in the shipped test world takes a one-field edit that
    /// changes exactly one line, and the world reloads with the same problems.
    #[test]
    fn editing_the_test_world_changes_one_line_per_file() {
        let dir = tempfile::tempdir().unwrap();
        copy_dir(Path::new("authors/mx/test-world"), dir.path());
        let mut engine = WorldEngine::new();
        let report = engine.load_world(dir.path()).unwrap();
        let problems_before = engine.problems(&report, true);
        let writer = WorldWriter::new(dir.path()).unwrap();

        let mut keys: Vec<_> = engine.sources.keys().cloned().collect();
        keys.sort();
        assert!(keys.len() >= 50);
        for (kind, id) in keys {
            let path = engine.source_path(kind, &id).unwrap().to_path_buf();
            let before = fs::read_to_string(&path).unwrap();
            let title = format!("Edited {}", id);
            writer
                .edit(
                    &engine,
                    kind,
                    &id,
                    &[Edit::SetScalar {
                        key: "title".into(),
                        value: serde_yaml::Value::String(title),
                    }],
                )
                .unwrap();
            let after = fs::read_to_string(&path).unwrap();
            let changed: Vec<_> = before.lines().zip(after.lines()).filter(|(a, b)| a != b).collect();
            assert_eq!(before.lines().count(), after.lines().count(), "{}", path.display());
            assert_eq!(changed.len(), 1, "{}: {:?}", path.display(), changed);
            assert!(changed[0].1.starts_with("title: "), "{}", path.display());
        }

        let mut reloaded = WorldEngine::new();
        let report = reloaded.load_world(dir.path()).unwrap();
        assert_eq!(reloaded.problems(&report, true), problems_before);
        assert_eq!(
            reloaded.get_location("castle/courtyard").unwrap().name,
            "Edited castle/courtyard"
        );
    }

    #[test]
    fn edit_loaded_updates_the_engine_and_reports_what_changed() {
        let dir = world();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let mut engine = loaded(dir.path());
        let link = |target: &str| Edit::SetMapEntry {
            key: "connections".into(),
            subkey: "west".into(),
            value: serde_yaml::Value::String(target.into()),
        };

        let report = writer.edit_loaded(&mut engine, EntityKind::Location, "castle/yard", &[link("castle/gone")], false).unwrap();
        assert_eq!(report.path, engine.source_path(EntityKind::Location, "castle/yard").unwrap());
        assert_eq!(report.added.len(), 1, "{:?}", report);
        assert!(report.added[0].contains("`west` points at unknown location `castle/gone`"));
        assert!(report.resolved.is_empty());
        assert_eq!(engine.get_location("castle/yard").unwrap().connections["west"], "castle/gone");

        let report = writer.edit_loaded(&mut engine, EntityKind::Location, "castle/yard", &[link("castle/hall")], false).unwrap();
        assert!(report.added.is_empty());
        assert_eq!(report.resolved.len(), 1);
        assert!(report.problems.is_empty());

        // The engine matches a fresh load of what's on disk.
        let fresh = loaded(dir.path());
        assert_eq!(fresh.get_location("castle/yard").unwrap().connections, engine.get_location("castle/yard").unwrap().connections);
    }

    #[test]
    fn create_loaded_adds_the_entity_under_the_engines_path() {
        let dir = world();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let mut engine = loaded(dir.path());
        let new = NewEntity { kind: EntityKind::Location, id: "castle/gate".into(), title: "Gate".into(), object_type: None };
        let link = Edit::SetMapEntry {
            key: "connections".into(),
            subkey: "south".into(),
            value: serde_yaml::Value::String("castle/moat".into()),
        };
        let report = writer.create_loaded(&mut engine, &new, &[link], false).unwrap();
        assert_eq!(report.path, dir.path().join("castle/gate.md"));
        assert_eq!(engine.source_path(EntityKind::Location, "castle/gate").unwrap(), dir.path().join("castle/gate.md"));
        assert_eq!(report.added.len(), 1);
        assert!(engine.get_location("castle/gate").is_some());
    }

    #[test]
    fn a_refused_edit_leaves_the_engine_alone() {
        let dir = world();
        let writer = WorldWriter::new(dir.path()).unwrap();
        let mut engine = loaded(dir.path());
        let before = engine.current_problems(true);
        let bad = Edit::SetScalar { key: "id".into(), value: serde_yaml::Value::String("x".into()) };
        assert!(writer.edit_loaded(&mut engine, EntityKind::Location, "castle/yard", &[bad], true).is_err());
        assert_eq!(engine.current_problems(true), before);
        assert_eq!(engine.get_location("castle/yard").unwrap().name, "Yard");
    }
}
