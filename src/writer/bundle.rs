//! Writing bundles (REQ-ENG-28, 29): turning a one-file thing into a
//! bundle, and saving a bundle's files together, as the console's bundle
//! editor holds them (one document, a section per file).

use super::content_file::ContentFile;
use super::{ChangeReport, WorldWriter};
use crate::engines::{EntityKind, WorldEngine};
use crate::parsers::MarkdownParser;
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Names a state file can't have (REQ-ENG-11).
const RESERVED: &[&str] = &["index", "_index", "chapters", "templates"];

/// One file of a bundle, as the editor holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// Relative to the world root: the bundle's `index.md`, a state file,
    /// or a file in its `chapters/`.
    pub path: PathBuf,
    pub text: String,
    /// The text the editor opened. If the file on disk differs, the save
    /// is refused. `None` skips the check (a forced save), and is what a
    /// file that doesn't exist yet takes.
    pub expected: Option<String>,
}

/// What a bundle write did. Paths are relative to the world root.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleReport {
    /// Existing files whose text changed.
    pub written: Vec<PathBuf>,
    /// Files that didn't exist before.
    pub created: Vec<PathBuf>,
    /// Sections whose text was already on disk, left alone.
    pub unchanged: Vec<PathBuf>,
    /// Files moved, from and to.
    pub moved: Vec<(PathBuf, PathBuf)>,
    /// Files removed.
    pub deleted: Vec<PathBuf>,
    /// References the writer found but left for the author, each with its
    /// file: a state rename it couldn't rewrite safely, or references to
    /// a deleted state.
    pub by_hand: Vec<String>,
    pub change: ChangeReport,
}

fn state_name_ok(name: &str) -> Result<()> {
    if name.is_empty() || name.contains(['/', '\\', '.', ':']) || name.starts_with('-') || name.chars().any(char::is_whitespace) {
        bail!("`{}` can't be a state name: use one word, like `open` or `burnt-out`", name);
    }
    if RESERVED.contains(&name) {
        bail!("`{}` is a reserved name and can't be a state", name);
    }
    Ok(())
}

fn relative_path_ok(path: &Path) -> Result<()> {
    if path.extension().and_then(|e| e.to_str()) != Some("md") || path.components().any(|c| !matches!(c, Component::Normal(_))) {
        bail!("must be a `.md` path inside the world, relative to its root");
    }
    Ok(())
}

impl WorldWriter {
    /// Turn a one-file thing into a bundle (REQ-ENG-28): `lantern.md`
    /// becomes `lantern/index.md`, with the same id. With `first_state`,
    /// a state file starting from the index is written beside it; the
    /// index is left as it was, so the thing has no starting state until
    /// one is set. The new files are written first and the old file is
    /// removed last, so a half-done move leaves a duplicate id that
    /// `validate` reports, never a lost file.
    pub fn make_bundle_loaded(
        &self,
        engine: &mut WorldEngine,
        kind: EntityKind,
        id: &str,
        first_state: Option<&str>,
        strict: bool,
    ) -> Result<BundleReport> {
        let engine_root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        let source = engine.source_path(kind, id)?.to_path_buf();
        let thing = engine.thing(kind, id).ok_or_else(|| anyhow!("no {} `{}` is loaded", kind.type_name(), id))?;
        if let Some(folder) = &thing.bundle {
            bail!("{} `{}` is already a bundle, in {}/", kind.type_name(), id, folder.display());
        }
        if let Some(name) = first_state {
            state_name_ok(name)?;
        }
        let file = source.strip_prefix(&engine_root).unwrap_or(&source).to_path_buf();
        let folder = file.with_extension("");
        let folder_abs = self.root().join(&folder);
        if folder_abs.exists() {
            bail!("{}/ already exists", folder.display());
        }
        let old_abs = self.root().join(&file);
        self.check_inside(&old_abs)?;
        let text = fs::read_to_string(&old_abs).with_context(|| format!("can't read {}", file.display()))?;
        let perms = fs::metadata(&old_abs)?.permissions();

        let index = folder.join("index.md");
        let index_file = ContentFile::from_text(self.root().join(&index), &text)?;
        let state = match first_state {
            Some(name) => {
                let path = folder.join(format!("{}.md", name));
                Some((path.clone(), ContentFile::layer_from_text(self.root().join(&path), "---\ninherits: index\n---\n")?))
            }
            None => None,
        };

        let before = engine.current_problems(strict);
        fs::create_dir_all(&folder_abs).with_context(|| format!("can't create {}/", folder.display()))?;
        self.check_inside(&folder_abs)?;
        index_file.save_new()?;
        fs::set_permissions(self.root().join(&index), perms)?;
        let mut created = vec![index.clone()];
        if let Some((path, file)) = &state {
            file.save_new()?;
            created.push(path.clone());
        }
        fs::remove_file(&old_abs)
            .with_context(|| format!("wrote {} but can't remove {}; delete it to clear the duplicate", index.display(), file.display()))?;

        engine.remove_file(&source);
        engine.set_file(&engine_root.join(&index), &text);
        if let Some((path, file)) = &state {
            engine.set_file(&engine_root.join(path), &file.text());
        }
        let change = ChangeReport::new(engine_root.join(&index), before, engine.current_problems(strict));
        Ok(BundleReport { written: Vec::new(), created, unchanged: Vec::new(), moved: vec![(file, index)], deleted: Vec::new(), by_hand: Vec::new(), change })
    }

