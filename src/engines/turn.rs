//! A turn's tags and the sections they match (`design/SECTIONS.md`,
//! REQ-ENG-36). Content never tests anything: each turn has a set of
//! tags, and a `## When:` section is shown when all its tags are in it
//! (`## Unless:`, when none are).
//!
//! The set holds `@` for each thing the player holds and the room they're
//! in, `#` for the chapter, and `#` for the current state of the thing
//! being shown. Tags on other things' states and on values wait on Bruce.
//!
//! A section is description when it matches without an action's tags,
//! and a reply when it names the turn's `#<verb>`. Replies match after
//! the action, against its outcome (`design/SECTIONS.md` › Outcomes and
//! replies): the state the target moved into, or the reason it didn't
//! (`no-action`, `wrong-key`). A section in a state file that names no
//! outcome means "ended up here". A thing's own reply replaces its
//! template's for the same action and outcome.

use crate::engines::bundles::Thing;
use crate::engines::layers::{strip_comments, Section};
use crate::engines::{EntityKind, GameEngine, WorldEngine};
use crate::models::game::PlayState;
use std::collections::BTreeSet;

/// The tags of one turn.
pub type Tags = BTreeSet<String>;

/// A section with where it came from: the state file it's in, if any,
/// and whether it's the thing's own or its template's.
struct Found<'a> {
    section: &'a Section,
    state_file: Option<&'a str>,
    own: bool,
}

/// A thing's sections for its state `current`: its templates' (the most
/// distant first) and their files for that state, then its own index and
/// its own file for that state. Reserved replies are left out.
fn sections<'a>(thing: &'a Thing, current: Option<&'a str>) -> Vec<Found<'a>> {
    let mut out = Vec::new();
    let mut push = |layer: &'a crate::engines::layers::Layer, state_file: Option<&'a str>, own: bool| {
        for section in layer.sections.iter().filter(|s| s.reserved().is_none() && !rewords_a_reply(s)) {
            out.push(Found { section, state_file, own });
        }
    };
    for template in &thing.templates {
        push(template, None, false);
    }
    if let Some(s) = current {
        for layer in thing.template_states.get(s).into_iter().flatten() {
            push(layer, Some(s), false);
        }
    }
    push(&thing.index, None, true);
    if let Some(s) = current {
        if let Some(layer) = thing.states.get(s) {
            push(layer, Some(s), true);
        }
    }
    out
}

fn prose_of(section: &Section) -> Option<String> {
    let text = section.layer.body.trim();
    (!strip_comments(text).trim().is_empty()).then(|| text.to_string())
}

/// Who may reword a built-in reply with a `## When:` section
/// (`design/ENGINE_TEXT.md` › Story overrides).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    /// The thing it's about, its room, the chapter, then `[engine]`.
    Thing,
    /// The chapter, then `[engine]`: the reply is about no thing.
    World,
}

/// Each `[engine]` reply's outcome tags and chain: a section whose `#`
/// tags are exactly these rewords it. `src/builtins/text/engine.toml`
/// gives the same after each key, which a test checks.
pub const OUTCOMES: &[(&str, &str, Chain)] = &[
    ("not_understood", "#not-understood", Chain::World),
    ("no_noun", "#no-noun", Chain::World),
    ("go_where", "#go #no-noun", Chain::World),
    ("take_what", "#take #no-noun", Chain::World),
    ("pick_up_what", "#take #no-noun", Chain::World),
    ("examine_what", "#examine #no-noun", Chain::World),
    ("look_at_what", "#examine #no-noun", Chain::World),
    ("not_here", "#not-here", Chain::World),
    ("which_verb", "#ambiguous-verb", Chain::World),
    ("did_you_mean", "#not-understood #suggest", Chain::World),
    ("which", "#ambiguous", Chain::Thing),
    ("who", "#ambiguous", Chain::Thing),
    ("nowhere", "#nowhere", Chain::World),
    ("nowhere_gone", "#nowhere", Chain::World),
    ("where", "#where", Chain::World),
    ("look_toward", "#look-toward", Chain::World),
    ("look_toward_door", "#look-toward", Chain::Thing),
    ("look_toward_none", "#look-toward #no-exit", Chain::World),
    ("no_exit", "#go #no-exit", Chain::World),
    ("door_locked", "#go #locked", Chain::Thing),
    ("door_closed", "#go #closed", Chain::Thing),
    ("taken", "#take", Chain::Thing),
    ("already_have", "#take #carried", Chain::Thing),
    ("take_person", "#take #person", Chain::Thing),
    ("take_scenery", "#take #scenery", Chain::Thing),
    ("immovable", "#take #immovable", Chain::Thing),
    ("drop_what", "#drop #no-noun", Chain::World),
    ("not_carried", "#not-carried", Chain::World),
    ("dropped", "#drop", Chain::Thing),
    ("give_what", "#give #no-noun", Chain::World),
    ("give_to_whom", "#give #no-person", Chain::World),
    ("give_refused", "#give #refused", Chain::Thing),
    ("take_all_nothing", "#take #nothing", Chain::World),
    ("jump", "#jump #nothing", Chain::World),
    ("run", "#run #nothing", Chain::World),
    ("kick", "#kick #nothing", Chain::World),
    ("hit", "#hit #nothing", Chain::World),
    ("jump_thing", "#jump #nothing", Chain::Thing),
    ("run_thing", "#run #nothing", Chain::Thing),
    ("kick_thing", "#kick #nothing", Chain::Thing),
    ("hit_thing", "#hit #nothing", Chain::Thing),
    ("thrown", "#throw", Chain::Thing),
    ("breaks", "#break", Chain::Thing),
    ("wont_break", "#break #refused", Chain::Thing),
    ("already_broken", "#break #nothing", Chain::Thing),
    ("stack_on_what", "#stack #no-noun", Chain::World),
    ("stacked", "#stack", Chain::Thing),
    ("cant_stack", "#stack #refused", Chain::Thing),
    ("wont_move", "#push #immovable", Chain::Thing),
    ("pushed", "#push", Chain::Thing),
    ("waited", "#wait", Chain::World),
    ("nothing_to_repeat", "#again #nothing", Chain::World),
    ("it_unknown", "#not-understood #it", Chain::World),
    ("nothing_special", "#examine #nothing", Chain::Thing),
    ("search_nothing", "#search #nothing", Chain::World),
    ("search_found", "#search #found", Chain::World),
    ("look_closely_nothing", "#examine-room #nothing", Chain::World),
    ("look_closely_found", "#examine-room #found", Chain::World),
    ("found", "#search #found", Chain::World),
    ("nothing", "#nothing", Chain::Thing),
    ("acted", "#acted", Chain::Thing),
    ("refused", "#refused", Chain::Thing),
    ("talk_nothing", "#talk #nothing", Chain::Thing),
    ("talk_to_whom", "#talk #no-person", Chain::World),
    ("ask_whom", "#ask #no-person", Chain::World),
    ("ask_about_what", "#ask #no-noun", Chain::Thing),
    ("ask_nothing", "#ask #nothing", Chain::Thing),
    ("cant_verb_person", "#person", Chain::Thing),
];

