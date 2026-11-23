//! Play mode's verbs and aliases, v1 (REQ-CON-10): turns a typed line
//! into an action for the engine. Matching nouns to things in the room
//! is the engine's job; this only splits verb from noun.

use super::session::COMPASS_ALIASES;

/// What a line of play-mode input asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    Look,
    Go(String),
    Take(String),
    Examine(String),
    Inventory,
    /// A known verb missing its noun, such as a bare `take`: a reply, not
    /// an action.
    Incomplete(String),
    /// Not understood; the console replies and nothing changes.
    Unknown(String),
}

/// Directions a player can type with no `go`.
pub(super) const DIRECTIONS: &[&str] = &[
    "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest", "up", "down", "in", "out",
];

/// Words dropped before a noun: `take the lantern`, `look at the well`.
const ARTICLES: &[&str] = &["the", "a", "an"];

pub fn parse(line: &str) -> Verb {
    let lower = line.trim().to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let Some((&verb, rest)) = words.split_first() else {
        return Verb::Unknown(String::new());
    };

    if rest.is_empty() {
        if let Some(direction) = direction(verb) {
            return Verb::Go(direction);
        }
    }
    match verb {
        "look" | "l" => match rest {
            [] => Verb::Look,
            ["at", noun @ ..] => with_noun(noun, Verb::Examine, "Look at what?"),
            noun => with_noun(noun, Verb::Examine, "Look at what?"),
        },
        "go" | "walk" | "move" => match rest {
            [] => Verb::Incomplete("Go where?".to_string()),
            [one] => Verb::Go(direction(one).unwrap_or_else(|| one.to_string())),
            more => Verb::Go(more.join(" ")),
        },
        "take" | "get" => match rest {
            ["up", noun @ ..] if !noun.is_empty() => with_noun(noun, Verb::Take, "Take what?"),
            noun => with_noun(noun, Verb::Take, "Take what?"),
        },
        "pick" => match rest {
            ["up", noun @ ..] => with_noun(noun, Verb::Take, "Pick up what?"),
            _ => Verb::Unknown(line.trim().to_string()),
        },
        "examine" | "x" | "inspect" => with_noun(rest, Verb::Examine, "Examine what?"),
        "inventory" | "i" | "inv" if rest.is_empty() => Verb::Inventory,
        _ => Verb::Unknown(line.trim().to_string()),
    }
}

/// The full direction name for a typed direction or its short form.
fn direction(typed: &str) -> Option<String> {
    if DIRECTIONS.contains(&typed) {
        return Some(typed.to_string());
    }
    COMPASS_ALIASES.iter().find(|(short, _)| *short == typed).map(|(_, long)| long.to_string())
}

fn with_noun(words: &[&str], make: fn(String) -> Verb, ask: &str) -> Verb {
    let words: Vec<&str> = match words {
        [first, rest @ ..] if ARTICLES.contains(first) => rest.to_vec(),
        _ => words.to_vec(),
    };
    if words.is_empty() {
        Verb::Incomplete(ask.to_string())
    } else {
        make(words.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbs_and_aliases() {
        assert_eq!(parse("look"), Verb::Look);
        assert_eq!(parse("L"), Verb::Look);
        assert_eq!(parse("n"), Verb::Go("north".into()));
        assert_eq!(parse("north"), Verb::Go("north".into()));
        assert_eq!(parse("go sw"), Verb::Go("southwest".into()));
        assert_eq!(parse("go behind"), Verb::Go("behind".into()));
        assert_eq!(parse("take the lantern"), Verb::Take("lantern".into()));
        assert_eq!(parse("get brass lantern"), Verb::Take("brass lantern".into()));
        assert_eq!(parse("pick up rope"), Verb::Take("rope".into()));
        assert_eq!(parse("x lantern"), Verb::Examine("lantern".into()));
        assert_eq!(parse("look at the well"), Verb::Examine("well".into()));
        assert_eq!(parse("i"), Verb::Inventory);
        assert_eq!(parse("inventory"), Verb::Inventory);
    }

    #[test]
    fn incomplete_and_unknown() {
        assert_eq!(parse("take"), Verb::Incomplete("Take what?".into()));
        assert_eq!(parse("take the"), Verb::Incomplete("Take what?".into()));
        assert_eq!(parse("go"), Verb::Incomplete("Go where?".into()));
        assert_eq!(parse("dance wildly"), Verb::Unknown("dance wildly".into()));
        assert_eq!(parse("n please"), Verb::Unknown("n please".into()));
    }
}