    /// Save a bundle's files together (REQ-ENG-29). Each section must be
    /// one of the bundle's files (a one-file thing has only its own), must
    /// parse as what it is there (the index as the thing, with the same id
    /// and type; a state or chapter file as a layer with no `id`, `type`
    /// or `state:`), and must match `expected` on disk. The merged thing
    /// must still load, checked on a copy of the engine. Only then is
    /// anything written, and only the sections that changed. If a write
    /// fails partway, the error names the files already written.
    pub fn save_bundle_loaded(
        &self,
        engine: &mut WorldEngine,
        kind: EntityKind,
        id: &str,
        sections: &[Section],
        strict: bool,
    ) -> Result<BundleReport> {
        let engine_root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        let source = engine.source_path(kind, id)?.to_path_buf();
        let thing = engine.thing(kind, id).ok_or_else(|| anyhow!("no {} `{}` is loaded", kind.type_name(), id))?.clone();
        let own_file = source.strip_prefix(&engine_root).unwrap_or(&source).to_path_buf();

        // Check every section, in memory.
        let mut files: Vec<(PathBuf, ContentFile, bool)> = Vec::new(); // path, file, exists
        let mut unchanged = Vec::new();
        for section in sections {
            let path = &section.path;
            let checked = (|| -> Result<Option<(ContentFile, bool)>> {
                relative_path_ok(path)?;
                let is_index = *path == own_file;
                if !is_index {
                    let folder = thing.bundle.as_ref().ok_or_else(|| {
                        anyhow!("{} `{}` is one file, {}; make it a bundle first", kind.type_name(), id, own_file.display())
                    })?;
                    let inner = path.strip_prefix(folder).map_err(|_| anyhow!("isn't in {}/", folder.display()))?;
                    let parts: Vec<String> = inner.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                    match parts.as_slice() {
                        [file] => state_name_ok(file.trim_end_matches(".md"))?,
                        [dir, ..] if dir == "chapters" => {}
                        _ => bail!("must be a state file or in {}/chapters/", folder.display()),
                    }
                }
                let abs = self.root().join(path);
                let on_disk = fs::read_to_string(&abs).ok();
                match (&section.expected, &on_disk) {
                    (Some(expected), Some(disk)) if expected != disk => bail!("changed on disk since it was opened; reload it first"),
                    (Some(_), None) => bail!("is gone from disk since it was opened"),
                    _ => {}
                }
                if on_disk.as_deref() == Some(section.text.as_str()) {
                    return Ok(None);
                }
                let file = if is_index {
                    let file = ContentFile::from_text(&abs, &section.text).map_err(|e| anyhow!("{:#}", e))?;
                    if file.type_name() != kind.type_name() || file.id() != id {
                        bail!("must stay {} `{}`; changing the id is a rename", kind.type_name(), id);
                    }
                    file
                } else {
                    let doc = MarkdownParser::new().split_document(&section.text).map_err(|e| anyhow!("doesn't parse: {}", e))?;
                    for key in ["id", "type", "state"] {
                        if doc.metadata.get(key).is_some() {
                            bail!("only the index sets `{}`", key);
                        }
                    }
                    ContentFile::layer_from_text(&abs, &section.text)?
                };
                let existing = abs.parent().unwrap_or(self.root()).ancestors().find(|p| p.exists()).unwrap_or(self.root()).to_path_buf();
                self.check_inside(&existing)?;
                Ok(Some((file, on_disk.is_some())))
            })()
            .map_err(|e| anyhow!("{}: {:#}", path.display(), e))?;
            match checked {
                None => unchanged.push(path.clone()),
                Some((file, exists)) => files.push((path.clone(), file, exists)),
            }
        }

        // The merged thing must still load.
        let mut trial = engine.clone();
        for (path, file, _) in &files {
            trial.set_file(&engine_root.join(path), &file.text());
        }
        if trial.thing(kind, id).is_none() {
            let shown: Vec<String> = files.iter().map(|(p, _, _)| format!("{}: ", engine_root.join(p).display())).collect();
            let why: Vec<String> = trial.current_problems(false).into_iter().filter(|p| shown.iter().any(|s| p.starts_with(s.as_str()))).collect();
            bail!("{} `{}` wouldn't load with these sections: {}", kind.type_name(), id, why.join("; "));
        }

        // Write only what changed.
        let before = engine.current_problems(strict);
        let mut written = Vec::new();
        let mut created = Vec::new();
        for (path, file, exists) in &files {
            let done = |w: &Vec<PathBuf>, c: &Vec<PathBuf>| {
                let all: Vec<String> = w.iter().chain(c).map(|p| p.display().to_string()).collect();
                if all.is_empty() { "nothing was written".to_string() } else { format!("already written: {}", all.join(", ")) }
            };
            if *exists {
                file.save().with_context(|| format!("can't save {} ({})", path.display(), done(&written, &created)))?;
                written.push(path.clone());
            } else {
                if let Some(parent) = self.root().join(path).parent() {
                    fs::create_dir_all(parent)?;
                }
                file.save_new().with_context(|| format!("can't create {} ({})", path.display(), done(&written, &created)))?;
                created.push(path.clone());
            }
        }
        for (path, file, _) in &files {
            engine.set_file(&engine_root.join(path), &file.text());
        }
        let change = ChangeReport::new(source, before, engine.current_problems(strict));
        Ok(BundleReport { written, created, unchanged, moved: Vec::new(), deleted: Vec::new(), by_hand: Vec::new(), change })
    }
}

