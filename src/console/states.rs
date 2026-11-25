//! States and actions from the console, so a common shape takes a few
//! commands rather than files written by hand (REQ-CON-44, 45; Bruce,
//! 2026-11-24).
//!
//! ```text
//! /state hanging start                 a state, and the one it starts in
//! /set weight immovable in hanging     one key in one state's file
//! /action pull hanging --> loose       an action from one state to another
//! /recipe fixed-until pull             the three above, for a common shape
//! ```
//!
//! A one-file thing becomes a bundle the first time it gets a state. State
//! files are layers (no id or type), so the console edits their frontmatter
//! as text and saves through the writer's bundle call, which checks the
//! merged thing still types before anything is written.

use super::author::entity_kind;
use super::session::{suggest, Console, Place};
use super::world::Kind;
use crate::engines::EntityKind;
use crate::writer::{Edit, Section};
use serde_yaml::Value;
use std::path::PathBuf;

impl Console {
    /// The thing edit mode is at, as (console kind, engine kind, id).
    fn thing_here(&self) -> Result<(Kind, EntityKind, String), String> {
        let Place::File(path) = &self.place else {
            return Err(suggest(&self.t("states_need_a_thing", &[]), &["/state {name} [start]"]));
        };
        let entry = self.world.files.get(path).ok_or_else(|| self.t("not_in_world", &[("id", path)]))?;
        let kind = entity_kind(entry.kind).ok_or_else(|| self.t("kind_has_no_states", &[("id", path), ("kind", entry.kind.label())]))?;
        Ok((entry.kind, kind, entry.id.clone()))
    }

    /// `/state <name> [start]`.
    pub(super) fn state_command(&mut self, args: &[&str]) -> String {
        let (name, start) = match args {
            [name] => (*name, false),
            [name, "start"] => (*name, true),
            _ => return suggest(&self.t("usage", &[]), &["/state {name} [start]"]),
        };
        let (kind, entity, id) = match self.thing_here() {
            Ok(thing) => thing,
            Err(e) => return e,
        };
        let mut written = Vec::new();
        let mut done = Vec::new();
        match self.make_state(kind, entity, &id, name, &mut written) {
            Ok(true) => done.push(self.t("made_state", &[("state", name)])),
            Ok(false) => done.push(self.t("state_exists", &[("id", &id), ("state", name)])),
            Err(e) => return self.after_writes(&written, e),
        }
        if start {
            let edit = Edit::SetScalar { key: "state".into(), value: Value::String(name.to_string()) };
            match self.save(kind, &id, &[edit]) {
                Ok(path) => {
                    written.push(path);
                    done.push("it's where the thing starts".to_string());
                }
                Err(e) => return self.after_writes(&written, e),
            }
        }
        let summary = format!("{}.", done.join("; "));
        self.after_writes(&written, summary)
    }

    /// `/set <key> <value...> in <state>`: one key in one state's file.
    pub(super) fn set_in_state(&mut self, args: &[&str]) -> String {
        let Some(at) = args.iter().rposition(|a| *a == "in") else {
            return suggest(&self.t("usage", &[]), &["/set {key} {value} in {state}"]);
        };
        let (key, value, state) = match (&args[..at], args.get(at + 1)) {
            ([key, value @ ..], Some(state)) if !value.is_empty() && at + 2 == args.len() => (*key, value.join(" "), *state),
            _ => return suggest(&self.t("usage", &[]), &["/set {key} {value} in {state}"]),
        };
        let (kind, entity, id) = match self.thing_here() {
            Ok(thing) => thing,
            Err(e) => return e,
        };
        let mut written = Vec::new();
        if let Err(e) = self.make_state(kind, entity, &id, state, &mut written) {
            return self.after_writes(&written, e);
        }
        let shown = crate::writer::content_file::emit_scalar(&super::author::scalar(&value), false).unwrap_or(value.clone());
        match self.edit_state_file(entity, &id, state, |text| set_key(text, key, &shown)) {
            Ok(path) => written.push(path),
            Err(e) => return self.after_writes(&written, e),
        }
        self.after_writes(&written, self.t("set_in_state", &[("key", key), ("value", &value), ("id", &id), ("state", state)]))
    }

