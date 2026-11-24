//! A turn's tags and the sections they match (`design/SECTIONS.md`,
//! REQ-ENG-36). Content never tests anything: each turn has a set of
//! tags, and a `## When:` section is shown when all its tags are in it
//! (`## Unless:`, when none are).
//!
//! The set holds `@` for each thing the player holds and the room they're
//! in, `#` for the chapter, and `#` for the current state of the thing
//! being shown. On an action's turn it also holds `#<verb>`, `@<target>`
//! and `@<what it's used with>`. Tags on other things' states and on
//! values wait on Bruce.
//!
//! A section is description when it matches without the action's tags,
//! and a reply when it names the turn's `#<verb>`.

use crate::engines::bundles::Thing;
use crate::engines::layers::{strip_comments, Section};
use crate::engines::{EntityKind, GameEngine, WorldEngine};
use crate::models::game::PlayState;
use std::collections::BTreeSet;

/// The tags of one turn.
pub type Tags = BTreeSet<String>;

/// A thing's sections, its templates' first (the most distant first),
/// then its own, skipping the reserved replies.
fn sections(thing: &Thing) -> impl Iterator<Item = &Section> {
    thing.templates.iter().chain(std::iter::once(&thing.index)).flat_map(|l| l.sections.iter()).filter(|s| s.reserved().is_none())
}

fn prose_of(section: &Section) -> Option<String> {
    let text = section.layer.body.trim();
    (!strip_comments(text).trim().is_empty()).then(|| text.to_string())
}

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

    /// `tags` plus the current state of one thing, as `#<state>`.
    fn with_state(&self, world: &WorldEngine, state: &PlayState, kind: EntityKind, id: &str, tags: &Tags) -> Tags {
        let mut tags = tags.clone();
        if let Some(thing) = world.thing(kind, id) {
            let current = state
                .state_of(kind.type_name(), id)
                .filter(|s| thing.has_state(s))
                .map(String::from)
                .or_else(|| thing.starting_state.clone());
            if let Some(s) = current {
                tags.insert(format!("#{}", s));
            }
        }
        tags
    }

    /// A thing's description for this turn: `base` (its prose) and then
    /// each of its sections that matches, in file order.
    pub(crate) fn describe(&self, world: &WorldEngine, state: &PlayState, kind: EntityKind, id: &str, base: &str) -> String {
        let Some(thing) = world.thing(kind, id) else { return base.to_string() };
        let tags = self.with_state(world, state, kind, id, &self.turn_tags(state));
        let mut parts: Vec<String> = Vec::new();
        if !base.trim().is_empty() {
            parts.push(base.trim().to_string());
        }
        parts.extend(sections(thing).filter(|s| s.matches(|t| tags.contains(t))).filter_map(prose_of));
        parts.join("\n\n")
    }

    /// The reply to `verb` on `target`, used with `with`: the sections
    /// naming `#<verb>` that match the turn, in the target, then in what's
    /// used, then in the room. `None` if none matches.
    pub(crate) fn reply(
        &self,
        world: &WorldEngine,
        state: &PlayState,
        verb: &str,
        target: (EntityKind, &str),
        with: Option<(EntityKind, &str)>,
    ) -> Option<(EntityKind, String, String)> {
        let action = format!("#{}", verb.trim().to_lowercase().replace(' ', "-"));
        let mut tags = self.turn_tags(state);
        tags.insert(action.clone());
        tags.insert(format!("@{}", target.1));
        if let Some((_, id)) = with {
            tags.insert(format!("@{}", id));
        }
        let room = (EntityKind::Location, state.location.as_str());
        for (kind, id) in std::iter::once(target).chain(with).chain(std::iter::once(room)) {
            let Some(thing) = world.thing(kind, id) else { continue };
            let tags = self.with_state(world, state, kind, id, &tags);
            let parts: Vec<String> = sections(thing)
                .filter(|s| !s.unless && s.tags.iter().any(|t| *t == action) && s.matches(|t| tags.contains(t)))
                .filter_map(prose_of)
                .collect();
            if !parts.is_empty() {
                return Some((kind, id.to_string(), parts.join("\n\n")));
            }
        }
        None
    }

    /// The room's `## When: #searched` section, if it has one that matches.
    pub(crate) fn searched(&self, world: &WorldEngine, state: &PlayState) -> Option<String> {
        let thing = world.thing(EntityKind::Location, &state.location)?;
        let mut tags = self.with_state(world, state, EntityKind::Location, &state.location, &self.turn_tags(state));
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
        write("doors/hatch/index.md", "---\nid: hatch\ntitle: Hatch\ntype: door\nstate: closed\nsides:\n  bedroom: { direction: up }\n  attic: { direction: down }\nactions:\n  open: { becomes: { this: open } }\n---\nA hatch.\n\n## When: #closed\nIt's bolted from this side.\n\n## When: #open\nThe hatch hangs open.\n");
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
        // Without the stick, no section matches.
        assert_eq!(s("hit window"), "Nothing happens.");
        assert_eq!(s("hit window with dragon"), "There's no dragon here.");
        assert_eq!(s("dance wildly"), "I don't understand `dance wildly`.");

        // What the player holds is in the turn.
        s("take lamp");
        assert!(s("look").contains("The lamp throws long shadows."));

        // `#open` is the open hatch's description and the reply to opening it.
        assert!(s("look").contains("It's bolted from this side."));
        assert_eq!(s("go up"), "The hatch is closed.");
        assert_eq!(s("open hatch"), "The hatch hangs open.");
        assert_eq!(s("x hatch"), "A hatch.\n\nThe hatch hangs open.");
        assert!(s("go up").starts_with("Attic"));
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
        // The target answers first; the hall's section is only its own room's.
        assert_eq!(s("hit window with stick"), "The glass cracks.");
        let mut state = PlayState::new("hall");
        state.inventory.push("stick".into());
        state.inventory.push("window".into());
        assert_eq!(say(&game, &world, &mut state, "hit window with stick"), "The glass cracks.");
        // A thing with no section for it falls back to the room's.
        let mut state = PlayState::new("hall");
        state.inventory.push("stick".into());
        state.inventory.push("lamp".into());
        assert_eq!(say(&game, &world, &mut state, "hit lamp with stick"), "The hall echoes with the crack.");
    }
}
