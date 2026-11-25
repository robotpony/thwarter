//! The player command language every client shares
//! (`design/PLAYER_COMMANDS.md`): a typed line becomes an action for
//! `GameEngine::act`, or a reply that changes nothing (REQ-PC-1 to 15).

use crate::engines::location_view::expand_direction;
use crate::engines::{GameEngine, WorldEngine};
use crate::models::game::{PlayAction, PlayState};

/// What a typed line asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    /// An action for `GameEngine::act`.
    Action(PlayAction),
    /// A known verb missing its noun (`Take what?`): a reply, not an
    /// action. It changes nothing and counts no turn.
    Reply(String),
    /// A line the engine can't read: `I don't understand "<line>".`
    /// It changes nothing and counts no turn (REQ-PC-5); a client may log
    /// the line.
    Unknown(String),
}

/// Directions a player can type with no `go` (REQ-PC-7).
pub const DIRECTIONS: &[&str] = &[
    "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest", "up", "down", "in", "out",
];

/// Words dropped before a noun: `take the lantern`, `look at the well`.
const ARTICLES: &[&str] = &["the", "a", "an"];

impl GameEngine {
    /// Parse one typed line (REQ-PC-1). The built-in verbs come first, so
    /// an object action named `take` never shadows `take` (REQ-PC-15).
    /// Any other verb with a noun is the object's named action
    /// (REQ-PC-13), unless the noun names nothing in scope and no thing in
    /// the world has that action, which isn't understood (REQ-PC-14).
    pub fn parse_command(&self, world: &WorldEngine, line: &str, state: &PlayState) -> Parsed {
        // The room's own name examines it, as `look around` does (REQ-PC-10).
        let typed = line.trim().to_lowercase();
        let typed = typed.strip_prefix("the ").unwrap_or(&typed).trim();
        if let Some(room) = world.location_at(&state.location, crate::engines::At::play(state)) {
            if !typed.is_empty() && direction(typed).is_none() && (typed == room.name.to_lowercase() || typed == room.id.to_lowercase()) {
                return Parsed::Action(PlayAction::Examine(typed.to_string()));
            }
        }
        // A reply the line gets: a chapter's `## When:` wording first
        // (`design/ENGINE_TEXT.md` › Story overrides), else `[engine]`'s.
        let say = |key: &str, args: &[(&str, &str)]| -> String {
            match self.story_reply(world, state, key, None, args) {
                Some(text) => crate::render::prose_plain(crate::engines::layers::strip_comments(&text).trim(), &|n| world.link_for(n)),
                None => world.texts().text_at("engine", key, args, state.turn),
            }
        };
        // The verb, forgiven (REQ-ENG-52 to 54): an alias or a unique
        // prefix becomes the verb it means; several prefixes ask; a typo
        // gets a suggestion that runs nothing.
        let typed = line;
        let trimmed = line.trim_start();
        let (head, rest) = trimmed.split_at(trimmed.find(char::is_whitespace).unwrap_or(trimmed.len()));
        let first = head.to_lowercase();
        let resolved;
        let line = match self.resolve_verb(world, state, &first) {
            VerbMatch::Keep => line,
            VerbMatch::Means(verb) => {
                resolved = format!("{}{}", verb, rest);
                resolved.as_str()
            }
            VerbMatch::Ask(verbs) => return Parsed::Reply(say("which_verb", &[("or", &or_list(&verbs, false))])),
            VerbMatch::Suggest(verbs) => return Parsed::Unknown(say("did_you_mean", &[("or", &or_list(&verbs, true)), ("line", line.trim())])),
        };
        let parsed = match parse_words(line, &say) {
            Words::Other { verb, noun, with } => {
                let known = world.has_action(&verb);
                match noun {
                    None if known => Parsed::Reply(say("no_noun", &[("verb", &verb)])),
                    Some(noun) if known || self.names_anything(world, state, &noun) => Parsed::Action(PlayAction::Use { verb, noun, with }),
                    _ => unknown(line, &say),
                }
            }
            Words::Parsed(parsed) => parsed,
        };
        // Not understood quotes the line as typed, not as an alias read it.
        match parsed {
            Parsed::Unknown(_) => unknown(typed, &say),
            other => other,
        }
    }
}

/// What a typed verb comes to (REQ-ENG-52, 53).
enum VerbMatch {
    /// Read the line as typed.
    Keep,
    /// The verb an alias or a unique prefix means.
    Means(String),
    /// A prefix of several verbs: ask which.
    Ask(Vec<String>),
    /// One small typo from these: suggest them, run nothing.
    Suggest(Vec<String>),
}

