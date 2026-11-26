//! Full editing: `/edit room`, `/edit object` and `/edit character`
//! (REQ-CON-6). This works out which file to open and saves what comes
//! back; `editor.rs` is the full-screen editor itself.
//!
//! The editor opens the file as it is, frontmatter and body. A save goes
//! through the writer as one whole-file replacement, which must parse and,
//! for a file that parsed before, still hold the same thing, and is then reported like any other change
//! (REQ-CON-5). A file that doesn't parse opens too, since editing is how
//! an author fixes it. Scenery, an object a room names with no card, opens as a
//! new card that inherits the built-in `scenery` template, written on the first save
//! (REQ-CF-8).

use super::author::{bare_id_problem, entity_kind, title_or_default};
use super::editor::{Closed, Document, Files, Saved};
use super::theme::Lookup;
use super::session::{Console, Place};
use super::world::Kind;
use super::Mode;
use crate::engines::EntityKind;
use crate::writer::{ContentFile, Edit, NewEntity};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The file a `/edit` opened, held while the editor is open.
#[derive(Debug, Clone)]
pub struct EditTarget {
    kind: Kind,
    id: String,
    /// Relative to the world root, without `.md`, as `World::files` keys it.
    path: String,
    /// The file's text when the editor opened it, or as last saved. `None`
    /// for scenery whose card isn't written yet.
    opened: Option<String>,
    /// The file on disk.
    file: PathBuf,
    /// A bundle's state files, which the editor shows as `## When:`
    /// sections under the index (`design/STATES.md` › Console).
    states: Vec<StateFile>,
}

/// One state file of a bundle being edited.
#[derive(Debug, Clone)]
pub(super) struct StateFile {
    name: String,
    /// Relative to the world root.
    rel: PathBuf,
    /// Its text when the editor opened it, or as last saved.
    opened: String,
}

impl EditTarget {
    /// What's being edited, as (kind, id, path in `World::files`).
    pub(super) fn thing(&self) -> (Kind, &str, &str) {
        (self.kind, &self.id, &self.path)
    }

    /// The file's text if it changed on disk since it was opened, saved or
    /// last reported, which then counts as what the editor opened.
    fn changed_on_disk(&mut self) -> Option<String> {
        let opened = self.opened.as_ref()?;
        let now = fs::read_to_string(&self.file).ok()?;
        if now == *opened {
            return None;
        }
        self.opened = Some(now.clone());
        Some(now)
    }

    /// Every section's text, if any of the bundle's files changed on disk.
    fn bundle_changed_on_disk(&mut self, root: &Path) -> Option<Vec<String>> {
        let index = self.changed_on_disk();
        let mut changed = index.is_some();
        for state in &mut self.states {
            if let Ok(now) = fs::read_to_string(root.join(&state.rel)) {
                if now != state.opened {
                    state.opened = now;
                    changed = true;
                }
            }
        }
        if !changed {
            return None;
        }
        let mut out = vec![self.opened.clone().unwrap_or_default()];
        out.extend(self.states.iter().map(|s| state_body(&s.opened)));
        Some(out)
    }
}

/// What the editor's highlighting asks of the world, for one file.
pub struct EditLookup<'a> {
    console: &'a Console,
    this: Option<(EntityKind, String)>,
}

impl Lookup for EditLookup<'_> {
    fn resolves(&self, id: &str) -> bool {
        let engine = &self.console.world.engine;
        // `@name` means any thing (SECTIONS.md, e930cb5): object, character or door.
        engine.get_location(id).is_some()
            || engine.characters.contains_key(id)
            || engine.objects.contains_key(id)
            || engine.doors.contains_key(id)
    }
    fn declared(&self, name: &str) -> bool {
        let engine = &self.console.world.engine;
        let this = self.this.as_ref().and_then(|(kind, id)| engine.thing(*kind, id));
        engine.undeclared(name, this).is_none()
    }
}

impl Console {
    /// The lookup the editor's highlighting uses for `target`.
    pub(super) fn lookup_for(&self, target: &EditTarget) -> EditLookup<'_> {
        EditLookup { console: self, this: entity_kind(target.kind).map(|kind| (kind, target.id.clone())) }
    }
}

/// The editor's view of a `/edit` target's files: saves lock the console
/// just while they write, and checks for changes on disk don't lock it.
pub struct EditFiles<'a> {
    pub console: &'a Mutex<Console>,
    pub target: &'a mut EditTarget,
}

impl Files for EditFiles<'_> {
    fn save(&mut self, sections: &[String], force: bool) -> Saved {
        let mut console = super::lock(self.console);
        if self.target.states.is_empty() {
            console.save_full_edit(self.target, &sections[0], force)
        } else {
            console.save_bundle_edit(self.target, sections, force)
        }
    }
    fn changed_on_disk(&mut self) -> Option<Vec<String>> {
        if self.target.states.is_empty() {
            return self.target.changed_on_disk().map(|text| vec![text]);
        }
        let root = super::lock(self.console).world.root.clone();
        self.target.bundle_changed_on_disk(&root)
    }
}

