//! Play: applying player actions to a `PlayState` (REQ-ENG-5). Nothing
//! here writes a world file (REQ-CON-3); the world is read-only.

use crate::engines::location_view::{expand_direction, EntityRef, LocationView};
use crate::engines::WorldEngine;
use crate::models::game::{ActionResult, Change, PlayAction, PlayState};
use crate::render::{render_location_as, RenderStyle};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Dispatches player actions to one handler per action.
#[derive(Debug, Clone, Default)]
pub struct GameEngine;

impl GameEngine {
    pub fn new() -> Self {
        Self
    }

    /// A new game at `location`, which must exist.
    pub fn start(&self, world: &WorldEngine, location: &str) -> Result<PlayState> {
        if world.get_location(location).is_none() {
            bail!("can't start at `{}`: no such location", location);
        }
        Ok(PlayState::new(location))
    }

    /// Apply one action. Only a success changes `state`, and its
    /// `changes` say how; every other reply leaves `state` as it was.
    pub fn act(&self, world: &WorldEngine, state: &mut PlayState, action: &PlayAction) -> ActionResult {
        if world.get_location(&state.location).is_none() {
            return ActionResult::say("You're nowhere: this place no longer exists.")
                .with_note(format!("location `{}` isn't in the world any more; use /play here to move", state.location));
        }
        match action {
            PlayAction::Look => self.look(world, state),
            PlayAction::Go(direction) => self.go(world, state, direction),
            PlayAction::Take(noun) => self.take(world, state, noun),
            PlayAction::Examine(noun) => self.examine(world, state, noun),
            PlayAction::Inventory => self.inventory(world, state),
        }
    }

    /// The current room as play mode shows it, without what's been taken.
    pub fn view(&self, world: &WorldEngine, state: &PlayState) -> Option<LocationView> {
        let mut view = world.describe_location(&state.location)?;
        view.objects.retain(|o| !state.was_taken(&state.location, &o.id));
        Some(view)
    }

    fn look(&self, world: &WorldEngine, state: &PlayState) -> ActionResult {
        match self.view(world, state) {
            Some(view) => ActionResult::say(render_location_as(&view, RenderStyle::Player)),
            None => ActionResult::say("You're nowhere."),
        }
    }

    fn go(&self, world: &WorldEngine, state: &mut PlayState, direction: &str) -> ActionResult {
        let direction = expand_direction(direction);
        if direction.is_empty() {
            return ActionResult::say("Go where?");
        }
        let here = world.get_location(&state.location).expect("checked in act");
        let Some(target) = here.connections.get(&direction) else {
            return ActionResult::say(format!("You can't go {} from here.", direction));
        };
        if world.get_location(target).is_none() {
            return ActionResult::say(format!("You can't go {} from here.", direction))
                .with_note(format!("the {} exit leads to `{}`, which doesn't exist yet", direction, target));
        }
        let from = std::mem::replace(&mut state.location, target.clone());
        let mut result = self.look(world, state);
        result.changes.push(Change::Moved { from, to: target.clone() });
        result
    }

    fn take(&self, world: &WorldEngine, state: &mut PlayState, noun: &str) -> ActionResult {
        if noun.trim().is_empty() {
            return ActionResult::say("Take what?");
        }
        let view = match self.view(world, state) {
            Some(v) => v,
            None => return ActionResult::say("You're nowhere."),
        };
        if let Some(carried) = resolve(noun, &self.carried(world, state)).ok().flatten() {
            return ActionResult::say(format!("You already have the {}.", lower(&carried.display_name())));
        }
        let object = match resolve(noun, &view.objects) {
            Ok(Some(o)) => o,
            Ok(None) => {
                if resolve(noun, &view.characters).ok().flatten().is_some() {
                    return ActionResult::say("You can't take people.");
                }
                return ActionResult::say(format!("There's no {} here.", noun.trim()));
            }
            Err(which) => return ActionResult::say(which),
        };
        let name = lower(&object.display_name());
        let Some(card) = world.objects.get(&object.id) else {
            return ActionResult::say(format!("You can't take the {}.", name))
                .with_note(format!("`{}` is scenery with no card, so it can't be taken", object.id));
        };
        if card.weight.as_deref().map_or(false, |w| w.eq_ignore_ascii_case("immovable")) {
            return ActionResult::say(format!("The {} won't budge.", name));
        }
        state.inventory.push(object.id.clone());
        state.taken.entry(state.location.clone()).or_default().push(object.id.clone());
        ActionResult {
            text: format!("You take the {}.", name),
            changes: vec![Change::Took { object: object.id.clone(), from: state.location.clone() }],
            note: None,
        }
    }

