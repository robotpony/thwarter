//! Play's verbs and their aliases (REQ-ENG-51 to 54): the built-in verbs,
//! a table of aliases with compiled-in defaults that a world's `[verbs]`
//! adds to, and the forgiving match a typed verb goes through.

use std::collections::BTreeMap;

/// The verbs the parser itself knows. Every other verb is a thing's.
pub const BUILT_IN: &[&str] = &["look", "examine", "go", "take", "inventory", "search", "where", "drop", "give"];

/// The compiled-in aliases.
pub const DEFAULTS: &str = include_str!("../builtins/verbs.toml");

/// The alias table: what each alias means.
#[derive(Debug, Clone, PartialEq)]
pub struct Verbs {
    aliases: BTreeMap<String, String>,
    /// The aliases a world added, which `validate` checks.
    added: Vec<String>,
}

impl Default for Verbs {
    fn default() -> Self {
        let mut verbs = Verbs { aliases: BTreeMap::new(), added: Vec::new() };
        let table: toml::Table = DEFAULTS.parse().expect("src/builtins/verbs.toml parses");
        verbs.apply(table.get("verbs"), "verbs.toml", &mut Vec::new());
        verbs.added.clear();
        verbs
    }
}

impl Verbs {
    /// Add a `[verbs]` table's aliases over these. A value that isn't a
    /// one-word verb is a warning, named after `source`.
    pub fn apply(&mut self, table: Option<&toml::Value>, source: &str, warnings: &mut Vec<String>) {
        let Some(table) = table else { return };
        let Some(table) = table.as_table() else {
            warnings.push(format!("{}: `[verbs]` is a table of `alias = \"verb\"`", source));
            return;
        };
        for (alias, verb) in table {
            match verb.as_str().map(|v| v.trim().to_lowercase()).filter(|v| !v.is_empty() && !v.contains(char::is_whitespace)) {
                Some(verb) => {
                    let alias = alias.trim().to_lowercase();
                    self.added.push(alias.clone());
                    self.aliases.insert(alias, verb);
                }
                None => warnings.push(format!("{}: `[verbs] {}` should be one verb, like `{} = \"take\"`", source, alias, alias)),
            }
        }
    }

    /// The verb `alias` means, if it's an alias.
    pub fn alias(&self, alias: &str) -> Option<&str> {
        self.aliases.get(alias).map(String::as_str)
    }

    /// The aliases a world's `[verbs]` added or changed, with their verbs.
    pub fn added(&self) -> impl Iterator<Item = (&str, &str)> {
        self.added.iter().filter_map(|a| self.aliases.get(a).map(|v| (a.as_str(), v.as_str())))
    }

    /// Every alias and the verb it means, sorted.
    pub fn aliases(&self) -> impl Iterator<Item = (&str, &str)> {
        self.aliases.iter().map(|(a, v)| (a.as_str(), v.as_str()))
    }
}

/// Whether `typed` is one small typo from `verb`: one letter added,
/// missing, changed, or two neighbours swapped.
pub fn one_typo(typed: &str, verb: &str) -> bool {
    let a: Vec<char> = typed.chars().collect();
    let b: Vec<char> = verb.chars().collect();
    if a == b {
        return false;
    }
    match a.len() as isize - b.len() as isize {
        0 => {
            let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
            match diff.as_slice() {
                [_] => true,
                [i, j] => *j == i + 1 && a[*i] == b[*j] && a[*j] == b[*i],
                _ => false,
            }
        }
        1 => (0..a.len()).any(|skip| a.iter().enumerate().filter(|(i, _)| *i != skip).map(|(_, c)| *c).eq(b.iter().copied())),
        -1 => one_typo(verb, typed),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_and_a_world_table() {
        let mut verbs = Verbs::default();
        assert_eq!(verbs.alias("x"), Some("examine"));
        assert_eq!(verbs.alias("get"), Some("take"));
        let mut warnings = Vec::new();
        let table: toml::Value = "yank = \"pull\"\nget = \"grab\"\nbad = \"two words\"".parse::<toml::Table>().unwrap().into();
        verbs.apply(Some(&table), "world.toml", &mut warnings);
        assert_eq!(verbs.alias("yank"), Some("pull"));
        assert_eq!(verbs.alias("get"), Some("grab"));
        assert_eq!(warnings, ["world.toml: `[verbs] bad` should be one verb, like `bad = \"take\"`"]);
    }

    #[test]
    fn a_typo_is_one_letter_added_missing_changed_or_swapped() {
        for typed in ["lookl", "lok", "lool", "olok"] {
            assert!(one_typo(typed, "look"), "{}", typed);
        }
        for typed in ["look", "lk", "kool", "lookup"] {
            assert!(!one_typo(typed, "look"), "{}", typed);
        }
    }
}