impl Console {
    /// `/edit <kind> [id]`. From play mode, switches to edit mode at the
    /// player's room first (REQ-CON-2). On a terminal, the editor opens
    /// once this returns; in a script, there's no screen to open it on.
    /// `/edit` alone opens where edit mode is, whatever it is (REQ-CON-2).
    pub(super) fn full_edit(&mut self, args: &[&str]) -> String {
        let (kind, rest) = match args.split_first() {
            None => (None, &[][..]),
            Some((word, rest)) => match *word {
                "room" => (Some(Kind::Room), rest),
                "object" => (Some(Kind::Object), rest),
                "character" => (Some(Kind::Character), rest),
                // `/edit world`: the world's own settings (Bruce, 2026-11-24).
                "world" if rest.is_empty() => {
                    return match self.world.files.get("world/index") {
                        Some(_) => match self.open_target(Kind::World, "world/index".to_string(), "world/index".to_string()) {
                            Ok(target) if self.interactive => {
                                self.pending_edit = Some(target);
                                String::new()
                            }
                            Ok(target) => self.t("edit_needs_terminal_for", &[("path", &target.path)]),
                            Err(e) => e,
                        },
                        None => self.offer_world_index(),
                    }
                }
                // Anything else names what to open (Bruce, 2026-11-23).
                _ => (None, args),
            },
        };
        let mut out = Vec::new();
        if self.mode == Mode::Play {
            self.mode = Mode::Edit;
            self.place_at_room();
            out.push(self.t("edit_mode_at", &[("place", &self.pwd())]));
        }
        let target = match (kind, rest) {
            (_, []) => self.edit_target_here(kind),
            (Some(kind), [id]) => self.edit_target_named(kind, &super::author::id_from(id).0),
            (Some(kind), _) => Err(super::session::suggest(&self.t("usage", &[]), &[&format!("/edit {} [{{id}}]", kind.label())])),
            (None, words) => self.edit_target_any(&words.join(" ")),
        };
        match target {
            Err(why) => out.push(why),
            Ok(target) if !self.interactive => {
                out.push(self.t("edit_needs_terminal", &[("path", &target.path)]))
            }
            Ok(target) => self.pending_edit = Some(target),
        }
        out.join("\n")
    }

    /// The editor a `/edit` asked for, if any. The terminal loop opens it.
    pub(super) fn take_pending_edit(&mut self) -> Option<EditTarget> {
        self.pending_edit.take()
    }

    /// What the editor opens for `target`.
    pub(super) fn edit_document(&self, target: &EditTarget) -> Document {
        let key = if target.opened.is_none() {
            "edit_heading_new_card"
        } else if target.kind == Kind::Broken {
            "edit_heading_broken"
        } else if target.kind == Kind::World {
            "edit_heading_world"
        } else {
            "edit_heading"
        };
        let heading = self.t(key, &[("kind", target.kind.label()), ("path", &target.path)]);
        let text = target.opened.clone().unwrap_or_else(|| scenery_card(&target.id, &title_or_default(&[], &target.id)));
        let mut document = Document::single(heading, text);
        for state in &target.states {
            document.sections.push(super::editor::Section { heading: Some(state_heading(&state.name)), text: state_body(&state.opened) });
        }
        document
    }

    /// Saves a bundle's document: the index section to `index.md` (with
    /// any `## When:` sections typed under a state moved to it), and each
    /// state section to its file, all through the writer's one bundle
    /// save, which checks every file and the merged thing before writing
    /// any (`design/STATES.md` › Console).
    pub(super) fn save_bundle_edit(&mut self, target: &mut EditTarget, sections: &[String], force: bool) -> Saved {
        let Some(entity) = entity_kind(target.kind) else { return Saved::No(self.t("edit_file_gone", &[("path", &target.path)])) };
        let mut index = sections[0].clone();
        let mut bodies = Vec::new();
        for body in &sections[1..] {
            let (own, typed) = split_typed_sections(body);
            bodies.push(own);
            if !typed.is_empty() {
                if !index.ends_with("\n\n") {
                    index.push('\n');
                }
                index.push_str(&typed);
            }
        }
        if let Err(why) = same_thing(self, target, &index) {
            return Saved::No(why);
        }
        let mut files = vec![crate::writer::Section {
            path: PathBuf::from(format!("{}.md", target.path)),
            text: index.clone(),
            expected: if force { None } else { target.opened.clone() },
        }];
        for (state, body) in target.states.iter().zip(&bodies) {
            files.push(crate::writer::Section {
                path: state.rel.clone(),
                text: state_file(body),
                expected: if force { None } else { Some(state.opened.clone()) },
            });
        }
        // A file changed on disk since it opened: a second save overwrites.
        let changed = files.iter().any(|f| f.expected.as_ref().is_some_and(|was| fs::read_to_string(self.world.root.join(&f.path)).ok().as_ref() != Some(was)));
        if changed {
            return Saved::Conflict(self.t("edit_changed_on_disk", &[("path", &target.path)]));
        }
        let writer = match crate::writer::WorldWriter::new(&self.world.root) {
            Ok(writer) => writer,
            Err(e) => return Saved::No(self.t("cant_write_world", &[("reason", &e.to_string())])),
        };
        if self.warnings_before.is_none() {
            self.warnings_before = Some(self.world.warnings());
        }
        match writer.save_bundle_loaded(&mut self.world.engine, entity, &target.id, &files, false) {
            Ok(_) => {
                let root = self.world.root.clone();
                target.opened = fs::read_to_string(&target.file).ok().or(Some(index));
                for (state, file) in target.states.iter_mut().zip(&files[1..]) {
                    state.opened = fs::read_to_string(root.join(&state.rel)).unwrap_or_else(|_| file.text.clone());
                }
                let written: Vec<PathBuf> = files.iter().map(|f| root.join(&f.path)).collect();
                Saved::Yes(self.after_writes(&written, self.t("edit_saved", &[("path", &target.path)])))
            }
            Err(e) => Saved::No(self.t("edit_not_saved", &[("reason", &format!("{:#}", e))])),
        }
    }