    fn examine(&self, world: &WorldEngine, state: &PlayState, noun: &str) -> ActionResult {
        if noun.trim().is_empty() {
            return ActionResult::say("Examine what?");
        }
        let view = match self.view(world, state) {
            Some(v) => v,
            None => return ActionResult::say("You're nowhere."),
        };
        let mut things = self.carried(world, state);
        things.extend(view.objects.iter().cloned());
        match resolve(noun, &things) {
            Ok(Some(object)) => {
                return match world.objects.get(&object.id) {
                    Some(card) if !card.description.trim().is_empty() => ActionResult::say(card.description.trim()),
                    _ => ActionResult::say(format!("You see nothing special about the {}.", lower(&object.display_name()))),
                }
            }
            Err(which) => return ActionResult::say(which),
            Ok(None) => {}
        }
        match resolve(noun, &view.characters) {
            Ok(Some(person)) => match world.characters.get(&person.id) {
                Some(card) if !card.description.trim().is_empty() => ActionResult::say(card.description.trim()),
                _ => ActionResult::say(format!("You see nothing special about {}.", person.display_name())),
            },
            Err(which) => ActionResult::say(which),
            Ok(None) => ActionResult::say(format!("There's no {} here.", noun.trim())),
        }
    }

    fn inventory(&self, world: &WorldEngine, state: &PlayState) -> ActionResult {
        let carried = self.carried(world, state);
        if carried.is_empty() {
            return ActionResult::say("You aren't carrying anything.");
        }
        let names: Vec<String> = carried.iter().map(|o| o.display_name()).collect();
        ActionResult::say(format!("You're carrying: {}.", names.join(", ")))
    }

    fn carried(&self, world: &WorldEngine, state: &PlayState) -> Vec<EntityRef> {
        state
            .inventory
            .iter()
            .map(|id| EntityRef { id: id.clone(), name: world.objects.get(id).map(|o| o.title.clone()) })
            .collect()
    }
}

impl PlayState {
    /// Fix up the state after the world changed under it (a reload, or an
    /// edit mid-game): drop carried and taken objects whose cards are gone.
    /// A location that's gone is reported but kept, so `/play here` or a
    /// restored room can recover. Returns a warning per fix.
    pub fn reconcile(&mut self, world: &WorldEngine) -> Vec<String> {
        let mut warnings = Vec::new();
        if world.get_location(&self.location).is_none() {
            warnings.push(format!("the current location `{}` no longer exists", self.location));
        }
        let before = self.inventory.len();
        self.inventory.retain(|id| {
            let keep = world.objects.contains_key(id);
            if !keep {
                warnings.push(format!("`{}` was carried but no longer exists, so it's gone", id));
            }
            keep
        });
        if self.inventory.len() != before {
            for taken in self.taken.values_mut() {
                taken.retain(|id| world.objects.contains_key(id));
            }
            self.taken.retain(|_, v| !v.is_empty());
        }
        warnings
    }
}

/// The current save format.
const SAVE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct SaveFile {
    thwarter_save: u32,
    state: PlayState,
}

