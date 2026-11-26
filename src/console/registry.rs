//! The console's command registry (REQ-CON-50 to 56, Bruce, 2026-11-24):
//! every command described once, in a `type: command` Markdown file, and
//! the `/` menu, `/help`, the mode guard, name lookup and the transcript
//! guard all read it. Built-ins live in `src/builtins/commands/`, in the
//! order the menu shows them.

use serde::Deserialize;
use std::sync::OnceLock;

/// The built-in command files, in menu order.
const BUILTIN: &[(&str, &str)] = &[
    ("edit", include_str!("../builtins/commands/edit.md")),
    ("play", include_str!("../builtins/commands/play.md")),
    ("new", include_str!("../builtins/commands/new.md")),
    ("set", include_str!("../builtins/commands/set.md")),
    ("link", include_str!("../builtins/commands/link.md")),
    ("unlink", include_str!("../builtins/commands/unlink.md")),
    ("add", include_str!("../builtins/commands/add.md")),
    ("move", include_str!("../builtins/commands/move.md")),
    ("state", include_str!("../builtins/commands/state.md")),
    ("action", include_str!("../builtins/commands/action.md")),
    ("recipe", include_str!("../builtins/commands/recipe.md")),
    ("save", include_str!("../builtins/commands/save.md")),
    ("load", include_str!("../builtins/commands/load.md")),
    ("reload", include_str!("../builtins/commands/reload.md")),
    ("restart", include_str!("../builtins/commands/restart.md")),
    ("status", include_str!("../builtins/commands/status.md")),
    ("map", include_str!("../builtins/commands/map.md")),
    ("list", include_str!("../builtins/commands/list.md")),
    ("scope", include_str!("../builtins/commands/scope.md")),
    ("validate", include_str!("../builtins/commands/validate.md")),
    ("start", include_str!("../builtins/commands/start.md")),
    ("write", include_str!("../builtins/commands/write.md")),
    ("menu", include_str!("../builtins/commands/menu.md")),
    ("chapter", include_str!("../builtins/commands/chapter.md")),
    ("remove", include_str!("../builtins/commands/remove.md")),
    ("inspect", include_str!("../builtins/commands/inspect.md")),
    ("clear", include_str!("../builtins/commands/clear.md")),
    ("help", include_str!("../builtins/commands/help.md")),
    ("quit", include_str!("../builtins/commands/quit.md")),
];

/// One command, as its file describes it.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CommandDef {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub about: String,
    #[serde(default)]
    pub args: String,
    pub modes: Vec<String>,
    #[serde(default)]
    pub changes: bool,
    pub handler: String,
    /// The file's body: the full help, with examples.
    #[serde(skip)]
    pub help: String,
}

impl CommandDef {
    /// Whether it only works in edit mode.
    pub fn edit_only(&self) -> bool {
        !self.modes.iter().any(|m| m == "play")
    }

    /// `/remove {thing}`, as usage lines and `/help` show it.
    pub fn usage(&self) -> String {
        if self.args.is_empty() { format!("/{}", self.name) } else { format!("/{} {}", self.name, self.args) }
    }
}

/// The built-in commands, parsed once.
pub fn commands() -> &'static [CommandDef] {
    static LOADED: OnceLock<Vec<CommandDef>> = OnceLock::new();
    LOADED.get_or_init(|| BUILTIN.iter().map(|(name, text)| parse(name, text)).collect())
}

fn parse(name: &str, text: &str) -> CommandDef {
    let rest = text.strip_prefix("---\n").unwrap_or_else(|| panic!("commands/{}.md has no frontmatter", name));
    let (front, body) = rest.split_once("\n---\n").unwrap_or_else(|| panic!("commands/{}.md's frontmatter isn't closed", name));
    let mut def: CommandDef = serde_yaml::from_str(front).unwrap_or_else(|e| panic!("commands/{}.md: {}", name, e));
    def.help = body.trim().to_string();
    def
}