/// A reply key's outcome tags and chain, if it has them.
pub fn outcome_of(key: &str) -> Option<(Vec<&'static str>, Chain)> {
    OUTCOMES.iter().find(|(k, _, _)| *k == key).map(|(_, tags, chain)| (tags.split_whitespace().collect(), *chain))
}

/// Whether a section rewords a built-in reply: its `#` tags are exactly
/// one with a reserved outcome name. Such a section is never description
/// or an action's reply.
fn rewords_a_reply(section: &Section) -> bool {
    if section.unless {
        return false;
    }
    let mut hashes: Vec<&str> = section.tags.iter().map(String::as_str).filter(|t| t.starts_with('#')).collect();
    hashes.sort();
    OUTCOMES.iter().any(|(_, tags, _)| {
        let mut want: Vec<&str> = tags.split_whitespace().collect();
        want.sort();
        want == hashes && want.iter().any(|t| crate::engines::layers::RESERVED_TAGS.contains(t) && *t != "#ambiguous")
    })
}

/// One matching reply section: the thing it's in, its prose, and its
/// ` ```thwarter ` block (empty for none), with the file it came from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reply {
    pub kind: EntityKind,
    pub id: String,
    pub text: String,
    pub block: serde_yaml::Mapping,
    pub path: std::path::PathBuf,
}

/// What an action turn did to its target: the state it moved into, or
/// the reason it didn't move (`no-action`, `wrong-key`).
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome(pub String);

impl GameEngine {
    /// The tags every part of this turn shares: what the player holds,
    /// the room, and the chapter with its parents.
    pub fn turn_tags(&self, state: &PlayState) -> Tags {
        let mut tags = Tags::new();
        for id in &state.inventory {
            tags.insert(format!("@{}", id));
        }
        tags.insert(format!("@{}", state.location));
        if let Some(chapter) = &state.chapter {
            for name in &chapter.path {
                tags.insert(format!("#{}", name));
            }
        }
        tags
    }

    /// `turn_tags`, plus `@thing:state` for every thing in a state, so a
    /// heading can test another thing's state from anywhere
    /// (`## When: @weather:rain`; `design/SECTIONS.md` › Open question 1).
    pub fn turn_tags_in(&self, world: &WorldEngine, state: &PlayState) -> Tags {
        let mut tags = self.turn_tags(state);
        for thing in world.things_iter() {
            if let Some(now) = self.current_state(world, state, thing.kind, &thing.id) {
                tags.insert(format!("@{}:{}", thing.id, now));
            }
        }
        tags
    }

    /// A thing's current state in play.
    pub(crate) fn current_state(&self, world: &WorldEngine, state: &PlayState, kind: EntityKind, id: &str) -> Option<String> {
        let thing = world.thing(kind, id)?;
        state
            .state_of(kind.type_name(), id)
            .filter(|s| thing.has_state(s))
            .map(String::from)
            .or_else(|| thing.starting_state.clone())
    }

    /// A thing's description for this turn: `base` (its prose) and then
    /// each of its sections that matches, in file order. A section in a
    /// state file applies only while the thing is in that state.
    pub(crate) fn describe(&self, world: &WorldEngine, state: &PlayState, kind: EntityKind, id: &str, base: &str) -> String {
        let Some(thing) = world.thing(kind, id) else { return base.to_string() };
        let current = self.current_state(world, state, kind, id);
        let mut tags = self.turn_tags_in(world, state);
        if let Some(s) = &current {
            tags.insert(format!("#{}", s));
        }
        let mut parts: Vec<String> = Vec::new();
        if !base.trim().is_empty() {
            parts.push(base.trim().to_string());
        }
        parts.extend(sections(thing, current.as_deref()).into_iter().filter(|f| f.section.matches(|t| tags.contains(t))).filter_map(|f| prose_of(f.section)));
        parts.join("\n\n")
    }

    /// `reply_sections`, each as the thing it came from and its text.
    pub(crate) fn replies(
        &self,
        world: &WorldEngine,
        state: &PlayState,
        verb: &str,
        target: (EntityKind, &str),
        with: Option<(EntityKind, &str)>,
        outcome: &Outcome,
    ) -> Vec<(EntityKind, String, String)> {
        self.reply_sections(world, state, verb, target, with, outcome).into_iter().filter(|r| !r.text.is_empty()).map(|r| (r.kind, r.id, r.text)).collect()
    }

    /// The replies to `verb` on `target`, used with `with`, after the
    /// action, given its outcome: the matching sections naming `#<verb>`
    /// in the target, then in what's used, then in the room, every match
    /// in that order, each with its ` ```thwarter ` block.
    pub(crate) fn reply_sections(
        &self,
        world: &WorldEngine,
        state: &PlayState,
        verb: &str,
        target: (EntityKind, &str),
        with: Option<(EntityKind, &str)>,
        outcome: &Outcome,
    ) -> Vec<Reply> {
        self.reply_sections_with(world, state, verb, target, with, outcome, &[])
    }