/// Write `state` to `path` as YAML, atomically (temp file, then rename).
/// The console picks the path, outside the world (REQ-CON-3).
pub fn save_game(path: &Path, state: &PlayState) -> Result<()> {
    let text = serde_yaml::to_string(&SaveFile { thwarter_save: SAVE_VERSION, state: state.clone() })?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("can't create {}", dir.display()))?;
    }
    let tmp = path.with_extension(format!("yaml.tmp-{}", std::process::id()));
    std::fs::write(&tmp, text).with_context(|| format!("can't write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        anyhow::anyhow!("can't save to {}: {}", path.display(), e)
    })
}

/// Read a save and reconcile it with `world`. Returns the state and a
/// warning per thing that no longer exists.
pub fn load_game(path: &Path, world: &WorldEngine) -> Result<(PlayState, Vec<String>)> {
    let text = std::fs::read_to_string(path).with_context(|| format!("can't read {}", path.display()))?;
    let save: SaveFile = serde_yaml::from_str(&text).with_context(|| format!("{} isn't a thwarter save", path.display()))?;
    if save.thwarter_save != SAVE_VERSION {
        bail!("{} is save format {}; this thwart reads format {}", path.display(), save.thwarter_save, SAVE_VERSION);
    }
    let mut state = save.state;
    let warnings = state.reconcile(world);
    Ok((state, warnings))
}

/// The thing `noun` names among `things`: an exact id, the id with spaces
/// for hyphens, the name ignoring case, then a unique whole word of
/// either. `Err` asks which, when more than one matches.
fn resolve(noun: &str, things: &[EntityRef]) -> std::result::Result<Option<EntityRef>, String> {
    let noun = noun.trim().to_lowercase();
    let noun = noun.strip_prefix("the ").unwrap_or(&noun).trim().to_string();
    if noun.is_empty() {
        return Ok(None);
    }
    let exact: Vec<&EntityRef> = things
        .iter()
        .filter(|t| {
            t.id.to_lowercase() == noun
                || t.id.to_lowercase().replace('-', " ") == noun
                || t.display_name().to_lowercase() == noun
        })
        .collect();
    if let Some(first) = exact.first() {
        return Ok(Some((*first).clone()));
    }
    let words = |t: &EntityRef| -> Vec<String> {
        let text = format!("{} {}", t.display_name(), t.id.replace(['-', '_', '/'], " ")).to_lowercase();
        text.split_whitespace().map(String::from).collect()
    };
    let partial: Vec<&EntityRef> = things.iter().filter(|t| words(t).iter().any(|w| *w == noun)).collect();
    let mut unique: Vec<&EntityRef> = Vec::new();
    for p in partial {
        if !unique.iter().any(|u| u.id == p.id) {
            unique.push(p);
        }
    }
    match unique.len() {
        0 => Ok(None),
        1 => Ok(Some(unique[0].clone())),
        _ => {
            let names: Vec<String> = unique.iter().map(|t| format!("the {}", lower(&t.display_name()))).collect();
            Err(format!("Which do you mean: {}?", join_or(&names)))
        }
    }
}

fn join_or(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} or {}", init.join(", "), last),
    }
}