    /// Saves the editor's text for `target`. Refuses text that doesn't
    /// parse as the same thing, and a file changed on disk since it was
    /// opened unless `force`.
    pub(super) fn save_full_edit(&mut self, target: &mut EditTarget, text: &str, force: bool) -> Saved {
        let file = target.file.clone();
        let written = match &target.opened {
            Some(opened) => {
                match fs::read_to_string(&file) {
                    Err(_) => {
                        return Saved::No(self.t("edit_file_gone", &[("path", &target.path)]))
                    }
                    Ok(now) if now != *opened && !force => {
                        return Saved::Conflict(self.t("edit_changed_on_disk", &[("path", &target.path)]))
                    }
                    Ok(_) => {}
                }
                if let Err(why) = same_thing(self, target, text) {
                    return Saved::No(why);
                }
                // The writer checks the file again as it writes, so a change
                // on disk between that read and this write is refused too.
                let expected = (!force).then_some(opened.as_str());
                self.replace_file(&target.path, text, expected)
            }
            None => {
                let new = NewEntity {
                    kind: EntityKind::Object,
                    id: target.id.clone(),
                    title: title_or_default(&[], &target.id),
                    object_type: Some("scenery".to_string()),
                };
                self.create(&new, &[Edit::ReplaceAll(text.to_string())])
            }
        };
        match written {
            Ok(path) => {
                target.opened = Some(fs::read_to_string(&file).unwrap_or_else(|_| text.to_string()));
                let was_broken = target.kind == Kind::Broken;
                self.world.refresh_index();
                // A file that didn't parse is now known: later saves check
                // it still holds the same thing.
                if let Some(entry) = self.world.files.get(&target.path) {
                    target.kind = entry.kind;
                    target.id = entry.id.clone();
                }
                let summary = if was_broken && target.kind != Kind::Broken {
                    self.t("edit_saved_parses", &[("path", &target.path), ("kind", target.kind.label()), ("id", &target.id)])
                } else {
                    self.t("edit_saved", &[("path", &target.path)])
                };
                Saved::Yes(self.after_writes(&[path], summary))
            }
            Err(why) => Saved::No(why),
        }
    }

    /// What the console prints once the editor closes.
    pub(super) fn edit_closed(&self, target: &EditTarget, closed: Closed) -> String {
        if closed.reports.is_empty() {
            return self.t("edit_closed_unsaved", &[("path", &target.path)]);
        }
        closed.reports.join("\n")
    }

    /// `/edit <kind>` with no id: the file edit mode is in.
    /// The file edit mode is in, if it fits `kind` (any kind for `None`).
    fn edit_target_here(&self, kind: Option<Kind>) -> Result<EditTarget, String> {
        let Place::File(path) = &self.place else {
            return Err(match kind {
                Some(kind) => {
                    let label = kind.label();
                    super::session::suggest(&self.t("edit_kind_here", &[("kind", label)]), &[&format!("/edit {} {{id}}", label)])
                }
                None if matches!(self.place, Place::Game(_)) => self.t("game_folder_not_a_file", &[]),
                None => super::session::suggest(
                    &self.t("edit_folder", &[("place", &self.pwd())]),
                    &["/edit {name}"],
                ),
            });
        };
        self.target_for_file(path, kind)
    }