impl GameEngine {
    /// How the line's first word reads, the first match winning: a
    /// direction or its shorthand, a built-in verb, an alias, a verb a
    /// world thing answers to, then a unique prefix of two letters or more
    /// among the built-in verbs and those in scope. With none, a word one
    /// typo from one of those or an alias is a suggestion (REQ-ENG-52 to 54).
    fn resolve_verb(&self, world: &WorldEngine, state: &PlayState, word: &str) -> VerbMatch {
        use crate::engines::verbs::{one_typo, BUILT_IN};
        if word.is_empty() || direction(word).is_some() || BUILT_IN.contains(&word) || PHRASES.contains(&word) {
            return VerbMatch::Keep;
        }
        if let Some(verb) = world.verbs().alias(word) {
            return VerbMatch::Means(verb.to_string());
        }
        if world.has_action(word) {
            return VerbMatch::Keep;
        }
        let mut candidates: Vec<String> = BUILT_IN.iter().map(|v| v.to_string()).collect();
        candidates.extend(self.verbs_in_scope(world, state));
        if word.chars().count() >= 2 {
            let fits: Vec<String> = candidates.iter().filter(|v| v.starts_with(word)).cloned().collect();
            match fits.as_slice() {
                [] => {}
                [one] => return VerbMatch::Means(one.clone()),
                _ => return VerbMatch::Ask(fits),
            }
        }
        candidates.extend(world.verbs().aliases().map(|(a, _)| a.to_string()));
        let mut near: Vec<String> = candidates.into_iter().filter(|v| one_typo(word, v)).collect();
        near.sort();
        near.dedup();
        if near.is_empty() { VerbMatch::Keep } else { VerbMatch::Suggest(near) }
    }
}

/// Words the parser reads as the start of a two-word verb.
const PHRASES: &[&str] = &["pick"];

/// Verbs joined with "or", each in plain quotes when `quoted`.
fn or_list(verbs: &[String], quoted: bool) -> String {
    let shown: Vec<String> = verbs.iter().map(|v| if quoted { format!("\"{}\"", v) } else { v.clone() }).collect();
    match shown.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} or {}", init.join(", "), last),
    }
}

/// A line split into a built-in verb, or a verb and noun to try as an
/// object action.
enum Words {
    Parsed(Parsed),
    Other { verb: String, noun: Option<String>, with: Option<String> },
}

/// How the parser words a reply: a key and its slots.
type Say<'a> = dyn Fn(&str, &[(&str, &str)]) -> String + 'a;

fn unknown(line: &str, say: &Say) -> Parsed {
    Parsed::Unknown(say("not_understood", &[("line", line.trim())]))
}

/// The syntax alone: no world, no state (REQ-PC-2 to 8).
fn parse_words(line: &str, say: &Say) -> Words {
    use PlayAction::*;
    let ask = |key: &str| say(key, &[]);
    let lower = line.trim().to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let Some((&verb, rest)) = words.split_first() else {
        return Words::Parsed(unknown(line, say));
    };
    let done = |p: Parsed| Words::Parsed(p);

    if rest.is_empty() {
        if let Some(direction) = direction(verb) {
            return done(Parsed::Action(Go(direction)));
        }
    }
    match verb {
        "look" => match rest {
            [] => done(Parsed::Action(Look)),
            ["at", noun @ ..] => done(with_noun(noun, Examine, ask("look_at_what"))),
            noun => done(with_noun(noun, Examine, ask("look_at_what"))),
        },
        "go" => match rest {
            [] => done(Parsed::Reply(ask("go_where"))),
            [one] => done(Parsed::Action(Go(direction(one).unwrap_or_else(|| one.to_string())))),
            more => done(Parsed::Action(Go(more.join(" ")))),
        },
        "take" => match rest {
            ["up", noun @ ..] if !noun.is_empty() => done(with_noun(noun, Take, ask("take_what"))),
            noun => done(with_noun(noun, Take, ask("take_what"))),
        },
        "pick" => match rest {
            ["up", noun @ ..] => done(with_noun(noun, Take, ask("pick_up_what"))),
            _ => done(unknown(line, say)),
        },
        "examine" => done(with_noun(rest, Examine, ask("examine_what"))),
        "inventory" if rest.is_empty() => done(Parsed::Action(Inventory)),
        "search" if rest.is_empty() => done(Parsed::Action(Search)),
        // Built-in verbs used another way (`inventory lamp`, `n please`)
        // aren't read.
        "inventory" => done(unknown(line, say)),
        _ if direction(verb).is_some() => done(unknown(line, say)),
        _ => {
            // `listen to the chest`, `talk to the captain` (REQ-PC-20).
            let rest = match rest {
                ["to" | "at", more @ ..] => more,
                _ => rest,
            };
            // `hit window with stick`: what the action uses follows `with`.
            let (noun, with) = match rest.iter().position(|w| *w == "with") {
                Some(at) => (&rest[..at], Some(strip_article(&rest[at + 1..]))),
                None => (rest, None),
            };
            let noun = strip_article(noun);
            let with = with.filter(|w| !w.is_empty()).map(|w| w.join(" "));
            Words::Other { verb: verb.to_string(), noun: (!noun.is_empty()).then(|| noun.join(" ")), with }
        }
    }
}

