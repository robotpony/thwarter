//! The world's own settings file, `world/index.md`: `/start` (Bruce,
//! 2026-11-24) and making the file when a world doesn't have one yet,
//! with its title, `authors: $USER` and `start:` (REQ-CON-36).

use super::doors::Asked;
use super::session::Console;
use super::world::Kind;

const INDEX: &str = "world/index";

/// The machine user, who is the author for now (REQ-CON-34).
/// `THWART_USER` stands in for it, so transcripts read the same anywhere.
pub fn machine_user() -> String {
    std::env::var("THWART_USER")
        .or_else(|_| std::env::var("USER")).or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "author".to_string())
}

impl Console {
    /// `/start`: the room edit mode is in becomes where play starts.
    pub(super) fn start_command(&mut self) -> String {
        let Some(room) = self.current_room() else {
            return super::session::suggest("Go to a room first, then:", &["/start"]);
        };
        if self.world.files.get(INDEX).is_none() {
            return self.make_world_index(Some(room.clone()), false);
        }
        let path = self.world.root.join(format!("{}.md", INDEX));
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => return format!("Can't read world/index.md: {}", e),
        };
        let text = super::states::set_key(&text, "start", &room);
        match self.write_world_index(&text) {
            Ok(path) => self.after_writes(&[path], format!("Play starts in {} now.", room)),
            Err(e) => e,
        }
    }

    /// What to say when something needs `world/index.md` and it isn't
    /// there: an offer, which the next line answers.
    pub(super) fn offer_world_index(&mut self) -> String {
        self.asked = Some(Asked::MakeWorldIndex);
        let start = self.current_room().or_else(|| self.world.default_room()).unwrap_or_else(|| "(no room yet)".to_string());
        format!(
            "This world has no world/index.md yet. Make one, with authors: {} and start: {}? (y/n)",
            machine_user(),
            start
        )
    }

    /// Writes `world/index.md` and reloads the world. The writer edits
    /// things, not the world's own file, and a new world/ folder changes
    /// how the world loads, so this writes it and reloads.
    fn write_world_index(&mut self, text: &str) -> Result<std::path::PathBuf, String> {
        self.note_warnings_before();
        let path = self.world.root.join(format!("{}.md", INDEX));
        std::fs::create_dir_all(path.parent().unwrap_or(&self.world.root))
            .and_then(|_| std::fs::write(&path, text))
            .map_err(|e| format!("Couldn't write world/index.md: {}", e))?;
        let reloaded = self.reload();
        if !reloaded.starts_with("Reloaded") {
            return Err(reloaded);
        }
        Ok(path)
    }

    /// Writes `world/index.md`, then opens it when `edit` is set.
    pub(super) fn make_world_index(&mut self, start: Option<String>, edit: bool) -> String {
        let start = start.or_else(|| self.current_room()).or_else(|| self.world.default_room());
        let title = self
            .world
            .title
            .clone()
            .or_else(|| self.world.root.file_name().map(|n| super::author::title_or_default(&[], &n.to_string_lossy())))
            .unwrap_or_else(|| "Untitled world".to_string());
        let title = crate::writer::content_file::emit_scalar(&serde_yaml::Value::String(title.clone()), false).unwrap_or(title);
        let mut text = format!("---\ntitle: {}\ntype: world\nauthors: {}\n", title, machine_user());
        if let Some(start) = &start {
            text.push_str(&format!("start: {}\n", start));
        }
        text.push_str("---\n");
        let path = match self.write_world_index(&text) {
            Ok(path) => path,
            Err(e) => return e,
        };
        let summary = match &start {
            Some(room) => format!("Made world/index.md: authors {}, play starts in {}.", machine_user(), room),
            None => format!("Made world/index.md: authors {}.", machine_user()),
        };
        let out = self.after_writes(&[path], summary);
        if edit && self.interactive {
            if let Ok(target) = self.open_target(Kind::World, INDEX.to_string(), INDEX.to_string()) {
                self.pending_edit = Some(target);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::console::session::Console;
    use crate::console::Mode;

    #[test]
    fn start_makes_world_index_then_moves_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for id in ["hall", "cellar"] {
            std::fs::write(root.join(format!("{}.md", id)), format!("---\nid: {}\ntitle: T\ntype: location\n---\n", id)).unwrap();
        }
        std::env::set_var("THWART_USER", "author");
        let mut console = Console::open(root, Mode::Edit).unwrap();
        console.handle("go hall");
        let (out, _) = console.handle("/start");
        assert!(out.starts_with("Made world/index.md: authors author, play starts in hall."), "{}", out);
        let text = std::fs::read_to_string(root.join("world/index.md")).unwrap();
        assert!(text.contains("type: world\nauthors: author\nstart: hall\n"), "{}", text);
        console.handle("go cellar");
        assert_eq!(console.handle("/start").0.lines().next(), Some("Play starts in cellar now."));
        assert!(std::fs::read_to_string(root.join("world/index.md")).unwrap().contains("start: cellar"));
        assert_eq!(console.world.starting_location.as_deref(), Some("cellar"));
    }
}