    /// `/new ...`, then the editor on what it made (Bruce, 2026-11-24), on
    /// a terminal. Scripts and transcripts only make it.
    pub(super) fn new_and_edit(&mut self, args: &[&str]) -> String {
        self.created.clear();
        let out = self.new_entity(args);
        let made = self.created.first().cloned();
        if !self.interactive {
            return out;
        }
        if let Some((kind, id)) = made {
            let kind = match kind {
                EntityKind::Location => Kind::Room,
                EntityKind::Character => Kind::Character,
                EntityKind::Object => Kind::Object,
                EntityKind::Door => Kind::Door,
            };
            self.pending_questions = self.questions_for_new(kind, &id);
            if let Some(path) = self.world.path_of(kind, &id) {
                if let Ok(target) = self.open_target(kind, id, path) {
                    self.pending_edit = Some(target);
                }
            }
        }
        out
    }

    /// `/add ...`, then the editor on what it added (Bruce, 2026-11-24): the
    /// door for `/add door`, or the thing; scenery with no card opens as a
    /// new card. On a terminal only, and not after a refusal.
    pub(super) fn add_and_edit(&mut self, args: &[&str]) -> String {
        self.added = None;
        self.created.clear();
        let knew: std::collections::BTreeSet<String> =
            self.world.files.values().filter(|e| e.kind.is_thing()).map(|e| e.id.clone()).collect();
        let out = self.add_thing(args);
        // Questions only for something new, not for placing what exists.
        if self.interactive {
            if let Some((kind, id)) = self.added.clone() {
                if !knew.contains(&id) {
                    self.pending_questions = self.questions_for_new(kind, &id);
                }
            }
        }
        if let (true, Some((kind, id))) = (self.interactive, self.added.take()) {
            let cardless = kind == Kind::Object && self.world.path_of(Kind::Object, &id).is_none();
            let target = if cardless {
                let relative = crate::writer::new_entity_relative_path(EntityKind::Object, &id);
                let path = relative.to_string_lossy().trim_end_matches(".md").replace('\\', "/");
                Ok(EditTarget { kind, id, path, opened: None, file: self.world.root.join(&relative), states: Vec::new() })
            } else {
                self.edit_target_named(kind, &id)
            };
            if let Ok(target) = target {
                self.pending_edit = Some(target);
            }
        }
        out
    }

    /// `/edit <name>`: whatever the name means, as `go <name>` finds it,
    /// without moving edit mode there. Scenery here opens as a new card.
    fn edit_target_any(&self, typed: &str) -> Result<EditTarget, String> {
        let (id, _) = super::author::id_from(typed);
        if self.is_scenery_here(&id) {
            return self.edit_target_named(Kind::Object, &id);
        }
        match self.resolve(&id)? {
            Place::File(path) => self.target_for_file(&path, None),
            Place::Game(_) => Err(self.t("game_folder_not_a_file", &[])),
            Place::Dir(dir) => Err(format!("{}/ is a folder, with nothing to edit. /edit a room or thing in it.", dir)),
        }
    }

    /// The file at `path`, if it fits `kind` (any kind for `None`).
    fn target_for_file(&self, path: &String, kind: Option<Kind>) -> Result<EditTarget, String> {
        let entry = self.world.files.get(path).ok_or_else(|| self.t("not_in_world", &[("id", path)]))?;
        match entry.kind {
            // Any kind opens a file that doesn't parse: what it holds is
            // what the author is fixing.
            Kind::Broken => self.open_target(Kind::Broken, entry.id.clone(), path.clone()),
            // World settings aren't a room, character or object; any kind
            // word opens them.
            Kind::World => self.open_target(Kind::World, entry.id.clone(), path.clone()),
            found if !found.is_thing() => Err(self.t("edit_not_a_thing", &[("path", path), ("kind", found.label())])),
            found if kind.is_none_or(|k| k == found) => self.open_target(found, entry.id.clone(), path.clone()),
            found => {
                let label = kind.map(Kind::label).unwrap_or_default();
                Err(self.t("edit_wrong_kind", &[("path", &path), ("found", found.label()), ("kind", label)]))
            }
        }
    }

    /// `/edit <kind> <id>`: a room by id (relative to this folder, as `cd`
    /// takes it), or a character or object by its bare id.
    pub(super) fn edit_target_named(&self, kind: Kind, typed: &str) -> Result<EditTarget, String> {
        if kind == Kind::Room {
            let id = self.room_id(typed);
            let path = self.world.path_of(Kind::Room, &id).ok_or_else(|| super::session::suggest(&self.t("edit_no_room", &[("id", &id)]), &[&format!("/new room {}", id)]))?;
            return self.open_target(kind, id, path);
        }
        if let Some(problem) = bare_id_problem(&self.texts, typed) {
            return Err(problem);
        }
        if let Some(path) = self.world.path_of(kind, typed) {
            return self.open_target(kind, typed.to_string(), path);
        }
        if kind == Kind::Object && self.is_scenery_here(typed) {
            let relative = crate::writer::new_entity_relative_path(EntityKind::Object, typed);
            let path = relative.to_string_lossy().trim_end_matches(".md").replace('\\', "/");
            let file = self.world.root.join(&relative);
            return Ok(EditTarget { kind, id: typed.to_string(), path, opened: None, file, states: Vec::new() });
        }
        Err(match kind {
            Kind::Character => super::session::suggest(&self.t("edit_no_character", &[("id", typed)]), &[&format!("/new character {}", typed)]),
            _ => super::session::suggest(&self.t("edit_no_object", &[("id", typed)]), &[&format!("/new object {} {{type}}", typed)]),
        })
    }