    /// `/action <verb> <from> --> <to> [with <thing>]`: the action, in the
    /// `from` state's file so it works only from there, moving the thing to
    /// `to` (made if it doesn't exist). With `with`, the player must hold
    /// that thing. The editor then opens on the reply.
    pub(super) fn action_command(&mut self, args: &[&str]) -> String {
        let usage = || suggest(&self.t("usage", &[]), &["/action {verb} {from} --> {to} [with {thing}]"]);
        let (verb, from, to, with) = match args {
            [verb, from, "-->", to] => (*verb, *from, *to, None),
            [verb, from, "-->", to, "with", thing] => (*verb, *from, *to, Some(*thing)),
            _ => return usage(),
        };
        let (kind, entity, id) = match self.thing_here() {
            Ok(thing) => thing,
            Err(e) => return e,
        };
        let mut written = Vec::new();
        for state in [from, to] {
            if let Err(e) = self.make_state(kind, entity, &id, state, &mut written) {
                return self.after_writes(&written, e);
            }
        }
        let requires = with.map(|t| format!(", requires: {{ has: {} }}", t)).unwrap_or_default();
        let mechanics = format!("{{ becomes: {{ {}: {} }}{} }}", id, to, requires);
        match self.edit_state_file(entity, &id, from, |text| set_action(text, verb, &mechanics)) {
            Ok(path) => written.push(path),
            Err(e) => return self.after_writes(&written, e),
        }
        // The reply goes where the thing ends up (SECTIONS.md): a section in
        // the thing's index, opened in the editor to write.
        let heading = format!("## When: #{} #{}", verb, to);
        let index = self.index_text(entity, &id).unwrap_or_default();
        if !index.lines().any(|l| l.trim() == heading) {
            let text = format!("{}\n{}\n", index.trim_end_matches('\n').to_string() + "\n", heading);
            match self.save(kind, &id, &[Edit::ReplaceAll(text)]) {
                Ok(path) => written.push(path),
                Err(e) => return self.after_writes(&written, e),
            }
        }
        if self.interactive {
            if let Some(path) = self.world.path_of(kind, &id) {
                if let Ok(target) = self.open_target(kind, id.clone(), path) {
                    self.pending_edit = Some(target);
                }
            }
        }
        let summary = format!("{}: {} → {}{}. The reply is under `{}`.", verb, from, to, with.map(|t| format!(", with {} held", t)).unwrap_or_default(), heading);
        self.after_writes(&written, summary)
    }

