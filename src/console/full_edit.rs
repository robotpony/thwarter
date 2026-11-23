//! Full editing: `/edit room`, `/edit object` and `/edit character`
//! (REQ-CON-6). This works out which file to open and saves what comes
//! back; `editor.rs` is the full-screen editor itself.
//!
//! The editor opens the file as it is, frontmatter and body. A save goes
//! through the writer as one whole-file replacement, which must parse and,
//! for a file that parsed before, still hold the same thing, and is then reported like any other change
//! (REQ-CON-5). A file that doesn't parse opens too, since editing is how
//! an author fixes it. Scenery, an object a room names with no card, opens as a
//! new card with `object_type: scenery`, written on the first save
//! (REQ-CF-8).

use super::author::{bare_id_problem, entity_kind, title_or_default};
use super::editor::{Closed, Document, Files, Saved};
use super::session::{Console, Place};
use super::world::Kind;
use super::Mode;
use crate::engines::EntityKind;
use crate::writer::{ContentFile, Edit, NewEntity};
use std::fs;
use std::path::PathBuf;
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
}

impl EditTarget {
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
}

/// The editor's view of a `/edit` target's files: saves lock the console
/// just while they write, and checks for changes on disk don't lock it.
pub struct EditFiles<'a> {
    pub console: &'a Mutex<Console>,
    pub target: &'a mut EditTarget,
}

impl Files for EditFiles<'_> {
    fn save(&mut self, sections: &[String], force: bool) -> Saved {
        super::lock(self.console).save_full_edit(self.target, &sections[0], force)
    }
    fn changed_on_disk(&mut self) -> Option<Vec<String>> {
        self.target.changed_on_disk().map(|text| vec![text])
    }
}

impl Console {
    /// `/edit <kind> [id]`. From play mode, switches to edit mode at the
    /// player's room first (REQ-CON-2). On a terminal, the editor opens
    /// once this returns; in a script, there's no screen to open it on.
    pub(super) fn full_edit(&mut self, args: &[&str]) -> String {
        let Some((kind_word, rest)) = args.split_first() else {
            return "Usage: /edit room|object|character [id]".to_string();
        };
        let kind = match *kind_word {
            "room" => Kind::Room,
            "object" => Kind::Object,
            "character" => Kind::Character,
            other => return format!("`/edit {}`: the kinds are room, object and character.", other),
        };
        let mut out = Vec::new();
        if self.mode == Mode::Play {
            self.mode = Mode::Edit;
            self.place_at_room();
            out.push(format!("Edit mode, at {}.", self.pwd()));
        }
        let target = match rest {
            [] => self.edit_target_here(kind),
            [id] => self.edit_target_named(kind, id),
            _ => Err(format!("Usage: /edit {} [id]", kind.label())),
        };
        match target {
            Err(why) => out.push(why),
            Ok(target) if !self.interactive => out.push(format!(
                "/edit {} opens {}.md in a full-screen editor, which needs a terminal.",
                kind.label(),
                target.path
            )),
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
        let mut heading = format!("/edit {}  {}.md", target.kind.label(), target.path);
        if target.opened.is_none() {
            heading.push_str(" (new card)");
        } else if target.kind == Kind::Broken {
            heading = format!("/edit  {}.md (doesn't parse)", target.path);
        } else if target.kind == Kind::World {
            heading = format!("/edit  {}.md (world settings)", target.path);
        }
        let text = target.opened.clone().unwrap_or_else(|| scenery_card(&target.id));
        Document::single(heading, text)
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
                        return Saved::No(format!(
                            "{}.md is gone from disk. Copy anything you want to keep, esc, then /reload.",
                            target.path
                        ))
                    }
                    Ok(now) if now != *opened && !force => {
                        return Saved::Conflict(format!(
                            "{}.md changed on disk since you opened it. Ctrl-s again overwrites that change; esc keeps it and drops yours.",
                            target.path
                        ))
                    }
                    Ok(_) => {}
                }
                if let Err(why) = same_thing(target, text) {
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
                    format!("Saved {}.md; it parses now, as {} `{}`.", target.path, target.kind.label(), target.id)
                } else {
                    format!("Saved {}.md.", target.path)
                };
                Saved::Yes(self.after_writes(&[path], summary))
            }
            Err(why) => Saved::No(why),
        }
    }

    /// What the console prints once the editor closes.
    pub(super) fn edit_closed(&self, target: &EditTarget, closed: Closed) -> String {
        if closed.reports.is_empty() {
            return format!("Closed {}.md without saving.", target.path);
        }
        closed.reports.join("\n")
    }

    /// `/edit <kind>` with no id: the file edit mode is in.
    fn edit_target_here(&self, kind: Kind) -> Result<EditTarget, String> {
        let label = kind.label();
        let Place::File(path) = &self.place else {
            return Err(format!("/edit {} edits the {} you're in. Go to one, or /edit {} <id>.", label, label, label));
        };
        let entry = self.world.files.get(path).ok_or_else(|| format!("{} is not in the world.", path))?;
        match entry.kind {
            // Any kind opens a file that doesn't parse: what it holds is
            // what the author is fixing.
            Kind::Broken => self.open_target(Kind::Broken, entry.id.clone(), path.clone()),
            // World settings aren't a room, character or object; any kind
            // word opens them.
            Kind::World => self.open_target(Kind::World, entry.id.clone(), path.clone()),
            found if !found.is_thing() => Err(format!(
                "{}.md is a {}. Editing those comes with bundle editing; look shows it for now.",
                path,
                found.label()
            )),
            found if found == kind => self.open_target(kind, entry.id.clone(), path.clone()),
            found => Err(format!("{} is a {}, not a {}: /edit {} opens it.", path, found.label(), label, found.label())),
        }
    }

    /// `/edit <kind> <id>`: a room by id (relative to this folder, as `cd`
    /// takes it), or a character or object by its bare id.
    fn edit_target_named(&self, kind: Kind, typed: &str) -> Result<EditTarget, String> {
        if kind == Kind::Room {
            let id = self.room_id(typed);
            let path = self.world.path_of(Kind::Room, &id).ok_or_else(|| format!("No room `{}`. /new room makes one.", id))?;
            return self.open_target(kind, id, path);
        }
        if let Some(problem) = bare_id_problem(typed) {
            return Err(problem);
        }
        if let Some(path) = self.world.path_of(kind, typed) {
            return self.open_target(kind, typed.to_string(), path);
        }
        if kind == Kind::Object && self.is_scenery_here(typed) {
            let relative = crate::writer::new_entity_relative_path(EntityKind::Object, typed);
            let path = relative.to_string_lossy().trim_end_matches(".md").replace('\\', "/");
            let file = self.world.root.join(&relative);
            return Ok(EditTarget { kind, id: typed.to_string(), path, opened: None, file });
        }
        Err(match kind {
            Kind::Character => format!("No character `{}`. /new character {} makes one.", typed, typed),
            _ => format!("No object `{}` here or in the world. /new object {} <type> makes one.", typed, typed),
        })
    }

    /// A target for an existing file, with its text as it is now.
    fn open_target(&self, kind: Kind, id: String, path: String) -> Result<EditTarget, String> {
        let file = self.world.root.join(format!("{}.md", path));
        let text = fs::read_to_string(&file).map_err(|e| format!("Can't open {}.md: {}", path, e))?;
        Ok(EditTarget { kind, id, path, opened: Some(text), file })
    }
}

