//! Question sets (REQ-CF-32, `design/CONTENT_FORMAT.md` › `type:
//! questions`): what the console's `/new` and `/add` ask when an author
//! makes a thing. A set is its own file in a reserved `questions/` folder,
//! in plain Markdown: each `##` heading a question, prose under it a hint,
//! each list item an option, and `->` (or `→`) before what the option does.
//!
//! ```markdown
//! ---
//! type: questions
//! ---
//! ## Where does it start?
//! - Loose, it can be taken (default)
//! - Hanging on a wall, pulled free -> recipe fixed-until pull
//! - On something here -> ask thing, then move into it
//! ```
//!
//! The file's name is the template or kind it sets up. A set is looked up
//! from the new thing's folder out to the world's root, then in the
//! built-ins; the nearest whole file wins, and nothing merges.

use std::path::{Path, PathBuf};

/// The built-in sets, compiled in (`src/builtins/questions/`).
pub const BUILTIN_QUESTIONS: &[(&str, &str)] = &[("object", include_str!("../builtins/questions/object.md"))];

/// One question set.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionSet {
    /// The template or kind it sets up: the file's name without `.md`.
    pub name: String,
    /// The folder its `questions/` folder sits in, relative to the world
    /// root (empty for the root); `None` for a built-in.
    pub scope: Option<PathBuf>,
    /// Its file, or `<built-in>/questions/<name>.md`.
    pub path: PathBuf,
    pub questions: Vec<Question>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    /// The heading's text: "Where does it start?"
    pub text: String,
    /// Prose under the heading, shown with it; empty for none.
    pub hint: String,
    pub options: Vec<Choice>,
}

impl Question {
    /// The option Esc takes: the one marked `(default)`, else the first.
    pub fn default_choice(&self) -> Option<&Choice> {
        self.options.iter().find(|o| o.default).or_else(|| self.options.first())
    }
}

/// One option: what the author sees, and what it does.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// The text shown, without `(default)`.
    pub label: String,
    pub default: bool,
    /// What it does; `None` changes nothing.
    pub action: Option<Construct>,
}

/// What an option does, after its `->`.
#[derive(Debug, Clone, PartialEq)]
pub enum Construct {
    /// `set <key>`: the author types the value; `set <key> <value>`: the
    /// value written in the option, with nothing to ask.
    Set { key: String, value: Option<String> },
    /// `recipe <name> <args>`: shorthand for console commands.
    Recipe { name: String, args: Vec<String> },
    /// `ask thing`: pick a thing here. `then` is what follows a comma,
    /// as written ("then move into it").
    AskThing { then: Option<String> },
    /// `ask value <name> <default>`.
    AskValue { name: String, default: Option<String> },
    /// Anything else, as written, which `validate` warns about.
    Unknown(String),
}

impl Construct {
    fn parse(text: &str) -> Self {
        let text = text.trim();
        let words: Vec<&str> = text.split_whitespace().collect();
        match words.as_slice() {
            ["set", key] => Construct::Set { key: key.to_string(), value: None },
            ["set", key, value @ ..] => Construct::Set { key: key.to_string(), value: Some(value.join(" ")) },
            ["recipe", name, args @ ..] => Construct::Recipe { name: name.to_string(), args: args.iter().map(|a| a.to_string()).collect() },
            ["ask", "value", name] => Construct::AskValue { name: name.to_string(), default: None },
            ["ask", "value", name, default @ ..] => Construct::AskValue { name: name.to_string(), default: Some(default.join(" ")) },
            ["ask", first, ..] if first.trim_end_matches(',') == "thing" => {
                let rest = text["ask".len()..].trim_start()["thing".len()..].trim_start_matches(',').trim();
                Construct::AskThing { then: (!rest.is_empty()).then(|| rest.to_string()) }
            }
            _ => Construct::Unknown(text.to_string()),
        }
    }
}