/// Where an action switching a thing to a state is written: the file,
/// the action's name, and whether it's in a `## When:` section's block
/// rather than the file's frontmatter.
struct BecomesAt {
    path: PathBuf,
    action: String,
    in_section: bool,
    /// A thing's index or one file, rather than a layer with no `id`.
    whole: bool,
}

/// Every action whose `becomes:` names `target` (by id or link) with state
/// `state`, in the file that holds it: each thing's index, state files and
/// chapter overlays, frontmatter and section blocks both. The merged
/// actions won't do, since an action in a state file isn't in the index.
fn becomes_naming(engine: &WorldEngine, target: &str, state: &str) -> Vec<BecomesAt> {
    let names_it = |actions: &serde_yaml::Value| -> Vec<String> {
        let mut found = Vec::new();
        if let serde_yaml::Value::Mapping(actions) = actions {
            for (name, action) in actions {
                let Some(serde_yaml::Value::Mapping(becomes)) = action.get("becomes") else { continue };
                let hit = becomes.iter().any(|(k, v)| {
                    v.as_str() == Some(state) && k.as_str().map_or(false, |k| engine.resolve_name(&crate::parsers::link_target(k), None) == target)
                });
                if hit {
                    found.push(name.as_str().map(str::to_string).unwrap_or_else(|| format!("{:?}", name)));
                }
            }
        }
        found
    };
    let mut things: Vec<&crate::engines::bundles::Thing> = engine.things_iter().collect();
    things.sort_by(|a, b| (a.kind, &a.id).cmp(&(b.kind, &b.id)));
    let mut found = Vec::new();
    for thing in things {
        let layers = std::iter::once((&thing.index, true)).chain(thing.states.values().chain(thing.overlays.values()).map(|l| (l, false)));
        for (layer, whole) in layers {
            if let Some(actions) = layer.metadata.get("actions") {
                for action in names_it(actions) {
                    found.push(BecomesAt { path: layer.path.clone(), action, in_section: false, whole });
                }
            }
            for section in &layer.sections {
                if let Some(actions) = section.layer.metadata.get("actions") {
                    for action in names_it(actions) {
                        found.push(BecomesAt { path: layer.path.clone(), action, in_section: true, whole });
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| (&a.path, &a.action).cmp(&(&b.path, &b.action)));
    found.dedup_by(|a, b| a.path == b.path && a.action == b.action && a.in_section == b.in_section);
    found
}

impl WorldWriter {
    /// Delete one of a bundle's states (`/state delete`). Refused for the
    /// starting state, which the index names; set another first. Actions
    /// that switch to it are listed in `by_hand`, and `validate` reports
    /// them until they're changed.
    pub fn delete_state_loaded(&self, engine: &mut WorldEngine, kind: EntityKind, id: &str, state: &str, strict: bool) -> Result<BundleReport> {
        let engine_root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        engine.source_path(kind, id)?;
        let thing = engine.thing(kind, id).ok_or_else(|| anyhow!("no {} `{}` is loaded", kind.type_name(), id))?;
        let layer = thing.states.get(state).ok_or_else(|| anyhow!("{} `{}` has no state `{}`", kind.type_name(), id, state))?;
        if thing.starting_state.as_deref() == Some(state) {
            bail!("`{}` is the starting state of {} `{}`; set another with `state:` first", state, kind.type_name(), id);
        }
        let path = layer.path.clone();
        let rel = path.strip_prefix(&engine_root).unwrap_or(&path).to_path_buf();
        let abs = self.root().join(&rel);
        self.check_inside(&abs)?;
        let by_hand = becomes_naming(engine, id, state)
            .into_iter()
            .map(|at| format!("{}: action `{}` switches {} to `{}`, which is gone", at.path.strip_prefix(&engine_root).unwrap_or(&at.path).display(), at.action, id, state))
            .collect();
        let before = engine.current_problems(strict);
        fs::remove_file(&abs).with_context(|| format!("can't remove {}", rel.display()))?;
        engine.remove_file(&path);
        let change = ChangeReport::new(path, before, engine.current_problems(strict));
        Ok(BundleReport { written: Vec::new(), created: Vec::new(), unchanged: Vec::new(), moved: Vec::new(), deleted: vec![rel], by_hand, change })
    }

    /// Rename one of a bundle's states (`/state rename`): its file, the
    /// index's `state:` if it's the starting state, and every `becomes:`
    /// that switches the thing to it, each checked by reading back. One
    /// that can't be rewritten safely is listed in `by_hand`.
    pub fn rename_state_loaded(&self, engine: &mut WorldEngine, kind: EntityKind, id: &str, old: &str, new: &str, strict: bool) -> Result<BundleReport> {
        state_name_ok(new)?;
        let engine_root = engine.world_root.clone().ok_or_else(|| anyhow!("the engine has no loaded world"))?;
        engine.source_path(kind, id)?;
        let thing = engine.thing(kind, id).ok_or_else(|| anyhow!("no {} `{}` is loaded", kind.type_name(), id))?.clone();
        let layer = thing.states.get(old).ok_or_else(|| anyhow!("{} `{}` has no state `{}`", kind.type_name(), id, old))?;
        if thing.states.contains_key(new) {
            bail!("{} `{}` already has a state `{}`", kind.type_name(), id, new);
        }
        let rel = |p: &Path| p.strip_prefix(&engine_root).unwrap_or(p).to_path_buf();
        let from = rel(&layer.path);
        let to = from.with_file_name(format!("{}.md", new));
        if self.root().join(&to).exists() {
            bail!("{} already exists", to.display());
        }

        // Plan: the index's `state:`, and each `becomes:` naming it.
        let index_rel = rel(&thing.index.path);
        let mut planned: Vec<(PathBuf, ContentFile)> = Vec::new();
        if thing.starting_state.as_deref() == Some(old) {
            let mut index = ContentFile::load(self.root().join(&index_rel))?;
            index.apply(&super::Edit::SetScalar { key: "state".into(), value: serde_yaml::Value::String(new.into()) })?;
            planned.push((index_rel.clone(), index));
        }
        let mut by_hand = Vec::new();
        let mut done: Vec<PathBuf> = Vec::new();
        for at in becomes_naming(engine, id, old) {
            let object_rel = rel(&at.path);
            let hand = || format!("{}: action `{}` switches {} to `{}`; change it to `{}` by hand", object_rel.display(), at.action, id, old, new);
            if at.in_section {
                by_hand.push(hand());
                continue;
            }
            // One rewrite covers every action in the file's frontmatter.
            if done.contains(&object_rel) {
                continue;
            }
            done.push(object_rel.clone());
            let slot = planned.iter().position(|(p, _)| *p == object_rel);
            let mut file = match slot {
                Some(i) => planned[i].1.clone(),
                None if at.whole => ContentFile::load(self.root().join(&object_rel))?,
                None => ContentFile::load_layer(self.root().join(&object_rel))?,
            };
            let doc = MarkdownParser::new().split_document(&file.text())?;
            let mut expected = match doc.metadata {
                serde_yaml::Value::Mapping(m) => m,
                _ => serde_yaml::Mapping::new(),
            };
            if let Some(serde_yaml::Value::Mapping(actions)) = expected.get_mut("actions") {
                for (_, a) in actions.iter_mut() {
                    if let Some(serde_yaml::Value::Mapping(becomes)) = a.get_mut("becomes") {
                        for (k, v) in becomes.iter_mut() {
                            let names_thing = k.as_str().map_or(false, |k| engine.resolve_name(&crate::parsers::link_target(k), None) == id);
                            if names_thing && v.as_str() == Some(old) {
                                *v = serde_yaml::Value::String(new.into());
                            }
                        }
                    }
                }
            }
            match file.replace_in_key("actions", old, new, &expected) {
                Ok(()) => match slot {
                    Some(i) => planned[i].1 = file,
                    None => planned.push((object_rel, file)),
                },
                Err(_) => by_hand.push(hand()),
            }
        }

        // Move the state file (with its own rewrite, if it names itself),
        // then save the rest.
        let before = engine.current_problems(strict);
        let text = match planned.iter().position(|(p, _)| *p == from) {
            Some(i) => planned.remove(i).1.text(),
            None => fs::read_to_string(self.root().join(&from))?,
        };
        ContentFile::layer_from_text(self.root().join(&to), &text)?.save_new()?;
        fs::remove_file(self.root().join(&from)).with_context(|| format!("wrote {} but can't remove {}", to.display(), from.display()))?;
        let mut written = Vec::new();
        for (path, file) in &planned {
            file.save()?;
            written.push(path.clone());
        }
        engine.remove_file(&engine_root.join(&from));
        engine.set_file(&engine_root.join(&to), &text);
        for (path, file) in &planned {
            engine.set_file(&engine_root.join(path), &file.text());
        }
        let change = ChangeReport::new(engine_root.join(&to), before, engine.current_problems(strict));
        Ok(BundleReport { written, created: Vec::new(), unchanged: Vec::new(), moved: vec![(from, to)], deleted: Vec::new(), by_hand, change })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn world() -> (tempfile::TempDir, WorldEngine, WorldWriter) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "hall.md", "---\nid: hall\ntitle: Hall\ntype: location\nobjects: [lantern]\n---\nStone.\n");
        write(d, "objects/lantern.md", "---\nid: lantern\ntitle: Lantern\ntype: object\nobject_type: tool   # brass\n---\nUnlit.\n");
        write(d, "vault/index.md", "---\nid: vault\ntitle: Vault\ntype: location\nstate: shut\n---\n");
        write(d, "vault/shut.md", "---\ninherits: index\n---\nShut.\n");
        write(d, "vault/open.md", "---\ninherits: index\n---\nOpen.\n");
        write(d, "chapters/10-after.md", "---\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        assert!(engine.current_problems(true).is_empty(), "{:?}", engine.current_problems(true));
        let writer = WorldWriter::new(d).unwrap();
        (dir, engine, writer)
    }

    fn section(dir: &Path, rel: &str, text: &str) -> Section {
        Section { path: PathBuf::from(rel), text: text.into(), expected: fs::read_to_string(dir.join(rel)).ok() }
    }

    #[test]
    fn a_one_file_thing_becomes_a_bundle_with_a_first_state() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        let before = fs::read_to_string(d.join("objects/lantern.md")).unwrap();
        let report = writer.make_bundle_loaded(&mut engine, EntityKind::Object, "lantern", Some("lit"), true).unwrap();
        assert_eq!(report.moved, vec![(PathBuf::from("objects/lantern.md"), PathBuf::from("objects/lantern/index.md"))]);
        assert_eq!(report.created, vec![PathBuf::from("objects/lantern/index.md"), PathBuf::from("objects/lantern/lit.md")]);
        assert!(!d.join("objects/lantern.md").exists());
        assert_eq!(fs::read_to_string(d.join("objects/lantern/index.md")).unwrap(), before);
        assert_eq!(fs::read_to_string(d.join("objects/lantern/lit.md")).unwrap(), "---\ninherits: index\n---\n");
        let thing = engine.thing(EntityKind::Object, "lantern").unwrap();
        assert_eq!(thing.bundle.as_deref(), Some(Path::new("objects/lantern")));
        assert_eq!(thing.starting_state, None);
        assert!(thing.states.contains_key("lit"));
        assert!(report.change.problems.is_empty(), "{:?}", report.change.problems);

        assert!(writer.make_bundle_loaded(&mut engine, EntityKind::Object, "lantern", None, true).unwrap_err().to_string().contains("already a bundle"));
        assert!(writer.make_bundle_loaded(&mut engine, EntityKind::Location, "hall", Some("index"), true).unwrap_err().to_string().contains("reserved"));
        assert!(d.join("hall.md").exists());
    }

    #[test]
    fn a_bundle_saves_only_what_changed_and_creates_new_sections() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        let index = fs::read_to_string(d.join("vault/index.md")).unwrap();
        let sections = vec![
            section(d, "vault/index.md", &index),
            section(d, "vault/shut.md", "---\ninherits: index\n---\nShut tight.\n"),
            Section { path: "vault/chapters/after.md".into(), text: "---\n---\nBroken.\n".into(), expected: None },
        ];
        let report = writer.save_bundle_loaded(&mut engine, EntityKind::Location, "vault", &sections, true).unwrap();
        assert_eq!(report.unchanged, vec![PathBuf::from("vault/index.md")]);
        assert_eq!(report.written, vec![PathBuf::from("vault/shut.md")]);
        assert_eq!(report.created, vec![PathBuf::from("vault/chapters/after.md")]);
        assert_eq!(engine.get_location("vault").unwrap().description, "Shut tight.");
        assert_eq!(engine.thing(EntityKind::Location, "vault").unwrap().overlays.len(), 1);
        assert_eq!(fs::read_to_string(d.join("vault/index.md")).unwrap(), index);
    }

    #[test]
    fn nothing_is_written_when_any_section_is_refused() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        let shut = fs::read_to_string(d.join("vault/shut.md")).unwrap();
        let good = section(d, "vault/shut.md", "---\ninherits: index\n---\nChanged.\n");
        let cases: Vec<(Section, &str)> = vec![
            (section(d, "vault/open.md", "---\ninherits: index\nid: x\n---\n"), "vault/open.md: only the index sets `id`"),
            (section(d, "vault/index.md", "---\nid: strongroom\ntitle: V\ntype: location\n---\n"), "vault/index.md: must stay location `vault`"),
            (section(d, "vault/index.md", "---\nid: vault\ntype: location\n---\n"), "vault/index.md:"),
            (Section { path: "vault/open.md".into(), text: "x".into(), expected: Some("not what's on disk".into()) }, "vault/open.md: changed on disk"),
            (Section { path: "hall.md".into(), text: "x".into(), expected: None }, "hall.md: isn't in vault/"),
            (Section { path: "vault/a/b.md".into(), text: "---\n---\n".into(), expected: None }, "vault/a/b.md: must be a state file"),
            (Section { path: "../x.md".into(), text: "".into(), expected: None }, "../x.md: must be a `.md` path inside the world, relative"),
        ];
        for (bad, why) in cases {
            let err = writer.save_bundle_loaded(&mut engine, EntityKind::Location, "vault", &[good.clone(), bad], true).unwrap_err();
            assert!(format!("{:#}", err).contains(why), "{}: {:#}", why, err);
        }
        assert_eq!(fs::read_to_string(d.join("vault/shut.md")).unwrap(), shut);
        assert_eq!(engine.get_location("vault").unwrap().description, "Shut.");

        // A one-file thing has just its own file.
        let lantern = section(d, "objects/lantern.md", "---\nid: lantern\ntitle: Lantern\ntype: object\nobject_type: tool   # brass\n---\nDim.\n");
        writer.save_bundle_loaded(&mut engine, EntityKind::Object, "lantern", &[lantern], true).unwrap();
        assert_eq!(engine.objects["lantern"].description, "Dim.");
    }

    #[test]
    fn states_are_renamed_with_their_references_and_deleted() {
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        write(d, "objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { becomes: { vault: open } }\n  push: { becomes: { vault: shut } }\n---\n<!-- thwarter:action pull -->\nUp.\n<!-- thwarter:action push -->\nDown.\n");
        engine.load_world(d).unwrap();

        let report = writer.rename_state_loaded(&mut engine, EntityKind::Location, "vault", "shut", "sealed", true).unwrap();
        assert_eq!(report.moved, vec![(PathBuf::from("vault/shut.md"), PathBuf::from("vault/sealed.md"))]);
        assert!(fs::read_to_string(d.join("vault/index.md")).unwrap().contains("state: sealed"));
        let lever = fs::read_to_string(d.join("objects/lever.md")).unwrap();
        assert!(lever.contains("push: { becomes: { vault: sealed } }") && lever.contains("pull: { becomes: { vault: open } }"), "{}", lever);
        assert!(report.by_hand.is_empty() && report.change.problems.is_empty(), "{:?}", report);
        assert_eq!(engine.thing(EntityKind::Location, "vault").unwrap().starting_state.as_deref(), Some("sealed"));

        assert!(writer.delete_state_loaded(&mut engine, EntityKind::Location, "vault", "sealed", true).unwrap_err().to_string().contains("starting state"));
        let report = writer.delete_state_loaded(&mut engine, EntityKind::Location, "vault", "open", true).unwrap();
        assert_eq!(report.deleted, vec![PathBuf::from("vault/open.md")]);
        assert!(!d.join("vault/open.md").exists());
        assert_eq!(report.by_hand.len(), 1, "{:?}", report.by_hand);
        assert!(report.change.added.iter().any(|p| p.contains("`open`")), "{:?}", report.change);
        assert!(writer.rename_state_loaded(&mut engine, EntityKind::Location, "vault", "sealed", "chapters", true).is_err());
    }

    #[test]
    fn a_becomes_in_a_state_file_is_renamed_and_found() {
        // Chair's actions live in its state files, not its index: `kick`
        // in sat.md, and kicked.md's own `kick` switching to itself.
        let (dir, mut engine, writer) = world();
        let d = dir.path();
        write(d, "objects/chair/index.md", "---\nid: chair\ntitle: Chair\ntype: object\nobject_type: furniture\nstate: sat\n---\nA chair.\n");
        write(d, "objects/chair/sat.md", "---\ninherits: index\nactions:\n  kick: { becomes: { chair: kicked } }\n  fold: { becomes: { chair: folded } }\n---\n");
        write(d, "objects/chair/kicked.md", "---\ninherits: index\nactions:\n  kick: { becomes: { chair: kicked } }\n---\n");
        write(d, "objects/chair/folded.md", "---\ninherits: index\n---\n");
        engine.load_world(d).unwrap();
        assert!(engine.current_problems(true).is_empty(), "{:?}", engine.current_problems(true));

        let report = writer.rename_state_loaded(&mut engine, EntityKind::Object, "chair", "kicked", "broken", true).unwrap();
        assert!(report.by_hand.is_empty() && report.change.problems.is_empty(), "{:?}", report);
        assert_eq!(report.written, vec![PathBuf::from("objects/chair/sat.md")]);
        let sat = fs::read_to_string(d.join("objects/chair/sat.md")).unwrap();
        assert!(sat.contains("kick: { becomes: { chair: broken } }") && sat.contains("fold: { becomes: { chair: folded } }"), "{}", sat);
        assert!(fs::read_to_string(d.join("objects/chair/broken.md")).unwrap().contains("kick: { becomes: { chair: broken } }"));
        assert!(!d.join("objects/chair/kicked.md").exists());

        // Delete names the file the action is in.
        let report = writer.delete_state_loaded(&mut engine, EntityKind::Object, "chair", "folded", true).unwrap();
        assert_eq!(report.by_hand, vec!["objects/chair/sat.md: action `fold` switches chair to `folded`, which is gone".to_string()]);
    }
}