    /// `reply_sections`, with `extra` tags true this turn too, such as
    /// the topic asked about (`#the-war`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reply_sections_with(
        &self,
        world: &WorldEngine,
        state: &PlayState,
        verb: &str,
        target: (EntityKind, &str),
        with: Option<(EntityKind, &str)>,
        outcome: &Outcome,
        extra: &[String],
    ) -> Vec<Reply> {
        let action = format!("#{}", verb.trim().to_lowercase().replace(' ', "-"));
        let outcome_tag = format!("#{}", outcome.0);
        let mut tags = self.turn_tags_in(world, state);
        tags.insert(action.clone());
        tags.insert(format!("@{}", target.1));
        tags.extend(extra.iter().cloned());
        if let Some((_, id)) = with {
            tags.insert(format!("@{}", id));
        }
        let room = (EntityKind::Location, state.location.as_str());
        let mut out = Vec::new();
        let mut seen: Vec<(EntityKind, &str)> = Vec::new();
        for (kind, id) in std::iter::once(target).chain(with).chain(std::iter::once(room)) {
            if seen.contains(&(kind, id)) {
                continue;
            }
            seen.push((kind, id));
            let Some(thing) = world.thing(kind, id) else { continue };
            let is_target = (kind, id) == target;
            let current = self.current_state(world, state, kind, id);
            // A `#` tag: this thing's state (for the target, the state it
            // ended up in by this action), the outcome, or the chapter.
            let state_tag = |t: &str| -> bool {
                let name = &t[1..];
                if thing.has_state(name) {
                    return if is_target { outcome.0 == name } else { current.as_deref() == Some(name) };
                }
                t == outcome_tag || tags.contains(t)
            };
            let mut matched: Vec<(Vec<String>, bool, String, serde_yaml::Mapping, std::path::PathBuf)> = Vec::new();
            for found in sections(thing, current.as_deref()) {
                let section = found.section;
                if section.unless || !section.tags.iter().any(|t| *t == action) {
                    continue;
                }
                // Whether it names an outcome: any `#` tag besides the action.
                let names_outcome = section.tags.iter().any(|t| t.starts_with('#') && *t != action);
                let holds = section.tags.iter().all(|t| {
                    if t.starts_with('@') {
                        return tags.contains(t);
                    }
                    if *t == action {
                        // An action that shares a state's name (`#open`) also
                        // means ending up in it, unless the outcome is named.
                        return names_outcome || !is_target || !thing.has_state(&t[1..]) || outcome.0 == t[1..];
                    }
                    state_tag(t)
                });
                // A state file's reply that names no outcome: ended up here.
                let here = match (found.state_file, names_outcome) {
                    (Some(s), false) => !is_target || outcome.0 == s,
                    _ => true,
                };
                if !(holds && here) {
                    continue;
                }
                // Its action and outcome: the tags besides the action, and
                // for a state file's reply that names none, that state.
                let mut key: Vec<String> = section.tags.iter().filter(|t| **t != action).cloned().collect();
                if let (Some(s), false) = (found.state_file, names_outcome) {
                    key.push(format!("#{}", s));
                }
                // An action that shares a state's name means ending up in
                // it, so `#open` replaces a template's open state's reply.
                if !names_outcome && thing.has_state(&action[1..]) {
                    key.push(action.clone());
                }
                key.sort();
                key.dedup();
                if let Some(text) = prose_of(section).or_else(|| (!section.layer.metadata.is_empty()).then(String::new)) {
                    matched.push((key, found.own, text, section.layer.metadata.clone(), section.layer.path.clone()));
                }
            }
            // A thing's own reply replaces its template's for the same
            // action and outcome.
            let own_keys: Vec<Vec<String>> = matched.iter().filter(|m| m.1).map(|m| m.0.clone()).collect();
            let matched: Vec<_> = matched.into_iter().filter(|m| m.1 || !own_keys.contains(&m.0)).collect();
            // Several sections with the same heading are one reply's
            // wordings: the game's seed picks one each turn
            // (`design/STATES.md` › Clocks).
            let mut done: Vec<&Vec<String>> = Vec::new();
            for (key, ..) in &matched {
                if done.contains(&key) {
                    continue;
                }
                done.push(key);
                let same: Vec<&(Vec<String>, bool, String, serde_yaml::Mapping, std::path::PathBuf)> = matched.iter().filter(|m| &m.0 == key).collect();
                let salt = format!("{}#{}@{}", id, key.join(" "), state.turn);
                let (_, _, text, block, path) = same[crate::engines::clock::draw(state.seed, &salt, same.len() as u64) as usize].clone();
                out.push(Reply { kind, id: id.to_string(), text, block, path });
            }
        }
        out
    }

    /// A story's wording for the built-in reply `key` (`design/ENGINE_TEXT.md`
    /// › Story overrides): the `## When:` section whose `#` tags are
    /// exactly the reply's outcome tags, and whose `@` tags all hold, in
    /// the first place along its chain that has one: for a reply about a
    /// thing, `about` (its own sections over its templates'), then the
    /// room; then the chapter, innermost first. The most tags wins within
    /// a place. Its prose, with `{{ turn.x }}` filled from `args`.
    pub(crate) fn story_reply(
        &self,
        world: &WorldEngine,
        state: &PlayState,
        key: &str,
        about: Option<(EntityKind, &str)>,
        args: &[(&str, &str)],
    ) -> Option<String> {
        let (outcome, chain) = outcome_of(key)?;
        let mut want: Vec<&str> = outcome.clone();
        want.sort();
        let mut tags = self.turn_tags_in(world, state);
        if let Some((_, id)) = about {
            tags.insert(format!("@{}", id));
        }
        let fits = |s: &Section| -> bool {
            if s.unless {
                return false;
            }
            let mut hashes: Vec<&str> = s.tags.iter().map(String::as_str).filter(|t| t.starts_with('#')).collect();
            hashes.sort();
            hashes == want && s.tags.iter().filter(|t| t.starts_with('@')).all(|t| tags.contains(t))
        };
        // Each place's layers, the most authoritative first.
        let mut places: Vec<(Option<(EntityKind, String)>, Vec<&crate::engines::layers::Layer>)> = Vec::new();
        if chain == Chain::Thing {
            let room = (EntityKind::Location, state.location.clone());
            for (kind, id) in about.map(|(k, id)| (k, id.to_string())).into_iter().chain(std::iter::once(room)) {
                let Some(thing) = world.thing(kind, &id) else { continue };
                let current = self.current_state(world, state, kind, &id);
                let mut layers: Vec<&crate::engines::layers::Layer> = Vec::new();
                if let Some(layer) = current.as_deref().and_then(|s| thing.states.get(s)) {
                    layers.push(layer);
                }
                layers.push(&thing.index);
                if let Some(s) = current.as_deref() {
                    layers.extend(thing.template_states.get(s).into_iter().flatten().rev());
                }
                layers.extend(thing.templates.iter().rev());
                places.push((Some((kind, id)), layers));
            }
        }
        if let Some(key) = state.chapter.as_ref().and_then(|c| world.chapter_key(c)) {
            for n in (1..=key.path.len()).rev() {
                let part = crate::engines::bundles::ChapterKey { scope: key.scope.clone(), path: key.path[..n].to_vec() };
                if let Some(chapter) = world.chapters().iter().find(|c| c.key == part) {
                    places.push((None, vec![&chapter.layer]));
                }
            }
        }
        for (this, layers) in places {
            let best = layers
                .iter()
                .flat_map(|l| l.sections.iter())
                .filter(|s| fits(s))
                .filter_map(|s| prose_of(s).map(|text| (s.tags.len(), text)))
                .fold(None::<(usize, String)>, |best, (n, text)| match best {
                    Some((m, _)) if m >= n => best,
                    _ => Some((n, text)),
                });
            if let Some((_, text)) = best {
                let scope = crate::engines::variables::Scope {
                    at: crate::engines::At::play(state),
                    this: this.as_ref().map(|(k, id)| (*k, id.as_str())),
                    here: None,
                };
                return Some(world.fill_turn(&text, &scope, args));
            }
        }
        None
    }

    /// The room's `## When: #searched` section, if it has one that matches.
    pub(crate) fn searched(&self, world: &WorldEngine, state: &PlayState) -> Option<String> {
        let thing = world.thing(EntityKind::Location, &state.location)?;
        let mut tags = self.turn_tags_in(world, state);
        if let Some(s) = self.current_state(world, state, EntityKind::Location, &state.location) {
            tags.insert(format!("#{}", s));
        }
        tags.insert("#searched".to_string());
        std::iter::once(&thing.index)
            .chain(thing.templates.iter().rev())
            .flat_map(|l| l.sections.iter())
            .find(|s| s.reserved() == Some("#searched") && s.matches(|t| tags.contains(t)))
            .and_then(prose_of)
    }
}

