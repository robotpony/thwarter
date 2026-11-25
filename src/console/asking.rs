//! Questions `/add` and `/new` ask about a thing they've just made
//! (REQ-CON-46, Bruce, 2026-11-24), from the nearest question set
//! (`design/CONTENT_FORMAT.md` › `type: questions`). The screen asks them;
//! this turns the answers into console commands and runs them, so an
//! answer does exactly what typing its commands would, and the author sees
//! which commands those were.

use super::session::{suggest, Console, Place};
use super::world::Kind;
use crate::engines::questions::{Choice, Construct, QuestionSet};
use std::path::Path;

/// A question set to ask about one new thing.
#[derive(Debug, Clone)]
pub struct Asking {
    pub set: QuestionSet,
    pub kind: Kind,
    pub id: String,
    /// The set's file as the author knows it: inside the world, or
    /// `<built-in>/questions/<name>.md`.
    pub shown_path: String,
}

/// One answer: the option chosen, and what was typed or picked for it.
#[derive(Debug, Clone)]
pub struct Answer {
    pub choice: Choice,
    pub given: Option<String>,
}

impl Console {
    /// The questions for a thing `/add` or `/new` just made, if a set
    /// applies: the thing's own name first (a template by that name), then
    /// its kind's, nearest the folder it's in.
    pub(super) fn questions_for_new(&self, kind: Kind, id: &str) -> Option<Asking> {
        let kind_word = match kind {
            Kind::Room => "location",
            Kind::Character => "character",
            Kind::Object => "object",
            Kind::Door => "door",
            _ => return None,
        };
        let folder = match &self.place {
            Place::File(path) => Path::new(path).parent().map(Path::to_path_buf).unwrap_or_default(),
            Place::Dir(dir) => Path::new(dir).to_path_buf(),
            Place::Game(_) => Default::default(),
        };
        let set = self.world.engine.questions_for_thing(&folder, id, None, kind_word)?;
        let root = self.world.engine.world_root.clone().unwrap_or_default();
        let shown_path = set.path.strip_prefix(&root).unwrap_or(&set.path).display().to_string();
        (!set.questions.is_empty()).then(|| Asking { set, kind, id: id.to_string(), shown_path })
    }

    /// What `/add` or `/new` left to ask, for the screen.
    pub(super) fn take_pending_questions(&mut self) -> Option<Asking> {
        self.pending_questions.take()
    }

    /// The things in the room edit mode is in, for an `ask thing` answer.
    pub(super) fn things_to_pick(&self, besides: &str) -> Vec<String> {
        let Some(room) = self.current_room() else { return Vec::new() };
        let Some(location) = self.world.engine.get_location(&room) else { return Vec::new() };
        location.objects.iter().chain(&location.characters).filter(|id| *id != besides).cloned().collect()
    }

    /// Runs the answers as the console commands they stand for, with edit
    /// mode at the thing for the length of it, and says what they were.
    pub(super) fn run_answers(&mut self, asking: &Asking, answers: &[Answer]) -> String {
        let mut commands = Vec::new();
        let mut made = Vec::new();
        // Recipes and /set need a card to write to. It's written straight,
        // not with /new, which would also place the thing in edit mode's
        // room: /add already put it where it belongs, and a thing starts in
        // one place (the "new object lands in two rooms" bug, 2026-11-24).
        if asking.kind == Kind::Object && self.world.path_of(Kind::Object, &asking.id).is_none() {
            let new = crate::writer::NewEntity {
                kind: crate::engines::EntityKind::Object,
                id: asking.id.clone(),
                title: super::author::title_or_default(&[], &asking.id),
                object_type: Some("thing".to_string()),
            };
            match self.create(&new, &[]) {
                Ok(path) => made.push(path),
                Err(e) => return e,
            }
            self.world.refresh_index();
        }
        for answer in answers {
            commands.extend(commands_for(&asking.id, answer));
        }
        if commands.is_empty() {
            return if made.is_empty() { String::new() } else { self.after_writes(&made, self.t("made_card", &[("id", &asking.id)])) };
        }
        let before = self.place.clone();
        let interactive = std::mem::replace(&mut self.interactive, false);
        let mut out = Vec::new();
        for command in &commands {
            if let Some(path) = self.world.path_of(asking.kind, &asking.id) {
                self.place = Place::File(path);
            }
            let (reply, _) = self.handle(command);
            if !reply.is_empty() {
                out.push(reply);
            }
        }
        self.interactive = interactive;
        self.place = before;
        // A thing turned into a bundle has a new path; the place follows
        // only if it was the thing.
        if !self.place_exists() {
            self.move_to_existing_folder();
        }
        if !made.is_empty() {
            out.insert(0, self.after_writes(&made, self.t("made_card", &[("id", &asking.id)])));
        }
        let commands: Vec<&str> = commands.iter().map(String::as_str).collect();
        suggest(&format!("{}\n\n{}", out.join("\n"), self.t("answers_were", &[])), &commands)
    }
}

