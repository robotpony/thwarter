use crate::engines::location_view::{direction_rank, DoorView, EntityRef, ExitView, LocationView};
use crate::models::character::Character;
use crate::models::world::*;
use crate::engines::bundles::{self, Chapter, ChapterKey, FileRecord, FileRole, Thing};
use crate::engines::layers::{Layer, Merged};
use crate::engines::variables;
use crate::engines::persona::Persona;
use crate::engines::world_def::WorldDef;
use crate::models::game::{PlayState, SavedChapter};
use crate::parsers::ParsedEntity;
use anyhow::Result;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// The three kinds of content file. An `id` is unique within a kind; a
/// location and an object may share one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Location,
    Character,
    Object,
    Door,
}

impl EntityKind {
    /// The frontmatter `type` value for this kind.
    pub fn type_name(self) -> &'static str {
        match self {
            EntityKind::Location => "location",
            EntityKind::Character => "character",
            EntityKind::Object => "object",
            EntityKind::Door => "door",
        }
    }

    pub fn from_type_name(name: &str) -> Option<Self> {
        match name {
            "location" => Some(EntityKind::Location),
            "character" => Some(EntityKind::Character),
            "object" => Some(EntityKind::Object),
            "door" => Some(EntityKind::Door),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldEngine {
    pub world_graph: WorldGraph,
    pub loaded_locations: HashMap<String, Location>,
    pub objects: HashMap<String, GameObject>,
    /// Character cards by id, as loaded. A read-only catalog for lookups
    /// and validation; placement and state belong to `CharacterEngine`.
    pub characters: HashMap<String, Character>,
    /// Door definitions by id (REQ-CF-19), each in its starting version.
    pub doors: HashMap<String, Door>,
    /// Every file each `(kind, id)` was loaded from, in load order (a
    /// bundle's `index.md`). More than one file means a duplicate id: the
    /// last one loaded wins in the tables above, `problems` reports it,
    /// and the writer refuses to edit it (REQ-ENG-7).
    pub sources: HashMap<(EntityKind, String), Vec<PathBuf>>,
    /// The directory `load_world` walked, as it was given.
    pub world_root: Option<PathBuf>,
    /// Each thing with every file that can apply to it, for building any
    /// version on request (`design/STATES.md`). The tables above hold
    /// each thing's starting version.
    things: HashMap<(EntityKind, String), Thing>,
    /// Every chapter, root ones first.
    chapters: Vec<Chapter>,
    /// Each file's split, in load order. Everything above is rebuilt from
    /// these, so an in-memory change (REQ-ENG-8) resolves duplicates,
    /// bundles and errors exactly as a fresh load would.
    files: Vec<FileRecord>,
    /// Every `actions.rhai`, by its full path (`design/SECTIONS.md` ›
    /// Scripts).
    scripts: std::collections::BTreeMap<PathBuf, String>,
    /// How play words rooms and the inventory (`world.toml`'s `[play]`).
    play_text: super::play_text::PlayText,
    play_text_warnings: Vec<String>,
    /// Problems and warnings from the last rebuild, each prefixed with its file.
    file_problems: Vec<String>,
    file_warnings: Vec<String>,
    /// The world's definition: `world/`, or `world.toml` for a world not
    /// yet moved.
    world: WorldDef,
    /// What `world.toml` says, read at load: the old definition keys and
    /// the extra folders to ignore.
    world_toml: WorldDef,
    ignore: Vec<PathBuf>,
    world_toml_warning: Option<String>,
    /// A thing's file path from the world root without `.md` (a bundle's
    /// folder), and its file name, to the things it may name, so an
    /// Obsidian link to a file finds the thing (REQ-CF-25).
    links: HashMap<String, Vec<(EntityKind, String)>>,
    /// Every file read, by its path in the engine's form, and what it is.
    roles: std::collections::BTreeMap<PathBuf, FileRole>,
    /// Every file a template name can find (`bundles::Built::library`).
    library: std::collections::BTreeMap<String, Layer>,
    /// The world's question sets, from its `questions/` folders (REQ-CF-32).
    question_sets: Vec<crate::engines::questions::QuestionSet>,
    /// The persona `world/index.md`'s `author:` names (REQ-WS-5), and a
    /// problem or warning about it.
    persona: Option<Persona>,
    persona_problem: Option<String>,
    persona_warning: Option<String>,
    /// Links out of the world that the last load skipped (REQ-CFG-10).
    link_warnings: Vec<String>,
}

/// How plainly a thing is there, from its current version's
/// `visibility:` (REQ-CF-18, `design/STATES.md` › Hidden things).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    /// Listed in the room.
    #[default]
    Visible,
    /// Not listed; found by searching or examining the room.
    Faint,
    /// Not listed or found by looking; only an action reveals it, though
    /// an action may name it.
    Hidden,
    /// Not there at all until something brings it in; can't be named.
    Absent,
}

impl Visibility {
    pub const NAMES: &'static [&'static str] = &["visible", "faint", "hidden", "absent"];

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "visible" => Visibility::Visible,
            "faint" => Visibility::Faint,
            "hidden" => Visibility::Hidden,
            "absent" => Visibility::Absent,
            _ => return None,
        })
    }
}

/// Where a thing starts: in a room, a character's inventory, or an
/// object (REQ-CF-31).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Holder {
    Room(String),
    Character(String),
    Object(String),
}

impl std::fmt::Display for Holder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Holder::Room(id) => write!(f, "room `{}`", id),
            Holder::Character(id) => write!(f, "character `{}`'s inventory", id),
            Holder::Object(id) => write!(f, "object `{}`", id),
        }
    }
}

/// Which version of each thing to see: the starting versions (the
/// default), or those of a game in progress.
#[derive(Debug, Clone, Copy, Default)]
pub struct At<'a> {
    pub play: Option<&'a PlayState>,
}

impl<'a> At<'a> {
    pub fn play(state: &'a PlayState) -> Self {
        Self { play: Some(state) }
    }
}

/// One version of a thing: its typed entity and the merged layers it came
/// from, with which file each part came from.
#[derive(Debug, Clone)]
pub struct Version {
    pub entity: ParsedEntity,
    pub merged: Merged,
    pub state: Option<String>,
    pub chapter: Option<ChapterKey>,
}

/// The result of checking a world (REQ-ENG-44): problems fail it,
/// warnings don't.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Validation {
    pub problems: Vec<String>,
    pub warnings: Vec<String>,
}

impl Validation {
    pub fn passed(&self) -> bool {
        self.problems.is_empty()
    }

    /// The closing line, worded the same in every client.
    pub fn summary(&self) -> String {
        match (self.problems.len(), self.warnings.len()) {
            (0, 0) => "Validation passed: no problems or warnings".to_string(),
            (0, w) => format!("Validation passed: no problems, {} warning(s)", w),
            (p, w) => format!("Validation failed: {} problem(s), {} warning(s)", p, w),
        }
    }
}

/// Summary of a `load_world` pass: what was loaded, and every file that
/// failed to parse. A partial load is not a hard error; callers (the
/// `validate` and `show` commands in particular) need to see both what
/// loaded and what didn't in the same report.
#[derive(Debug, Clone, Default)]
pub struct WorldLoadReport {
    pub locations_loaded: usize,
    pub objects_loaded: usize,
    pub doors_loaded: usize,
    /// The characters loaded, also kept in `WorldEngine::characters`.
    /// Placement and state belong to `CharacterEngine`, per
    /// `design/ARCHITECTURE.md`.
    pub characters: Vec<Character>,
    pub errors: Vec<String>,
    /// Problems that don't fail validation: a key that differs only in
    /// case from a known one, a list a layer replaces, an overlay that
    /// applies by where it sits (each prefixed with its file), or an id
    /// shared between kinds.
    pub warnings: Vec<String>,
}

impl WorldEngine {
    pub fn new() -> Self {
        Self {
            world_graph: WorldGraph::new(),
            loaded_locations: HashMap::new(),
            objects: HashMap::new(),
            characters: HashMap::new(),
            doors: HashMap::new(),
            sources: HashMap::new(),
            world_root: None,
            things: HashMap::new(),
            chapters: Vec::new(),
            files: Vec::new(),
            scripts: Default::default(),
            play_text: Default::default(),
            play_text_warnings: Vec::new(),
            file_problems: Vec::new(),
            file_warnings: Vec::new(),
            world: WorldDef::default(),
            world_toml: WorldDef::default(),
            ignore: Vec::new(),
            world_toml_warning: None,
            links: HashMap::new(),
            roles: std::collections::BTreeMap::new(),
            library: std::collections::BTreeMap::new(),
            question_sets: Vec::new(),
            persona: None,
            persona_problem: None,
            persona_warning: None,
            link_warnings: Vec::new(),
        }
    }

    /// The world's author, from the persona `world/index.md`'s `author:`
    /// names (REQ-WS-4, 5).
    pub fn persona(&self) -> Option<&Persona> {
        self.persona.as_ref()
    }

    /// The template `name` means, seen from `scope` (a folder relative to
    /// the world root; empty for the root): a path from the root, or a
    /// bare name in `templates/` folders from `scope` outward, then a
    /// built-in. Its layer's path is `<built-in>/<name>.md` for a built-in.
    pub fn find_template(&self, name: &str, scope: &Path) -> Option<Layer> {
        bundles::find_template(&crate::parsers::link_target(name), scope, &self.library)
    }

    /// The questions `/new` and `/add` ask for a new thing `id` made in
    /// `folder` (relative to the world root) from `template`, if any, of
    /// `kind` (REQ-CF-32): the set named for its id, else its template's,
    /// else its kind's, each the nearest from `folder` out to the root,
    /// then built in. The whole set, never merged.
    pub fn questions_for_thing(&self, folder: &Path, id: &str, template: Option<&str>, kind: &str) -> Option<crate::engines::questions::QuestionSet> {
        crate::engines::questions::find(&self.question_sets, folder, Some(id), template, kind)
    }

    /// `questions_for_thing` without an id: the template's set, else the
    /// kind's.
    pub fn questions_for(&self, folder: &Path, template: Option<&str>, kind: &str) -> Option<crate::engines::questions::QuestionSet> {
        crate::engines::questions::find(&self.question_sets, folder, None, template, kind)
    }