#[cfg(test)]
mod tests {
    use crate::engines::{GameEngine, Parsed, WorldEngine};
    use crate::models::game::PlayState;

    fn world() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("bedroom.md", "---\nid: bedroom\ntitle: Bedroom\ntype: location\nobjects: [window, stick, lamp]\nconnections: { north: hall, up: hatch }\n---\nA small room.\n\n## When: @lamp\nThe lamp throws long shadows.\n\n## When: #searched\nYou look under the bed.\n");
        write("hall.md", "---\nid: hall\ntitle: Hall\ntype: location\nconnections: { south: bedroom }\n---\nA hall.\n\n## When: #hit @stick\nThe hall echoes with the crack.\n");
        // No `actions:` at all: the window answers through its sections.
        write("objects/window.md", "---\nid: window\ntitle: Window\ntype: object\nobject_type: x\n---\nA grimy window.\n\n## When: #hit @stick\nThe glass cracks.\n\n## Unless: #night\nDaylight comes through it.\n");
        write("objects/stick.md", "---\nid: stick\ntitle: Stick\ntype: object\nobject_type: x\n---\nA stick.\n");
        write("objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: x\n---\nA lamp.\n");
        // A door whose state and action share a name.
        write("doors/hatch/index.md", "---\nid: hatch\ntitle: Hatch\ntype: door\nstate: closed\nsides:\n  bedroom: { direction: up }\n  attic: { direction: down }\nactions:\n  open: { becomes: { this: open } }\n---\nA hatch.\n\n## When: #closed\nIt's bolted from this side.\n\n## When: #go #closed\nThe bolt holds.\n\n## When: #open\nThe hatch hangs open.\n");
        write("doors/hatch/closed.md", "---\ninherits: index\npassable: no\n---\n");
        write("doors/hatch/open.md", "---\ninherits: index\npassable: yes\n---\n");
        write("attic.md", "---\nid: attic\ntitle: Attic\ntype: location\nconnections: { down: hatch }\n---\nDust.\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        (dir, engine)
    }

    fn say(game: &GameEngine, world: &WorldEngine, state: &mut PlayState, line: &str) -> String {
        match game.parse_command(world, line, state) {
            Parsed::Action(a) => game.act(world, state, &a).text,
            Parsed::Reply(t) | Parsed::Unknown(t) => t,
        }
    }

    #[test]
    fn verbs_forgive_prefixes_aliases_and_typos() {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("kitchen.md", "---\nid: kitchen\ntitle: Kitchen\ntype: location\nobjects: [lever, pot]\ncharacters: [cook]\nconnections: { north: hall }\n---\nSteam.\n");
        write("hall.md", "---\nid: hall\ntitle: Hall\ntype: location\n---\nQuiet.\n");
        write("objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { sets: { this.pulled: yes } }\nvars: { pulled: no }\n---\nA lever.\n\n## When: #pull\nClunk.\n");
        write("objects/pot.md", "---\nid: pot\ntitle: Pot\ntype: object\nobject_type: x\n---\nA pot.\n");
        write("npcs/cook.md", "---\nid: cook\ntitle: Cook\ntype: character\n---\nBusy.\n\n## When: #talk\n\"Out of my kitchen.\"\n");
        write("world.toml", "[verbs]\nyank = \"pull\"\nn = \"pull\"\nlook = \"pull\"\nfly = \"soar\"\n");
        let mut world = WorldEngine::new();
        world.load_world(dir.path()).unwrap();
        let game = GameEngine::new();
        let mut state = game.start(&world, "kitchen").unwrap();
        let mut s = |line: &str| say(&game, &world, &mut state, line);

        // An alias from the table, the defaults' and the world's.
        assert_eq!(s("x pot"), "A pot.");
        assert_eq!(s("yank lever"), "Clunk.");
        // A unique prefix of two letters or more.
        assert_eq!(s("exa pot"), "A pot.");
        assert_eq!(s("pu lever"), "Clunk.");
        // Several fit: ask, never guess.
        assert_eq!(s("ta cook"), "Do you mean take or talk?");
        // An exact match beats a prefix: `in` is a direction.
        assert_eq!(s("in"), "You can't go in from here.");
        // A typo is a suggestion, and runs nothing.
        assert_eq!(s("lookl"), "Did you mean \"look\"?");
        assert_eq!(s("pul lever"), "Clunk.", "a prefix, not a typo");
        assert_eq!(s("pyll lever"), "Did you mean \"pull\"?");
        assert_eq!(s("dance"), "I don't understand \"dance\".");
        // Not understood quotes the line as typed.
        assert_eq!(s("i lamp"), "I don't understand \"i lamp\".");
        // The player's verbs here, for a client.
        let verbs = game.verbs(&world, &state);
        assert!(["look", "pull", "talk", "x", "yank"].iter().all(|v| verbs.contains(&v.to_string())), "{:?}", verbs);

        // Aliases that can't work are warned about.
        let warnings = world.current_warnings();
        for expected in ["`[verbs] n` is a direction", "`[verbs] look` is a built-in verb", "`[verbs] fly = \"soar\"`: nothing in the world answers to `soar`"] {
            assert!(warnings.iter().any(|w| w.contains(expected)), "{}: {:?}", expected, warnings);
        }
        assert!(!warnings.iter().any(|w| w.contains("yank")), "{:?}", warnings);
    }

