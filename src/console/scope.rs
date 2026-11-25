//! Scope: where a new or moved thing's file goes (REQ-CON-26 to 29,
//! `design/STATES.md` › Scope). Global is the world's `objects/`, `npcs/`
//! and `doors/`; area is a folder for the kind inside the area edit mode
//! is in; local is that folder inside the current room's bundle.

use std::path::{Path, PathBuf};

use crate::engines::EntityKind;

use super::author::entity_kind;
use super::session::Console;

/// The scope `/new` uses until `/scope` changes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Scope {
    #[default]
    Global,
    Area,
    Local,
}

impl Scope {
    /// As typed: `here` is local, as in `/new object torch tool in here`.
    pub(super) fn from_word(word: &str) -> Option<Scope> {
        match word {
            "global" => Some(Scope::Global),
            "area" => Some(Scope::Area),
            "local" | "here" => Some(Scope::Local),
            _ => None,
        }
    }

    pub(super) fn word(self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Area => "area",
            Scope::Local => "local",
        }
    }
}

/// Whether the engine loads things from a bundle's own kind folders
/// (`castle/hall/objects/torch.md`); until it does, local scope refuses.
const LOCAL_LOADS: bool = false;

/// The folder a kind's things go in at the root, an area or a bundle.
fn kind_folder(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Character => "npcs",
        EntityKind::Door => "doors",
        _ => "objects",
    }
}

impl Console {
    /// `/scope global|area|local` (REQ-CON-26).
    pub(super) fn scope_command(&mut self, word: &str) -> String {
        match Scope::from_word(word) {
            Some(scope) => {
                self.scope = scope;
                self.t(&format!("scope_now_{}", scope.word()), &[])
            }
            None => super::session::suggest(&self.t("scope_what", &[("word", word)]), &["/scope global", "/scope area", "/scope local"]),
        }
    }

    /// The area edit mode is in: the current room's folder, or the
    /// current folder. `None` at the world's root.
    fn area_here(&self) -> Option<String> {
        let dir = match self.current_room() {
            Some(room) => room.rsplit_once('/').map(|(area, _)| area.to_string()).unwrap_or_default(),
            None => self.current_dir(),
        };
        (!dir.is_empty()).then_some(dir)
    }

    /// The folder a `kind` thing goes in for `scope`; `None` for global.
    /// Local makes the current room a bundle first if it isn't one.
    pub(super) fn scope_folder(&mut self, scope: Scope, kind: EntityKind) -> Result<Option<PathBuf>, String> {
        match scope {
            Scope::Global => Ok(None),
            Scope::Area => {
                let area = self.area_here().ok_or_else(|| self.t("scope_no_area", &[]))?;
                Ok(Some(Path::new(&area).join(kind_folder(kind))))
            }
            // The engine reads a bundle as a leaf, with only state files
            // and `chapters/` (`a bundle is a leaf`), so local things wait
            // until it also takes kind folders (asked of the coder).
            Scope::Local if !LOCAL_LOADS => Err(self.t("scope_local_waits", &[])),
            Scope::Local => {
                let room = self.current_room().ok_or_else(|| self.t("scope_local_needs_room", &[]))?;
                let bundle = match self.bundle_dir(EntityKind::Location, &room) {
                    Some(dir) => dir,
                    None => {
                        let writer = crate::writer::WorldWriter::new(&self.world.root)
                            .map_err(|e| self.t("cant_write_world", &[("reason", &e.to_string())]))?;
                        writer
                            .make_bundle_loaded(&mut self.world.engine, EntityKind::Location, &room, None, false)
                            .map_err(|e| self.t("not_made", &[("reason", &format!("{:#}", e))]))?;
                        self.follow_room_bundle(&room);
                        self.bundle_dir(EntityKind::Location, &room).ok_or_else(|| self.t("scope_local_needs_room", &[]))?
                    }
                };
                Ok(Some(bundle.join(kind_folder(kind))))
            }
        }
    }

    /// `/move <id> global|area|local` (REQ-CON-29): the thing's file, or
    /// its bundle, moves; its id and every reference stay as they are.
    pub(super) fn move_scope(&mut self, id: &str, scope: Scope) -> String {
        let Some(kind) = self.world.files.values().find(|e| e.id == id).map(|e| e.kind) else {
            return self.t("nothing_called", &[("name", id)]);
        };
        let Some(kind) = entity_kind(kind).filter(|k| *k != EntityKind::Location) else {
            return self.t("scope_move_not_a_thing", &[("id", id)]);
        };
        let folder = match self.scope_folder(scope, kind) {
            Ok(Some(folder)) => folder,
            Ok(None) => PathBuf::from(kind_folder(kind)),
            Err(message) => return message,
        };
        let root = self.world.root.clone();
        let rel = |p: &Path| p.strip_prefix(&root).unwrap_or(p).to_string_lossy().replace('\\', "/");
        let from = match self.world.engine.source_path(kind, id) {
            Ok(path) => path.to_path_buf(),
            Err(_) => return self.t("nothing_called", &[("name", id)]),
        };
        let writer = match crate::writer::WorldWriter::new(&self.world.root) {
            Ok(w) => w,
            Err(e) => return self.t("cant_write_world", &[("reason", &e.to_string())]),
        };
        self.note_scope_warnings();
        match writer.move_loaded(&mut self.world.engine, kind, id, &folder, false) {
            Ok(report) => {
                let summary = self.t("scope_moved", &[("id", id), ("from", &rel(&from)), ("to", &rel(&report.path))]);
                self.world.refresh_index();
                self.after_writes(&[from, report.path.clone()], summary)
            }
            Err(e) => self.t("scope_not_moved", &[("id", id), ("reason", &format!("{:#}", e))]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::session::Console;
    use super::super::Mode;
    use std::path::Path;

    #[test]
    fn the_prompt_shows_the_scope_only_when_it_isnt_global() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        let mut console = Console::open(&root, Mode::Edit).unwrap();
        console.handle("go /village/square");
        assert_eq!(console.prompt(), "in village/square > ");
        console.handle("/scope area");
        assert_eq!(console.prompt(), "in village/square [area] > ");
        console.handle("/scope global");
        assert_eq!(console.prompt(), "in village/square > ");
    }
}