/// Read a set from its file's body (the text after the frontmatter),
/// with what's odd about it as warnings prefixed with `shown`.
pub fn parse(name: &str, scope: Option<PathBuf>, path: &Path, body: &str, shown: &str) -> (QuestionSet, Vec<String>) {
    let mut questions: Vec<Question> = Vec::new();
    let mut warnings = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            questions.push(Question { text: heading.trim().to_string(), hint: String::new(), options: Vec::new() });
            continue;
        }
        let item = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* "));
        let Some(question) = questions.last_mut() else {
            if !trimmed.is_empty() && !trimmed.starts_with("<!--") {
                warnings.push(format!("{}: text before the first `##` question isn't asked", shown));
            }
            continue;
        };
        match item {
            Some(item) => {
                let (label, action) = match item.split_once("->").or_else(|| item.split_once('→')) {
                    Some((label, action)) => (label.trim(), Some(Construct::parse(action))),
                    None => (item.trim(), None),
                };
                let (label, default) = match label.strip_suffix("(default)") {
                    Some(rest) => (rest.trim(), true),
                    None => (label, false),
                };
                if let Some(Construct::Unknown(what)) = &action {
                    warnings.push(format!(
                        "{}: \"{}\" under \"{}\" does `{}`, which isn't set, recipe, ask thing or ask value",
                        shown, label, question.text, what
                    ));
                }
                question.options.push(Choice { label: label.to_string(), default, action });
            }
            None if question.options.is_empty() => {
                if !trimmed.is_empty() {
                    if !question.hint.is_empty() {
                        question.hint.push('\n');
                    }
                    question.hint.push_str(trimmed);
                }
            }
            None if !trimmed.is_empty() => {
                warnings.push(format!("{}: text after the options of \"{}\" isn't shown", shown, question.text));
            }
            None => {}
        }
    }
    for q in &questions {
        if q.options.iter().filter(|o| o.default).count() > 1 {
            warnings.push(format!("{}: \"{}\" has more than one `(default)`; the first is used", shown, q.text));
        }
    }
    (QuestionSet { name: name.to_string(), scope, path: path.to_path_buf(), questions }, warnings)
}

/// What's odd about the built-in sets a world still asks (an answer
/// that does something unknown, say), as `parse` warns about a world's
/// own: every built-in but one that a set of the same name at the
/// world's root replaces.
pub fn builtin_warnings(sets: &[QuestionSet]) -> Vec<String> {
    builtin_warnings_in(BUILTIN_QUESTIONS, sets)
}

fn builtin_warnings_in(builtins: &[(&str, &str)], sets: &[QuestionSet]) -> Vec<String> {
    let mut out = Vec::new();
    for (name, text) in builtins {
        if sets.iter().any(|s| s.name == *name && s.scope.as_deref() == Some(Path::new(""))) {
            continue;
        }
        let path = PathBuf::from(format!("<built-in>/questions/{}.md", name));
        let shown = path.to_string_lossy().into_owned();
        match crate::engines::layers::Layer::parse(&path, text) {
            Ok(layer) => out.extend(parse(name, None, &path, &layer.body, &shown).1),
            Err(e) => out.push(format!("{}: doesn't parse, so it isn't asked: {}", shown, e)),
        }
    }
    out
}

/// The built-in set `name`, if there is one.
pub fn builtin(name: &str) -> Option<QuestionSet> {
    let (_, text) = BUILTIN_QUESTIONS.iter().find(|(n, _)| *n == name)?;
    let path = PathBuf::from(format!("<built-in>/questions/{}.md", name));
    let layer = crate::engines::layers::Layer::parse(&path, text).ok()?;
    Some(parse(name, None, &path, &layer.body, &path.to_string_lossy()).0)
}

/// The set to ask for a thing `id` made in `folder` (relative to the
/// world root) from `template` (if any) of `kind`: the set named for its
/// id, else its template's, else its kind's, each the nearest from
/// `folder` out to the root, then built in. Ids and templates match by
/// their last part (`castle/lantern` is `lantern`).
pub fn find(sets: &[QuestionSet], folder: &Path, id: Option<&str>, template: Option<&str>, kind: &str) -> Option<QuestionSet> {
    let short = |t: &str| t.rsplit('/').next().unwrap_or(t).to_string();
    let names: Vec<String> = id.map(short).into_iter().chain(template.map(short)).chain(std::iter::once(kind.to_string())).collect();
    for name in &names {
        let name = name.as_str();
        let mut scope = Some(folder);
        while let Some(at) = scope {
            if let Some(set) = sets.iter().find(|s| s.name == name && s.scope.as_deref() == Some(at)) {
                return Some(set.clone());
            }
            scope = at.parent();
        }
        if let Some(set) = builtin(name) {
            return Some(set);
        }
    }
    None
}