    /// `/recipe <name> ...`: a common shape, as the small verbs it stands for.
    pub(super) fn recipe_command(&mut self, args: &[&str]) -> String {
        let commands: Vec<String> = match args {
            ["fixed-until", verb] => vec![
                "/state fixed start".into(),
                "/set weight immovable in fixed".into(),
                format!("/action {} fixed --> free", verb),
            ],
            ["fixed-until", verb, fixed, free] => vec![
                format!("/state {} start", fixed),
                format!("/set weight immovable in {}", fixed),
                format!("/action {} {} --> {}", verb, fixed, free),
            ],
            ["locked-with", key] => vec![
                "/state locked start".into(),
                format!("/action unlock locked --> closed with {}", key),
                "/action open closed --> open".into(),
                "/action close open --> closed".into(),
            ],
            ["switch", verbs, states] => {
                let (Some((on_verb, off_verb)), Some((first, second))) = (verbs.split_once('/'), states.split_once('/')) else {
                    return suggest(&self.t("recipe_switch_usage", &[]), &["/recipe switch light/put-out unlit/lit"]);
                };
                vec![
                    format!("/state {} start", first),
                    format!("/action {} {} --> {}", on_verb, first, second),
                    format!("/action {} {} --> {}", off_verb, second, first),
                ]
            }
            ["container", "on"] => vec!["/set holds on".into()],
            ["container", "in"] => vec![
                "/set holds in".into(),
                "/state closed start".into(),
                "/action open closed --> open".into(),
                "/action close open --> closed".into(),
            ],
            _ => {
                return suggest(
                    &self.t("recipes", &[]),
                    &[
                        "/recipe fixed-until {verb} [{fixed} {free}]",
                        "/recipe locked-with {key}",
                        "/recipe switch {verb}/{verb} {state}/{state}",
                        "/recipe container in|on",
                    ],
                )
            }
        };
        // Run them as typed, so the recipe is exactly the verbs it prints.
        let interactive = self.interactive;
        self.interactive = false;
        let mut out = Vec::new();
        for command in &commands {
            self.said.borrow_mut().clear();
            let (reply, _) = self.handle(command);
            if self.said.borrow().iter().any(|kind| kind.failed()) {
                self.interactive = interactive;
                return format!("{}\n{}", out.join("\n"), self.t("recipe_stopped", &[("command", command), ("reply", &reply)]));
            }
            out.push(reply);
        }
        self.interactive = interactive;
        let commands: Vec<&str> = commands.iter().map(String::as_str).collect();
        suggest(&format!("{}\n\n{}", out.join("\n"), self.t("recipe_that_was", &[])), &commands)
    }

    /// Makes state `name` if the thing hasn't one: a one-file thing becomes
    /// a bundle first. Returns whether it made one.
    fn make_state(&mut self, kind: Kind, entity: EntityKind, id: &str, name: &str, written: &mut Vec<PathBuf>) -> Result<bool, String> {
        if name.is_empty() || name.contains(['/', '.', ' ']) {
            return Err(self.t("bad_state_name", &[("name", name)]));
        }
        let writer = self.writer_for_states()?;
        let bundled = self.state_dir(entity, id).is_some();
        if !bundled {
            self.note_warnings_before_states();
            let report = writer
                .make_bundle_loaded(&mut self.world.engine, entity, id, Some(name), false)
                .map_err(|e| self.t("not_made", &[("reason", &format!("{:#}", e))]))?;
            written.extend(report.created.iter().cloned());
            for (from, to) in &report.moved {
                written.push(from.clone());
                written.push(to.clone());
            }
            self.follow_bundle(kind, id);
            return Ok(true);
        }
        let dir = self.state_dir(entity, id).expect("bundled");
        let file = dir.join(format!("{}.md", name));
        if self.world.root.join(&file).exists() {
            return Ok(false);
        }
        let section = Section { path: file.clone(), text: "---\ninherits: index\n---\n".to_string(), expected: None };
        self.note_warnings_before_states();
        let report = writer
            .save_bundle_loaded(&mut self.world.engine, entity, id, &[section], false)
            .map_err(|e| self.t("not_made", &[("reason", &format!("{:#}", e))]))?;
        written.extend(report.created.iter().cloned());
        let _ = kind;
        Ok(true)
    }

    /// Edits state `name`'s file as text with `change`, and saves it
    /// through the writer, which refuses text that doesn't merge.
    fn edit_state_file(&mut self, entity: EntityKind, id: &str, name: &str, change: impl Fn(&str) -> String) -> Result<PathBuf, String> {
        let dir = self.state_dir(entity, id).ok_or_else(|| self.t("no_states_yet", &[("id", id)]))?;
        let file = dir.join(format!("{}.md", name));
        let before = std::fs::read_to_string(self.world.root.join(&file)).map_err(|e| self.t("cant_read", &[("file", &file.display().to_string()), ("reason", &e.to_string())]))?;
        let after = change(&before);
        let writer = self.writer_for_states()?;
        self.note_warnings_before_states();
        writer
            .save_bundle_loaded(&mut self.world.engine, entity, id, &[Section { path: file.clone(), text: after, expected: Some(before) }], false)
            .map_err(|e| self.t("not_saved", &[("reason", &format!("{:#}", e))]))?;
        Ok(file)
    }