/// What a typed command name means: the command, if one fits.
#[derive(Debug, PartialEq)]
pub enum Lookup<'a> {
    Found(&'a CommandDef),
    /// A prefix more than one command starts with.
    Ambiguous(Vec<&'a CommandDef>),
    /// Nothing fits; the nearest names, if any are close.
    Unknown(Vec<&'a CommandDef>),
}

/// Finds `typed` by name, then alias, then a unique prefix of a name
/// (REQ-CON-56: forgiving where it can be).
pub fn lookup(typed: &str) -> Lookup<'static> {
    let typed = typed.to_lowercase();
    let all = commands();
    if let Some(c) = all.iter().find(|c| c.name == typed) {
        return Lookup::Found(c);
    }
    if let Some(c) = all.iter().find(|c| c.aliases.iter().any(|a| *a == typed)) {
        return Lookup::Found(c);
    }
    if !typed.is_empty() {
        let starting: Vec<&CommandDef> = all.iter().filter(|c| c.name.starts_with(&typed)).collect();
        match starting.len() {
            1 => return Lookup::Found(starting[0]),
            n if n > 1 => return Lookup::Ambiguous(starting),
            _ => {}
        }
    }
    let mut near: Vec<(usize, &CommandDef)> =
        all.iter().map(|c| (distance(&typed, &c.name), c)).filter(|(d, c)| *d <= 2.max(c.name.len() / 3)).collect();
    near.sort_by_key(|(d, _)| *d);
    Lookup::Unknown(near.into_iter().take(3).map(|(_, c)| c).collect())
}

/// The names of the commands that change the world, for the transcript
/// guard.
pub fn changing() -> Vec<String> {
    commands().iter().filter(|c| c.changes).flat_map(|c| std::iter::once(c.name.clone()).chain(c.aliases.clone())).collect()
}

/// Edit distance, for "did you mean".
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let here = row[j + 1];
            row[j + 1] = (row[j + 1] + 1).min(row[j] + 1).min(prev + usize::from(ca != *cb));
            prev = here;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_file_parses_and_names_itself() {
        for (file, _) in BUILTIN {
            let c = commands().iter().find(|c| c.name == *file).unwrap_or_else(|| panic!("{}.md names another command", file));
            assert!(!c.about.is_empty() && !c.help.is_empty(), "{}", file);
            assert!(c.modes.iter().all(|m| m == "edit" || m == "play"), "{}", file);
        }
    }

    #[test]
    fn no_name_or_alias_is_used_twice() {
        let mut seen = std::collections::BTreeSet::new();
        for c in commands() {
            for n in std::iter::once(&c.name).chain(&c.aliases) {
                assert!(seen.insert(n.clone()), "`{}` is used twice", n);
            }
        }
    }

    #[test]
    fn names_aliases_prefixes_and_near_misses() {
        let name = |l: Lookup| match l {
            Lookup::Found(c) => c.name.clone(),
            other => format!("{:?}", other.names()),
        };
        assert_eq!(name(lookup("remove")), "remove");
        assert_eq!(name(lookup("rm")), "remove");
        assert_eq!(name(lookup("insp")), "inspect");
        assert_eq!(name(lookup("re")), "[\"recipe\", \"reload\", \"restart\", \"remove\"]");
        assert!(matches!(lookup("remvoe"), Lookup::Unknown(near) if near[0].name == "remove"));
        assert!(matches!(lookup("xyzzy"), Lookup::Unknown(near) if near.is_empty()));
    }
}

impl Lookup<'_> {
    /// The names it lists, for a reply.
    pub fn names(&self) -> Vec<String> {
        match self {
            Lookup::Found(c) => vec![c.name.clone()],
            Lookup::Ambiguous(list) | Lookup::Unknown(list) => list.iter().map(|c| c.name.clone()).collect(),
        }
    }
}