    /// A target for an existing file, with its text as it is now.
    pub(super) fn open_target(&self, kind: Kind, id: String, path: String) -> Result<EditTarget, String> {
        let file = self.world.root.join(format!("{}.md", path));
        let text = fs::read_to_string(&file).map_err(|e| self.t("edit_cant_open", &[("path", &path), ("reason", &e.to_string())]))?;
        let states = self.state_files(kind, &id, &path);
        Ok(EditTarget { kind, id, path, opened: Some(text), file, states })
    }

    /// A bundle index's own state files, the starting state first and the
    /// rest A to Z (`design/STATES.md` › Console). None for a one-file thing.
    fn state_files(&self, kind: Kind, id: &str, path: &str) -> Vec<StateFile> {
        let Some(entity) = entity_kind(kind) else { return Vec::new() };
        if !path.ends_with("/index") {
            return Vec::new();
        }
        let Some(thing) = self.world.engine.thing(entity, id) else { return Vec::new() };
        let dir = Path::new(path).parent().map(Path::to_path_buf).unwrap_or_default();
        let mut names: Vec<String> = thing.states.keys().cloned().collect();
        names.sort_by_key(|n| (Some(n) != thing.starting_state.as_ref(), n.clone()));
        names
            .into_iter()
            .filter_map(|name| {
                let rel = dir.join(format!("{}.md", name));
                let opened = fs::read_to_string(self.world.root.join(&rel)).ok()?;
                Some(StateFile { name, rel, opened })
            })
            .collect()
    }
}

/// A state section's heading.
fn state_heading(name: &str) -> String {
    format!("## When: #{}", name)
}

/// A state file as its section shows it: its frontmatter, apart from
/// `inherits: index`, as a `thwarter` block, then its prose.
fn state_body(file: &str) -> String {
    let Some(rest) = file.strip_prefix("---\n") else { return file.to_string() };
    let Some(end) = rest.find("\n---\n").map(|i| i + 1).or_else(|| rest.starts_with("---\n").then_some(0)) else {
        return file.to_string();
    };
    let (front, prose) = (&rest[..end], &rest[end + 4..]);
    let keys: Vec<&str> = front.lines().filter(|l| l.trim() != "inherits: index").collect();
    if keys.iter().all(|l| l.trim().is_empty()) {
        return prose.to_string();
    }
    format!("```thwarter\n{}\n```\n{}", keys.join("\n"), prose)
}

/// A state section back as its file: the inverse of `state_body`.
fn state_file(body: &str) -> String {
    match body.strip_prefix("```thwarter\n").and_then(|rest| rest.split_once("\n```\n").or_else(|| rest.strip_suffix("\n```").map(|b| (b, "")))) {
        Some((keys, prose)) => format!("---\ninherits: index\n{}\n---\n{}", keys.trim_end(), prose),
        None => format!("---\ninherits: index\n---\n{}", body),
    }
}

/// Splits a state section at the first `## When:` heading typed into it:
/// what's above stays the state's, and the rest goes to the index, since a
/// typed heading never makes a state (`design/STATES.md` › Console).
fn split_typed_sections(body: &str) -> (String, String) {
    let mut at = 0;
    for line in body.split_inclusive('\n') {
        if line.starts_with("## When:") {
            return (body[..at].to_string(), body[at..].to_string());
        }
        at += line.len();
    }
    (body.to_string(), String::new())
}

/// Refuses text that no longer holds the thing the editor opened: ids
/// can't be renamed yet, and a room can't turn into an object.
fn same_thing(console: &Console, target: &EditTarget, text: &str) -> Result<(), String> {
    let Some(kind) = entity_kind(target.kind) else { return Ok(()) };
    let file = ContentFile::from_text(format!("{}.md", target.path), text).map_err(|e| console.t("edit_not_saved", &[("reason", &format!("{:#}", e))]))?;
    if file.type_name() != kind.type_name() || file.id() != target.id {
        return Err(console.t(
            "edit_not_same_thing",
            &[("kind", kind.type_name()), ("id", &target.id), ("new_kind", file.type_name()), ("new_id", file.id())],
        ));
    }
    Ok(())
}