    /// A bundled thing's folder, relative to the world root.
    fn state_dir(&self, entity: EntityKind, id: &str) -> Option<PathBuf> {
        let source = self.world.engine.source_path(entity, id).ok()?;
        if source.file_name()? != "index.md" {
            return None;
        }
        let root = self.world.engine.world_root.clone().unwrap_or_default();
        Some(source.parent()?.strip_prefix(&root).unwrap_or(source.parent()?).to_path_buf())
    }

    fn index_text(&self, entity: EntityKind, id: &str) -> Option<String> {
        std::fs::read_to_string(self.world.engine.source_path(entity, id).ok()?).ok()
    }

    /// After a thing became a bundle: edit mode's place follows its file.
    fn follow_bundle(&mut self, kind: Kind, id: &str) {
        self.world.refresh_index();
        if let Some(path) = self.world.path_of(kind, id) {
            self.place = Place::File(path);
        }
    }

    fn writer_for_states(&self) -> Result<crate::writer::WorldWriter, String> {
        crate::writer::WorldWriter::new(&self.world.root).map_err(|e| self.t("cant_write_world", &[("reason", &e.to_string())]))
    }

    fn note_warnings_before_states(&mut self) {
        if self.warnings_before.is_none() {
            self.warnings_before = Some(self.world.warnings());
        }
    }
}

/// `text` with frontmatter `key: value` set: the line replaced if the key
/// is there at the top level, otherwise added before the closing `---`.
pub(super) fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let close = lines.iter().skip(1).position(|l| l.trim_end() == "---").map(|i| i + 1).unwrap_or(lines.len());
    let prefix = format!("{}:", key);
    match lines[1..close].iter().position(|l| l.starts_with(&prefix)) {
        Some(i) => lines[i + 1] = format!("{}: {}", key, value),
        None => lines.insert(close, format!("{}: {}", key, value)),
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// `text` with `verb: mechanics` under frontmatter `actions:`, replacing
/// that verb's line if it's there.
fn set_action(text: &str, verb: &str, mechanics: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let close = lines.iter().skip(1).position(|l| l.trim_end() == "---").map(|i| i + 1).unwrap_or(lines.len());
    let entry = format!("  {}: {}", verb, mechanics);
    match lines[1..close].iter().position(|l| l.trim_end() == "actions:") {
        Some(i) => {
            let start = i + 2;
            let mut end = start;
            while end < close && lines[end].starts_with(' ') {
                end += 1;
            }
            match lines[start..end].iter().position(|l| l.trim_start().starts_with(&format!("{}:", verb))) {
                Some(j) => lines[start + j] = entry,
                None => lines.insert(end, entry),
            }
        }
        None => {
            lines.insert(close, entry);
            lines.insert(close, "actions:".to_string());
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_edits_keep_the_rest() {
        let text = "---\ninherits: index\n---\nBody.\n";
        assert_eq!(set_key(text, "weight", "immovable"), "---\ninherits: index\nweight: immovable\n---\nBody.\n");
        assert_eq!(set_key("---\nweight: light\n---\n", "weight", "immovable"), "---\nweight: immovable\n---\n");
        assert_eq!(
            set_action(text, "pull", "{ becomes: { lantern: loose } }"),
            "---\ninherits: index\nactions:\n  pull: { becomes: { lantern: loose } }\n---\nBody.\n"
        );
        let twice = set_action(&set_action(text, "pull", "{ a }"), "pull", "{ b }");
        assert!(twice.contains("  pull: { b }") && !twice.contains("{ a }"), "{}", twice);
    }
}