/// The commands one answer stands for.
fn commands_for(id: &str, answer: &Answer) -> Vec<String> {
    let given = answer.given.as_deref().map(str::trim).filter(|g| !g.is_empty());
    match (&answer.choice.action, given) {
        (None, _) => Vec::new(),
        // A value written into the option needs nothing typed.
        (Some(Construct::Set { key, value: Some(preset) }), _) => vec![format!("/set {} {}", key, quote(preset))],
        (Some(Construct::Set { key, .. }), Some(value)) => vec![format!("/set {} {}", key, quote(value))],
        (Some(Construct::Set { .. }), None) => Vec::new(),
        (Some(Construct::Recipe { name, args }), _) => {
            vec![format!("/recipe {} {}", name, args.join(" ")).trim_end().to_string()]
        }
        (Some(Construct::AskThing { then }), Some(thing)) => {
            let into = then.as_deref().is_some_and(|t| t.contains("move") || t.contains("into") || t.contains("on"));
            if into {
                vec![format!("/move {} {}", id, thing)]
            } else {
                vec![format!("/add {} {}", id, thing)]
            }
        }
        (Some(Construct::AskThing { .. }), None) => Vec::new(),
        (Some(Construct::AskValue { name, default }), given) => {
            let Some(value) = given.map(str::to_string).or_else(|| default.clone()) else { return Vec::new() };
            // `lantern.fuel` or `this.fuel` is one of this thing's values.
            let key = match name.split_once('.') {
                Some((level, rest)) if level == id || level == "this" => format!("vars.{}", rest),
                _ => name.clone(),
            };
            vec![format!("/set {} {}", key, quote(&value))]
        }
        (Some(Construct::Unknown(_)), _) => Vec::new(),
    }
}

/// A typed value as a command takes it, quoted when it holds a space.
fn quote(value: &str) -> String {
    if value.contains(' ') { format!("\"{}\"", value) } else { value.to_string() }
}

/// Whether an answer to this option needs something typed or picked.
pub fn needs(choice: &Choice) -> Option<Needs> {
    match &choice.action {
        Some(Construct::Set { value: Some(_), .. }) => None,
        Some(Construct::Set { key, .. }) => Some(Needs::Text { prompt: format!("{}:", key), default: None }),
        Some(Construct::AskValue { name, default }) => Some(Needs::Text { prompt: format!("{}:", name), default: default.clone() }),
        Some(Construct::AskThing { .. }) => Some(Needs::Thing),
        _ => None,
    }
}

/// What an option needs before it's answered.
#[derive(Debug, Clone, PartialEq)]
pub enum Needs {
    Text { prompt: String, default: Option<String> },
    Thing,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(action: Option<Construct>) -> Choice {
        Choice { label: "x".into(), default: false, action }
    }

    #[test]
    fn answers_become_the_commands_they_stand_for() {
        let set = |key: &str| Some(Construct::Set { key: key.into(), value: None });
        let a = |c, g: Option<&str>| Answer { choice: choice(c), given: g.map(str::to_string) };
        assert_eq!(commands_for("lantern", &a(set("title"), Some("Old lamp"))), vec!["/set title \"Old lamp\""]);
        assert!(commands_for("lantern", &a(set("title"), None)).is_empty());
        let preset = Some(Construct::Set { key: "title".into(), value: Some("Brass lantern".into()) });
        assert_eq!(commands_for("lantern", &a(preset.clone(), None)), vec!["/set title \"Brass lantern\""]);
        assert!(needs(&choice(preset)).is_none(), "nothing to type");
        let recipe = Some(Construct::Recipe { name: "fixed-until".into(), args: vec!["pull".into()] });
        assert_eq!(commands_for("lantern", &a(recipe, None)), vec!["/recipe fixed-until pull"]);
        let thing = Some(Construct::AskThing { then: Some("then move into it".into()) });
        assert_eq!(commands_for("lantern", &a(thing, Some("bench"))), vec!["/move lantern bench"]);
        let value = Some(Construct::AskValue { name: "lantern.fuel".into(), default: Some("50".into()) });
        assert_eq!(commands_for("lantern", &a(value.clone(), None)), vec!["/set vars.fuel 50"]);
        assert_eq!(commands_for("lantern", &a(value, Some("20"))), vec!["/set vars.fuel 20"]);
        assert!(commands_for("lantern", &a(None, None)).is_empty());
    }
}