/// Refuses text that no longer holds the thing the editor opened: ids
/// can't be renamed yet, and a room can't turn into an object.
fn same_thing(target: &EditTarget, text: &str) -> Result<(), String> {
    let Some(kind) = entity_kind(target.kind) else { return Ok(()) };
    let file = ContentFile::from_text(format!("{}.md", target.path), text).map_err(|e| format!("Not saved: {:#}", e))?;
    if file.type_name() != kind.type_name() || file.id() != target.id {
        return Err(format!(
            "Not saved: this file holds {} `{}`, and the text now says {} `{}`. Ids and types can't be changed here yet.",
            kind.type_name(),
            target.id,
            file.type_name(),
            file.id()
        ));
    }
    Ok(())
}

/// The card scenery starts with (REQ-CF-8), as `/new object <id>` writes it.
fn scenery_card(id: &str) -> String {
    format!("---\nid: {}\ntitle: {}\ntype: object\nobject_type: scenery\n---\n", id, title_or_default(&[], id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// A copy of the transcript fixture world, safe to write to.
    fn tiny_copy() -> (tempfile::TempDir, PathBuf) {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/transcripts/worlds/tiny");
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
        assert!(document.sections[0].text.contains("object_type: scenery"));
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
        assert_eq!(console.handle("/edit character").0, "village/square is a room, not a character: /edit room opens it.");
        console.handle("go /");
        assert_eq!(console.handle("/edit room").0, "/edit room edits the room you're in. Go to one, or /edit room <id>.");
        assert_eq!(console.handle("/edit object rope").0, "No object `rope` here or in the world. /new object rope <type> makes one.");
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
    fn named_things_open_from_anywhere() {
        let (_dir, root) = tiny_copy();
        let mut console = console_at(&root, "/");
        assert_eq!(open(&mut console, "/edit character elder").path, "npcs/elder");
        assert_eq!(open(&mut console, "/edit room village/well").path, "village/well");
    }
}