    #[test]
    fn a_story_rewords_a_built_in_reply_by_its_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("forge.md", "---\nid: forge\ntitle: Forge\ntype: location\nobjects: [anvil, safe, lamp, box]\n---\nHot.\n\n## When: #take #immovable\nEverything here is bolted down.\n");
        write("objects/anvil.md", "---\nid: anvil\ntitle: Anvil\ntype: object\nobject_type: x\nweight: immovable\n---\nHeavy.\n\n## When: #take #immovable\nThe anvil laughs at you.\n");
        write("objects/safe.md", "---\nid: safe\ntitle: Safe\ntype: object\nobject_type: x\nweight: immovable\n---\nSteel.\n");
        write("objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: x\n---\nA lamp.\n\n## When: #nothing\nYou {{ turn.verb }} the lamp. It flickers.\n");
        write("objects/box/index.md", "---\nid: box\ntitle: Box\ntype: object\nobject_type: x\nstate: nothing\n---\nA box.\n");
        write("objects/box/nothing.md", "---\ninherits: index\n---\n");
        write("chapters/10-dark.md", "---\ntitle: Dark\n---\n## When: #not-here\nIn the dark, you can't find any {{ turn.noun }}.\n\n## When: #not-understood\nThe dark swallows \"{{ turn.line }}\".\n");
        let mut world = WorldEngine::new();
        world.load_world(dir.path()).unwrap();
        let game = GameEngine::new();
        let mut state = game.start(&world, "forge").unwrap();
        let s = |state: &mut PlayState, line: &str| say(&game, &world, state, line);

        // The thing's own wording, then its room's, then the built-in.
        assert_eq!(s(&mut state, "take anvil"), "The anvil laughs at you.");
        assert_eq!(s(&mut state, "take safe"), "Everything here is bolted down.");
        assert_eq!(s(&mut state, "take ghost"), "There's no ghost here.");
        // A slot in the prose form, and a section that rewords a reply is
        // never part of the description.
        assert_eq!(s(&mut state, "kick lamp"), "You kick the lamp. It flickers.");
        assert_eq!(s(&mut state, "x anvil"), "Heavy.");
        // A chapter rewords the replies about no thing, the parser's too.
        state.enter_chapter(&world, std::path::Path::new(""), "dark").unwrap();
        assert_eq!(s(&mut state, "take ghost"), "In the dark, you can't find any ghost.");
        assert_eq!(s(&mut state, "dance wildly"), "The dark swallows \"dance wildly\".");

