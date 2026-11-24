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
        for section in layer.sections.iter().filter(|s| s.reserved().is_none()) {
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
        let mut tags = self.turn_tags(state);
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

    /// The replies to `verb` on `target`, used with `with`, after the
    /// action, given its outcome: the matching sections naming `#<verb>`
    /// in the target, then in what's used, then in the room, every match
    /// in that order. Each is the thing it came from and its text.
    pub(crate) fn replies(
        &self,
        world: &WorldEngine,
        state: &PlayState,
        verb: &str,
        target: (EntityKind, &str),
        with: Option<(EntityKind, &str)>,
        outcome: &Outcome,
    ) -> Vec<(EntityKind, String, String)> {
        let action = format!("#{}", verb.trim().to_lowercase().replace(' ', "-"));
        let outcome_tag = format!("#{}", outcome.0);
        let mut tags = self.turn_tags(state);
        tags.insert(action.clone());
        tags.insert(format!("@{}", target.1));
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
            let mut matched: Vec<(Vec<String>, bool, String)> = Vec::new();
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
                key.sort();
                key.dedup();
                if let Some(text) = prose_of(section) {
                    matched.push((key, found.own, text));
                }
            }
            // A thing's own reply replaces its template's for the same
            // action and outcome.
            let own_keys: Vec<Vec<String>> = matched.iter().filter(|m| m.1).map(|m| m.0.clone()).collect();
            for (key, own, text) in matched {
                if !own && own_keys.contains(&key) {
                    continue;
                }
                out.push((kind, id.to_string(), text));
            }
        }
        out
    }

    /// The room's `## When: #searched` section, if it has one that matches.
    pub(crate) fn searched(&self, world: &WorldEngine, state: &PlayState) -> Option<String> {
        let thing = world.thing(EntityKind::Location, &state.location)?;
        let mut tags = self.turn_tags(state);
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
        // A refused exit's reply is the door's `#go` section.
        assert_eq!(s("go up"), "The bolt holds.");
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
        assert_eq!(s("hit window with stick"), "The glass cracks.");
        let mut state = PlayState::new("hall");
        state.inventory.push("stick".into());
        state.inventory.push("window".into());
        // Every match shows: the target's first, then the room's.
        assert_eq!(say(&game, &world, &mut state, "hit window with stick"), "The glass cracks.\n\nThe hall echoes with the crack.");
        // A thing with no section for it falls back to the room's.
        let mut state = PlayState::new("hall");
        state.inventory.push("stick".into());
        state.inventory.push("lamp".into());
        assert_eq!(say(&game, &world, &mut state, "hit lamp with stick"), "The hall echoes with the crack.");
    }

    #[test]
    fn the_test_worlds_replies_are_sections() {
        let mut world = WorldEngine::new();
        world.load_world(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/authors/mx/test-world"))).unwrap();
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
        assert!(warnings.iter().any(|w| w.contains("`@nobody` names nothing in the world")), "{:?}", warnings);
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
        assert_eq!(s("down"), "It's locked.");
        assert_eq!(s("open iron door"), "It's locked.");
        assert_eq!(s("unlock iron door"), "It's locked. Unlock it with what?");
        assert_eq!(s("unlock iron door with spoon"), "That doesn't fit the lock.");
        assert_eq!(s("unlock iron door with key"), "The padlock snaps open with a rusty clunk. The key stays in it.");
        // The key is in the lock now; unlocking again does nothing.
        assert_eq!(s("unlock iron door with key"), "It isn't locked.");
        assert_eq!(s("unlock iron door"), "It isn't locked.");
        assert!(s("inventory").contains("Spoon") && !s("inventory").contains("Brass key"));
        assert_eq!(s("open iron door"), "The door stands open.\n\nCold air rises from the dark below.");
        assert_eq!(s("take key"), "You take the brass key.");
        assert!(s("inventory").contains("Brass key"));
        assert!(s("down").starts_with("The Vault"));
    }

    #[test]
    fn a_deadbolt_needs_no_key_and_scripts_reach_only_what_they_connect() {
        let (_d, world) = vault();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        let mut s = |line: &str| say(&game, &world, &mut state, line);
        assert_eq!(s("east"), "It's locked.");
        assert_eq!(s("unlock shed door"), "The lock falls open.");
        assert_eq!(s("lock shed door"), "The lock clicks shut.");
        // The lever opens the shed door through `connects:`.
        assert_eq!(s("pull lever"), "Something clanks to the east.");
        assert!(s("east").starts_with("Shed"));
        // But it can't reach the vault door, which it doesn't connect to.
        let mut state = game.start(&world, "castle/hall").unwrap();
        let r = game.act(&world, &mut state, &crate::models::game::PlayAction::Use { verb: "kick".into(), noun: "lever".into(), with: None });
        assert_eq!(r.text, "Nothing happens.");
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
        assert!(look.contains("You see: Walls, Old photo (on the walls), Shelf (on the walls), Vase (on the shelf)."), "{}", look);
        // A holder lists what it holds; what that holds is in scope too.
        assert_eq!(s("x walls"), "Papered walls.\n\nYou see: Old photo, Shelf.");
        assert_eq!(s("look at photo"), "A family, unsmiling.");
        assert_eq!(s("x vase"), "Blue glass.");
        assert_eq!(s("take vase"), "You take the vase.");
        assert_eq!(s("x shelf"), "A narrow shelf.");
        assert_eq!(s("take photo"), "You take the old photo.");
        assert_eq!(s("x walls"), "Papered walls.\n\nYou see: Shelf.");
        assert!(s("look").contains("You see: Walls, Shelf (on the walls)."));
        assert!(s("inventory").contains("Old photo"));

        // Placed twice is a warning; a cycle is a problem.
        let d = dir.path();
        world.set_file(&d.join("hallway.md"), "---\nid: hallway\ntitle: Hallway\ntype: location\nobjects: [walls, vase]\n---\n");
        assert!(world.current_warnings().iter().any(|w| w.contains("`vase` starts in more than one place: room `hallway`, object `shelf`")), "{:?}", world.current_warnings());
        world.set_file(&d.join("objects/vase.md"), "---\nid: vase\ntitle: Vase\ntype: object\nobject_type: x\nobjects: [walls]\n---\n");
        assert!(world.current_problems(false).iter().any(|p| p.contains("object `walls` holds itself")), "{:?}", world.current_problems(false));
        // Play still follows a cycle only once.
        let mut state = game.start(&world, "hallway").unwrap();
        assert_eq!(say(&game, &world, &mut state, "x vase"), "You see: Walls.");
        assert_eq!(say(&game, &world, &mut state, "x photo"), "A family, unsmiling.");
    }

    #[test]
    fn edit_modes_room_view_lists_held_things_under_their_holder() {
        use crate::engines::At;
        use crate::render::{render_location_as, RenderStyle};
        let (_d, world) = hallway();
        let view = world.describe_location_at("hallway", At::default()).unwrap();
        assert_eq!(view.held.len(), 3);
        let text = render_location_as(&view, RenderStyle::Author);
        assert!(text.contains("Objects:\n  Walls (walls)\n    Old photo (photo-1)\n    Shelf (shelf)\n      Vase (vase)"), "{}", text);
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
        assert!(look.contains("Old photo (on the walls)") && look.contains("Coins (in the chest)"), "{}", look);
        world.set_file(&d.join("objects/chest.md"), "---\nid: chest\ntitle: Chest\ntype: object\nobject_type: box\nholds:\nobjects: [coins]\n---\n");
        assert!(world.current_warnings().iter().any(|w| w.contains("object `chest`: `holds:` is empty")), "{:?}", world.current_warnings());

        // The built-in locked door holds its key in the lock.
        let (_v, world) = vault();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        say(&game, &world, &mut state, "take key");
        say(&game, &world, &mut state, "unlock iron door with key");
        assert!(say(&game, &world, &mut state, "look").contains("Brass key (in the iron door)"));
    }
}
