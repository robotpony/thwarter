//! `/chapter` (`design/STATES.md` › Console; Bruce, 2026-11-25):
//!
//! ```text
//! /chapter new 20 after-the-king   a chapter, at the world root
//! /chapter later                   a `## When: #later` section in this thing
//! /chapter later file              its own `chapters/later.md` instead
//! ```
//!
//! A small chapter change is a section in the thing's own file, and a big
//! one a `chapters/` file (`design/SECTIONS.md` › Open question 2).

use super::author::entity_kind;
use super::session::{suggest, Console, Place};
use super::world::Kind;
use crate::writer::{Edit, Section};
use std::path::PathBuf;

impl Console {
    /// `/chapter ...`.
    pub(super) fn chapter_command(&mut self, args: &[&str]) -> String {
        match args {
            ["new", number, name] => self.new_chapter(number, name),
            [name] => self.chapter_here(name, false),
            [name, "file"] => self.chapter_here(name, true),
            _ => suggest(&self.t("usage", &[]), &["/chapter new {number} {name}", "/chapter {name} [file]"]),
        }
    }

    /// The chapters the world has, in story order, for asking which.
    pub(super) fn chapter_names(&self) -> Vec<String> {
        let mut chapters: Vec<_> = self.world.engine.chapters().iter().collect();
        chapters.sort_by(|a, b| a.order.cmp(&b.order));
        let mut names: Vec<String> = chapters.iter().map(|c| c.name().to_string()).collect();
        names.dedup();
        names
    }

    /// `/chapter new <number> <name>`: `chapters/<number>-<name>.md` at the
    /// world root, refused if the number or name is taken.
    fn new_chapter(&mut self, number: &str, name: &str) -> String {
        if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
            return suggest(&self.t("chapter_bad_number", &[("number", number)]), &["/chapter new 20 after-the-king"]);
        }
        if name.is_empty() || name.contains(['/', '.', ' ']) {
            return self.t("chapter_bad_name", &[("name", name)]);
        }
        let dir = self.world.root.join("chapters");
        let taken = std::fs::read_dir(&dir).into_iter().flatten().flatten().any(|e| {
            let file = e.file_name().to_string_lossy().to_string();
            let stem = file.trim_end_matches(".md");
            stem.split_once('-').is_some_and(|(n, rest)| n == number || rest == name)
        });
        if taken || self.chapter_names().iter().any(|n| n == name) {
            return self.t("chapter_taken", &[("number", number), ("name", name)]);
        }
        let rel = PathBuf::from("chapters").join(format!("{}-{}.md", number, name));
        let title = super::author::title_or_default(&[], name);
        let text = format!("---\ntitle: {}\n---\n", title);
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(self.world.root.join(&rel), text)) {
            return self.t("not_saved", &[("reason", &e.to_string())]);
        }
        self.note_warnings_before_chapter();
        let _ = self.reload();
        let written = vec![self.world.root.join(&rel)];
        self.after_writes(&written, self.t("chapter_made", &[("name", name), ("path", &rel.display().to_string())]))
    }

    /// `/chapter <name> [file]` at a thing: a section in its own file, or
    /// its own `chapters/` file, then the editor on it.
    fn chapter_here(&mut self, name: &str, file: bool) -> String {
        let here = match &self.place {
            Place::File(path) => self.world.files.get(path).map(|e| (e.kind, e.id.clone())),
            _ => None,
        };
        let Some((kind, id)) = here.filter(|(k, _)| matches!(k, Kind::Room | Kind::Object | Kind::Character | Kind::Door)) else {
            return suggest(&self.t("chapter_needs_a_thing", &[]), &["/chapter {name} [file]"]);
        };
        let names = self.chapter_names();
        if !names.iter().any(|n| n == name) {
            let known = if names.is_empty() { self.t("qm_none_yet", &[]) } else { names.join(", ") };
            return suggest(&self.t("chapter_unknown", &[("name", name), ("chapters", &known)]), &["/chapter new {number} {name}"]);
        }
        let Some(entity) = entity_kind(kind) else { return self.t("not_in_world", &[("id", &id)]) };
        let mut written = Vec::new();
        if file {
            let writer = match crate::writer::WorldWriter::new(&self.world.root) {
                Ok(w) => w,
                Err(e) => return self.t("cant_write_world", &[("reason", &e.to_string())]),
            };
            self.note_warnings_before_chapter();
            if self.bundle_dir(entity, &id).is_none() {
                match writer.make_bundle_loaded(&mut self.world.engine, entity, &id, None, false) {
                    Ok(report) => {
                        written.extend(report.created.iter().cloned());
                        for (from, to) in &report.moved {
                            written.push(from.clone());
                            written.push(to.clone());
                        }
                    }
                    Err(e) => return self.t("not_made", &[("reason", &format!("{:#}", e))]),
                }
                self.world.refresh_index();
            }
            let Some(dir) = self.bundle_dir(entity, &id) else { return self.after_writes(&written, self.t("not_in_world", &[("id", &id)])) };
            let rel = dir.join("chapters").join(format!("{}.md", name));
            if self.world.root.join(&rel).exists() {
                return self.after_writes(&written, self.t("chapter_here_exists", &[("id", &id), ("path", &rel.display().to_string())]));
            }
            let section = Section { path: rel.clone(), text: "---\n---\n".to_string(), expected: None };
            match writer.save_bundle_loaded(&mut self.world.engine, entity, &id, &[section], false) {
                Ok(report) => written.extend(report.created.iter().cloned()),
                Err(e) => return self.after_writes(&written, self.t("not_saved", &[("reason", &format!("{:#}", e))])),
            }
            self.world.refresh_index();
            if let Some(path) = self.world.path_of(kind, &id) {
                self.place = Place::File(path);
            }
            self.open_after_chapter(kind, &id);
            return self.after_writes(&written, self.t("chapter_file_made", &[("id", &id), ("name", name), ("path", &rel.display().to_string())]));
        }
        // A section in the thing's own file.
        let heading = format!("## When: #{}", name);
        let Some(index) = self.world.engine.source_path(entity, &id).ok().and_then(|p| std::fs::read_to_string(p).ok()) else {
            return self.t("not_in_world", &[("id", &id)]);
        };
        if index.lines().any(|l| l.trim_end() == heading) {
            self.open_after_chapter(kind, &id);
            return self.t("chapter_section_exists", &[("id", &id), ("heading", &heading)]);
        }
        let text = format!("{}\n{}\n", index.trim_end_matches('\n').to_string() + "\n", heading);
        match self.save(kind, &id, &[Edit::ReplaceAll(text)]) {
            Ok(path) => written.push(path),
            Err(e) => return e,
        }
        self.open_after_chapter(kind, &id);
        self.after_writes(&written, self.t("chapter_section_made", &[("id", &id), ("heading", &heading)]))
    }

    fn open_after_chapter(&mut self, kind: Kind, id: &str) {
        if !self.interactive {
            return;
        }
        if let Some(path) = self.world.path_of(kind, id) {
            if let Ok(target) = self.open_target(kind, id.to_string(), path) {
                self.pending_edit = Some(target);
            }
        }
    }

    fn note_warnings_before_chapter(&mut self) {
        if self.warnings_before.is_none() {
            self.warnings_before = Some(self.world.warnings());
        }
    }
}