/// Lowercase a name for use mid-sentence, unless it looks like a proper
/// name (two capitalized words, as in "Captain Marcus").
fn lower(name: &str) -> String {
    let capitals = name.split_whitespace().filter(|w| w.chars().next().map_or(false, char::is_uppercase)).count();
    if capitals > 1 {
        name.to_string()
    } else {
        let mut chars = name.chars();
        match chars.next() {
            Some(c) => c.to_lowercase().chain(chars).collect(),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn world() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        };
        write(
            "castle/yard.md",
            "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  north: castle/hall\n  west: castle/gone\ncharacters: [king]\nobjects: [rusty-sword, old-sword, stone-bench, well]\n---\nGrass.\n",
        );
        write("castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\nconnections:\n  south: castle/yard\n---\nStone.\n");
        write("npcs/king.md", "---\nid: king\ntitle: King Aldric\ntype: character\n---\nAn old king.\n");
        write("objects/rusty-sword.md", "---\nid: rusty-sword\ntitle: Rusty sword\ntype: object\nobject_type: weapon\n---\nPitted with rust.\n");
        write("objects/old-sword.md", "---\nid: old-sword\ntitle: Old sword\ntype: object\nobject_type: weapon\n---\n");
        write("objects/well.md", "---\nid: well\ntitle: Well\ntype: object\nobject_type: environmental\nweight: immovable\n---\nDeep.\n");
        let mut engine = WorldEngine::new();
        engine.load_world(dir.path()).unwrap();
        (dir, engine)
    }

    fn play(world: &WorldEngine) -> (GameEngine, PlayState) {
        let game = GameEngine::new();
        let state = game.start(world, "castle/yard").unwrap();
        (game, state)
    }

    #[test]
    fn start_needs_a_real_location() {
        let (_d, world) = world();
        assert!(GameEngine::new().start(&world, "castle/nowhere").is_err());
    }

    #[test]
    fn look_shows_the_room_for_the_player() {
        let (_d, world) = world();
        let (game, mut state) = play(&world);
        let r = game.act(&world, &mut state, &PlayAction::Look);
        assert!(r.text.starts_with("Yard\n\nGrass.\n\nExits: north, west.\n"), "{}", r.text);
        assert!(r.text.contains("You see: Rusty sword, Old sword, Stone bench, Well."), "{}", r.text);
        assert!(r.text.ends_with("Here: King Aldric."), "{}", r.text);
        assert!(r.changes.is_empty());
    }

    #[test]
    fn go_moves_through_real_exits_only() {
        let (_d, world) = world();
        let (game, mut state) = play(&world);
        let r = game.act(&world, &mut state, &PlayAction::Go("n".into()));
        assert_eq!(state.location, "castle/hall");
        assert_eq!(r.changes, vec![Change::Moved { from: "castle/yard".into(), to: "castle/hall".into() }]);
        assert!(r.text.starts_with("Hall\n"));

        let r = game.act(&world, &mut state, &PlayAction::Go("east".into()));
        assert_eq!(r.text, "You can't go east from here.");
        assert!(r.changes.is_empty() && r.note.is_none());

        game.act(&world, &mut state, &PlayAction::Go("south".into()));
        let r = game.act(&world, &mut state, &PlayAction::Go("W".into()));
        assert_eq!(state.location, "castle/yard");
        assert_eq!(r.text, "You can't go west from here.");
        assert!(r.note.unwrap().contains("`castle/gone`, which doesn't exist yet"));
    }

    #[test]
    fn take_moves_a_card_into_the_inventory_and_out_of_the_room() {
        let (dir, world) = world();
        let (game, mut state) = play(&world);
        let before = fs::read_to_string(dir.path().join("castle/yard.md")).unwrap();

        let r = game.act(&world, &mut state, &PlayAction::Take("rusty sword".into()));
        assert_eq!(r.text, "You take the rusty sword.");
        assert_eq!(r.changes, vec![Change::Took { object: "rusty-sword".into(), from: "castle/yard".into() }]);
        assert_eq!(state.inventory, vec!["rusty-sword"]);
        assert!(!game.act(&world, &mut state, &PlayAction::Look).text.contains("Rusty sword"));
        assert_eq!(game.act(&world, &mut state, &PlayAction::Inventory).text, "You're carrying: Rusty sword.");
        assert_eq!(game.act(&world, &mut state, &PlayAction::Take("rusty-sword".into())).text, "You already have the rusty sword.");

        // Playing never writes the world (REQ-CON-3).
        assert_eq!(fs::read_to_string(dir.path().join("castle/yard.md")).unwrap(), before);
    }

    #[test]
    fn take_refuses_scenery_immovables_people_and_ambiguity() {
        let (_d, world) = world();
        let (game, mut state) = play(&world);
        let cases = [
            ("bench", "You can't take the stone bench."),
            ("well", "The well won't budge."),
            ("king", "You can't take people."),
            ("sword", "Which do you mean: the rusty sword or the old sword?"),
            ("lamp", "There's no lamp here."),
            ("", "Take what?"),
        ];
        for (noun, reply) in cases {
            let r = game.act(&world, &mut state, &PlayAction::Take(noun.into()));
            assert_eq!(r.text, reply, "take {:?}", noun);
            assert!(r.changes.is_empty(), "take {:?} changed state", noun);
        }
        assert!(state.inventory.is_empty() && state.taken.is_empty());
        let r = game.act(&world, &mut state, &PlayAction::Take("stone bench".into()));
        assert!(r.note.unwrap().contains("scenery with no card"));
    }

    #[test]
    fn examine_reads_cards_scenery_people_and_the_inventory() {
        let (_d, world) = world();
        let (game, mut state) = play(&world);
        let ex = |game: &GameEngine, state: &mut PlayState, n: &str| game.act(&world, state, &PlayAction::Examine(n.into())).text;
        assert_eq!(ex(&game, &mut state, "rusty sword"), "Pitted with rust.");
        assert_eq!(ex(&game, &mut state, "old sword"), "You see nothing special about the old sword.");
        assert_eq!(ex(&game, &mut state, "the stone bench"), "You see nothing special about the stone bench.");
        assert_eq!(ex(&game, &mut state, "king aldric"), "An old king.");
        assert_eq!(ex(&game, &mut state, "dragon"), "There's no dragon here.");
        game.act(&world, &mut state, &PlayAction::Take("rusty sword".into()));
        game.act(&world, &mut state, &PlayAction::Go("north".into()));
        assert_eq!(ex(&game, &mut state, "rusty sword"), "Pitted with rust.");
    }

    #[test]
    fn an_empty_inventory_says_so() {
        let (_d, world) = world();
        let (game, mut state) = play(&world);
        assert_eq!(game.act(&world, &mut state, &PlayAction::Inventory).text, "You aren't carrying anything.");
    }

    #[test]
    fn a_vanished_location_is_reported_not_a_crash() {
        let (dir, mut world) = world();
        let (game, mut state) = play(&world);
        world.remove_file(&dir.path().join("castle/yard.md"));
        let r = game.act(&world, &mut state, &PlayAction::Look);
        assert!(r.text.starts_with("You're nowhere"));
        assert!(r.note.is_some());
        assert_eq!(state.reconcile(&world), vec!["the current location `castle/yard` no longer exists".to_string()]);
    }

    #[test]
    fn reconcile_drops_carried_objects_that_are_gone() {
        let (dir, mut world) = world();
        let (game, mut state) = play(&world);
        game.act(&world, &mut state, &PlayAction::Take("rusty sword".into()));
        assert!(state.reconcile(&world).is_empty());
        world.remove_file(&dir.path().join("objects/rusty-sword.md"));
        let warnings = state.reconcile(&world);
        assert_eq!(warnings.len(), 1);
        assert!(state.inventory.is_empty() && state.taken.is_empty());
    }

    #[test]
    fn save_and_load_round_trip_and_reconcile() {
        let (dir, mut world) = world();
        let (game, mut state) = play(&world);
        game.act(&world, &mut state, &PlayAction::Take("rusty sword".into()));
        game.act(&world, &mut state, &PlayAction::Go("north".into()));
        let saves = tempfile::tempdir().unwrap();
        let path = saves.path().join("my-world/quicksave.yaml");
        save_game(&path, &state).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("thwarter_save: 1\n"), "{}", text);

        let (loaded, warnings) = load_game(&path, &world).unwrap();
        assert_eq!(loaded, state);
        assert!(warnings.is_empty());

        world.remove_file(&dir.path().join("objects/rusty-sword.md"));
        let (loaded, warnings) = load_game(&path, &world).unwrap();
        assert!(loaded.inventory.is_empty());
        assert_eq!(warnings.len(), 1);

        fs::write(&path, "thwarter_save: 9\nstate:\n  location: x\n").unwrap();
        assert!(load_game(&path, &world).unwrap_err().to_string().contains("save format 9"));
        fs::write(&path, "not: a save\n").unwrap();
        assert!(load_game(&path, &world).is_err());
    }
}
