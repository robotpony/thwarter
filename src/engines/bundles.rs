//! Bundles, states, chapters and overlays (`design/STATES.md` › Files,
//! Engine rules › Loading).
//!
//! A file's role comes from where it sits, never from its contents:
//! - a folder holding `index.md` is one thing (a bundle); its other files
//!   are its states, and it may have one `chapters/` folder;
//! - in any `chapters/` folder, a numbered file (`20-after-the-king.md`, or
//!   a numbered folder with `_index.md`) defines a chapter in that scope,
//!   and an unnumbered one (`after-the-king.md`) is an overlay, changing
//!   that folder's things in the chapter of that name seen from there;
//! - `_index.md` marks an area;
//! - `world/` and `game/` at the root, any `templates/` folder, and any
//!   file with `template: true` are kept out of the world;
//! - everything else is a one-file thing.

use super::layers::{Layer, Merged};
use super::world_engine::EntityKind;
use crate::parsers::{key_case_warnings, MarkdownParser, ParsedEntity};
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Component, Path, PathBuf};

/// One loaded `.md` file: split into a layer, or why it couldn't be.
#[derive(Debug, Clone)]
pub(crate) struct FileRecord {
    pub path: PathBuf,
    pub layer: Result<Layer, String>,
}

/// Which chapter: the folder whose `chapters/` defines it (relative to the
/// world root, empty for the root) and its names from the top-level
/// chapter down (`["after-the-king", "the-funeral"]`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChapterKey {
    pub scope: PathBuf,
    pub path: Vec<String>,
}

impl ChapterKey {
    pub fn name(&self) -> &str {
        self.path.last().map(String::as_str).unwrap_or("")
    }

    /// This chapter and its parents, top level first.
    fn lineage(&self) -> Vec<ChapterKey> {
        (1..=self.path.len())
            .map(|n| ChapterKey { scope: self.scope.clone(), path: self.path[..n].to_vec() })
            .collect()
    }
}

/// A chapter, defined in its scope's `chapters/` folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Chapter {
    pub key: ChapterKey,
    /// The numbers that order it, top level first.
    pub order: Vec<u64>,
    pub layer: Layer,
}

impl Chapter {
    pub fn name(&self) -> &str {
        self.key.name()
    }

    pub fn title(&self) -> Option<&str> {
        self.layer.str_key("title")
    }
}

/// An overlay from a folder above a thing.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaOverlay {
    /// The area folder, relative to the world root.
    pub area: PathBuf,
    pub chapter: ChapterKey,
    pub layer: Layer,
}

/// A room, object or character with every file that can apply to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Thing {
    pub kind: EntityKind,
    pub id: String,
    /// `index.md` for a bundle, or the thing's one file.
    pub index: Layer,
    /// The bundle's folder, relative to the world root; `None` for a
    /// one-file thing.
    pub bundle: Option<PathBuf>,
    /// The folder chapter names resolve from: the bundle, or the folder a
    /// one-file thing sits in. Relative to the world root.
    pub scope: PathBuf,
    /// What `index.md`'s `state:` names, if it names a state that exists.
    pub starting_state: Option<String>,
    /// State files by name.
    pub states: BTreeMap<String, Layer>,
    /// The thing's own chapter overlays, by the chapter each applies in.
    pub overlays: BTreeMap<ChapterKey, Layer>,
    /// Overlays from the folders above it, outermost folder first.
    pub area_overlays: Vec<AreaOverlay>,
}

impl Thing {
    /// The layers for one version, bottom first: the folders' overlays
    /// above it, then its index, its state, then its own overlays. Within
    /// each level a parent chapter comes before its sub-chapter.
    pub fn layers<'a>(&'a self, state: Option<&str>, chapter: Option<&ChapterKey>) -> Vec<&'a Layer> {
        let lineage = chapter.map(ChapterKey::lineage).unwrap_or_default();
        let mut layers = Vec::new();
        let mut areas: Vec<&Path> = Vec::new();
        for overlay in &self.area_overlays {
            if !areas.contains(&overlay.area.as_path()) {
                areas.push(&overlay.area);
            }
        }
        for area in areas {
            for key in &lineage {
                if let Some(o) = self.area_overlays.iter().find(|o| o.area == area && o.chapter == *key) {
                    layers.push(&o.layer);
                }
            }
        }
        layers.push(&self.index);
        if let Some(layer) = state.and_then(|s| self.states.get(s)) {
            layers.push(layer);
        }
        for key in &lineage {
            if let Some(layer) = self.overlays.get(key) {
                layers.push(layer);
            }
        }
        layers
    }

    /// Merge and type one version. The error is the parser's.
    pub fn build(&self, state: Option<&str>, chapter: Option<&ChapterKey>) -> Result<(ParsedEntity, Merged), String> {
        let merged = Merged::of(self.layers(state, chapter));
        let outcome = MarkdownParser::new()
            .build_entity(&merged.document(), &self.index.path)
            .map_err(|e| e.to_string())?;
        Ok((outcome.entity, merged))
    }
}