    /// Every template the world defines, by path from the root, then the
    /// built-ins by name.
    pub fn template_names(&self) -> Vec<String> {
        let root = self.world_root.clone().unwrap_or_default();
        let mut names: Vec<String> = self
            .roles
            .iter()
            .filter(|(_, role)| **role == FileRole::Template)
            .filter_map(|(path, _)| {
                // Question sets share the role for now, but aren't templates.
                if crate::engines::questions::question_file(path.strip_prefix(&root).ok()?).is_some() {
                    return None;
                }
                let rel = path.strip_prefix(&root).ok()?.with_extension("");
                Some(rel.to_string_lossy().replace('\\', "/"))
            })
            .collect();
        names.extend(crate::engines::builtins::BUILTINS.iter().map(|(n, _)| n.to_string()));
        names
    }

    /// What one file is, by its path relative to the world root: a thing,
    /// a state, an overlay, a chapter, an area, a `world/` file, a template,
    /// or a problem with why. `None` for a file the engine didn't read
    /// (another tool's, or not `.md`).
    pub fn file_role(&self, relative: &Path) -> Option<&FileRole> {
        let root = self.world_root.clone().unwrap_or_default();
        self.roles.get(&root.join(relative))
    }

    /// Every file read, in path order, each with its role. Paths are in
    /// the engine's form, as in `sources`.
    pub fn file_roles(&self) -> impl Iterator<Item = (&Path, &FileRole)> {
        self.roles.iter().map(|(p, r)| (p.as_path(), r))
    }

    /// Whether the loader skips this path, relative to the world root:
    /// other tools' files and the world's own `ignore` list (REQ-CF-24).
    pub fn is_ignored(&self, relative: &Path) -> bool {
        let parts: Vec<String> = relative.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        bundles::ignored(&parts, &self.ignore)
    }

    /// The world's own definition: title, author, starting room, intro,
    /// story values and the player.
    pub fn world(&self) -> &WorldDef {
        &self.world
    }