        // A state named like an outcome is warned about.
        let warnings = world.current_warnings();
        assert!(warnings.iter().any(|w| w.contains("the state `nothing` has a reserved name")), "{:?}", warnings);
        // `{{ turn.x }}` is declared, so no variable warning.
        assert!(!warnings.iter().any(|w| w.contains("turn.")), "{:?}", warnings);
    }

    /// `engine.toml` gives each key's outcome tag and chain after it;
    /// they're the same as `OUTCOMES`, which play uses.
    #[test]
    fn engine_toml_comments_match_the_outcomes() {
        let mut seen = 0;
        let engine = crate::engines::texts::DEFAULTS.iter().find(|(name, _)| *name == "engine").unwrap().1;
        for line in engine.lines() {
            let Some((key, rest)) = line.split_once(" = ") else { continue };
            let Some((_, comment)) = rest.rsplit_once("  # ") else { continue };
            if !comment.starts_with('#') {
                continue;
            }
            let (tags, chain) = comment.split_once(", ").unwrap_or((comment, ""));
            let chain = chain.split_whitespace().next().unwrap_or("");
            let (want, want_chain) = super::outcome_of(key.trim()).unwrap_or_else(|| panic!("`{}` isn't in OUTCOMES", key));
            assert_eq!(tags.split_whitespace().collect::<Vec<_>>(), want, "{}", key);
            let want_chain = match want_chain {
                super::Chain::Thing => "thing",
                super::Chain::World => "world",
            };
            assert_eq!(chain, want_chain, "{}", key);
            seen += 1;
        }
        assert_eq!(seen, super::OUTCOMES.len());
    }

    #[test]
    fn sections_describe_what_matches_and_reply_to_their_action() {
        let (_d, world) = world();
        assert!(world.current_problems(false).is_empty(), "{:?}", world.current_problems(false));
        let game = GameEngine::new();
        let mut state = game.start(&world, "bedroom").unwrap();
        let mut s = |line: &str| say(&game, &world, &mut state, line);

        // Description: the default prose and every section that matches.
        assert_eq!(s("x window"), "A grimy window.\n\nDaylight comes through it.");
        let look = s("look");
        assert!(look.contains("A small room.") && !look.contains("long shadows"), "{}", look);
        // A verb only a heading knows is still understood (REQ-PC-14).
        assert_eq!(s("hit window with stick"), "The glass cracks.");
        // Without the stick, no section matches, and `hit` says so itself.
        assert_eq!(s("hit window"), "Hitting the window does nothing.");
        assert_eq!(s("hit window with dragon"), "There's no dragon here.");
        assert_eq!(s("dance wildly"), "I don't understand \"dance wildly\".");

        // What the player holds is in the turn.
        s("take lamp");
        assert!(s("look").contains("The lamp throws long shadows."));

        // `#open` is the open hatch's description and the reply to opening
        // it. A door's own text shows when it's examined, not on a look.
        assert!(s("x hatch").contains("It's bolted from this side."));
        assert!(!s("look").contains("It's bolted from this side."));
        // A refused exit's reply is the door's `#go` section.
        assert_eq!(s("go up"), "The bolt holds.");
        assert_eq!(s("open hatch"), "The hatch hangs open.");
        assert_eq!(s("x hatch"), "A hatch.\n\nThe hatch hangs open.");
        assert!(s("go up").starts_with("Dust."));
    }

    #[test]
    fn the_room_answers_last_and_searching_has_its_own_section() {
        let (_d, world) = world();
        let game = GameEngine::new();
        let mut state = game.start(&world, "hall").unwrap();
        assert_eq!(say(&game, &world, &mut state, "hit lamp with stick"), "There's no lamp here.");
        state.inventory.push("stick".into());
        state.location = "bedroom".into();
        let mut s = |line: &str| say(&game, &world, &mut state, line);
        assert_eq!(s("search"), "You look under the bed.");
        assert_eq!(s("hit window with stick"), "The glass cracks.");
        let mut state = PlayState::new("hall");
        state.inventory.push("stick".into());
        state.inventory.push("window".into());
        // Every match shows: the target's first, then the room's.
        assert_eq!(say(&game, &world, &mut state, "hit window with stick"), "The glass cracks. The hall echoes with the crack.");
        // A thing with no section for it falls back to the room's.
        let mut state = PlayState::new("hall");
        state.inventory.push("stick".into());
        state.inventory.push("lamp".into());
        assert_eq!(say(&game, &world, &mut state, "hit lamp with stick"), "The hall echoes with the crack.");
    }

    /// The sample fixture world loads without problems, `--strict` too.
    /// Bruce's `authors/mx/test-world` isn't checked: it's his to rewrite,
    /// and a draft mid-write may name rooms not written yet.
    #[test]
    fn the_sample_world_validates() {
        let mut world = WorldEngine::new();
        world.load_world(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/worlds/sample"))).unwrap();
        assert!(world.current_problems(true).is_empty(), "{:?}", world.current_problems(true));
        assert!(world.get_location("castle/treasury").is_some());
    }

    #[test]
    fn the_sample_worlds_replies_are_sections() {
        let mut world = WorldEngine::new();
        world.load_world(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/worlds/sample"))).unwrap();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/treasury").unwrap();
        assert_eq!(say(&game, &world, &mut state, "listen chest"), "You hear the faint sound of coins shifting inside.");
        // An `examine` reply is part of the description now.
        assert!(say(&game, &world, &mut state, "x chest").ends_with("The craftsmanship rivals the finest furniture."));
    }

    #[test]
    fn an_at_name_shows_the_things_name_in_play() {
        use crate::engines::variables::mentions;
        let found: Vec<String> = mentions("Ask @guard-captain, or @castle/hall. Mail b@example.com. `@code` @-").into_iter().map(|(_, id)| id).collect();
        assert_eq!(found, ["guard-captain", "castle/hall"]);

        let (dir, mut world) = world();
        let d = dir.path();
        world.set_file(&d.join("npcs/captain.md"), "---\nid: guard-captain\ntitle: Captain Marcus\ntype: character\n---\nStern.\n");
        world.set_file(&d.join("objects/stick.md"), "---\nid: stick\ntitle: Stick\ntype: object\nobject_type: x\n---\n@guard-captain dropped it near @hall. @nobody knows why.\n");
        let warnings = world.current_warnings();
        assert!(!warnings.iter().any(|w| w.contains("@nobody")), "{:?}", warnings);
        assert_eq!(world.not_written().iter().map(|u| u.name.as_str()).collect::<Vec<_>>(), ["nobody"]);
        assert!(!warnings.iter().any(|w| w.contains("@guard-captain") || w.contains("@hall")), "{:?}", warnings);
        let game = GameEngine::new();
        let mut state = game.start(&world, "bedroom").unwrap();
        assert_eq!(say(&game, &world, &mut state, "x stick"), "Captain Marcus dropped it near Hall. nobody knows why.");
    }

    #[test]
    fn go_to_tries_the_whole_phrase_then_without_to() {
        let (dir, mut world) = world();
        let d = dir.path();
        world.set_file(&d.join("yard.md"), "---\nid: yard\ntitle: Yard\ntype: location\nconnections: { river: bedroom, to the well: hall, north: hall }\n---\nGrass.\n");
        let game = GameEngine::new();
        for (line, to) in [("go to the river", "bedroom"), ("go to the well", "hall"), ("go river", "bedroom"), ("go to north", "hall"), ("go to n", "hall")] {
            let mut state = game.start(&world, "yard").unwrap();
            say(&game, &world, &mut state, line);
            assert_eq!(state.location, to, "{}", line);
        }
        let mut state = game.start(&world, "yard").unwrap();
        assert_eq!(say(&game, &world, &mut state, "go to the sea"), "You can't go to the sea from here.");
    }

    fn vault() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\nconnections: { down: vault-door, east: shed-door }\nobjects: [brass-key, spoon, lever]\n---\nA hall.\n");
        write("castle/vault.md", "---\nid: castle/vault\ntitle: The Vault\ntype: location\nconnections: { up: vault-door }\n---\nGold.\n");
        write("castle/shed.md", "---\nid: castle/shed\ntitle: Shed\ntype: location\nconnections: { west: shed-door }\n---\nTools.\n");
        write("objects/brass-key.md", "---\nid: brass-key\ntitle: Brass key\ntype: object\nobject_type: key\n---\nA brass key.\n");
        write("objects/spoon.md", "---\nid: spoon\ntitle: Spoon\ntype: object\nobject_type: x\n---\n");
        // design/STATES.md › Keys, as written.
        write("castle/doors/vault-door.md", "---\nid: vault-door\ntitle: Iron door\ntype: door\ninherits: locked-door\nvars: { key: brass-key }\nsides:\n  castle/hall:  { direction: down }\n  castle/vault: { direction: up }\n---\nA heavy iron door, set into the floor.\n\n## When: #locked\nA padlock hangs from the hasp.\n\n## When: #unlock #closed\nThe padlock snaps open with a rusty clunk. The key stays in it.\n\n## When: #open\nCold air rises from the dark below.\n");
        // A deadbolt: no key needed.
        write("castle/doors/shed-door.md", "---\nid: shed-door\ntitle: Shed door\ntype: door\ninherits: locked-door\nsides:\n  castle/hall: { direction: east }\n  castle/shed: { direction: west }\n---\nA plank door.\n");
        // A lever whose script reaches the shed door through `connects:`.
        write("objects/lever/index.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nweight: immovable\nstate: up\nconnects: [shed-door]\n---\nA lever.\n");
        write("objects/lever/up.md", "---\ninherits: index\n---\n");
        write("objects/lever/down.md", "---\ninherits: index\n---\n## When: #pull\nSomething clanks to the east.\n");
        write("objects/lever/actions.rhai", "fn pull(lever, turn) {\n    if lever.state == \"up\" { [become(\"down\"), change(\"shed-door\", \"open\")] }\n}\nfn kick(lever, turn) { change(\"vault-door\", \"open\") }\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        (dir, engine)
    }

    #[test]
    fn a_views_things_carry_their_state() {
        let (_d, world) = vault();
        let lever = |view: &crate::engines::LocationView| view.objects.iter().find(|o| o.id == "lever").and_then(|o| o.state.clone());
        let view = world.describe_location_at("castle/hall", crate::engines::At::default()).unwrap();
        assert_eq!(lever(&view).as_deref(), Some("up"), "its starting state, outside play");
        assert_eq!(view.objects.iter().find(|o| o.id == "spoon").unwrap().state, None, "no states");
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        say(&game, &world, &mut state, "pull lever");
        assert_eq!(lever(&game.view(&world, &state).unwrap()).as_deref(), Some("down"), "the game's");
    }

    #[test]
    fn the_vault_door_plays_as_the_keys_table_says() {
        let (_d, world) = vault();
        assert!(world.current_problems(false).is_empty(), "{:?}", world.current_problems(false));
        assert!(world.current_warnings().is_empty(), "{:?}", world.current_warnings());
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        let mut s = |line: &str| say(&game, &world, &mut state, line);
        s("take key");
        s("take spoon");
        assert!(s("x iron door").contains("A padlock hangs from the hasp."));
        assert_eq!(s("down"), "The iron door is locked.");
        assert_eq!(s("open iron door"), "It's locked.");
        assert_eq!(s("unlock iron door"), "It's locked. Unlock it with what?");
        assert_eq!(s("unlock iron door with spoon"), "That doesn't fit the lock.");
        assert_eq!(s("unlock iron door with key"), "The padlock snaps open with a rusty clunk. The key stays in it.");
        // The key is in the lock now; unlocking again does nothing.
        assert_eq!(s("unlock iron door with key"), "It isn't locked.");
        assert_eq!(s("unlock iron door"), "It isn't locked.");
        assert!(s("inventory").contains("a spoon") && !s("inventory").contains("brass key"));
        // The door's own `#open` replaces the built-in's (STATES.md).
        assert_eq!(s("open iron door"), "Cold air rises from the dark below.");
        assert_eq!(s("open iron door"), "It's already open.");
        assert_eq!(s("take key"), "You take the brass key.");
        assert!(s("inventory").contains("a brass key"));
        assert!(s("down").starts_with("Gold."));
    }

    #[test]
    fn a_deadbolt_needs_no_key_and_scripts_reach_only_what_they_connect() {
        let (_d, world) = vault();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        let mut s = |line: &str| say(&game, &world, &mut state, line);
        assert_eq!(s("east"), "The shed door is locked.");
        assert_eq!(s("unlock shed door"), "The lock falls open.");
        assert_eq!(s("lock shed door"), "The lock clicks shut.");
        // The lever opens the shed door through `connects:`.
        assert_eq!(s("pull lever"), "Something clanks to the east.");
        assert!(s("east").starts_with("Tools."));
        // But it can't reach the vault door, which it doesn't connect to.
        let mut state = game.start(&world, "castle/hall").unwrap();
        let r = game.act(&world, &mut state, &crate::models::game::PlayAction::Use { verb: "kick".into(), noun: "lever".into(), with: None });
        assert_eq!(r.text, "Kicking the lever does nothing.");
        assert!(r.note.unwrap().contains("isn't connected"));
        assert_eq!(state.state_of("door", "vault-door"), None);
    }

    #[test]
    fn a_key_or_connection_that_names_nothing_is_warned_about() {
        let (dir, mut world) = vault();
        let d = dir.path();
        world.set_file(&d.join("castle/doors/shed-door.md"), "---\nid: shed-door\ntitle: Shed door\ntype: door\ninherits: locked-door\nvars: { key: iron-key }\nsides:\n  castle/hall: { direction: east }\n  castle/shed: { direction: west }\n---\n");
        world.set_file(&d.join("objects/lever/index.md"), "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nstate: up\nconnects: [trapdoor]\n---\n");
        let warnings = world.current_warnings();
        assert!(warnings.iter().any(|w| w.contains("`key: iron-key` names nothing")), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("`connects:` names `trapdoor`")), "{:?}", warnings);
        // A script that doesn't compile is a problem, naming its file.
        world.set_file(&d.join("objects/lever/actions.rhai"), "fn pull(lever, turn) {");
        assert!(world.current_problems(false).iter().any(|p| p.contains("actions.rhai: the script doesn't compile")), "{:?}", world.current_problems(false));
        // A held thing that's gone is dropped on reload.
        let mut state = PlayState::new("castle/hall");
        state.held.insert("vault-door".into(), vec!["brass-key".into()]);
        world.remove_file(&d.join("objects/brass-key.md"));
        assert_eq!(state.reconcile(&world).len(), 1);
        assert!(state.held.is_empty());
    }

    fn hallway() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("hallway.md", "---\nid: hallway\ntitle: Hallway\ntype: location\nobjects: [walls]\n---\nA long hallway.\n");
        write("objects/walls.md", "---\nid: walls\ntitle: Walls\ntype: object\nobject_type: scenery\nweight: immovable\nobjects: [photo-1, shelf]\n---\nPapered walls.\n");
        write("objects/photo-1.md", "---\nid: photo-1\ntitle: Old photo\ntype: object\nobject_type: picture\n---\nA family, unsmiling.\n");
        write("objects/shelf.md", "---\nid: shelf\ntitle: Shelf\ntype: object\nobject_type: furniture\nweight: immovable\nobjects: [vase]\n---\nA narrow shelf.\n");
        write("objects/vase.md", "---\nid: vase\ntitle: Vase\ntype: object\nobject_type: x\n---\nBlue glass.\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        (dir, engine)
    }

    #[test]
    fn objects_hold_objects_as_deep_as_written() {
        use crate::engines::Holder;
        let (dir, mut world) = hallway();
        assert!(world.current_problems(false).is_empty(), "{:?}", world.current_problems(false));
        assert!(world.current_warnings().is_empty(), "{:?}", world.current_warnings());
        assert_eq!(world.placement("photo-1"), [Holder::Object("walls".into())]);
        assert_eq!(world.placement("walls"), [Holder::Room("hallway".into())]);
        assert!(world.placement("nowhere").is_empty());

        let game = GameEngine::new();
        let mut state = game.start(&world, "hallway").unwrap();
        let mut s = |line: &str| say(&game, &world, &mut state, line);
        // The room lists every visible thing by title, each held thing
        // after what holds it.
        let look = s("look");
        assert!(look.contains("There's an old photo and a shelf on the walls and a vase on the shelf."), "{}", look);
        assert!(!look.contains("You see"), "the walls are scenery: {}", look);
        // A holder says what it holds; what that holds is in scope too.
        assert_eq!(s("x walls"), "Papered walls.\nThere's an old photo and a shelf on the walls.");
        assert_eq!(s("look at photo"), "A family, unsmiling.");
        assert_eq!(s("x vase"), "Blue glass.");
        assert_eq!(s("take vase"), "You take the vase.");
        assert_eq!(s("x shelf"), "A narrow shelf.");
        assert_eq!(s("take photo"), "You take the old photo.");
        assert_eq!(s("x walls"), "Papered walls.\nThere's a shelf on the walls.");
        assert!(s("look").contains("There's a shelf on the walls."), "the vase and photo are taken");
        assert!(s("inventory").contains("an old photo"));

        // Placed twice is a warning; a cycle is a problem.
        let d = dir.path();
        world.set_file(&d.join("hallway.md"), "---\nid: hallway\ntitle: Hallway\ntype: location\nobjects: [walls, vase]\n---\n");
        assert!(world.current_warnings().iter().any(|w| w.contains("`vase` starts in more than one place: room `hallway`, object `shelf`")), "{:?}", world.current_warnings());
        world.set_file(&d.join("objects/vase.md"), "---\nid: vase\ntitle: Vase\ntype: object\nobject_type: x\nobjects: [walls]\n---\n");
        assert!(world.current_problems(false).iter().any(|p| p.contains("object `walls` holds itself")), "{:?}", world.current_problems(false));
        // Play still follows a cycle only once.
        let mut state = game.start(&world, "hallway").unwrap();
        assert_eq!(say(&game, &world, &mut state, "x vase"), "There's some walls on the vase.");
        assert_eq!(say(&game, &world, &mut state, "x photo"), "A family, unsmiling.");
    }

    #[test]
    fn edit_modes_room_view_reads_as_play_and_keeps_the_detail_as_data() {
        use crate::engines::At;
        use crate::render::{render_location_as, RenderStyle};
        let (_d, world) = hallway();
        let view = world.describe_location_at("hallway", At::default()).unwrap();
        assert_eq!(view.held.len(), 3);
        let text = render_location_as(&view, RenderStyle::Author);
        // The text is play's; who holds what is the view's data, for the
        // console's sidebar (Bruce, 2026-11-24).
        assert!(!text.contains("Hallway"), "no title line; the sidebar has it: {}", text);
        assert_eq!((view.title.as_str(), view.id.as_str()), ("Hallway", "hallway"));
        assert!(text.contains("There's an old photo and a shelf on the walls and a vase on the shelf."), "{}", text);
        assert!(!text.contains("(photo-1)") && !text.contains("\n  "), "no detail lines: {}", text);
        let held: Vec<(&str, &str)> = view.held.iter().map(|h| (h.thing.id.as_str(), h.holder.as_str())).collect();
        assert_eq!(held, [("photo-1", "walls"), ("shelf", "walls"), ("vase", "shelf")]);
    }

    #[test]
    fn a_holder_says_how_it_holds_things() {
        let (dir, mut world) = hallway();
        let d = dir.path();
        world.set_file(&d.join("hallway.md"), "---\nid: hallway\ntitle: Hallway\ntype: location\nobjects: [walls, chest]\n---\n");
        world.set_file(&d.join("objects/chest.md"), "---\nid: chest\ntitle: Chest\ntype: object\nobject_type: box\nholds: in\nobjects: [coins]\n---\n");
        world.set_file(&d.join("objects/coins.md"), "---\nid: coins\ntitle: Coins\ntype: object\nobject_type: x\n---\n");
        let game = GameEngine::new();
        let mut state = game.start(&world, "hallway").unwrap();
        let look = say(&game, &world, &mut state, "look");
        assert!(look.contains("an old photo and a shelf on the walls") && look.contains("some coins in the chest"), "{}", look);
        world.set_file(&d.join("objects/chest.md"), "---\nid: chest\ntitle: Chest\ntype: object\nobject_type: box\nholds:\nobjects: [coins]\n---\n");
        assert!(world.current_warnings().iter().any(|w| w.contains("object `chest`: `holds:` is empty")), "{:?}", world.current_warnings());

        // The built-in locked door holds its key in the lock.
        let (_v, world) = vault();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        say(&game, &world, &mut state, "take key");
        say(&game, &world, &mut state, "unlock iron door with key");
        assert!(say(&game, &world, &mut state, "look").contains("There's a brass key in the iron door."));
    }

    #[test]
    fn every_door_built_in_opens_and_closes_and_a_wrong_kind_of_exit_says_what_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("bedroom.md", "---\nid: bedroom\ntitle: Bedroom\ntype: location\nconnections: { west: bedroom-door, north: chair }\n---\nA bedroom.\n");
        write("hallway.md", "---\nid: hallway\ntitle: Hallway\ntype: location\nconnections: { east: bedroom-door }\n---\nA hallway.\n");
        write("chair.md", "---\nid: chair\ntitle: Chair\ntype: object\nobject_type: furniture\n---\n");
        write("doors/bedroom-door.md", "---\nid: bedroom-door\ntitle: Door\ntype: door\ninherits: door\nsides:\n  bedroom: { direction: west }\n  hallway: { direction: east }\n---\n");
        let mut world = WorldEngine::new();
        world.load_world(dir.path()).unwrap();
        assert!(world.current_problems(false).iter().any(|p| p == "bedroom: the exit north names `chair`, which is an object, not a room or a door"), "{:?}", world.current_problems(false));
        let game = GameEngine::new();
        let mut state = game.start(&world, "bedroom").unwrap();
        let mut s = |line: &str| say(&game, &world, &mut state, line);
        assert_eq!(s("west"), "The door is closed.");
        assert_eq!(s("close door"), "It's already shut.");
        assert_eq!(s("open door"), "It swings open.");
        assert_eq!(s("open door"), "It's already open.");
        assert!(s("west").starts_with("A hallway."));
        assert_eq!(s("close door"), "It's shut now.");
        // A portal opens the same way.
        let (_d, mut world) = (dir, world);
        world.set_file(&_d.path().join("doors/bedroom-door.md"), "---\nid: bedroom-door\ntitle: Ring\ntype: door\ninherits: portal\nsides:\n  bedroom: { direction: west }\n  hallway: { direction: east }\n---\n");
        let mut state = game.start(&world, "bedroom").unwrap();
        assert_eq!(say(&game, &world, &mut state, "open ring"), "It swings open.");
    }
}