/// The chapter `path` names as seen from `scope`: defined there, or in the
/// nearest folder above it, out to the root.
pub fn resolve_chapter<'a>(chapters: &'a [Chapter], scope: &Path, path: &[String]) -> Option<&'a Chapter> {
    let mut dir = Some(scope);
    while let Some(d) = dir {
        if let Some(c) = chapters.iter().find(|c| c.key.scope == d && c.key.path == path) {
            return Some(c);
        }
        dir = if d.as_os_str().is_empty() { None } else { Some(d.parent().unwrap_or(Path::new(""))) };
    }
    None
}

/// The chapters visible from `scope`, nearest scope first and in chapter
/// order within each; a nearer chapter hides an outer one of the same name.
pub fn visible_chapters<'a>(chapters: &'a [Chapter], scope: &Path) -> Vec<&'a Chapter> {
    let mut seen: HashSet<&[String]> = HashSet::new();
    let mut out = Vec::new();
    let mut dir = Some(scope);
    while let Some(d) = dir {
        for c in chapters.iter().filter(|c| c.key.scope == d) {
            if seen.insert(c.key.path.as_slice()) {
                out.push(c);
            }
        }
        dir = if d.as_os_str().is_empty() { None } else { Some(d.parent().unwrap_or(Path::new(""))) };
    }
    out
}

/// What a world's files add up to.
#[derive(Debug, Clone, Default)]
pub(crate) struct Built {
    /// Each thing whose starting version types, with that version, in load order.
    pub things: Vec<(Thing, ParsedEntity)>,
    /// Every chapter, sorted by scope and then chapter order.
    pub chapters: Vec<Chapter>,
    pub problems: Vec<String>,
    pub warnings: Vec<String>,
}

enum Role {
    Entity { bundle: Option<PathBuf> },
    State { bundle: PathBuf, name: String },
    /// A file in the `chapters/` folder of `scope`, with its path inside it.
    InChapters { scope: PathBuf, parts: Vec<String> },
    Area,
    Excluded,
    Invalid(String),
}

const RESERVED_STATES: &[&str] = &["index", "_index", "chapters", "templates"];