/// The card scenery starts with (REQ-CF-8): one that inherits the built-in
/// `scenery` template (REQ-CF-21), as `/new object <id>` writes it.
pub(super) fn scenery_card(id: &str, title: &str) -> String {
    let title = crate::writer::content_file::emit_scalar(&serde_yaml::Value::String(title.to_string()), false)
        .unwrap_or_else(|_| title.to_string());
    format!("---\nid: {}\ntitle: {}\ntype: object\ninherits: scenery\n---\n", id, title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn a_state_file_shows_as_a_section_and_saves_back_as_itself() {
        let file = "---\ninherits: index\nweight: immovable\n---\nIt hangs.\n";
        assert_eq!(state_body(file), "```thwarter\nweight: immovable\n```\nIt hangs.\n");
        assert_eq!(state_file(&state_body(file)), file);
        let bare = "---\ninherits: index\n---\nLoose.\n";
        assert_eq!(state_body(bare), "Loose.\n");
        assert_eq!(state_file("Loose.\n"), bare);
        assert_eq!(split_typed_sections("Lit.\n## When: #lit #night\nGlows.\n"), ("Lit.\n".to_string(), "## When: #lit #night\nGlows.\n".to_string()));
    }

    #[test]
    fn a_bundle_edits_as_one_document_and_saves_each_file() {
        let (_dir, root) = tiny_copy();
        let mut console = Console::open(&root, super::super::Mode::Edit).unwrap();
        console.handle("go /objects/lantern");
        console.handle("/state unlit start");
        console.handle("/state lit");
        let path = console.world.path_of(Kind::Object, "lantern").unwrap();
        let mut target = console.open_target(Kind::Object, "lantern".into(), path).unwrap();
        let document = console.edit_document(&target);
        let headings: Vec<_> = document.sections.iter().map(|s| s.heading.clone()).collect();
        assert_eq!(headings, vec![None, Some("## When: #unlit".into()), Some("## When: #lit".into())], "the starting state first");
        // Each section goes back to its own file; a heading typed under a
        // state goes to the index.
        let sections = vec![document.sections[0].text.clone(), "Dark.\n".to_string(), "It glows.\n## When: #lit #night\nA beacon.\n".to_string()];
        match console.save_bundle_edit(&mut target, &sections, false) {
            Saved::Yes(_) => {}
            other => panic!("{:?}", other),
        }
        let dir = root.join("objects/lantern");
        assert_eq!(fs::read_to_string(dir.join("unlit.md")).unwrap(), "---\ninherits: index\n---\nDark.\n");
        assert_eq!(fs::read_to_string(dir.join("lit.md")).unwrap(), "---\ninherits: index\n---\nIt glows.\n");
        assert!(fs::read_to_string(dir.join("index.md")).unwrap().contains("## When: #lit #night\nA beacon."));
        // A change on disk since it opened is a conflict; force overwrites.
        fs::write(dir.join("lit.md"), "---\ninherits: index\n---\nChanged.\n").unwrap();
        assert!(matches!(console.save_bundle_edit(&mut target, &sections, false), Saved::Conflict(_)));
    }

    /// A copy of the transcript fixture world, safe to write to.
    fn tiny_copy() -> (tempfile::TempDir, PathBuf) {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tiny");
        for entry in walkdir::WalkDir::new(&fixture) {
            let entry = entry.unwrap();
            let to = root.join(entry.path().strip_prefix(&fixture).unwrap());
            if entry.file_type().is_dir() {
                fs::create_dir_all(&to).unwrap();
            } else {
                fs::copy(entry.path(), &to).unwrap();
            }
        }
        (dir, root)
    }

    fn console_at(root: &Path, place: &str) -> Console {
        let mut console = Console::open(root, Mode::Edit).unwrap();
        console.interactive = true;
        console.handle(&format!("go {}", place));
        console
    }

    fn open(console: &mut Console, command: &str) -> EditTarget {
        let (out, _) = console.handle(command);
        console.take_pending_edit().unwrap_or_else(|| panic!("{} opened nothing: {}", command, out))
    }

    #[test]
    fn the_room_opens_as_its_file_and_saves_back() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "village/square");
        let mut target = open(&mut console, "/edit room");
        let document = console.edit_document(&target);
        assert_eq!(document.heading, "/edit room  village/square.md");
        let original = fs::read_to_string(root.join("village/square.md")).unwrap();
        assert_eq!(document.sections[0].text, original);

        let edited = original.replace("title: ", "title: Old ");
        let saved = console.save_full_edit(&mut target, &edited, false);
        assert!(matches!(&saved, Saved::Yes(report) if report.starts_with("Saved village/square.md.")), "{:?}", saved);
        assert_eq!(fs::read_to_string(root.join("village/square.md")).unwrap(), edited);
        assert!(console.world.engine.get_location("village/square").unwrap().name.starts_with("Old "));
    }

    #[test]
    fn text_that_doesnt_parse_is_refused_and_nothing_is_written() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "village/square");
        let mut target = open(&mut console, "/edit room");
        let original = fs::read_to_string(root.join("village/square.md")).unwrap();
        let saved = console.save_full_edit(&mut target, "---\nid: [unclosed\n---\n", false);
        assert!(matches!(saved, Saved::No(_)), "{:?}", saved);
        assert_eq!(fs::read_to_string(root.join("village/square.md")).unwrap(), original);
    }

    #[test]
    fn a_change_on_disk_is_a_conflict_until_forced() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "village/square");
        let mut target = open(&mut console, "/edit room");
        let file = root.join("village/square.md");
        let original = fs::read_to_string(&file).unwrap();
        fs::write(&file, format!("{}\nAdded elsewhere.\n", original)).unwrap();

        let mine = original.replace("title: ", "title: Mine ");
        assert!(matches!(console.save_full_edit(&mut target, &mine, false), Saved::Conflict(_)));
        assert!(fs::read_to_string(&file).unwrap().contains("Added elsewhere."));
        assert!(matches!(console.save_full_edit(&mut target, &mine, true), Saved::Yes(_)));
        assert_eq!(fs::read_to_string(&file).unwrap(), mine);
    }

    #[test]
    fn scenery_opens_as_a_new_card_written_on_first_save() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "village/square");
        let mut target = open(&mut console, "/edit object bench");
        let document = console.edit_document(&target);
        assert_eq!(document.heading, "/edit object  objects/bench.md (new card)");
        assert!(document.sections[0].text.contains("inherits: scenery"));
        assert!(!root.join("objects/bench.md").exists());

        let text = format!("{}A worn bench.\n", document.sections[0].text);
        assert!(matches!(console.save_full_edit(&mut target, &text, false), Saved::Yes(_)));
        assert_eq!(fs::read_to_string(root.join("objects/bench.md")).unwrap(), text);
        assert_eq!(console.world.engine.objects["bench"].object_type, "scenery");
        // A second save edits the card it just wrote.
        let more = format!("{}It wobbles.\n", text);
        assert!(matches!(console.save_full_edit(&mut target, &more, false), Saved::Yes(_)));
    }

    #[test]
    fn the_wrong_kind_or_a_folder_says_what_would_work() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "village/square");
        assert_eq!(console.handle("/edit character").0, "village/square is a room, not a character: /edit opens it.");
        console.handle("go /");
        assert_eq!(console.handle("/edit room").0, "/edit room edits the room you're in. Go to one, or name it:\n\n  /edit room {id}");
        assert_eq!(console.handle("/edit object rope").0, "No object `rope` here or in the world. To make one:\n\n  /new object rope {type}");
        assert!(console.take_pending_edit().is_none());
    }

    #[test]
    fn a_file_that_doesnt_parse_opens_and_can_be_fixed() {
        let (_dir, root) = tiny_copy();
        fs::write(root.join("objects/rope.md"), "---\nid: rope\ntitle: [unclosed\n---\n").unwrap();
        let mut console = console_at(&root, "objects/rope");
        assert_eq!(console.world.blocking_problems().len(), 1);
        let mut target = open(&mut console, "/edit object");
        assert_eq!(console.edit_document(&target).heading, "/edit  objects/rope.md (doesn't parse)");

        assert!(matches!(console.save_full_edit(&mut target, "---\nid: [still\n---\n", false), Saved::No(_)));
        let fixed = "---\nid: rope\ntitle: Rope\ntype: object\nobject_type: tool\n---\n";
        let saved = console.save_full_edit(&mut target, fixed, false);
        assert!(matches!(&saved, Saved::Yes(r) if r.starts_with("Saved objects/rope.md; it parses now, as object `rope`.")), "{:?}", saved);
        assert!(console.world.blocking_problems().is_empty());
        assert_eq!(console.world.files["objects/rope"].kind, Kind::Object);
        assert!(console.world.engine.objects.contains_key("rope"));
    }

    #[test]
    fn changing_the_id_is_refused() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "village/square");
        let mut target = open(&mut console, "/edit room");
        let original = fs::read_to_string(root.join("village/square.md")).unwrap();
        let renamed = original.replace("id: village/square", "id: village/plaza");
        let saved = console.save_full_edit(&mut target, &renamed, false);
        assert!(matches!(&saved, Saved::No(why) if why.contains("can't be changed here yet")), "{:?}", saved);
        assert_eq!(fs::read_to_string(root.join("village/square.md")).unwrap(), original);
    }

    #[test]
    fn a_change_on_disk_is_reported_once_and_then_saves_over_it() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "village/well");
        let mut target = open(&mut console, "/edit room");
        assert_eq!(target.changed_on_disk(), None);
        let file = root.join("village/well.md");
        let pulled = format!("{}Pulled from git.\n", fs::read_to_string(&file).unwrap());
        fs::write(&file, &pulled).unwrap();
        assert_eq!(target.changed_on_disk().as_deref(), Some(pulled.as_str()));
        assert_eq!(target.changed_on_disk(), None, "reported once");
        // Having seen it, a save is no longer a conflict.
        let mine = format!("{}Mine.\n", pulled);
        assert!(matches!(console.save_full_edit(&mut target, &mine, false), Saved::Yes(_)));
    }

    #[test]
    fn edit_alone_opens_where_you_are() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/npcs/elder");
        assert_eq!(open(&mut console, "/edit").path, "npcs/elder");
        console.handle("go /");
        assert!(console.handle("/edit").0.starts_with("/ is a folder, with nothing to edit."));
        // From play mode: edit mode at the player's room, opened.
        console.handle("go /village/well");
        console.handle("/play here");
        let (out, _) = console.handle("/edit");
        assert_eq!(out, "Edit mode, at village/well.");
        assert_eq!(console.take_pending_edit().unwrap().path, "village/well");
    }

    #[test]
    fn moving_a_room_takes_edit_mode_and_the_game_with_it() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/village/well");
        console.handle("/play here");
        console.handle("/edit");
        console.take_pending_edit();
        let (out, _) = console.handle("/move village/well house");
        assert!(out.starts_with("Moved village/well → house/well: moved village/well.md → house/well.md"), "{}", out);
        assert!(root.join("house/well.md").exists() && !root.join("village/well.md").exists());
        assert_eq!(console.pwd(), "house/well");
        assert_eq!(console.game.as_ref().unwrap().location, "house/well");
        let square = fs::read_to_string(root.join("village/square.md")).unwrap();
        assert!(square.contains("north: house/well"), "{}", square);
        // A room standing for its own area: beside the square again.
        let (out, _) = console.handle("/move house/well village/square");
        assert!(out.starts_with("Moved house/well → village/well"), "{}", out);
    }

    #[test]
    fn edit_takes_a_name_as_well_as_a_kind() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/village/square");
        assert_eq!(open(&mut console, "/edit lantern").path, "objects/lantern");
        assert_eq!(open(&mut console, "/edit elder").path, "npcs/elder");
        assert_eq!(open(&mut console, "/edit village/well").path, "village/well");
        assert_eq!(open(&mut console, "/edit north").path, "village/well", "an exit names its room");
        assert_eq!(open(&mut console, "/edit bench").path, "objects/bench", "scenery here gets its card");
        assert_eq!(console.pwd(), "village/square", "edit mode stays where it was");
        assert!(console.handle("/edit nothing-at-all").0.starts_with("Nothing called"));
        assert!(console.handle("/edit village").0.contains("is a folder"));
    }

    #[test]
    fn a_thing_moves_into_scenery_which_gets_a_card_and_shows_it_held() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/village/square");
        let (out, _) = console.handle("/move lantern bench");
        assert_eq!(out.lines().next().unwrap(), "Moved lantern: village/square → bench.");
        let card = fs::read_to_string(root.join("objects/bench.md")).unwrap();
        assert!(card.contains("inherits: scenery") && card.contains("lantern"), "{}", card);
        let square = fs::read_to_string(root.join("village/square.md")).unwrap();
        assert!(!square.contains("lantern"), "{}", square);
        let rows: Vec<String> = console
            .sidebar()
            .rest
            .iter()
            .filter_map(|r| match r {
                crate::console::panels::Row::Text(t) | crate::console::panels::Row::Dim(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        let bench = rows.iter().position(|r| r == "  bench").unwrap_or_else(|| panic!("{:?}", rows));
        assert_eq!(rows[bench + 1], "    lantern", "held under its holder");
    }

    #[test]
    fn new_opens_the_editor_on_what_it_made() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/village/square");
        console.handle("/new room south village/garden");
        assert_eq!(console.take_pending_edit().map(|t| t.path), Some("village/garden".to_string()));
        console.handle("/new object rope tool");
        assert_eq!(console.take_pending_edit().map(|t| t.path), Some("objects/rope".to_string()));
        console.handle("/new object rope tool");
        assert!(console.take_pending_edit().is_none(), "nothing made, nothing opened");
    }

    #[test]
    fn add_opens_the_editor_on_what_it_added() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/village/square");
        console.handle("/add door west village/hallway");
        assert_eq!(console.take_pending_edit().map(|t| t.path), Some("doors/door-village-hallway-village-square".to_string()));
        console.handle("/add chair village/well");
        let target = console.take_pending_edit().expect("scenery opens as a new card");
        assert_eq!(target.path, "objects/chair");
        console.handle("/add chair nowhere");
        assert!(console.take_pending_edit().is_none(), "a refusal opens nothing");
    }

    #[test]
    fn named_things_open_from_anywhere() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/");
        assert_eq!(open(&mut console, "/edit character elder").path, "npcs/elder");
        assert_eq!(open(&mut console, "/edit room village/well").path, "village/well");
    }
}
