//! TO WRITE (REQ-CON-57 to 59, Bruce, 2026-11-24): each name the world
//! mentions but doesn't define, once, from the engine's `not_written()`.
//! Ctrl-1 to Ctrl-9 (or Alt) and `/write <name>` make one with defaults and
//! open it in the editor.

use super::full_edit::scenery_card;
use super::panels::Row;
use super::session::{suggest, Console};
use super::world::Kind;
use crate::engines::{EntityKind, Holder};
use crate::writer::{Edit, NewEntity};

/// The key label for entry `n` (1-based), as the sidebar shows it.
pub fn key_label(n: usize) -> String {
    if cfg!(target_os = "macos") { format!("⌃{}", n) } else { format!("ctrl-{}", n) }
}

impl Console {
    /// The names to write, in first-mention order.
    pub(super) fn to_write(&self) -> Vec<crate::engines::Unwritten> {
        self.world.engine.not_written()
    }

    /// TO WRITE rows for the sidebar, or nothing when all's written.
    pub(super) fn to_write_panel(&self) -> Vec<Row> {
        let list = self.to_write();
        if list.is_empty() {
            return Vec::new();
        }
        let mut rows = vec![Row::Heading("TO WRITE".to_string())];
        for (i, entry) in list.iter().enumerate() {
            let key = if i < 9 { format!("{:<5}", key_label(i + 1)) } else { "     ".to_string() };
            let count = if entry.mentions.len() > 1 { format!("  ×{}", entry.mentions.len()) } else { String::new() };
            let room = if entry.wants == EntityKind::Location { "  room" } else { "" };
            rows.push(Row::Text(format!("  {} {}{}{}", key, entry.name, room, count)));
        }
        rows
    }

    /// The name for key `n` (1-based), if there's an entry there.
    pub(super) fn to_write_at(&self, n: usize) -> Option<String> {
        self.to_write().get(n.checked_sub(1)?).map(|e| e.name.clone())
    }

    /// `/write [<name>]`: makes it with defaults and opens it.
    pub(super) fn write_command(&mut self, args: &[&str]) -> String {
        let list = self.to_write();
        let typed = args.join(" ");
        if typed.trim().is_empty() {
            if list.is_empty() {
                return self.t("nothing_to_write", &[]);
            }
            let lines: Vec<String> = list.iter().map(|e| format!("/write {}", e.name)).collect();
            let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
            return suggest(&self.t("not_written_yet", &[]), &lines);
        }
        let Some(entry) = list.iter().find(|e| e.name == typed || e.name.eq_ignore_ascii_case(&typed)) else {
            return self.t("not_on_to_write", &[("name", &typed)]);
        };
        // The link resolves by exact id or file name, so the new thing
        // takes the name as written (the coder's 999e382).
        let id = entry.name.clone();
        let title = super::author::title_or_default(&[], &id);
        let mut written = Vec::new();
        let (kind, made) = if entry.wants == EntityKind::Location {
            let new = NewEntity { kind: EntityKind::Location, id: id.clone(), title, object_type: None };
            (Kind::Room, self.create(&new, &[]))
        } else {
            let new = NewEntity { kind: EntityKind::Object, id: id.clone(), title: title.clone(), object_type: Some("scenery".to_string()) };
            (Kind::Object, self.create(&new, &[Edit::ReplaceAll(scenery_card(&id, &title))]))
        };
        match made {
            Ok(path) => written.push(path),
            Err(e) => return e,
        }
        let mut summary = self.t("wrote", &[("id", &id), ("kind", kind.label())]);
        // A thing goes in the first room, thing or person whose file names it.
        if kind == Kind::Object {
            if let Some(holder) = entry.mentions.iter().find_map(|m| m.holder.clone()) {
                let to = match &holder {
                    Holder::Room(room) => format!("/{}", room),
                    Holder::Character(id) | Holder::Object(id) => id.clone(),
                };
                self.world.refresh_index();
                let interactive = std::mem::replace(&mut self.interactive, false);
                let placed = self.add_thing(&[&id, &to]);
                self.interactive = interactive;
                self.added = None;
                summary = format!("{}\n{}", summary, placed);
            }
        }
        let out = self.after_writes(&written, summary);
        self.world.refresh_index();
        if self.interactive {
            if let Ok(target) = self.edit_target_named(kind, &id) {
                self.pending_edit = Some(target);
            }
        }
        out
    }
}