    /// Walk every `.md` file under `world_path`, sort each into its role
    /// (a thing, a state, a chapter, an overlay, an area) by where it sits,
    /// and load each thing's starting version. Files that fail are
    /// collected as errors rather than aborting the whole load, so
    /// `validate` can report every problem in one pass.
    pub fn load_world<P: AsRef<Path>>(&mut self, world_path: P) -> Result<WorldLoadReport> {
        let root = world_path.as_ref().to_path_buf();
        self.world_root = Some(root.clone());
        self.files.clear();
        self.scripts.clear();
        self.link_warnings.clear();
        self.read_world_toml(&root);
        let ignore = self.ignore.clone();
        let mut paths: Vec<PathBuf> = WalkDir::new(&world_path)
            .into_iter()
            .filter_entry(|e| {
                // Don't descend into other tools' folders (.git, .obsidian, public/).
                let rel = e.path().strip_prefix(&root).unwrap_or(e.path());
                let parts: Vec<String> = rel.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                parts.is_empty() || !bundles::ignored(&parts, &ignore) || e.file_type().is_file()
            })
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("md")) || p.file_name().map_or(false, |n| n == "actions.rhai"))
            .collect();
        paths.sort();
        // A link may point inside the world, never out of it (REQ-CFG-10).
        let real_root = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        for path in paths {
            if std::fs::symlink_metadata(&path).map_or(false, |m| m.file_type().is_symlink()) {
                let inside = std::fs::canonicalize(&path).map_or(false, |target| target.starts_with(&real_root));
                if !inside {
                    self.link_warnings.push(format!("{}: is a link to outside the world, so it isn't read", path.display()));
                    continue;
                }
            }
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if path.extension().and_then(|e| e.to_str()) == Some("rhai") {
                match std::fs::read_to_string(&path) {
                    Ok(text) if text.len() <= crate::engines::layers::MAX_FILE_BYTES => {
                        self.scripts.insert(path, text);
                    }
                    Ok(_) => self.link_warnings.push(format!("{}: is over the {} KB a script may be, so it isn't read", path.display(), crate::engines::layers::MAX_FILE_BYTES / 1024)),
                    Err(e) => self.link_warnings.push(format!("{}: can't be read: {}", path.display(), e)),
                }
                continue;
            }
            let layer = if size > crate::engines::layers::MAX_FILE_BYTES as u64 {
                Err(format!(
                    "is {} KB, over the {} KB a content file may be, so it isn't read",
                    size / 1024,
                    crate::engines::layers::MAX_FILE_BYTES / 1024
                ))
            } else {
                match std::fs::read_to_string(&path) {
                    Ok(text) => Layer::parse(&path, &text),
                    Err(e) => Err(format!("Failed to read file {:?}: {}", path, e)),
                }
            };
            self.files.push(FileRecord { path, layer });
        }
        let built = self.rebuild();

        let mut report = WorldLoadReport::default();
        for (_, entity) in &built {
            match entity {
                ParsedEntity::Location(_) => report.locations_loaded += 1,
                ParsedEntity::Object(_) => report.objects_loaded += 1,
                ParsedEntity::Character(c) => report.characters.push(c.clone()),
                ParsedEntity::Door(_) => report.doors_loaded += 1,
            }
        }
        report.errors = self.file_problems.clone();
        report.errors.extend(self.persona_problem.clone());
        report.warnings = self.current_warnings();
        Ok(report)
    }

    /// Recompute everything from the file records, returning each thing
    /// whose starting version typed, in load order.
    fn rebuild(&mut self) -> Vec<(Thing, ParsedEntity)> {
        let root = self.world_root.clone().unwrap_or_default();
        let mut built = bundles::build(&root, &self.files, &self.scripts, &self.ignore);
        self.world_graph.locations.clear();
        self.objects.clear();
        self.characters.clear();
        self.doors.clear();
        self.sources.clear();
        self.things.clear();
        self.links.clear();
        for (thing, _) in &built.things {
            let key = (thing.kind, thing.id.clone());
            self.things.insert(key.clone(), thing.clone());
            let rel = thing.index.path.strip_prefix(&root).unwrap_or(&thing.index.path);
            let path = match &thing.bundle {
                Some(folder) => folder.clone(),
                None => rel.with_extension(""),
            };
            let path = path.to_string_lossy().replace('\\', "/");
            let stem = path.rsplit('/').next().unwrap_or(&path).to_string();
            // A bundle is also named by its index file (`castle/vault/index`).
            let file = rel.with_extension("").to_string_lossy().replace('\\', "/");
            for name in [path, stem, file] {
                let entry = self.links.entry(name).or_default();
                if !entry.contains(&key) {
                    entry.push(key.clone());
                }
            }
        }
        for (thing, entity) in &built.things {
            let mut entity = entity.clone();
            self.resolve_links(&mut entity);
            let key = (thing.kind, thing.id.clone());
            self.sources.entry(key).or_default().push(thing.index.path.clone());
            match entity {
                ParsedEntity::Location(l) => self.world_graph.add_location(l),
                ParsedEntity::Object(o) => {
                    self.objects.insert(o.id.clone(), o);
                }
                ParsedEntity::Character(c) => {
                    self.characters.insert(c.id.clone(), c);
                }
                ParsedEntity::Door(d) => {
                    self.doors.insert(d.id.clone(), d);
                }
            }
        }
        self.loaded_locations = self.world_graph.locations.clone();
        self.chapters = built.chapters;
        self.world = if built.has_world_index {
            built.world
        } else {
            WorldDef { story: built.world.story, player: built.world.player, ..self.world_toml.clone() }
        };
        self.read_persona();
        if let Some(start) = self.world.start.take() {
            self.world.start = Some(self.resolve_name(&start, Some(EntityKind::Location)));
        }
        self.roles.clear();
        for (path, role) in &built.roles {
            self.roles.insert(path.clone(), role.clone());
        }
        for (thing, _) in &built.things {
            let (kind, id) = (thing.kind, thing.id.clone());
            self.roles.insert(thing.index.path.clone(), FileRole::Thing { kind, id: id.clone() });
            for (name, layer) in &thing.states {
                self.roles.insert(layer.path.clone(), FileRole::State { kind, id: id.clone(), name: name.clone() });
            }
            for (chapter, layer) in &thing.overlays {
                self.roles.insert(layer.path.clone(), FileRole::Overlay { kind, id: id.clone(), chapter: chapter.clone() });
            }
        }
        for file in &self.files {
            if self.roles.contains_key(&file.path) {
                continue;
            }
            let rel = file.path.strip_prefix(&root).unwrap_or(&file.path);
            if file.path.starts_with(&root) && self.is_ignored(rel) {
                continue;
            }
            let shown = format!("{}: ", file.path.display());
            let why = built
                .problems
                .iter()
                .find_map(|p| p.strip_prefix(&shown))
                .map(str::to_string)
                .or_else(|| match &file.layer {
                    Err(e) => Some(e.clone()),
                    Ok(_) => None,
                })
                .unwrap_or_else(|| "isn't part of the world as loaded".to_string());
            self.roles.insert(file.path.clone(), FileRole::Problem(why));
        }
        self.library = std::mem::take(&mut built.library);
        self.question_sets = std::mem::take(&mut built.questions);
        self.file_problems = built.problems;
        self.file_warnings = built.warnings;
        if let Some(w) = &self.world_toml_warning {
            self.file_warnings.insert(0, w.clone());
        }
        built.things
    }

    /// The id `name` means for a reference of `kind` (any kind if `None`):
    /// itself if a thing has that id, else the one thing whose file path
    /// or file name it is (an Obsidian link to the file), else unchanged,
    /// so the usual "unknown" checks report it.
    pub fn resolve_name(&self, name: &str, kind: Option<EntityKind>) -> String {
        let has_id = match kind {
            Some(k) => self.things.contains_key(&(k, name.to_string())),
            None => self.things.keys().any(|(_, id)| id == name),
        };
        if has_id {
            return name.to_string();
        }
        let matches: Vec<&(EntityKind, String)> = self
            .links
            .get(name)
            .map(|v| v.iter().filter(|(k, _)| kind.map_or(true, |want| *k == want)).collect())
            .unwrap_or_default();
        match matches.as_slice() {
            [(_, id)] => id.clone(),
            _ => name.to_string(),
        }
    }

    /// What a link in prose names (REQ-CF-29): the thing's kind, id and
    /// title, by id, path or unique file name, of any kind.
    pub fn link_for(&self, name: &str) -> Option<(EntityKind, String, String)> {
        let id = self.resolve_name(name, None);
        let thing = self.thing_by_id(&id).ok()?;
        let title = self.current(thing.kind, &thing.id, At::default())?.title().to_string();
        Some((thing.kind, thing.id.clone(), title))
    }

    /// Every tag (REQ-CF-30), from `tags:` in a thing's starting version
    /// and `#tags` in its own files' prose, to the things that carry it.
    pub fn tags(&self) -> std::collections::BTreeMap<String, Vec<(EntityKind, String)>> {
        let mut tags: std::collections::BTreeMap<String, Vec<(EntityKind, String)>> = std::collections::BTreeMap::new();
        let mut things: Vec<&Thing> = self.things.values().collect();
        things.sort_by(|a, b| (a.kind, &a.id).cmp(&(b.kind, &b.id)));
        for thing in things {
            let mut own: Vec<String> = Vec::new();
            if let Ok((_, merged)) = thing.build(thing.starting_state.as_deref(), None) {
                if let Some(list) = merged.metadata.get("tags").and_then(|v| v.as_sequence()) {
                    own.extend(list.iter().filter_map(|t| t.as_str()).map(|t| t.trim_start_matches('#').to_string()));
                }
            }
            let layers = std::iter::once(&thing.index).chain(thing.states.values()).chain(thing.overlays.values());
            for layer in layers {
                own.extend(crate::render::prose_tags(&layer.body));
            }
            own.sort();
            own.dedup();
            for tag in own {
                tags.entry(tag).or_default().push((thing.kind, thing.id.clone()));
            }
        }
        tags
    }

    /// Warnings for links in prose that name nothing (REQ-CF-29).
    fn prose_link_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut things: Vec<&Thing> = self.things.values().collect();
        things.sort_by(|a, b| a.index.path.cmp(&b.index.path));
        for thing in things {
            let files = std::iter::once(&thing.index).chain(thing.states.values()).chain(thing.overlays.values());
            let layers = files.flat_map(|l| std::iter::once(l).chain(l.sections.iter().map(|s| &s.layer)));
            for layer in layers {
                for (_, id) in super::variables::mentions(&layer.body) {
                    if self.thing_by_id(&id).is_err() {
                        warnings.push(format!("{}: `@{}` names nothing in the world, so play shows `{}`", layer.path.display(), id, id));
                    }
                }
                for name in crate::render::prose_links(&layer.body) {
                    if self.link_for(&name).is_some() {
                        continue;
                    }
                    match self.link_ambiguity(&name, None) {
                        Some(why) => warnings.push(format!("{}: a link in the prose is ambiguous: {}", layer.path.display(), why)),
                        None => warnings.push(format!("{}: the link to `{}` names nothing in the world", layer.path.display(), name)),
                    }
                }
            }
        }
        warnings
    }

    /// How a holder holds things (`holds:`), `on` when it doesn't say.
    pub fn holds_word(&self, holder: &str, at: At) -> String {
        let word = match self.object_at(holder, at) {
            Some(o) => o.holds,
            None => self.door_at(holder, at).and_then(|d| d.holds),
        };
        word.filter(|w| !w.is_empty()).unwrap_or_else(|| "on".to_string())
    }

    /// Where a thing starts (REQ-CF-31): every room, character and object
    /// that lists it, in the starting versions, sorted. More than one is a
    /// problem `validate` reports.
    pub fn placement(&self, id: &str) -> Vec<Holder> {
        let mut out = Vec::new();
        for l in self.loaded_locations.values() {
            if l.objects.iter().any(|o| o == id) || l.characters.iter().any(|c| c == id) {
                out.push(Holder::Room(l.id.clone()));
            }
        }
        for c in self.characters.values() {
            if c.inventory.iter().any(|o| o == id) {
                out.push(Holder::Character(c.id.clone()));
            }
        }
        for o in self.objects.values() {
            if o.objects.iter().any(|x| x == id) {
                out.push(Holder::Object(o.id.clone()));
            }
        }
        out.sort();
        out
    }

    /// Things placed in more than one place (REQ-CF-31). Scenery, with or
    /// without a card, may stand in several rooms (a bench in two
    /// gardens), so it isn't counted.
    fn placement_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut ids: BTreeSet<&String> = BTreeSet::new();
        for l in self.loaded_locations.values() {
            ids.extend(l.objects.iter().chain(l.characters.iter()));
        }
        for c in self.characters.values() {
            ids.extend(c.inventory.iter());
        }
        for o in self.objects.values() {
            ids.extend(o.objects.iter());
        }
        for id in ids {
            let scenery = !self.characters.contains_key(id.as_str()) && self.objects.get(id.as_str()).map_or(true, |o| o.object_type == "scenery");
            if scenery {
                continue;
            }
            let places = self.placement(id);
            if places.len() > 1 {
                let named: Vec<String> = places.iter().map(Holder::to_string).collect();
                warnings.push(format!("`{}` starts in more than one place: {}; play shows it in each", id, named.join(", ")));
            }
        }
        // A room shows who it lists in `characters:`; `default_location`
        // alone doesn't put anyone there, so the character never appears.
        let mut people: Vec<&Character> = self.characters.values().collect();
        people.sort_by(|a, b| a.id.cmp(&b.id));
        for c in people {
            let Some(room) = c.default_location.as_deref().and_then(|at| self.get_location(at)) else { continue };
            if !room.characters.contains(&c.id) {
                warnings.push(format!(
                    "character `{}`: `default_location: {}` doesn't put it there; add `{}` to {}'s `characters:` so play shows it",
                    c.id, room.id, c.id, room.id
                ));
            }
        }
        warnings
    }

    /// A `holds:` with no word (REQ-CF-31).
    fn holds_warnings(&self) -> Vec<String> {
        let mut ids: Vec<&String> = self.objects.keys().collect();
        ids.sort();
        let objects = ids.into_iter().filter(|id| self.objects[*id].holds.as_deref() == Some("")).map(|id| format!("object `{}`: `holds:` is empty; say how it holds things, like `holds: in`", id));
        let mut doors: Vec<&String> = self.doors.keys().collect();
        doors.sort();
        let doors = doors.into_iter().filter(|id| self.doors[*id].holds.as_deref() == Some("")).map(|id| format!("door `{}`: `holds:` is empty; say how it holds things, like `holds: in`", id));
        objects.chain(doors).collect()
    }

    /// Objects that hold themselves through what they hold (REQ-CF-31).
    fn cycle_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut objects: Vec<&String> = self.objects.keys().collect();
        objects.sort();
        for id in objects {
            // Follow what it holds; reaching itself again is a cycle.
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut todo: Vec<&str> = self.objects[id].objects.iter().map(String::as_str).collect();
            while let Some(next) = todo.pop() {
                if next == id {
                    problems.push(format!("object `{}` holds itself, through what it holds", id));
                    break;
                }
                if seen.insert(next) {
                    if let Some(o) = self.objects.get(next) {
                        todo.extend(o.objects.iter().map(String::as_str));
                    }
                }
            }
        }
        problems
    }

    /// A door's `key` and a thing's `connects:` that name nothing
    /// (`design/STATES.md` › Keys, One thing affecting another).
    fn script_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut things: Vec<&Thing> = self.things.values().collect();
        things.sort_by(|a, b| a.index.path.cmp(&b.index.path));
        for thing in things {
            let shown = thing.index.path.display();
            if let Ok((_, merged)) = thing.build(thing.starting_state.as_deref(), None) {
                if let Some(key) = merged.metadata.get("vars").and_then(|v| v.get("key")).and_then(|v| v.as_str()) {
                    if !key.is_empty() && self.thing_by_id(key).is_err() {
                        warnings.push(format!("{}: `key: {}` names nothing in the world, so the lock can't be opened", shown, key));
                    }
                }
            }
            match thing.index.metadata.get("connects") {
                None => {}
                Some(serde_yaml::Value::Sequence(list)) => {
                    for id in list {
                        match id.as_str() {
                            Some(id) if self.thing_by_id(id).is_ok() => {}
                            Some(id) => warnings.push(format!("{}: `connects:` names `{}`, which is nothing in the world", shown, id)),
                            None => warnings.push(format!("{}: `connects:` is a list of ids", shown)),
                        }
                    }
                }
                Some(_) => warnings.push(format!("{}: `connects:` is a list of ids, like `connects: [vault-door]`", shown)),
            }
        }
        warnings
    }

    /// Why `name` names no one thing, if it's a file name more than one
    /// file has: "it matches more than one file: a.md, b.md".
    pub fn link_ambiguity(&self, name: &str, kind: Option<EntityKind>) -> Option<String> {
        let matches: Vec<&(EntityKind, String)> = self
            .links
            .get(name)?
            .iter()
            .filter(|(k, _)| kind.map_or(true, |want| *k == want))
            .collect();
        if matches.len() < 2 {
            return None;
        }
        let mut files: Vec<String> = matches
            .iter()
            .filter_map(|key| self.things.get(key))
            .map(|t| t.index.path.display().to_string())
            .collect();
        files.sort();
        Some(format!("`{}` matches more than one file: {}", name, files.join(", ")))
    }

    /// Turn every reference in `entity` that names a file into the id of
    /// the thing in it.
    fn resolve_links(&self, entity: &mut ParsedEntity) {
        use EntityKind::*;
        let r = |name: &mut String, kind: Option<EntityKind>| *name = self.resolve_name(name, kind);
        match entity {
            ParsedEntity::Location(l) => {
                // An exit names a room or a door.
                for t in l.connections.values_mut() {
                    let as_room = self.resolve_name(t, Some(Location));
                    *t = if self.things.contains_key(&(Location, as_room.clone())) { as_room } else { self.resolve_name(t, Some(Door)) };
                }
                l.characters.iter_mut().for_each(|c| r(c, Some(Character)));
                l.objects.iter_mut().for_each(|o| r(o, Some(Object)));
            }
            ParsedEntity::Character(c) => {
                if let Some(at) = c.default_location.as_mut() {
                    r(at, Some(Location));
                }
                c.inventory.iter_mut().for_each(|o| r(o, Some(Object)));
            }
            ParsedEntity::Door(d) => {
                for side in d.sides.iter_mut() {
                    r(&mut side.room, Some(Location));
                    if let Some(to) = side.to.as_mut() {
                        r(to, Some(Location));
                    }
                }
                for action in d.actions.values_mut() {
                    action.requires_has.iter_mut().for_each(|h| r(h, Some(Object)));
                    let becomes = std::mem::take(&mut action.becomes);
                    action.becomes = becomes.into_iter().map(|(k, v)| (self.resolve_name(&k, None), v)).collect();
                }
            }
            ParsedEntity::Object(o) => {
                for action in o.actions.values_mut() {
                    action.requires_has.iter_mut().for_each(|h| r(h, Some(Object)));
                    let becomes = std::mem::take(&mut action.becomes);
                    action.becomes = becomes.into_iter().map(|(k, v)| (self.resolve_name(&k, None), v)).collect();
                }
            }
        }
    }

    /// Find the persona the world's `author:` names, beside the world.
    fn read_persona(&mut self) {
        self.persona = None;
        self.persona_problem = None;
        self.persona_warning = None;
        let (Some(id), Some(root)) = (self.world.author.clone(), self.world_root.clone()) else { return };
        let source = self.world.source.clone().unwrap_or_else(|| root.join("world/index.md"));
        let Some(folder) = crate::engines::persona::persona_folder(&root, &id) else { return };
        match crate::engines::persona::read_persona(&folder) {
            Ok(Some(persona)) => self.persona = Some(persona),
            Ok(None) => {
                self.persona_warning = Some(format!(
                    "{}: `author: {}` names no persona; a persona is a folder beside the world, {}",
                    source.display(),
                    id,
                    folder.display()
                ))
            }
            Err(e) => self.persona_problem = Some(e),
        }
    }

    /// Read `world.toml`: the extra folders to ignore (`ignore = [...]`,
    /// paths from the root) and, for a world not yet moved to `world/`,
    /// its old `[metadata]` and `[settings] starting_location`.
    fn read_world_toml(&mut self, root: &Path) {
        self.ignore.clear();
        self.play_text = Default::default();
        self.play_text_warnings.clear();
        self.world_toml = WorldDef::default();
        self.world_toml_warning = None;
        let path = root.join("world.toml");
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let value = match text.parse::<toml::Value>() {
            Ok(v) => v,
            Err(e) => {
                self.world_toml_warning = Some(format!("{}: doesn't parse, so it is ignored: {}", path.display(), e.message()));
                return;
            }
        };
        if let Some(list) = value.get("ignore").and_then(|v| v.as_array()) {
            self.ignore = list.iter().filter_map(|v| v.as_str()).map(PathBuf::from).collect();
        }
        self.world_toml = WorldDef::from_world_toml(root, &value);
        self.play_text = super::play_text::PlayText::from_toml(value.get("play"), &mut self.play_text_warnings);
    }

    /// How play words rooms and the inventory, from `world.toml`'s
    /// `[play]` over the defaults.
    pub fn play_text(&self) -> &super::play_text::PlayText {
        &self.play_text
    }

    /// Every id used by more than one kind, with those kinds, sorted. Ids
    /// are scoped by kind, so this isn't an error, but the console's
    /// `cd <id>` can't tell such entities apart.
    pub fn shared_ids(&self) -> Vec<(String, Vec<EntityKind>)> {
        let mut by_id: std::collections::BTreeMap<&str, Vec<EntityKind>> = std::collections::BTreeMap::new();
        for (kind, id) in self.sources.keys() {
            by_id.entry(id.as_str()).or_default().push(*kind);
        }
        by_id
            .into_iter()
            .filter(|(_, kinds)| kinds.len() > 1)
            .map(|(id, mut kinds)| {
                kinds.sort();
                (id.to_string(), kinds)
            })
            .collect()
    }

    fn shared_id_warnings(&self) -> Vec<String> {
        self.shared_ids()
            .into_iter()
            .map(|(id, kinds)| {
                let names: Vec<&str> = kinds.iter().map(|k| k.type_name()).collect();
                format!("id `{}` is used by more than one kind ({}); give each a different id", id, names.join(", "))
            })
            .collect()
    }

    /// Change the loaded world in memory (REQ-ENG-8): replace whatever
    /// `path` held with `text`, as if the file had been loaded with it. A
    /// parse error becomes that file's problem, as it would on a load.
    /// Nothing is read from or written to disk. `path` is in the form
    /// `load_world` uses (its root joined with the file's relative path),
    /// the same form as `sources`. A state or overlay file changes its
    /// thing; an `index.md` can turn its folder into a bundle.
    pub fn set_file(&mut self, path: &Path, text: &str) {
        if path.extension().and_then(|e| e.to_str()) == Some("rhai") {
            self.scripts.insert(path.to_path_buf(), text.to_string());
            self.rebuild();
            return;
        }
        let record = FileRecord { path: path.to_path_buf(), layer: Layer::parse(path, text) };
        match self.files.iter_mut().find(|r| r.path == path) {
            Some(existing) => *existing = record,
            None => self.files.push(record),
        }
        self.rebuild();
    }

    /// Forget a file and everything it defined, in memory only.
    pub fn remove_file(&mut self, path: &Path) {
        if self.scripts.remove(path).is_some() {
            self.rebuild();
            return;
        }
        if let Some(i) = self.files.iter().position(|r| r.path == path) {
            self.files.remove(i);
            self.rebuild();
        }
    }

    /// A thing with all its files: index, states and overlays.
    pub fn thing(&self, kind: EntityKind, id: &str) -> Option<&Thing> {
        self.things.get(&(kind, id.to_string()))
    }

    /// The kinds that have a thing with this id, in kind order. More than
    /// one makes a bare reference such as `becomes:` ambiguous.
    pub fn kinds_with_id(&self, id: &str) -> Vec<EntityKind> {
        let mut kinds: Vec<EntityKind> = self.things.keys().filter(|(_, i)| i == id).map(|(k, _)| *k).collect();
        kinds.sort();
        kinds
    }

    /// The one thing a bare id names, for an action's `becomes:`.
    pub fn thing_by_id(&self, id: &str) -> Result<&Thing> {
        match self.kinds_with_id(id).as_slice() {
            [] => anyhow::bail!("nothing has the id `{}`", id),
            [kind] => Ok(&self.things[&(*kind, id.to_string())]),
            kinds => {
                let names: Vec<&str> = kinds.iter().map(|k| k.type_name()).collect();
                anyhow::bail!("`{}` is the id of more than one kind ({}), so it's ambiguous", id, names.join(", "))
            }
        }
    }

    /// Problems with what each object's actions name: a `becomes:` that
    /// names no thing or no state of it, a `chapter:` that can't be seen
    /// from the object, a `requires: { has: }` naming no object.
    fn action_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut ids: Vec<&String> = self.objects.keys().collect();
        ids.sort();
        for id in ids {
            let object = &self.objects[id];
            let scope = self.thing(EntityKind::Object, id).map(|t| t.scope.clone()).unwrap_or_default();
            let mut names: Vec<&String> = object.actions.keys().collect();
            names.sort();
            for name in names {
                let action = &object.actions[name];
                let at = format!("object `{}`, action `{}`", id, name);
                for (target, state) in &action.becomes {
                    match self.thing_by_id(target) {
                        Err(e) => match self.link_ambiguity(target, None) {
                            Some(why) => problems.push(format!("{}: `becomes:` is ambiguous: {}", at, why)),
                            None => problems.push(format!("{}: `becomes:` {}", at, e)),
                        },
                        Ok(thing) if !thing.has_state(state) => problems.push(format!(
                            "{}: `becomes:` names state `{}`, but {} `{}` has no such state file",
                            at,
                            state,
                            thing.kind.type_name(),
                            target
                        )),
                        Ok(_) => {}
                    }
                }
                if let Some(chapter) = &action.chapter {
                    if self.find_chapter(&scope, chapter).is_none() {
                        problems.push(format!("{}: `chapter: {}` names no chapter that can be seen from the object", at, chapter));
                    }
                }
                for needed in &action.requires_has {
                    if !self.objects.contains_key(needed) {
                        problems.push(format!("{}: `requires: {{ has: {} }}` names no object", at, needed));
                    }
                }
            }
        }
        problems
    }

    /// Problems with doors (REQ-CF-19): a door without exactly two sides
    /// (a portal may have more, each saying `to:`), a side that isn't a
    /// room, and an exit through a door the room isn't a side of.
    fn door_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut ids: Vec<&String> = self.doors.keys().collect();
        ids.sort();
        for id in ids {
            let door = &self.doors[id];
            let at = format!("door `{}`", id);
            // A door with one side is half-built (`/add door bedroom`, before
            // its other room is linked): a warning, not a problem.
            if door.sides.is_empty() {
                problems.push(format!("{}: a door needs a side in at least one room", at));
            } else if door.sides.len() == 1 {
            } else if door.portal {
                if door.sides.len() > 2 {
                    for side in door.sides.iter().filter(|s| s.to.is_none()) {
                        problems.push(format!("{}: the side in `{}` needs `to:`, since a portal with more than two sides can't tell where it leads", at, side.room));
                    }
                }
            } else if door.sides.len() > 2 {
                problems.push(format!("{}: a door joins exactly two rooms, and this one has {} sides; a portal can join more", at, door.sides.len()));
            }
            for side in &door.sides {
                if self.get_location(&side.room).is_none() {
                    problems.push(format!("{}: side `{}` is not a known location", at, side.room));
                }
                if let Some(to) = side.to.as_ref().filter(|t| self.get_location(t).is_none()) {
                    problems.push(format!("{}: the side in `{}` leads `to: {}`, which is not a known location", at, side.room, to));
                }
            }
        }
        let mut rooms: Vec<&Location> = self.world_graph.locations.values().collect();
        rooms.sort_by(|a, b| a.id.cmp(&b.id));
        for room in rooms {
            for (direction, target) in &room.connections {
                if let Some(door) = self.doors.get(target) {
                    if door.side(&room.id).is_none() {
                        problems.push(format!("{}: connection `{}` goes through door `{}`, which doesn't join this room", room.id, direction, target));
                    }
                }
            }
        }
        problems
    }

    /// Warnings about doors: a side whose direction differs from the exit
    /// that names the door, and a side whose room has no exit through it.
    fn door_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut ids: Vec<&String> = self.doors.keys().collect();
        ids.sort();
        for id in &ids {
            if let [side] = self.doors[*id].sides.as_slice() {
                warnings.push(format!("door `{}`: it's only in `{}` so far, so it leads nowhere; give it a side in the room it opens onto", id, side.room));
            }
        }
        for id in ids {
            for side in &self.doors[id].sides {
                let Some(room) = self.get_location(&side.room) else { continue };
                let exits: Vec<&String> = room.connections.iter().filter(|(_, t)| *t == id).map(|(d, _)| d).collect();
                match exits.as_slice() {
                    [] => warnings.push(format!("door `{}`: {} has no exit through it", id, side.room)),
                    dirs if !dirs.iter().any(|d| **d == side.direction) => warnings.push(format!(
                        "door `{}`: its side in {} says `{}`, but the room's exit through it is `{}`",
                        id, side.room, side.direction, dirs[0]
                    )),
                    _ => {}
                }
            }
        }
        warnings
    }

    /// Every thing, in no particular order.
    pub fn things_iter(&self) -> impl Iterator<Item = &Thing> {
        self.things.values()
    }

    /// Whether any object or door has an action named `verb`, in any of
    /// its states or chapters (REQ-PC-14). Spaces in `verb` match hyphens.
    pub fn has_action(&self, verb: &str) -> bool {
        let key = verb.trim().to_lowercase().replace(' ', "-");
        let named = |actions: &HashMap<String, ObjectAction>| actions.keys().any(|a| a.eq_ignore_ascii_case(&key));
        if self.objects.values().any(|o| named(&o.actions)) || self.doors.values().any(|d| named(&d.actions)) {
            return true;
        }
        // A script's action is one too.
        let function = super::scripts::function_name(&key);
        if self.things.values().any(|t| t.script_actions.contains(&function)) {
            return true;
        }
        // A `#verb` in any section's heading is a verb the world knows
        // (`## When: #hit @stick`).
        let tag = format!("#{}", key);
        let in_heading = |t: &Thing| t.templates.iter().chain(std::iter::once(&t.index)).any(|l| l.sections.iter().any(|s| s.tags.iter().any(|g| g.eq_ignore_ascii_case(&tag))));
        if self.things.values().any(in_heading) {
            return true;
        }
        let chapters: Vec<Option<String>> = std::iter::once(None).chain(self.chapters.iter().map(|c| Some(c.name().to_string()))).collect();
        self.things.values().filter(|t| matches!(t.kind, EntityKind::Object | EntityKind::Door)).any(|t| {
            let states: Vec<Option<String>> = std::iter::once(None).chain(t.state_names().into_iter().map(Some)).collect();
            states.iter().any(|s| {
                chapters.iter().any(|c| match self.version(t.kind, &t.id, s.as_deref(), c.as_deref()).map(|v| v.entity) {
                    Ok(ParsedEntity::Object(o)) => named(&o.actions),
                    Ok(ParsedEntity::Door(d)) => named(&d.actions),
                    _ => false,
                })
            })
        })
    }

    /// Every chapter, root ones first, each scope in chapter order.
    pub fn chapters(&self) -> &[Chapter] {
        &self.chapters
    }

    /// One version of a thing (`design/STATES.md` › Current version):
    /// `state` or its starting state, in `chapter` (a name, resolved from
    /// the thing's folder outward) or in no chapter.
    pub fn version(&self, kind: EntityKind, id: &str, state: Option<&str>, chapter: Option<&str>) -> Result<Version> {
        let thing = self
            .thing(kind, id)
            .ok_or_else(|| anyhow::anyhow!("no {} `{}` is loaded", kind.type_name(), id))?;
        let state = match state {
            Some(name) if thing.has_state(name) => Some(name.to_string()),
            Some(name) => {
                let names = thing.state_names();
                let known: Vec<&str> = names.iter().map(String::as_str).collect();
                if known.is_empty() {
                    anyhow::bail!("{} `{}` has no states, so it has no state `{}`", kind.type_name(), id, name);
                }
                anyhow::bail!("{} `{}` has no state `{}` (it has {})", kind.type_name(), id, name, known.join(", "));
            }
            None => thing.starting_state.clone(),
        };
        let chapter = match chapter {
            None => None,
            Some(name) => {
                let path: Vec<String> = name.split('/').map(String::from).collect();
                let found = bundles::resolve_chapter(&self.chapters, &thing.scope, &path)
                    .or_else(|| {
                        bundles::visible_chapters(&self.chapters, &thing.scope)
                            .into_iter()
                            .find(|c| c.name() == name)
                    })
                    .ok_or_else(|| anyhow::anyhow!("no chapter `{}` can be seen from {} `{}`", name, kind.type_name(), id))?;
                Some(found.key.clone())
            }
        };
        let (mut entity, merged) = thing
            .build(state.as_deref(), chapter.as_ref())
            .map_err(|e| anyhow::anyhow!(e))?;
        self.resolve_links(&mut entity);
        Ok(Version { entity, merged, state, chapter })
    }

    /// `problems` for the world as it is now, after any in-memory changes:
    /// each file's parse error, then the same checks. No load report needed.
    pub fn current_problems(&self, strict: bool) -> Vec<String> {
        let mut errors = self.file_problems.clone();
        errors.extend(self.persona_problem.clone());
        self.problems_from(errors, strict)
    }

    /// The warnings for the world as it is now: each file's (prefixed with
    /// its path), then ids shared between kinds.
    pub fn current_warnings(&self) -> Vec<String> {
        let mut warnings = self.file_warnings.clone();
        warnings.extend(self.persona_warning.clone());
        warnings.extend(self.link_warnings.iter().cloned());
        warnings.extend(self.variable_warnings());
        warnings.extend(self.prose_link_warnings());
        warnings.extend(self.script_warnings());
        warnings.extend(self.placement_warnings());
        warnings.extend(self.holds_warnings());
        warnings.extend(self.play_text_warnings.iter().cloned());
        warnings.extend(self.door_warnings());
        warnings.extend(self.shared_id_warnings());
        warnings
    }

    /// What `thwart validate` reports, for the world as it is now: every
    /// client that checks a world calls this, so the console's check and
    /// the command line's never differ (REQ-ENG-44).
    pub fn validation(&self, strict: bool) -> Validation {
        Validation { problems: self.current_problems(strict), warnings: self.current_warnings() }
    }

    /// The one file `(kind, id)` was loaded from. An error if it wasn't
    /// loaded, or if two files define it.
    pub fn source_path(&self, kind: EntityKind, id: &str) -> Result<&Path> {
        match self.sources.get(&(kind, id.to_string())).map(|v| v.as_slice()) {
            None | Some([]) => Err(anyhow::anyhow!("no {} `{}` is loaded", kind.type_name(), id)),
            Some([one]) => Ok(one.as_path()),
            Some(many) => Err(anyhow::anyhow!(
                "{} `{}` is defined in {} files ({}); fix the duplicate first",
                kind.type_name(),
                id,
                many.len(),
                join_paths(many)
            )),
        }
    }

    /// Every `(kind, id)` defined in more than one file, sorted.
    pub fn duplicate_ids(&self) -> Vec<(EntityKind, String, Vec<PathBuf>)> {
        let mut dups: Vec<_> = self
            .sources
            .iter()
            .filter(|(_, paths)| paths.len() > 1)
            .map(|((kind, id), paths)| {
                let mut paths = paths.clone();
                paths.sort();
                (*kind, id.clone(), paths)
            })
            .collect();
        dups.sort();
        dups
    }

    pub fn get_location(&self, location_id: &str) -> Option<&Location> {
        self.world_graph.get_location(location_id)
    }

    pub fn add_location(&mut self, location: Location) {
        self.world_graph.add_location(location);
    }

    pub fn get_connections(&self, location_id: &str) -> Vec<(String, String)> {
        if let Some(location) = self.get_location(location_id) {
            location
                .connections
                .iter()
                .map(|(direction, target)| (direction.clone(), target.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Locations whose connections point at a location ID that was never
    /// loaded. This is the core check behind `thwart validate`.
    pub fn broken_connections(&self) -> Vec<(String, String, String)> {
        let mut broken = Vec::new();
        for (location_id, location) in &self.world_graph.locations {
            for (direction, target) in &location.connections {
                if !self.world_graph.locations.contains_key(target) && !self.doors.contains_key(target) {
                    broken.push((location_id.clone(), direction.clone(), target.clone()));
                }
            }
        }
        broken.sort();
        broken
    }

    /// Every problem `validate` reports for this world: the load's parse
    /// errors, broken connections, and with `strict`, character
    /// `default_location`s that aren't known locations. Shared by
    /// `thwart validate` and the console's `/reload` (REQ-CON-7), so "fails
    /// validation" means the same thing in both.
    pub fn problems(&self, report: &WorldLoadReport, strict: bool) -> Vec<String> {
        self.problems_from(report.errors.clone(), strict)
    }

    fn problems_from(&self, errors: Vec<String>, strict: bool) -> Vec<String> {
        let mut problems = errors;
        for (kind, id, paths) in self.duplicate_ids() {
            problems.push(format!(
                "{} `{}` is defined in {} files: {}",
                kind.type_name(),
                id,
                paths.len(),
                join_paths(&paths)
            ));
        }
        problems.extend(self.action_problems());
        problems.extend(self.door_problems());
        problems.extend(self.cycle_problems());
        let mut reserved: Vec<&(EntityKind, String)> = self
            .things
            .keys()
            .filter(|(_, id)| variables::LEVELS.contains(&id.as_str()) || id.contains('.'))
            .collect();
        reserved.sort();
        for (kind, id) in reserved {
            let why = if id.contains('.') { "ids can't contain a dot, which splits a variable's name" } else { "it's reserved as a variable level" };
            problems.push(format!("{} `{}` can't be an id: {}", kind.type_name(), id, why));
        }
        if let (Some(start), Some(source)) = (&self.world.start, &self.world.source) {
            if self.get_location(start).is_none() {
                problems.push(format!("{}: the starting room `{}` is not a known location", source.display(), start));
            }
        }
        for (from, direction, target) in self.broken_connections() {
            // An existing thing of the wrong kind says what it is.
            let kinds: Vec<&str> = self.kinds_with_id(&target).into_iter().filter(|k| !matches!(k, EntityKind::Location | EntityKind::Door)).map(|k| k.type_name()).collect();
            match self.link_ambiguity(&target, Some(EntityKind::Location)) {
                Some(why) => problems.push(format!("{}: connection `{}` is ambiguous: {}", from, direction, why)),
                None if !kinds.is_empty() => problems.push(format!(
                    "{}: the exit {} names `{}`, which is {} {}, not a room or a door",
                    from,
                    direction,
                    target,
                    if kinds[0].starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" },
                    kinds.join(" and ")
                )),
                None => problems.push(format!(
                    "{}: connection `{}` points at unknown location `{}`",
                    from, direction, target
                )),
            }
        }
        if strict {
            let mut characters: Vec<&Character> = self.characters.values().collect();
            characters.sort_by(|a, b| a.id.cmp(&b.id));
            for character in characters {
                if let Some(default_location) = &character.default_location {
                    if self.get_location(default_location).is_none() {
                        problems.push(format!(
                            "{}: default_location `{}` is not a known location",
                            character.id, default_location
                        ));
                    }
                }
            }
        }
        problems
    }

    /// A location with every reference resolved (REQ-ENG-9): exits in
    /// compass order with whether each target exists, and characters and
    /// objects with their card names, `None` where there's no card. The
    /// starting version of each; `describe_location_at` shows a game's.
    pub fn describe_location(&self, id: &str) -> Option<LocationView> {
        self.describe_location_at(id, At::default())
    }

    /// `describe_location`, with each thing in the version `at` sees.
    pub fn describe_location_at(&self, id: &str, at: At) -> Option<LocationView> {
        let location = self.location_at(id, at)?;
        let mut exits: Vec<ExitView> = location
            .connections
            .iter()
            .map(|(direction, target)| {
                if self.get_location(target).is_none() {
                    if let Some(door) = self.door_at(target, at) {
                        let to = door.leads_to(id).map(str::to_string);
                        let beyond = to
                            .as_deref()
                            .filter(|_| door.transparent)
                            .and_then(|room| self.location_at(room, at))
                            .map(|room| room.name);
                        return ExitView {
                            direction: direction.clone(),
                            target_exists: to.as_deref().map_or(false, |t| self.get_location(t).is_some()),
                            target: to.unwrap_or_else(|| target.clone()),
                            door: Some(DoorView {
                                id: door.id.clone(),
                                title: door.title.clone(),
                                description: door.description.clone(),
                                passable: door.passable,
                                transparent: door.transparent,
                                beyond,
                                state: None,
                            }),
                        };
                    }
                }
                ExitView { direction: direction.clone(), target: target.clone(), target_exists: self.get_location(target).is_some(), door: None }
            })
            .collect();
        exits.sort_by_key(|e| direction_rank(&e.direction));

        let characters = location
            .characters
            .iter()
            .map(|cid| EntityRef {
                id: cid.clone(),
                name: self.character_at(cid, at).map(|c| c.name),
            })
            .collect();
        let objects: Vec<EntityRef> = location
            .objects
            .iter()
            .map(|oid| EntityRef {
                id: oid.clone(),
                name: self.object_at(oid, at).map(|o| o.title),
            })
            .collect();

        // What the room's objects and doors hold, as written (REQ-CF-31);
        // play fills this from the game instead.
        let mut held = Vec::new();
        let mut holders: Vec<(String, String)> = objects.iter().map(|o| (o.id.clone(), o.display_name())).collect();
        holders.extend(exits.iter().filter_map(|e| e.door.as_ref()).map(|d| (d.id.clone(), d.title.clone())));
        let mut seen: BTreeSet<String> = holders.iter().map(|(id, _)| id.clone()).collect();
        let mut i = 0;
        while i < holders.len() {
            let (holder, holder_name): (String, String) = holders[i].clone();
            i += 1;
            for id in self.object_at(&holder, at).map(|o| o.objects).unwrap_or_default() {
                if seen.insert(id.clone()) {
                    let thing = EntityRef { name: self.object_at(&id, at).map(|o| o.title), id: id.clone() };
                    holders.push((id, thing.display_name()));
                    held.push(crate::engines::Held { thing, holder: holder.clone(), holder_name: holder_name.clone(), word: self.holds_word(&holder, at) });
                }
            }
        }

        Some(LocationView {
            id: location.id.clone(),
            title: location.name.clone(),
            coordinates: location.coordinates,
            body: location.description.clone(),
            exits,
            characters,
            scenery: objects
                .iter()
                .filter(|o| self.objects.get(o.id.as_str()).map_or(true, |card| card.object_type == "scenery"))
                .map(|o| o.id.clone())
                .collect(),
            objects,
            held,
        })
    }

    /// A thing as `at` sees it: in a game, its current state in the
    /// game's chapter; otherwise its starting version. A version that
    /// doesn't type (a problem `validate` reports) falls back to the
    /// starting one, so play never loses a thing that loaded.
    pub fn current(&self, kind: EntityKind, id: &str, at: At) -> Option<ParsedEntity> {
        let starting = || -> Option<ParsedEntity> {
            match kind {
                EntityKind::Location => self.get_location(id).cloned().map(ParsedEntity::Location),
                EntityKind::Object => self.objects.get(id).cloned().map(ParsedEntity::Object),
                EntityKind::Character => self.characters.get(id).cloned().map(ParsedEntity::Character),
                EntityKind::Door => self.doors.get(id).cloned().map(ParsedEntity::Door),
            }
        };
        let Some(play) = at.play else { return starting() };
        let Some(thing) = self.thing(kind, id) else { return starting() };
        let state = play
            .state_of(kind.type_name(), id)
            .filter(|s| thing.has_state(s))
            .map(String::from)
            .or_else(|| thing.starting_state.clone());
        let chapter = play.chapter.as_ref().and_then(|c| self.chapter_key(c));
        if state == thing.starting_state && chapter.is_none() {
            return starting();
        }
        match thing.build(state.as_deref(), chapter.as_ref()) {
            Ok((mut entity, _)) => {
                self.resolve_links(&mut entity);
                Some(entity)
            }
            Err(_) => starting(),
        }
    }

    /// How plainly a thing is there in the version `at` sees. A thing
    /// that isn't loaded, or says nothing, is visible.
    pub fn visibility(&self, kind: EntityKind, id: &str, at: At) -> Visibility {
        let Some(thing) = self.thing(kind, id) else { return Visibility::Visible };
        let state = at
            .play
            .and_then(|p| p.state_of(kind.type_name(), id))
            .filter(|s| thing.has_state(s))
            .map(String::from)
            .or_else(|| thing.starting_state.clone());
        let chapter = at.play.and_then(|p| p.chapter.as_ref()).and_then(|c| self.chapter_key(c));
        thing
            .build(state.as_deref(), chapter.as_ref())
            .ok()
            .and_then(|(_, merged)| merged.metadata.get("visibility").and_then(|v| v.as_str()).and_then(Visibility::from_name))
            .unwrap_or_default()
    }

    pub fn location_at(&self, id: &str, at: At) -> Option<Location> {
        match self.current(EntityKind::Location, id, at)? {
            ParsedEntity::Location(l) => Some(l),
            _ => None,
        }
    }

    pub fn object_at(&self, id: &str, at: At) -> Option<GameObject> {
        match self.current(EntityKind::Object, id, at)? {
            ParsedEntity::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn door_at(&self, id: &str, at: At) -> Option<Door> {
        match self.current(EntityKind::Door, id, at)? {
            ParsedEntity::Door(d) => Some(d),
            _ => None,
        }
    }

    pub fn character_at(&self, id: &str, at: At) -> Option<Character> {
        match self.current(EntityKind::Character, id, at)? {
            ParsedEntity::Character(c) => Some(c),
            _ => None,
        }
    }

    /// The loaded chapter a save names, if it still exists.
    pub fn chapter_key(&self, saved: &SavedChapter) -> Option<ChapterKey> {
        let key = ChapterKey { scope: PathBuf::from(&saved.scope), path: saved.path.clone() };
        self.chapters.iter().any(|c| c.key == key).then_some(key)
    }

    /// How a save names a loaded chapter.
    pub fn saved_chapter(&self, chapter: &Chapter) -> SavedChapter {
        SavedChapter {
            scope: chapter.key.scope.to_string_lossy().replace('\\', "/"),
            path: chapter.key.path.clone(),
            order: chapter.order.clone(),
        }
    }

    /// The chapter `name` names as seen from `scope` (a folder relative to
    /// the world root; empty for the root): a sub-chapter path
    /// (`after-the-king/the-funeral`) or a bare name.
    pub fn find_chapter(&self, scope: &Path, name: &str) -> Option<&Chapter> {
        let path: Vec<String> = name.split('/').map(String::from).collect();
        bundles::resolve_chapter(&self.chapters, scope, &path)
            .or_else(|| bundles::visible_chapters(&self.chapters, scope).into_iter().find(|c| c.name() == name))
    }
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

impl Default for WorldEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> WorldEngine {
        let mut engine = WorldEngine::new();
        let mut yard = Location::new("castle/yard".into(), "Yard".into(), "Grass.".into());
        yard.connections.insert("west".into(), "castle/hall".into());
        yard.connections.insert("north".into(), "castle/gone".into());
        yard.connections.insert("behind".into(), "castle/hall".into());
        yard.characters = vec!["king".into(), "ghost".into()];
        yard.objects = vec!["well".into(), "grass".into()];
        engine.add_location(yard);
        engine.add_location(Location::new("castle/hall".into(), "Hall".into(), "Stone.".into()));
        let mut king = Character::new("king".into(), "The King".into(), "Old.".into());
        king.default_location = Some("castle/throne".into());
        engine.characters.insert("king".into(), king);
        engine.objects.insert(
            "well".into(),
            GameObject::new("well".into(), "Old Well".into(), "Deep.".into(), "environmental".into()),
        );
        engine
    }

    #[test]
    fn describe_location_resolves_references() {
        let view = engine().describe_location("castle/yard").unwrap();
        assert_eq!(view.title, "Yard");
        let exits: Vec<(&str, bool)> = view
            .exits
            .iter()
            .map(|e| (e.direction.as_str(), e.target_exists))
            .collect();
        assert_eq!(exits, vec![("north", false), ("west", true), ("behind", true)]);
        assert_eq!(view.characters[0].name.as_deref(), Some("The King"));
        assert_eq!(view.characters[1].name, None);
        assert_eq!(view.objects[0].name.as_deref(), Some("Old Well"));
        assert_eq!(view.objects[1].name, None);
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn load_records_sources_and_reports_duplicate_ids() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "castle/yard.md",
            "---\nid: castle/yard\ntitle: Yard\ntype: location\n---\n",
        );
        write(
            dir.path(),
            "npcs/king.md",
            "---\nid: king\ntitle: King\ntype: character\n---\n",
        );
        write(
            dir.path(),
            "npcs/king-copy.md",
            "---\nid: king\ntitle: King 2\ntype: character\n---\n",
        );
        // Same id, different kind: not a duplicate.
        write(
            dir.path(),
            "objects/yard.md",
            "---\nid: castle/yard\ntitle: Yard\ntype: object\nobject_type: x\n---\n",
        );

        let mut engine = WorldEngine::new();
        let report = engine.load_world(dir.path()).unwrap();

        let yard = engine.source_path(EntityKind::Location, "castle/yard").unwrap();
        assert!(yard.ends_with("castle/yard.md"));
        assert!(engine
            .source_path(EntityKind::Object, "castle/yard")
            .unwrap()
            .ends_with("objects/yard.md"));
        assert!(engine
            .source_path(EntityKind::Location, "nowhere")
            .unwrap_err()
            .to_string()
            .contains("no location `nowhere`"));
        let dup_err = engine
            .source_path(EntityKind::Character, "king")
            .unwrap_err()
            .to_string();
        assert!(dup_err.contains("defined in 2 files"), "{}", dup_err);

        let dups = engine.duplicate_ids();
        assert_eq!(dups.len(), 1);
        assert_eq!((dups[0].0, dups[0].1.as_str()), (EntityKind::Character, "king"));

        let problems = engine.problems(&report, false);
        assert_eq!(problems.len(), 1, "{:?}", problems);
        assert!(problems[0].starts_with("character `king` is defined in 2 files: "));
    }

    fn small_world() -> (tempfile::TempDir, WorldEngine, WorldLoadReport) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "castle/yard.md", "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  north: castle/hall\n---\nGrass.\n");
        write(dir.path(), "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\ncharacters: [king]\n---\n");
        write(dir.path(), "npcs/king.md", "---\nid: king\ntitle: King\ntype: character\ndefault_location: castle/hall\n---\n");
        write(dir.path(), "broken.md", "no frontmatter\n");
        let mut engine = WorldEngine::new();
        let report = engine.load_world(dir.path()).unwrap();
        (dir, engine, report)
    }

    #[test]
    fn a_character_its_room_doesnt_list_is_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "hall.md", "---\nid: hall\ntitle: Hall\ntype: location\n---\n");
        write(dir.path(), "npcs/cook.md", "---\nid: cook\ntitle: The cook\ntype: character\ndefault_location: hall\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        let warnings = engine.current_warnings();
        assert!(warnings.iter().any(|w| w.starts_with("character `cook`: `default_location: hall` doesn't put it there")), "{:?}", warnings);
        write(dir.path(), "hall.md", "---\nid: hall\ntitle: Hall\ntype: location\ncharacters: [cook]\n---\n");
        engine.load_world(dir.path()).unwrap();
        assert!(engine.current_warnings().is_empty(), "{:?}", engine.current_warnings());
    }

    #[test]
    fn question_sets_load_from_questions_folders_and_never_as_things() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\n---\n");
        write(dir.path(), "objects/templates/lantern.md", "---\ntype: object\nobject_type: light\n---\n");
        write(dir.path(), "questions/lantern.md", "---\ntype: questions\n---\n## How bright?\n- Dim (default)\n- Bright -> set brightness\n");
        write(dir.path(), "castle/questions/object.md", "---\ntype: questions\n---\n## Is it castle-made?\n- Yes\n- No\n");
        write(dir.path(), "questions/teapot.md", "---\ntype: questions\n---\n## Hot?\n- Yes\n");
        write(dir.path(), "questions/index.md", "---\ntitle: Not a set\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        assert_eq!(engine.objects.len(), 0, "a set is never a thing");
        let q = |folder: &str, template: Option<&str>| engine.questions_for(Path::new(folder), template, "object").map(|s| s.questions[0].text.clone());
        assert_eq!(q("castle", Some("lantern")).as_deref(), Some("How bright?"));
        assert_eq!(q("castle/keep", None).as_deref(), Some("Is it castle-made?"));
        assert_eq!(q("forest", None).as_deref(), Some("Where does it start?"), "the built-in");
        let problems = engine.current_problems(false);
        assert!(problems.iter().any(|p| p.contains("questions/index.md: a file in questions/ is a question set, so it needs `type: questions`")), "{:?}", problems);
        // A set for a thing not written yet is fine (Bruce, 2026-11-24).
        assert!(engine.current_warnings().is_empty(), "{:?}", engine.current_warnings());
        assert_eq!(engine.questions_for_thing(Path::new("castle"), "teapot", None, "object").unwrap().questions[0].text, "Hot?");
    }

    #[test]
    fn validation_is_the_current_problems_and_warnings() {
        let (_dir, engine, report) = small_world();
        let v = engine.validation(false);
        assert_eq!(v.problems, engine.problems(&report, false));
        assert_eq!(v.warnings, report.warnings);
        assert!(!v.passed());
        assert!(v.summary().starts_with("Validation failed: 1 problem(s)"), "{}", v.summary());
        assert_eq!(Validation::default().summary(), "Validation passed: no problems or warnings");
    }

    #[test]
    fn current_problems_match_a_fresh_load() {
        let (_dir, engine, report) = small_world();
        assert_eq!(engine.current_problems(true), engine.problems(&report, true));
        assert_eq!(engine.current_problems(false).len(), 1); // broken.md
        assert_eq!(engine.current_warnings(), report.warnings);
    }

    #[test]
    fn set_file_changes_an_entity_and_its_problems_in_memory() {
        let (dir, mut engine, _) = small_world();
        let yard = dir.path().join("castle/yard.md");
        engine.set_file(
            &yard,
            "---\nid: castle/yard\ntitle: New Yard\ntype: location\nconnections:\n  north: castle/hall\n  west: castle/gone\n---\n",
        );
        assert_eq!(engine.get_location("castle/yard").unwrap().name, "New Yard");
        assert_eq!(engine.loaded_locations["castle/yard"].name, "New Yard");
        let problems = engine.current_problems(false);
        assert!(problems.iter().any(|p| p.contains("`west` points at unknown location `castle/gone`")), "{:?}", problems);
        // Nothing was written.
        assert!(std::fs::read_to_string(&yard).unwrap().contains("title: Yard\n"));

        // A new file adds the missing room and fixes the exit.
        engine.set_file(&dir.path().join("castle/gone.md"), "---\nid: castle/gone\ntitle: Gone\ntype: location\n---\n");
        assert!(engine.current_problems(false).iter().all(|p| !p.contains("castle/gone")));
        assert!(engine.source_path(EntityKind::Location, "castle/gone").is_ok());
    }

    #[test]
    fn a_file_that_stops_parsing_drops_its_entity_and_reports_why() {
        let (dir, mut engine, _) = small_world();
        let hall = dir.path().join("castle/hall.md");
        engine.set_file(&hall, "---\nid: castle/hall\ntype: location\n---\n");
        assert!(engine.get_location("castle/hall").is_none());
        assert!(engine.source_path(EntityKind::Location, "castle/hall").is_err());
        let problems = engine.current_problems(true);
        assert!(problems.iter().any(|p| p.contains("hall.md") && p.contains("`title`")), "{:?}", problems);
        assert!(problems.iter().any(|p| p.contains("`north` points at unknown location `castle/hall`")));
        assert!(problems.iter().any(|p| p.starts_with("king: default_location `castle/hall`")));

        // Fixing it brings everything back.
        engine.set_file(&hall, "---\nid: castle/hall\ntitle: Hall\ntype: location\n---\n");
        assert_eq!(engine.current_problems(true).len(), 1); // only broken.md
    }

    #[test]
    fn duplicates_and_removal_resolve_like_a_load() {
        let (dir, mut engine, _) = small_world();
        let copy = dir.path().join("castle/yard-copy.md");
        engine.set_file(&copy, "---\nid: castle/yard\ntitle: Copy\ntype: location\n---\n");
        assert_eq!(engine.get_location("castle/yard").unwrap().name, "Copy"); // last one wins
        assert!(engine.current_problems(false).iter().any(|p| p.contains("defined in 2 files")));
        assert!(engine.source_path(EntityKind::Location, "castle/yard").is_err());

        engine.remove_file(&copy);
        assert_eq!(engine.get_location("castle/yard").unwrap().name, "Yard");
        assert!(engine.source_path(EntityKind::Location, "castle/yard").is_ok());

        engine.remove_file(&dir.path().join("npcs/king.md"));
        assert!(!engine.characters.contains_key("king"));
        assert!(!engine.sources.contains_key(&(EntityKind::Character, "king".to_string())));
    }

    #[test]
    fn a_file_whose_id_changes_moves_its_entity() {
        let (dir, mut engine, _) = small_world();
        engine.set_file(&dir.path().join("castle/hall.md"), "---\nid: castle/great-hall\ntitle: Great Hall\ntype: location\n---\n");
        assert!(engine.get_location("castle/hall").is_none());
        assert_eq!(engine.get_location("castle/great-hall").unwrap().name, "Great Hall");
    }

    #[test]
    fn ids_shared_between_kinds_are_warnings_not_problems() {
        let (dir, mut engine, report) = small_world();
        assert!(engine.shared_ids().is_empty());
        assert!(report.warnings.is_empty());
        engine.set_file(&dir.path().join("objects/hall.md"), "---\nid: castle/hall\ntitle: Hall Bell\ntype: object\nobject_type: x\n---\n");
        assert_eq!(engine.shared_ids(), vec![("castle/hall".to_string(), vec![EntityKind::Location, EntityKind::Object])]);
        let warnings: Vec<String> = engine.current_warnings().into_iter().filter(|w| w.starts_with("id ")).collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("id `castle/hall` is used by more than one kind (location, object)"), "{}", warnings[0]);
        assert_eq!(engine.current_problems(false).len(), 1); // still only broken.md

        // On a load, it's in the report's warnings too.
        let mut fresh = WorldEngine::new();
        write(dir.path(), "objects/hall.md", "---\nid: castle/hall\ntitle: Hall Bell\ntype: object\nobject_type: x\n---\n");
        let report = fresh.load_world(dir.path()).unwrap();
        assert_eq!(report.warnings, fresh.current_warnings());
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn describe_location_unknown_id_is_none() {
        assert!(engine().describe_location("castle/nowhere").is_none());
    }

    #[test]
    fn problems_include_load_errors_broken_exits_and_strict_checks() {
        let engine = engine();
        let report = WorldLoadReport {
            errors: vec!["bad.md: no type".into()],
            ..Default::default()
        };
        let loose = engine.problems(&report, false);
        assert_eq!(loose.len(), 2);
        assert_eq!(loose[0], "bad.md: no type");
        assert!(loose[1].contains("`north` points at unknown location `castle/gone`"));
        let strict = engine.problems(&report, true);
        assert_eq!(strict.len(), 3);
        assert!(strict[2].starts_with("king: default_location `castle/throne`"));
    }

    #[test]
    fn obsidian_links_name_things_by_id_or_by_file() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\nconnections:\n  south: \"[[castle/yard]]\"\n  down: \"[[castle/vault/index|the vault]]\"\n  up: [[castle/tower.md]]\nobjects: [\"[[objects/lever]]\", \"[[lamp]]\"]\ncharacters: [\"[[npcs/king]]\"]\n---\n");
        write(d, "castle/yard.md", "---\nid: castle/yard\ntitle: Yard\ntype: location\n---\n");
        write(d, "castle/tower.md", "---\nid: castle/tower\ntitle: Tower\ntype: location\n---\n");
        write(d, "castle/vault/index.md", "---\nid: castle/vault\ntitle: Vault\ntype: location\nstate: shut\n---\n");
        write(d, "castle/vault/shut.md", "---\ninherits: index\n---\n");
        write(d, "castle/vault/open.md", "---\ninherits: index\n---\n");
        write(d, "objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull:\n    becomes: { \"[[castle/vault]]\": open }\n    requires: { has: \"[[objects/lamp]]\" }\n---\n<!-- thwarter:action pull -->\nClunk.\n<!-- thwarter:action pull refused -->\nDark.\n");
        write(d, "objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: x\n---\n");
        write(d, "npcs/king.md", "---\nid: king\ntitle: King\ntype: character\ndefault_location: \"[[castle/hall]]\"\ninventory: [\"[[objects/lamp.md]]\"]\n---\n");
        write(d, "world/index.md", "---\ntitle: T\ntype: world\nstart: \"[[castle/hall]]\"\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        assert!(engine.current_problems(true).is_empty(), "{:?}", engine.current_problems(true));

        let hall = engine.get_location("castle/hall").unwrap();
        assert_eq!(hall.connections["south"], "castle/yard");
        assert_eq!(hall.connections["down"], "castle/vault");
        assert_eq!(hall.connections["up"], "castle/tower");
        assert_eq!(hall.objects, vec!["lever", "lamp"]);
        assert_eq!(hall.characters, vec!["king"]);
        let king = &engine.characters["king"];
        assert_eq!(king.default_location.as_deref(), Some("castle/hall"));
        assert_eq!(king.inventory, vec!["lamp"]);
        let pull = &engine.objects["lever"].actions["pull"];
        assert_eq!(pull.becomes["castle/vault"], "open");
        assert_eq!(pull.requires_has, vec!["lamp"]);
        assert_eq!(engine.world().start.as_deref(), Some("castle/hall"));

        // A link to nothing is reported like any unknown id.
        engine.set_file(&d.join("castle/yard.md"), "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  east: \"[[castle/nowhere]]\"\n---\n");
        assert!(engine.current_problems(false).iter().any(|p| p.contains("`east` points at unknown location `castle/nowhere`")));
    }

    #[test]
    fn a_file_name_two_files_share_is_reported_as_ambiguous() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\n---\n");
        write(d, "forest/hall.md", "---\nid: forest/hall\ntitle: Hall\ntype: location\n---\n");
        write(d, "yard.md", "---\nid: yard\ntitle: Yard\ntype: location\nconnections: { north: \"[[hall]]\" }\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        let problems = engine.current_problems(false);
        assert!(problems.iter().any(|p| p.starts_with("yard: connection `north` is ambiguous: `hall` matches more than one file:")
            && p.contains("castle/hall.md") && p.contains("forest/hall.md")), "{:?}", problems);
    }

    #[test]
    fn every_file_read_has_a_role() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "castle/_index.md", "---\ntitle: Castle\n---\n");
        write(d, "castle/vault/index.md", "---\nid: castle/vault\ntitle: Vault\ntype: location\nstate: shut\n---\n");
        write(d, "castle/vault/shut.md", "---\ninherits: index\n---\n");
        write(d, "castle/vault/chapters/after.md", "---\n---\n");
        write(d, "castle/chapters/after.md", "---\n---\n");
        write(d, "chapters/10-after.md", "---\n---\n");
        write(d, "objects/templates/book.md", "---\ntitle: Book\n---\n");
        write(d, "world/index.md", "---\ntitle: T\ntype: world\n---\n");
        write(d, "broken.md", "nope\n");
        write(d, ".obsidian/x.md", "nope\n");
        write(d, "world.toml", "ignore = [\"drafts\"]\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        let role = |p: &str| engine.file_role(Path::new(p)).cloned();
        let after = |scope: &str| ChapterKey { scope: PathBuf::from(scope), path: vec!["after".into()] };
        assert_eq!(role("castle/vault/index.md"), Some(FileRole::Thing { kind: EntityKind::Location, id: "castle/vault".into() }));
        assert_eq!(role("castle/vault/shut.md"), Some(FileRole::State { kind: EntityKind::Location, id: "castle/vault".into(), name: "shut".into() }));
        assert_eq!(role("castle/vault/chapters/after.md"), Some(FileRole::Overlay { kind: EntityKind::Location, id: "castle/vault".into(), chapter: after("") }));
        assert_eq!(role("castle/chapters/after.md"), Some(FileRole::AreaOverlay { area: "castle".into(), chapter: after("") }));
        assert_eq!(role("chapters/10-after.md"), Some(FileRole::Chapter(after(""))));
        assert_eq!(role("castle/_index.md"), Some(FileRole::Area));
        assert_eq!(role("objects/templates/book.md"), Some(FileRole::Template));
        assert_eq!(role("world/index.md"), Some(FileRole::World));
        assert!(matches!(role("broken.md"), Some(FileRole::Problem(why)) if why.contains("frontmatter")));
        assert_eq!(role(".obsidian/x.md"), None);
        assert_eq!(engine.file_roles().count(), 9);
        assert!(engine.is_ignored(Path::new("drafts/a.md")) && engine.is_ignored(Path::new(".git/x")));
        assert!(!engine.is_ignored(Path::new("castle/hall.md")));
    }

    #[test]
    fn hostile_files_are_refused_at_load() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "hall.md", "---\nid: hall\ntitle: Hall\ntype: location\n---\n");
        write(d, "big.md", &format!("---\nid: big\ntitle: Big\ntype: location\n---\n{}", "x".repeat(crate::engines::layers::MAX_FILE_BYTES + 1)));
        write(d, "deep.md", &format!("---\nid: deep\ntitle: Deep\ntype: location\nvars: {}{}\n---\n", "[".repeat(40), "]".repeat(40)));
        let bomb = "---\nid: bomb\ntitle: Bomb\ntype: location\na: &a [x, x, x, x, x, x, x, x, x, x]\nb: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]\nc: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]\nd: &d [*c, *c, *c, *c, *c, *c, *c, *c, *c, *c]\ne: &e [*d, *d, *d, *d, *d, *d, *d, *d, *d, *d]\nf: &f [*e, *e, *e, *e, *e, *e, *e, *e, *e, *e]\ng: &g [*f, *f, *f, *f, *f, *f, *f, *f, *f, *f]\nh: [*g, *g, *g, *g, *g, *g, *g, *g, *g, *g]\n---\n";
        write(d, "bomb.md", bomb);
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "secret.md", "---\nid: secret\ntitle: Secret\ntype: location\n---\n");
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path().join("secret.md"), d.join("secret.md")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(d.join("hall.md"), d.join("hall-link.md")).unwrap();

        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        let problems = engine.current_problems(false);
        assert!(problems.iter().any(|p| p.contains("big.md") && p.contains("over the 1024 KB")), "{:?}", problems);
        assert!(problems.iter().any(|p| p.contains("deep.md") && p.contains("more than 32 levels")), "{:?}", problems);
        assert!(problems.iter().any(|p| p.contains("bomb.md")), "{:?}", problems);
        assert!(engine.get_location("bomb").is_none());
        #[cfg(unix)]
        {
            assert!(engine.get_location("secret").is_none());
            assert!(engine.current_warnings().iter().any(|w| w.contains("secret.md: is a link to outside the world")));
            // A link inside the world is read (here, a duplicate of the hall).
            assert!(engine.current_problems(false).iter().any(|p| p.contains("location `hall` is defined in 2 files")));
        }
    }
}