/// The question folder a world file is in, if any: the scope it sits in
/// and the set's name. Only a file straight inside `questions/` counts.
pub fn question_file(rel: &Path) -> Option<(PathBuf, String)> {
    let parent = rel.parent()?;
    if parent.file_name()? != "questions" || rel.extension()? != "md" {
        return None;
    }
    Some((parent.parent().unwrap_or(Path::new("")).to_path_buf(), rel.file_stem()?.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_built_in_set_warns_as_a_world_set_does_unless_the_world_replaces_it() {
        let builtins = [("object", "---\ntype: questions\n---\n## Where is it?\n- Loose (default)\n- A fixture -> location in room\n")];
        let warnings = builtin_warnings_in(&builtins, &[]);
        assert_eq!(warnings, ["<built-in>/questions/object.md: \"A fixture\" under \"Where is it?\" does `location in room`, which isn't set, recipe, ask thing or ask value"]);
        // A world's own object set at its root replaces the built-in, so
        // the built-in isn't asked and doesn't warn; one in a folder doesn't.
        let own = |scope: &str| QuestionSet { name: "object".into(), scope: Some(PathBuf::from(scope)), path: PathBuf::new(), questions: Vec::new() };
        assert!(builtin_warnings_in(&builtins, &[own("")]).is_empty());
        assert_eq!(builtin_warnings_in(&builtins, &[own("castle")]).len(), 1);
    }

    use super::*;

    const LAMP: &str = "## Where does it start?\nPick one.\n- Loose, it can be taken (default)\n- Hanging on a wall, pulled free -> recipe fixed-until pull\n- On something here -> ask thing, then move into it\n\n## How bright?\n- Dim → set brightness\n- Preset -> set title Brass  lantern\n- Ask me -> ask value brightness 3\n- Glowing (default)\n- Wobbly (default) -> dance about\n";

    #[test]
    fn a_set_reads_questions_hints_options_and_constructs() {
        let (set, warnings) = parse("lantern", Some("".into()), Path::new("questions/lantern.md"), LAMP, "questions/lantern.md");
        assert_eq!(set.questions.len(), 2);
        let q = &set.questions[0];
        assert_eq!((q.text.as_str(), q.hint.as_str()), ("Where does it start?", "Pick one."));
        assert_eq!(q.default_choice().unwrap().label, "Loose, it can be taken");
        assert_eq!(q.options[0].action, None);
        assert_eq!(q.options[1].action, Some(Construct::Recipe { name: "fixed-until".into(), args: vec!["pull".into()] }));
        assert_eq!(q.options[2].action, Some(Construct::AskThing { then: Some("then move into it".into()) }));
        let q = &set.questions[1];
        assert_eq!(q.options[0].action, Some(Construct::Set { key: "brightness".into(), value: None }));
        assert_eq!(q.options[1].action, Some(Construct::Set { key: "title".into(), value: Some("Brass lantern".into()) }));
        assert_eq!(q.options[2].action, Some(Construct::AskValue { name: "brightness".into(), default: Some("3".into()) }));
        assert_eq!(q.default_choice().unwrap().label, "Glowing", "the first (default) wins");
        assert_eq!(warnings.len(), 2, "{:?}", warnings);
        assert!(warnings[0].contains("does `dance about`, which isn't set, recipe, ask thing or ask value"), "{:?}", warnings);
        assert!(warnings[1].contains("\"How bright?\" has more than one `(default)`"), "{:?}", warnings);
    }

    #[test]
    fn the_nearest_set_wins_then_the_kinds_then_the_built_in() {
        let set = |name: &str, scope: &str| parse(name, Some(scope.into()), Path::new("x"), "## Q?\n- A\n", "x").0;
        let sets = vec![set("lantern", ""), set("lantern", "castle"), set("object", "castle/cellar")];
        let found = |folder: &str, template: Option<&str>, kind: &str| find(&sets, Path::new(folder), None, template, kind).map(|s| (s.name, s.scope));
        assert_eq!(found("castle/hall", Some("lantern"), "object"), Some(("lantern".into(), Some("castle".into()))));
        assert_eq!(found("forest", Some("templates/lantern"), "object"), Some(("lantern".into(), Some("".into()))));
        assert_eq!(found("castle/cellar", None, "object"), Some(("object".into(), Some("castle/cellar".into()))));
        assert_eq!(found("forest", None, "object"), Some(("object".into(), None)), "the built-in");
        assert_eq!(found("forest", None, "location"), None);
        // The thing's own id comes first, then its template.
        let by_id = |id: &str, template: Option<&str>| find(&sets, Path::new("castle/hall"), Some(id), template, "object").map(|s| s.name);
        assert_eq!(by_id("castle/hall/lantern", Some("teapot")).as_deref(), Some("lantern"));
        assert_eq!(by_id("brass-lamp", Some("templates/lantern")).as_deref(), Some("lantern"));
        assert_eq!(by_id("brass-lamp", None).as_deref(), Some("object"));
        assert!(builtin("object").unwrap().questions.iter().all(|q| !q.options.is_empty()));
        assert_eq!(question_file(Path::new("castle/questions/lantern.md")), Some(("castle".into(), "lantern".into())));
        assert_eq!(question_file(Path::new("questions/door.md")), Some(("".into(), "door".into())));
        assert_eq!(question_file(Path::new("questions/old/door.md")), None);
    }
}