/// The full direction name for a typed direction or its short form.
fn direction(typed: &str) -> Option<String> {
    let full = expand_direction(typed);
    DIRECTIONS.contains(&full.as_str()).then_some(full)
}

fn strip_article<'a>(words: &[&'a str]) -> Vec<&'a str> {
    match words {
        [first, rest @ ..] if ARTICLES.contains(first) => rest.to_vec(),
        _ => words.to_vec(),
    }
}

fn with_noun(words: &[&str], make: fn(String) -> PlayAction, ask: String) -> Parsed {
    let words = strip_article(words);
    if words.is_empty() {
        Parsed::Reply(ask)
    } else {
        Parsed::Action(make(words.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::texts::Texts;
    use PlayAction::*;

    /// The syntax alone, as verbs.rs had it.
    fn parse(line: &str) -> Parsed {
        let texts = Texts::defaults();
        let say = |key: &str, args: &[(&str, &str)]| texts.text_at("engine", key, args, 0);
        // The default aliases, as `parse_command` reads them first.
        let verbs = crate::engines::verbs::Verbs::default();
        let trimmed = line.trim_start();
        let (head, rest) = trimmed.split_at(trimmed.find(char::is_whitespace).unwrap_or(trimmed.len()));
        let resolved = match verbs.alias(&head.to_lowercase()) {
            Some(verb) if super::direction(head).is_none() => format!("{}{}", verb, rest),
            _ => line.to_string(),
        };
        let parsed = match parse_words(&resolved, &say) {
            Words::Parsed(p) => p,
            Words::Other { verb, noun, with } => match noun {
                Some(noun) => Parsed::Action(Use { verb, noun, with }),
                None => unknown(line, &say),
            },
        };
        match parsed {
            Parsed::Unknown(_) => unknown(line, &say),
            other => other,
        }
    }
    fn act(a: PlayAction) -> Parsed {
        Parsed::Action(a)
    }

    #[test]
    fn verbs_and_aliases() {
        assert_eq!(parse("look"), act(Look));
        assert_eq!(parse("L"), act(Look));
        assert_eq!(parse("n"), act(Go("north".into())));
        assert_eq!(parse("north"), act(Go("north".into())));
        assert_eq!(parse("u"), act(Go("up".into())));
        assert_eq!(parse("go sw"), act(Go("southwest".into())));
        assert_eq!(parse("go behind"), act(Go("behind".into())));
        assert_eq!(parse("go through the hedge"), act(Go("through the hedge".into())));
        assert_eq!(parse("take the lantern"), act(Take("lantern".into())));
        assert_eq!(parse("get brass lantern"), act(Take("brass lantern".into())));
        assert_eq!(parse("pick up rope"), act(Take("rope".into())));
        assert_eq!(parse("x lantern"), act(Examine("lantern".into())));
        assert_eq!(parse("look at the well"), act(Examine("well".into())));
        assert_eq!(parse("look around"), act(Examine("around".into())));
        assert_eq!(parse("i"), act(Inventory));
        assert_eq!(parse("inventory"), act(Inventory));
        assert_eq!(parse("  Search "), act(Search));
        assert_eq!(parse("pull the lever"), act(Use { verb: "pull".into(), noun: "lever".into(), with: None }));
        assert_eq!(parse("hit the window with a stick"), act(Use { verb: "hit".into(), noun: "window".into(), with: Some("stick".into()) }));
        assert_eq!(parse("hit window with"), act(Use { verb: "hit".into(), noun: "window".into(), with: None }));
        assert_eq!(parse("listen to the chest"), act(Use { verb: "listen".into(), noun: "chest".into(), with: None }));
        assert_eq!(parse("talk to captain"), act(Use { verb: "talk".into(), noun: "captain".into(), with: None }));
        assert_eq!(parse("stare at a wall"), act(Use { verb: "stare".into(), noun: "wall".into(), with: None }));
        // Other prepositions change the meaning and stay.
        assert_eq!(parse("peer into the well"), act(Use { verb: "peer".into(), noun: "into the well".into(), with: None }));
        // `go to` is left alone: an exit's name is free text.
        assert_eq!(parse("go to the river"), act(Go("to the river".into())));
    }

    #[test]
    fn incomplete_and_unknown() {
        assert_eq!(parse("take"), Parsed::Reply("Take what?".into()));
        assert_eq!(parse("take the"), Parsed::Reply("Take what?".into()));
        assert_eq!(parse("go"), Parsed::Reply("Go where?".into()));
        assert_eq!(parse("look at"), Parsed::Reply("Look at what?".into()));
        assert_eq!(parse("n please"), Parsed::Unknown("I don't understand \"n please\".".into()));
        assert_eq!(parse(""), Parsed::Unknown("I don't understand \"\".".into()));
        assert_eq!(parse("i lamp"), Parsed::Unknown("I don't understand \"i lamp\".".into()));
    }

    fn world() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("yard.md", "---\nid: yard\ntitle: Yard\ntype: location\nobjects: [lever, idol]\n---\nGrass.\n");
        write("hall.md", "---\nid: hall\ntitle: Hall\ntype: location\nobjects: [gong]\n---\nStone.\n");
        write("objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { sets: { this.pulled: yes } }\n---\nA lever.\n\n<!-- thwarter:action pull -->\nClunk.\n");
        // An action named like a built-in verb.
        write("objects/idol.md", "---\nid: idol\ntitle: Idol\ntype: object\nobject_type: x\nactions:\n  take: {}\n---\nGold.\n\n<!-- thwarter:action take -->\nIt burns.\n");
        // An action only in a state the gong isn't in yet.
        write("objects/gong/index.md", "---\nid: gong\ntitle: Gong\ntype: object\nobject_type: x\nstate: still\n---\nBrass.\n");
        write("objects/gong/still.md", "---\n---\n");
        write("objects/gong/ringing.md", "---\nactions:\n  hush: { becomes: { gong: still } }\n---\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        (dir, engine)
    }

    #[test]
    fn other_verbs_are_object_actions_when_something_answers() {
        let (_d, world) = world();
        let game = GameEngine::new();
        let mut state = game.start(&world, "yard").unwrap();
        let p = |state: &PlayState, line: &str| game.parse_command(&world, line, state);
        let use_ = |verb: &str, noun: &str| Parsed::Action(Use { verb: verb.into(), noun: noun.into(), with: None });

        // REQ-PC-13: a verb some thing has.
        assert_eq!(p(&state, "pull the lever"), use_("pull", "lever"));
        assert_eq!(p(&state, "pull"), Parsed::Reply("What do you want to pull?".into()));
        // A verb no thing has, on a thing that's here: the engine says
        // you can't.
        assert_eq!(p(&state, "lick lever"), use_("lick", "lever"));
        let Parsed::Action(lick) = p(&state, "lick lever") else { panic!() };
        assert_eq!(game.act(&world, &mut state, &lick).text, "Nothing happens.");
        // REQ-PC-14: a verb no thing has, on nothing here.
        assert_eq!(p(&state, "dance wildly"), Parsed::Unknown("I don't understand \"dance wildly\".".into()));
        assert_eq!(p(&state, "dance"), Parsed::Unknown("I don't understand \"dance\".".into()));
        // A verb some thing has, on nothing here: the engine says so.
        assert_eq!(p(&state, "pull wildly"), use_("pull", "wildly"));
        // An action in a state the thing isn't in yet still counts.
        assert_eq!(p(&state, "hush the wind"), use_("hush", "wind"));
        // The room is something to name.
        assert_eq!(p(&state, "lick yard"), use_("lick", "yard"));
        // REQ-PC-15: `take` is the built-in verb, even for the idol.
        assert_eq!(p(&state, "take idol"), Parsed::Action(Take("idol".into())));
        assert!(game.act(&world, &mut state, &Take("idol".into())).text.starts_with("You take"));
        // A reply counts no turn, since it never reaches `act`.
        let turn = state.turn;
        let _ = p(&state, "dance");
        assert_eq!(state.turn, turn);
    }
}