fn names(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

fn excluded(parts: &[String]) -> bool {
    matches!(parts.first().map(String::as_str), Some("world") | Some("game")) && parts.len() > 1
        || parts[..parts.len().saturating_sub(1)].iter().any(|p| p == "templates")
}

fn classify(rel: &Path, bundles: &BTreeSet<PathBuf>) -> Role {
    let parts = names(rel);
    if excluded(&parts) {
        return Role::Excluded;
    }
    // The outermost bundle holding the file: a nested index.md is then
    // just a file in a subfolder of a bundle, which is refused.
    if let Some(bundle) = bundles.iter().find(|b| rel.starts_with(b) && rel != b.as_path()) {
        let inner = names(rel.strip_prefix(bundle).unwrap());
        return match inner.as_slice() {
            [file] if file == "index.md" => Role::Entity { bundle: Some(bundle.clone()) },
            [file] => {
                let name = file.trim_end_matches(".md");
                if RESERVED_STATES.contains(&name) {
                    Role::Invalid(format!("`{}` is a reserved name and can't be a state of {}/", name, bundle.display()))
                } else {
                    Role::State { bundle: bundle.clone(), name: name.to_string() }
                }
            }
            [dir, rest @ ..] if dir == "chapters" => Role::InChapters { scope: bundle.clone(), parts: rest.to_vec() },
            _ => Role::Invalid(format!(
                "a bundle is a leaf: {}/ may hold only its state files and one `chapters/` folder",
                bundle.display()
            )),
        };
    }
    let dirs = &parts[..parts.len().saturating_sub(1)];
    if let Some(at) = dirs.iter().position(|p| p == "chapters") {
        return Role::InChapters { scope: parts[..at].iter().collect(), parts: parts[at + 1..].to_vec() };
    }
    let file = parts.last().map(String::as_str).unwrap_or("");
    if file == "_index.md" {
        return Role::Area;
    }
    if file == "chapters.md" || file == "templates.md" || (file == "index.md" && parts.len() == 1) {
        return Role::Invalid(format!("`{}` is a reserved name here", file.trim_end_matches(".md")));
    }
    Role::Entity { bundle: None }
}

/// `20-after-the-king` → `(20, "after-the-king")`.
fn numbered(name: &str) -> Option<(u64, String)> {
    let (number, rest) = name.split_once('-')?;
    if number.is_empty() || rest.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((number.parse().ok()?, rest.to_string()))
}

/// Take away keys a layer other than an index may not set, reporting each.
fn strip_identity(layer: &mut Layer, what: &str, problems: &mut Vec<String>) {
    for key in ["id", "type"] {
        if layer.metadata.remove(key).is_some() {
            problems.push(format!(
                "{}: {} has no `{}`; it comes from the thing's index.md, and no layer may change it",
                layer.path.display(),
                what,
                key
            ));
        }
    }
    if layer.metadata.remove("state").is_some() {
        problems.push(format!(
            "{}: only index.md may set `state:`, which names the starting state",
            layer.path.display()
        ));
    }
}

/// An overlay waiting for its chapter to be resolved.
struct PendingOverlay {
    scope: PathBuf,
    is_bundle: bool,
    path: Vec<String>,
    layer: Layer,
}

pub(crate) fn build(root: &Path, files: &[FileRecord]) -> Built {
    let mut built = Built::default();
    let rel = |p: &Path| p.strip_prefix(root).unwrap_or(p).to_path_buf();

    let is_template = |f: &FileRecord| {
        matches!(&f.layer, Ok(l) if matches!(l.metadata.get("template"), Some(Value::Bool(true))))
    };
    let bundle_dirs: BTreeSet<PathBuf> = files
        .iter()
        .filter(|f| !is_template(f))
        .map(|f| rel(&f.path))
        .filter(|r| r.file_name().map_or(false, |n| n == "index.md") && !excluded(&names(r)))
        .filter_map(|r| r.parent().map(Path::to_path_buf))
        .filter(|d| !d.as_os_str().is_empty() && !names(d).iter().any(|n| n == "chapters"))
        .collect();

    let mut entities: Vec<(Layer, Option<PathBuf>)> = Vec::new();
    let mut states: BTreeMap<PathBuf, BTreeMap<String, Layer>> = BTreeMap::new();
    let mut pending: Vec<PendingOverlay> = Vec::new();
    let mut definitions: Vec<(PathBuf, Vec<String>, Layer)> = Vec::new();

    for file in files {
        let shown = file.path.display();
        // world/ and game/ have their own formats, not built yet.
        if excluded(&names(&rel(&file.path))) {
            continue;
        }
        let mut layer = match &file.layer {
            Ok(layer) => layer.clone(),
            Err(e) => {
                built.problems.push(format!("{}: {}", shown, e));
                continue;
            }
        };
        if is_template(file) {
            continue;
        }
        match classify(&rel(&file.path), &bundle_dirs) {
            Role::Excluded => {}
            Role::Invalid(reason) => built.problems.push(format!("{}: {}", shown, reason)),
            Role::Entity { bundle } => {
                if layer.has_key("inherits") {
                    built.problems.push(format!(
                        "{}: `inherits:` names a template, and templates aren't supported yet",
                        shown
                    ));
                }
                entities.push((layer, bundle));
            }
            Role::State { bundle, name } => {
                strip_identity(&mut layer, "a state file", &mut built.problems);
                match layer.metadata.get("inherits") {
                    Some(Value::String(s)) if s == "index" => {}
                    None => built.problems.push(format!(
                        "{}: a state file starts from its bundle's index, so it must say `inherits: index`",
                        shown
                    )),
                    Some(other) => built.problems.push(format!(
                        "{}: `inherits: {}`: a state can only inherit from `index`",
                        shown,
                        serde_yaml::to_string(other).unwrap_or_default().trim()
                    )),
                }
                states.entry(bundle).or_default().insert(name, layer);
            }
            Role::InChapters { scope, parts } => {
                let is_bundle = bundle_dirs.contains(&scope);
                in_chapters(scope, is_bundle, parts, layer, &mut definitions, &mut pending, &mut built);
            }
            Role::Area => {
                for key in layer.metadata.keys().filter_map(Value::as_str) {
                    if key != "title" {
                        built.warnings.push(format!(
                            "{}: an area's _index.md takes only `title` for now, so `{}` is ignored",
                            shown, key
                        ));
                    }
                }
            }
        }
    }

    built.chapters = build_chapters(definitions, &mut built.problems, &mut built.warnings);

    // Resolve each overlay's chapter from its folder outward.
    let mut overlays: BTreeMap<PathBuf, BTreeMap<ChapterKey, Layer>> = BTreeMap::new();
    let mut area_overlays: Vec<AreaOverlay> = Vec::new();
    for o in pending {
        let Some(chapter) = resolve_chapter(&built.chapters, &o.scope, &o.path) else {
            let mut hint = String::new();
            if let Some((_, bare)) = o.path.last().and_then(|n| numbered(n)) {
                hint = format!("; overlays are named without the chapter's number (`{}.md`)", bare);
            }
            built.problems.push(format!(
                "{}: no chapter `{}` is defined here or in a folder above{}",
                o.layer.path.display(),
                o.path.join("/"),
                hint
            ));
            continue;
        };
        let target = if o.is_bundle {
            format!("{}/", o.scope.display())
        } else if o.scope.as_os_str().is_empty() {
            "everything in the world".to_string()
        } else {
            format!("everything in {}/", o.scope.display())
        };
        built.warnings.push(format!(
            "{}: applies to {} in chapter `{}` by where it sits",
            o.layer.path.display(),
            target,
            chapter.name()
        ));
        if o.is_bundle {
            overlays.entry(o.scope).or_default().insert(chapter.key.clone(), o.layer);
        } else {
            area_overlays.push(AreaOverlay { area: o.scope, chapter: chapter.key.clone(), layer: o.layer });
        }
    }

    for (index, bundle) in entities {
        let relative = rel(&index.path);
        if let Some(thing) = build_thing(index, relative, bundle, &states, &overlays, &area_overlays, &built.chapters.clone(), &mut built) {
            built.things.push(thing);
        }
    }

    dedup(&mut built.problems);
    dedup(&mut built.warnings);
    built
}

/// Sort a file in a `chapters/` folder into a chapter definition (all
/// numbered) or an overlay (none numbered).
fn in_chapters(
    scope: PathBuf,
    is_bundle: bool,
    mut parts: Vec<String>,
    mut layer: Layer,
    definitions: &mut Vec<(PathBuf, Vec<String>, Layer)>,
    pending: &mut Vec<PendingOverlay>,
    built: &mut Built,
) {
    let shown = layer.path.display().to_string();
    if let Some(last) = parts.last_mut() {
        if let Some(stem) = last.strip_suffix(".md") {
            *last = stem.to_string();
        }
    }
    let folder_chapter = parts.last().map(String::as_str) == Some("_index");
    if folder_chapter {
        parts.pop();
    }
    if parts.is_empty() || parts.iter().any(|p| p == "index" || p == "_index") {
        built.problems.push(format!(
            "{}: name chapters like chapters/10-the-summons.md and overlays like chapters/the-summons.md",
            shown
        ));
        return;
    }
    let numbered_count = parts.iter().filter(|p| numbered(p).is_some()).count();
    if numbered_count == parts.len() {
        for key in ["id", "type"] {
            if layer.has_key(key) {
                built.problems.push(format!("{}: a chapter has no `{}`", shown, key));
            }
        }
        for key in layer.metadata.keys().filter_map(Value::as_str) {
            if !["title", "sets", "id", "type"].contains(&key) {
                built.warnings.push(format!("{}: `{}` isn't a chapter key yet, so it is ignored", shown, key));
            }
        }
        definitions.push((scope, parts, layer));
    } else if numbered_count == 0 && !folder_chapter {
        if scope.as_os_str().is_empty() {
            built.problems.push(format!(
                "{}: a root chapter's name starts with a number that orders it, like `10-{}`",
                shown,
                parts.join("/")
            ));
            return;
        }
        strip_identity(&mut layer, "a chapter overlay", &mut built.problems);
        if layer.metadata.remove("inherits").is_some() {
            built.problems.push(format!(
                "{}: an overlay doesn't take `inherits:`; it applies to whatever version is current",
                shown
            ));
        }
        pending.push(PendingOverlay { scope, is_bundle, path: parts, layer });
    } else {
        built.problems.push(format!(
            "{}: a chapter is numbered all the way down (chapters/20-a/21-b.md); an overlay isn't numbered at all (chapters/a/b.md)",
            shown
        ));
    }
}

fn build_chapters(
    definitions: Vec<(PathBuf, Vec<String>, Layer)>,
    problems: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Vec<Chapter> {
    let mut chapters: Vec<Chapter> = Vec::new();
    for (scope, parts, layer) in definitions {
        let (order, path): (Vec<u64>, Vec<String>) = parts.iter().filter_map(|p| numbered(p)).unzip();
        let key = ChapterKey { scope, path };
        if let Some(first) = chapters.iter().find(|c| c.key.scope == key.scope && c.name() == key.name()) {
            problems.push(format!(
                "{}: chapter `{}` is already defined in {}",
                layer.path.display(),
                key.name(),
                first.layer.path.display()
            ));
            continue;
        }
        chapters.push(Chapter { key, order, layer });
    }
    // A sub-chapter's folder must be a chapter itself (it has an _index.md).
    let keys: HashSet<ChapterKey> = chapters.iter().map(|c| c.key.clone()).collect();
    chapters.retain(|c| {
        let parent = ChapterKey { scope: c.key.scope.clone(), path: c.key.path[..c.key.path.len() - 1].to_vec() };
        if parent.path.is_empty() || keys.contains(&parent) {
            return true;
        }
        problems.push(format!(
            "{}: its folder needs an _index.md to be the chapter `{}`",
            c.layer.path.display(),
            parent.name()
        ));
        false
    });
    chapters.sort_by(|a, b| (a.key.scope.components().count(), &a.key.scope, &a.order).cmp(&(b.key.scope.components().count(), &b.key.scope, &b.order)));
    // A chapter in a narrow scope that hides an outer one of the same name
    // is most likely an overlay written with its number.
    for c in chapters.iter().filter(|c| !c.key.scope.as_os_str().is_empty()) {
        let outer = c.key.scope.parent().unwrap_or(Path::new(""));
        if let Some(hidden) = resolve_chapter(&chapters, outer, &c.key.path) {
            warnings.push(format!(
                "{}: defines a new chapter `{}` here, hiding the one in {}; to change things here in that chapter, name the file `{}.md` without the number",
                c.layer.path.display(),
                c.name(),
                hidden.layer.path.display(),
                c.name()
            ));
        }
    }
    chapters
}

#[allow(clippy::too_many_arguments)]
fn build_thing(
    index: Layer,
    relative: PathBuf,
    bundle: Option<PathBuf>,
    states: &BTreeMap<PathBuf, BTreeMap<String, Layer>>,
    overlays: &BTreeMap<PathBuf, BTreeMap<ChapterKey, Layer>>,
    area_overlays: &[AreaOverlay],
    chapters: &[Chapter],
    built: &mut Built,
) -> Option<(Thing, ParsedEntity)> {
    let shown = index.path.display().to_string();
    let own_states = bundle.as_ref().and_then(|b| states.get(b)).cloned().unwrap_or_default();
    let own_overlays = bundle.as_ref().and_then(|b| overlays.get(b)).cloned().unwrap_or_default();
    let scope = match &bundle {
        Some(b) => b.clone(),
        None => relative.parent().map(Path::to_path_buf).unwrap_or_default(),
    };

    let starting_state = match index.metadata.get("state") {
        None => None,
        Some(Value::String(name)) if own_states.contains_key(name) => Some(name.clone()),
        Some(Value::String(name)) => {
            let place = match &bundle {
                Some(b) => format!("no state file {}/{}.md exists", b.display(), name),
                None => "a one-file thing has no states; it becomes a bundle when it gets its first".to_string(),
            };
            built.problems.push(format!("{}: `state: {}` names a starting state, but {}", shown, name, place));
            None
        }
        Some(_) => {
            built.problems.push(format!("{}: `state:` must name one of the bundle's state files", shown));
            None
        }
    };

    // Overlays from the folders above the thing (for a bundle, not its own
    // folder), outermost first.
    let above_dir = match &bundle {
        Some(b) => b.parent().map(Path::to_path_buf).unwrap_or_default(),
        None => scope.clone(),
    };
    let mut above: Vec<AreaOverlay> = area_overlays
        .iter()
        .filter(|o| above_dir.starts_with(&o.area))
        .cloned()
        .collect();
    above.sort_by_key(|o| o.area.components().count());

    let mut thing = Thing {
        kind: index.str_key("type").and_then(EntityKind::from_type_name).unwrap_or(EntityKind::Location),
        id: index.str_key("id").unwrap_or_default().to_string(),
        index,
        bundle,
        scope,
        starting_state,
        states: own_states,
        overlays: own_overlays,
        area_overlays: above,
    };

    // The starting version is what the world tables hold.
    let start = thing.starting_state.clone();
    let (entity, merged) = match thing.build(start.as_deref(), None) {
        Ok(ok) => ok,
        Err(e) => {
            built.problems.push(format!("{}: {}", shown, e));
            return None;
        }
    };
    built.warnings.extend(merged.warnings);
    let (kind, id) = match &entity {
        ParsedEntity::Location(l) => (EntityKind::Location, l.id.clone()),
        ParsedEntity::Character(c) => (EntityKind::Character, c.id.clone()),
        ParsedEntity::Object(o) => (EntityKind::Object, o.id.clone()),
    };
    thing.kind = kind;
    thing.id = id;

    // Keys that differ only in case, in each of the thing's own files.
    let type_name = kind.type_name();
    let own: Vec<&Layer> = std::iter::once(&thing.index).chain(thing.states.values()).chain(thing.overlays.values()).collect();
    for layer in own {
        for w in key_case_warnings(&Value::Mapping(layer.metadata.clone()), type_name) {
            built.warnings.push(format!("{}: {}", layer.path.display(), w));
        }
    }

    if let Some(folder) = &thing.bundle {
        let matches = match kind {
            EntityKind::Location => folder == Path::new(&thing.id),
            _ => folder.ends_with(&thing.id),
        };
        if !matches {
            built.warnings.push(format!("{}: id `{}` doesn't match its folder {}/", shown, thing.id, folder.display()));
        }
    }

    // Every state against every chapter visible from the thing.
    let mut state_options: Vec<Option<String>> = thing.states.keys().cloned().map(Some).collect();
    if thing.starting_state.is_none() {
        state_options.insert(0, None);
    }
    let mut chapter_options: Vec<Option<&Chapter>> = vec![None];
    chapter_options.extend(visible_chapters(chapters, &thing.scope).into_iter().map(Some));
    for state in &state_options {
        for chapter in &chapter_options {
            if *state == start && chapter.is_none() {
                continue;
            }
            match thing.build(state.as_deref(), chapter.map(|c| &c.key)) {
                Ok((_, merged)) => built.warnings.extend(merged.warnings),
                Err(e) => {
                    let mut context = Vec::new();
                    if let Some(s) = state {
                        context.push(format!("in state `{}`", s));
                    }
                    if let Some(c) = chapter {
                        context.push(format!("in chapter `{}`", c.name()));
                    }
                    built.problems.push(format!("{} ({}): {}", shown, context.join(", "), e));
                }
            }
        }
    }

    Some((thing, entity))
}

fn dedup(items: &mut Vec<String>) {
    let mut seen = HashSet::new();
    items.retain(|i| seen.insert(i.clone()));
}

#[cfg(test)]
mod tests {
    use crate::engines::{EntityKind, WorldEngine};
    use crate::parsers::ParsedEntity;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// A vault bundle with two states, a courtyard with chapter overlays,
    /// an area overlay for the castle, and root chapters with a sub-chapter.
    fn castle() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "castle/_index.md", "---\ntitle: Castle\n---\n");
        write(d, "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\n---\nStone.\n");
        write(
            d,
            "castle/vault/index.md",
            "---\nid: castle/vault\ntitle: Vault\ntype: location\nstate: closed\nconnections:\n  north: castle/hall\n---\n",
        );
        write(d, "castle/vault/closed.md", "---\ninherits: index\n---\nA door, shut fast.\n");
        write(
            d,
            "castle/vault/open.md",
            "---\ninherits: index\nconnections:\n  down: castle/hall\n---\nThe door stands open.\n",
        );
        write(d, "castle/vault/chapters/after-the-king.md", "---\n---\nThe door hangs off one hinge.\n");
        write(d, "castle/courtyard/index.md", "---\nid: castle/courtyard\ntitle: Courtyard\ntype: location\n---\nSunlit.\n");
        write(d, "castle/courtyard/chapters/after-the-king.md", "---\ntitle: Courtyard in mourning\n---\n<!-- black drapes -->\n");
        write(d, "castle/courtyard/chapters/after-the-king/the-funeral.md", "---\n---\nA procession passes.\n");
        write(d, "castle/chapters/after-the-king.md", "---\ncoordinates: [1, 2, 3]\ntitle: Castle title\n---\n");
        write(d, "chapters/10-the-summons.md", "---\ntitle: The summons\n---\n");
        write(d, "chapters/20-after-the-king/_index.md", "---\ntitle: After the king\nsets: { story.king: dead }\n---\n");
        write(d, "chapters/20-after-the-king/21-the-funeral.md", "---\ntitle: The funeral\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        (dir, engine)
    }

    fn location(version: &crate::engines::Version) -> &crate::models::world::Location {
        match &version.entity {
            ParsedEntity::Location(l) => l,
            other => panic!("not a location: {:?}", other),
        }
    }

    #[test]
    fn a_bundle_loads_its_starting_state_and_any_other_version() {
        let (dir, engine) = castle();
        let vault = engine.get_location("castle/vault").unwrap();
        assert_eq!(vault.description, "A door, shut fast.");
        assert_eq!(vault.connections.len(), 1);
        assert_eq!(
            engine.source_path(EntityKind::Location, "castle/vault").unwrap(),
            dir.path().join("castle/vault/index.md")
        );
        let thing = engine.thing(EntityKind::Location, "castle/vault").unwrap();
        assert_eq!(thing.starting_state.as_deref(), Some("closed"));
        assert_eq!(thing.states.keys().collect::<Vec<_>>(), ["closed", "open"]);

        let open = engine.version(EntityKind::Location, "castle/vault", Some("open"), None).unwrap();
        assert_eq!(location(&open).description, "The door stands open.");
        assert_eq!(location(&open).connections["down"], "castle/hall");
        assert!(open.merged.source_of("connections.north").unwrap().ends_with("castle/vault/index.md"));
        assert!(open.merged.source_of("connections.down").unwrap().ends_with("castle/vault/open.md"));
        assert!(engine.version(EntityKind::Location, "castle/vault", Some("ajar"), None).is_err());

        // The overlay applies over whichever state is current.
        let later = engine.version(EntityKind::Location, "castle/vault", Some("open"), Some("after-the-king")).unwrap();
        assert_eq!(location(&later).description, "The door hangs off one hinge.");
        assert_eq!(location(&later).connections["down"], "castle/hall");

        let problems = engine.current_problems(false);
        assert!(problems.is_empty(), "{:?}", problems);
    }

    #[test]
    fn layers_stack_by_lineage_only() {
        let (_dir, engine) = castle();
        let courtyard = |chapter| engine.version(EntityKind::Location, "castle/courtyard", None, chapter).unwrap();

        // The castle's overlay comes before the courtyard's own index, so
        // it only adds what the room doesn't say; the room's overlay wins.
        let mourning = courtyard(Some("after-the-king"));
        let l = location(&mourning);
        assert_eq!(l.name, "Courtyard in mourning");
        assert!(l.coordinates.is_some());
        assert_eq!(l.description, "Sunlit."); // a comment-only overlay keeps the prose

        // A sub-chapter applies its parent first.
        let funeral = courtyard(Some("the-funeral"));
        assert_eq!(location(&funeral).name, "Courtyard in mourning");
        assert_eq!(location(&funeral).description, "A procession passes.");

        // A sibling chapter applies nothing from the others.
        let summons = courtyard(Some("the-summons"));
        assert_eq!(location(&summons).name, "Courtyard");
        assert!(location(&summons).coordinates.is_none());

        // Things outside the castle never see its overlay.
        assert!(engine.get_location("castle/hall").unwrap().coordinates.is_none());
    }

    #[test]
    fn overlays_warn_that_they_apply_by_placement() {
        let (_dir, engine) = castle();
        let warnings = engine.current_warnings();
        assert!(warnings.iter().any(|w| w.contains("castle/vault/chapters/after-the-king.md: applies to castle/vault/ in chapter `after-the-king`")), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("castle/chapters/after-the-king.md: applies to everything in castle/ in chapter `after-the-king`")));
    }

    #[test]
    fn misplaced_and_malformed_layers_are_problems() {
        let (dir, mut engine) = castle();
        let d = dir.path();
        let cases = [
            ("castle/vault/ajar.md", "---\nid: x\ninherits: index\n---\n", "a state file has no `id`"),
            ("castle/vault/stuck.md", "---\ntitle: Stuck\n---\n", "must say `inherits: index`"),
            ("castle/vault/odd.md", "---\ninherits: open\n---\n", "a state can only inherit from `index`"),
            ("castle/vault/late.md", "---\ninherits: index\nstate: open\n---\n", "only index.md may set `state:`"),
            ("castle/vault/inner/index.md", "---\nid: inner\ntitle: Inner\ntype: location\n---\n", "a bundle is a leaf"),
            ("castle/vault/chapters.md", "---\ninherits: index\n---\n", "reserved name"),
            ("castle/vault/chapters/20-after-the-king.md", "---\n---\n", "hiding the one in"),
            ("castle/hall/chapters/no-such-chapter.md", "---\n---\n", "no chapter `no-such-chapter`"),
            ("chapters/prologue.md", "---\n---\n", "starts with a number"),
            ("chapters/30-war/31-siege.md", "---\n---\n", "needs an _index.md to be the chapter `war`"),
            ("castle/gate/chapters/20-x/y.md", "---\n---\n", "numbered all the way down"),
        ];
        for (rel, text, expected) in cases {
            engine.set_file(&d.join(rel), text);
            let all: Vec<String> = engine.current_problems(false).into_iter().chain(engine.current_warnings()).collect();
            assert!(all.iter().any(|p| p.contains(rel) && p.contains(expected)), "{}: {:?}", rel, all);
            engine.remove_file(&d.join(rel));
        }
        engine.set_file(&d.join("castle/vault/index.md"), "---\nid: castle/vault\ntitle: Vault\ntype: location\nstate: ajar\n---\n");
        assert!(engine.current_problems(false).iter().any(|p| p.contains("`state: ajar` names a starting state, but no state file")));
    }

    #[test]
    fn replaced_lists_removed_keys_and_folder_mismatches_warn() {
        let (dir, mut engine) = castle();
        let d = dir.path();
        engine.set_file(&d.join("castle/vault/index.md"), "---\nid: castle/strongroom\ntitle: Vault\ntype: location\nstate: closed\nobjects: [chest]\nconnections:\n  north: castle/hall\n---\n");
        engine.set_file(&d.join("castle/vault/open.md"), "---\ninherits: index\nobjects: [chest, lamp]\nconnections:\n  north:\n---\n");
        let warnings = engine.current_warnings();
        assert!(warnings.iter().any(|w| w.contains("open.md: `objects` replaces the whole list inherited from")), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("open.md: `connections.north` is empty, so it removes the value inherited")));
        assert!(warnings.iter().any(|w| w.contains("id `castle/strongroom` doesn't match its folder castle/vault/")));
        assert!(engine.current_problems(false).is_empty(), "{:?}", engine.current_problems(false));
    }

    #[test]
    fn a_version_that_does_not_type_is_a_problem_naming_state_and_chapter() {
        let (dir, mut engine) = castle();
        engine.set_file(&dir.path().join("castle/vault/open.md"), "---\ninherits: index\ntitle:\n---\n");
        let problems = engine.current_problems(false);
        assert!(problems.iter().any(|p| p.contains("castle/vault/index.md (in state `open`)") && p.contains("`title`")), "{:?}", problems);
        // The starting version still loads.
        assert!(engine.get_location("castle/vault").is_some());
    }

    #[test]
    fn chapters_defined_in_an_area_are_seen_only_inside_it() {
        let (dir, mut engine) = castle();
        let d = dir.path();
        engine.set_file(&d.join("castle/chapters/25-the-vault-opens.md"), "---\ntitle: The vault opens\n---\n");
        engine.set_file(&d.join("castle/vault/chapters/the-vault-opens.md"), "---\n---\nGold everywhere.\n");
        engine.set_file(&d.join("forest/path.md"), "---\nid: forest/path\ntitle: Path\ntype: location\n---\n");
        assert!(engine.current_problems(false).is_empty(), "{:?}", engine.current_problems(false));
        let opens = engine.version(EntityKind::Location, "castle/vault", None, Some("the-vault-opens")).unwrap();
        assert_eq!(location(&opens).description, "Gold everywhere.");
        assert!(engine.version(EntityKind::Location, "forest/path", None, Some("the-vault-opens")).is_err());
        assert!(engine.version(EntityKind::Location, "forest/path", None, Some("after-the-king")).is_ok());

        // An overlay outside the area can't name it.
        engine.set_file(&d.join("forest/chapters/the-vault-opens.md"), "---\n---\n");
        assert!(engine.current_problems(false).iter().any(|p| p.contains("forest/chapters/the-vault-opens.md: no chapter")));
    }

    #[test]
    fn world_game_and_templates_are_kept_out_of_the_world() {
        let (dir, mut engine) = castle();
        let d = dir.path();
        let card = "---\nid: book\ntitle: Book\ntype: object\nobject_type: tool\n---\n";
        engine.set_file(&d.join("world/intro.md"), "Welcome, no frontmatter needed yet.\n");
        engine.set_file(&d.join("game/start.md"), "anything\n");
        engine.set_file(&d.join("objects/templates/book.md"), card);
        engine.set_file(&d.join("objects/book-card.md"), "---\nid: book\ntitle: Book\ntype: object\nobject_type: tool\ntemplate: true\n---\n");
        assert!(engine.objects.get("book").is_none());
        let problems = engine.current_problems(false);
        assert!(problems.is_empty(), "{:?}", problems);
    }

    #[test]
    fn set_file_on_a_state_or_a_new_index_changes_the_thing() {
        let (dir, mut engine) = castle();
        let d = dir.path();
        engine.set_file(&d.join("castle/vault/closed.md"), "---\ninherits: index\n---\nBarred.\n");
        assert_eq!(engine.get_location("castle/vault").unwrap().description, "Barred.");

        // A folder becomes a bundle when it gets an index.md.
        engine.set_file(&d.join("objects/bread/stale.md"), "---\ninherits: index\nvars: { tasty: no }\n---\nHard.\n");
        assert!(!engine.current_problems(false).is_empty()); // a lone state file is a broken one-file thing
        engine.set_file(&d.join("objects/bread/index.md"), "---\nid: bread\ntitle: Bread\ntype: object\nobject_type: food\n---\nFresh.\n");
        assert!(engine.current_problems(false).is_empty(), "{:?}", engine.current_problems(false));
        assert_eq!(engine.objects["bread"].description, "Fresh."); // no `state:`, so the index alone
        let stale = engine.version(EntityKind::Object, "bread", Some("stale"), None).unwrap();
        assert_eq!(stale.merged.metadata["vars"]["tasty"], serde_yaml::Value::from("no"));
    }
}
