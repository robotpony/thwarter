//! Play: applying player actions to a `PlayState` (REQ-ENG-5). Nothing
//! here writes a world file (REQ-CON-3); the world is read-only.

use crate::engines::location_view::{expand_direction, EntityRef, LocationView};
use crate::engines::{At, EntityKind, WorldEngine};
use crate::models::game::{ActionResult, Change, PlayAction, PlayState};
use crate::render::{render_location_as, RenderStyle};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Dispatches player actions to one handler per action.
#[derive(Debug, Clone)]
pub struct GameEngine {
    /// The reply when a noun matches more than one thing: given the verb
    /// (`take`, `examine`, `pull`) and the candidates, already phrased
    /// ("the rusty sword"). Replaceable, since such replies are part of a
    /// story's voice ("Which door do you want to open?").
    pub ask_which: fn(verb: &str, candidates: &[String]) -> String,
}

/// "Which do you mean: the rusty sword or the old sword?"
pub fn default_ask_which(_verb: &str, candidates: &[String]) -> String {
    format!("Which do you mean: {}?", join_or(candidates))
}

impl Default for GameEngine {
    fn default() -> Self {
        Self { ask_which: default_ask_which }
    }
}

impl GameEngine {
    pub fn new() -> Self {
        Self::default()
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
        state.turn += 1;
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
            PlayAction::Use { verb, noun } => self.use_object(world, state, verb, noun),
        }
    }

    /// The current room as play mode shows it: each thing in its current
    /// version, without what's been taken or is carried, so a chapter that
    /// adds an object the player holds never shows a second one.
    pub fn view(&self, world: &WorldEngine, state: &PlayState) -> Option<LocationView> {
        let mut view = world.describe_location_at(&state.location, At::play(state))?;
        view.objects
            .retain(|o| !state.was_taken(&state.location, &o.id) && !state.inventory.contains(&o.id));
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
        let Some(here) = world.location_at(&state.location, At::play(state)) else {
            return ActionResult::say("You're nowhere.");
        };
        let Some(target) = here.connections.get(&direction) else {
            return ActionResult::say(format!("You can't go {} from here.", direction));
        };
        if world.get_location(target).is_none() {
            return ActionResult::say(format!("You can't go {} from here.", direction))
                .with_note(format!("the {} exit leads to `{}`, which doesn't exist yet", direction, target));
        }
        let from = std::mem::replace(&mut state.location, target.clone());
        *state.visits.entry(target.clone()).or_default() += 1;
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
        if let Some(carried) = resolve(noun, &self.carried(world, state), the_object).ok().flatten() {
            return ActionResult::say(format!("You already have {}.", the_object(&carried)));
        }
        let object = match resolve(noun, &view.objects, the_object) {
            Ok(Some(o)) => o,
            Ok(None) => {
                if resolve(noun, &view.characters, person).ok().flatten().is_some() {
                    return ActionResult::say("You can't take people.");
                }
                return ActionResult::say(format!("There's no {} here.", noun.trim()));
            }
            Err(which) => return ActionResult::say((self.ask_which)("take", &which)),
        };
        let name = object_name(&object);
        let Some(card) = world.object_at(&object.id, At::play(state)) else {
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
        match resolve(noun, &things, the_object) {
            Ok(Some(object)) => {
                // The description, then an `examine` response if the author wrote one.
                return match world.object_at(&object.id, At::play(state)) {
                    Some(card) if !card.description.trim().is_empty() || card.responses.contains_key("examine") => {
                        let parts: Vec<&str> = [card.description.trim(), card.responses.get("examine").map_or("", |t| t.trim())]
                            .into_iter()
                            .filter(|p| !p.is_empty())
                            .collect();
                        ActionResult::say(parts.join("\n\n"))
                    }
                    _ => ActionResult::say(format!("You see nothing special about {}.", the_object(&object))),
                }
            }
            Err(which) => return ActionResult::say((self.ask_which)("examine", &which)),
            Ok(None) => {}
        }
        match resolve(noun, &view.characters, person) {
            Ok(Some(person)) => match world.character_at(&person.id, At::play(state)) {
                Some(card) if !card.description.trim().is_empty() => ActionResult::say(card.description.trim()),
                _ => ActionResult::say(format!("You see nothing special about {}.", person.display_name())),
            },
            Err(which) => ActionResult::say((self.ask_which)("examine", &which)),
            Ok(None) => ActionResult::say(format!("There's no {} here.", noun.trim())),
        }
    }

    /// Run an object's named action: check `requires`, then switch states
    /// (`becomes`), set values (`sets`) and move the story (`chapter`).
    /// A target that can't be found is skipped with an author note, which
    /// `validate` also reports.
    fn use_object(&self, world: &WorldEngine, state: &mut PlayState, verb: &str, noun: &str) -> ActionResult {
        let verb = verb.trim().to_lowercase();
        if noun.trim().is_empty() {
            return ActionResult::say(format!("What do you want to {}?", verb));
        }
        let view = match self.view(world, state) {
            Some(v) => v,
            None => return ActionResult::say("You're nowhere."),
        };
        let mut things = self.carried(world, state);
        things.extend(view.objects.iter().cloned());
        let object = match resolve(noun, &things, the_object) {
            Ok(Some(o)) => o,
            Err(which) => return ActionResult::say((self.ask_which)(&verb, &which)),
            Ok(None) => {
                return match resolve(noun, &view.characters, person) {
                    Ok(Some(p)) => ActionResult::say(format!("You can't {} {}.", verb, person(&p))),
                    Err(which) => ActionResult::say((self.ask_which)(&verb, &which)),
                    Ok(None) => ActionResult::say(format!("There's no {} here.", noun.trim())),
                }
            }
        };
        let cant = || ActionResult::say(format!("You can't {} {}.", verb, the_object(&object)));
        let Some(card) = world.object_at(&object.id, At::play(state)) else { return cant() };
        let key = verb.replace(' ', "-");
        let Some((found, action)) = card.actions.iter().find(|(name, _)| name.eq_ignore_ascii_case(&key)) else {
            return cant();
        };
        let (key, action) = (found.clone(), action.clone());

        let name = object_name(&object);
        if action.requires_has.iter().any(|id| !state.inventory.contains(id)) {
            return ActionResult::say(match card.responses.get(&format!("{} refused", key)) {
                Some(text) if !text.trim().is_empty() => text.trim().to_string(),
                _ => builtin_refusal(&verb, &name, state.turn),
            });
        }

        let mut changes = Vec::new();
        let mut notes = Vec::new();
        for (target, to) in &action.becomes {
            match world.thing_by_id(target) {
                Ok(thing) => {
                    let (kind, id) = (thing.kind, thing.id.clone());
                    match state.switch_state(world, kind, &id, to) {
                        Ok(()) => changes.push(Change::Became { type_name: kind.type_name().to_string(), id, state: to.clone() }),
                        Err(e) => notes.push(e.to_string()),
                    }
                }
                Err(e) => notes.push(format!("`becomes:` {}", e)),
            }
        }
        for (name, value) in &action.sets {
            let name = match name.strip_prefix("this.") {
                Some(rest) => format!("{}.{}", object.id, rest),
                None => name.clone(),
            };
            state.values.insert(name.clone(), value.clone());
            changes.push(Change::Set { name, value: value.clone() });
        }
        if let Some(chapter) = &action.chapter {
            let scope = world.thing(EntityKind::Object, &object.id).map(|t| t.scope.clone()).unwrap_or_default();
            match state.enter_chapter(world, &scope, chapter) {
                Ok(()) => changes.push(Change::EnteredChapter { name: state.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_default() }),
                Err(e) => notes.push(e.to_string()),
            }
        }

        let text = match card.responses.get(&key) {
            Some(text) if !text.trim().is_empty() => text.trim().to_string(),
            _ => builtin_reply(&verb, &name, state.turn),
        };
        let mut result = ActionResult { text, changes, note: None };
        if !notes.is_empty() {
            result.note = Some(notes.join("; "));
        }
        result
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
            .map(|id| EntityRef { id: id.clone(), name: world.object_at(id, At::play(state)).map(|o| o.title) })
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

        // A state that's gone falls back to the starting state; a thing
        // that's gone is dropped.
        for (type_name, by_id) in self.states.iter_mut() {
            let kind = EntityKind::from_type_name(type_name);
            by_id.retain(|id, state| {
                let Some(thing) = kind.and_then(|k| world.thing(k, id)) else {
                    warnings.push(format!("{} `{}` no longer exists, so its state `{}` is dropped", type_name, id, state));
                    return false;
                };
                if thing.states.contains_key(state.as_str()) {
                    return true;
                }
                let start = thing.starting_state.as_deref().map_or("its index".to_string(), |s| format!("`{}`", s));
                warnings.push(format!("{} `{}` has no state `{}` any more, so it is back in {}", type_name, id, state, start));
                false
            });
        }
        self.states.retain(|_, by_id| !by_id.is_empty());

        // A chapter that's gone falls back to the nearest earlier one in
        // its scope, or to no chapter.
        if let Some(saved) = self.chapter.clone() {
            if world.chapter_key(&saved).is_none() {
                let earlier = world
                    .chapters()
                    .iter()
                    .filter(|c| c.key.scope.to_string_lossy().replace('\\', "/") == saved.scope && c.order < saved.order)
                    .max_by(|a, b| a.order.cmp(&b.order));
                match earlier {
                    Some(c) => {
                        warnings.push(format!(
                            "chapter `{}` no longer exists, so the story is back in `{}`",
                            saved.name(),
                            c.name()
                        ));
                        self.chapter = Some(world.saved_chapter(c));
                    }
                    None => {
                        warnings.push(format!("chapter `{}` no longer exists, so the story is in no chapter", saved.name()));
                        self.chapter = None;
                    }
                }
            }
        }
        warnings
    }

    /// Switch a thing to one of its states (`design/STATES.md` › Changing
    /// state). States can go back as well as forward.
    pub fn switch_state(&mut self, world: &WorldEngine, kind: EntityKind, id: &str, state: &str) -> Result<()> {
        let thing = world
            .thing(kind, id)
            .ok_or_else(|| anyhow::anyhow!("no {} `{}` is loaded", kind.type_name(), id))?;
        if !thing.states.contains_key(state) {
            bail!("{} `{}` has no state `{}`", kind.type_name(), id, state);
        }
        self.set_state(kind.type_name(), id, state, thing.starting_state.as_deref());
        Ok(())
    }

    /// Move the story to the chapter `name`, as seen from `scope` (the
    /// folder of the thing whose action moves it; empty for the root).
    /// Chapters can go back; only the new chapter and its parents apply.
    pub fn enter_chapter(&mut self, world: &WorldEngine, scope: &Path, name: &str) -> Result<()> {
        let chapter = world
            .find_chapter(scope, name)
            .ok_or_else(|| anyhow::anyhow!("no chapter `{}` can be seen from {}", name, if scope.as_os_str().is_empty() { "the root".to_string() } else { format!("{}/", scope.display()) }))?;
        self.chapter = Some(world.saved_chapter(chapter));
        Ok(())
    }
}

/// The current save format. Version 1 (before states and chapters) loads
/// as is; its unused `flags` are ignored.
const SAVE_VERSION: u32 = 2;

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
    if !(1..=SAVE_VERSION).contains(&save.thwarter_save) {
        bail!("{} is save format {}; this thwart reads format {}", path.display(), save.thwarter_save, SAVE_VERSION);
    }
    let mut state = save.state;
    let warnings = state.reconcile(world);
    Ok((state, warnings))
}

/// The thing `noun` names among `things`: an exact id, the id with spaces
/// for hyphens, the name ignoring case, then a unique whole word of
/// either. `Err` holds the candidates, phrased, when more than one matches.
fn resolve(noun: &str, things: &[EntityRef], phrase: fn(&EntityRef) -> String) -> std::result::Result<Option<EntityRef>, Vec<String>> {
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
        _ => Err(unique.iter().map(|t| phrase(t)).collect()),
    }
}

/// The reply to an action with no response block, from its verb and the
/// object's name, varied by turn so repeats don't read the same.
fn builtin_reply(verb: &str, name: &str, turn: u64) -> String {
    let forms = ["You {verb} the {name}.", "You {verb} the {name}. Something changes."];
    forms[(turn as usize) % forms.len()].replace("{verb}", verb).replace("{name}", name)
}

/// The reply to a refused action with no `refused` block.
fn builtin_refusal(verb: &str, name: &str, turn: u64) -> String {
    let forms = ["You can't {verb} the {name} yet.", "Not yet: you can't {verb} the {name}."];
    forms[(turn as usize) % forms.len()].replace("{verb}", verb).replace("{name}", name)
}

fn join_or(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} or {}", init.join(", "), last),
    }
}

/// An object's name for use mid-sentence: lowercased, since object titles
/// are often written in title case ("Brass Lantern" reads "the brass
/// lantern"). Characters keep their names as written (`person`).
fn object_name(object: &EntityRef) -> String {
    object.display_name().to_lowercase()
}

fn the_object(object: &EntityRef) -> String {
    format!("the {}", object_name(object))
}

fn person(character: &EntityRef) -> String {
    character.display_name()
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
    fn object_names_are_lowercased_mid_sentence_and_people_keep_theirs() {
        let (dir, mut world) = world();
        world.set_file(
            &dir.path().join("objects/brass-lantern.md"),
            "---\nid: brass-lantern\ntitle: Brass Lantern\ntype: object\nobject_type: tool\n---\n",
        );
        world.set_file(&dir.path().join("npcs/queen.md"), "---\nid: queen\ntitle: Queen Aldric\ntype: character\n---\n");
        world.set_file(
            &dir.path().join("castle/yard.md"),
            "---\nid: castle/yard\ntitle: Yard\ntype: location\ncharacters: [king, queen]\nobjects: [brass-lantern]\n---\n",
        );
        let (game, mut state) = play(&world);
        assert_eq!(game.act(&world, &mut state, &PlayAction::Take("lantern".into())).text, "You take the brass lantern.");
        assert_eq!(game.act(&world, &mut state, &PlayAction::Take("Brass Lantern".into())).text, "You already have the brass lantern.");
        assert_eq!(
            game.act(&world, &mut state, &PlayAction::Examine("aldric".into())).text,
            "Which do you mean: King Aldric or Queen Aldric?"
        );
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
        assert!(text.starts_with("thwarter_save: 2\n"), "{}", text);

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
    /// The yard world plus a vault bundle (closed, open), a lamp object the
    /// chapter `after` puts in the yard, and root chapters 10-before, 20-after.
    fn story() -> (tempfile::TempDir, WorldEngine) {
        let (dir, mut world) = world();
        let d = dir.path();
        let set = |world: &mut WorldEngine, rel: &str, text: &str| world.set_file(&d.join(rel), text);
        set(&mut world, "castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\nconnections:\n  south: castle/yard\n  down: castle/vault\n---\nStone.\n");
        set(&mut world, "castle/vault/index.md", "---\nid: castle/vault\ntitle: Vault\ntype: location\nstate: closed\nconnections:\n  up: castle/hall\n---\n");
        set(&mut world, "castle/vault/closed.md", "---\ninherits: index\n---\nA shut door.\n");
        set(&mut world, "castle/vault/open.md", "---\ninherits: index\nconnections:\n  down: castle/hall\n---\nAn open door.\n");
        set(&mut world, "objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: tool\n---\n");
        set(&mut world, "castle/yard/index.md", "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  north: castle/hall\nobjects: [lamp]\n---\nGrass.\n");
        world.remove_file(&d.join("castle/yard.md"));
        set(&mut world, "castle/yard/chapters/after.md", "---\nobjects: [lamp, well]\n---\nAshes.\n");
        set(&mut world, "chapters/10-before.md", "---\ntitle: Before\n---\n");
        set(&mut world, "chapters/20-after.md", "---\ntitle: After\n---\n");
        assert!(world.current_problems(false).is_empty(), "{:?}", world.current_problems(false));
        (dir, world)
    }

    #[test]
    fn play_sees_each_things_current_state_and_counts_visits_and_turns() {
        let (_d, world) = story();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        assert_eq!(state.visits_to("castle/hall"), 1);
        assert!(game.act(&world, &mut state, &PlayAction::Go("down".into())).text.contains("A shut door."));
        assert_eq!(game.act(&world, &mut state, &PlayAction::Go("down".into())).text, "You can't go down from here.");

        state.switch_state(&world, EntityKind::Location, "castle/vault", "open").unwrap();
        assert_eq!(state.state_of("location", "castle/vault"), Some("open"));
        assert!(game.act(&world, &mut state, &PlayAction::Look).text.contains("An open door."));
        game.act(&world, &mut state, &PlayAction::Go("down".into()));
        assert_eq!(state.location, "castle/hall");
        assert_eq!(state.visits_to("castle/hall"), 2);
        assert_eq!(state.turn, 4); // two goes, a refused go, a look

        // Switching back to the starting state leaves no delta.
        state.switch_state(&world, EntityKind::Location, "castle/vault", "closed").unwrap();
        assert!(state.states.is_empty());
        assert!(state.switch_state(&world, EntityKind::Location, "castle/vault", "ajar").is_err());
    }

    #[test]
    fn a_chapter_applies_its_overlays_and_never_duplicates_a_carried_object() {
        let (_d, world) = story();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        game.act(&world, &mut state, &PlayAction::Take("lamp".into()));
        state.enter_chapter(&world, Path::new("castle/yard"), "after").unwrap();
        let look = game.act(&world, &mut state, &PlayAction::Look).text;
        assert!(look.contains("Ashes."), "{}", look);
        assert!(look.contains("You see: Well.") && !look.contains("Lamp"), "{}", look);

        // Chapters can go back; only the new one applies.
        state.enter_chapter(&world, Path::new(""), "before").unwrap();
        assert!(game.act(&world, &mut state, &PlayAction::Look).text.contains("Grass."));
        assert!(state.enter_chapter(&world, Path::new(""), "never").is_err());
    }

    #[test]
    fn reconcile_falls_back_for_states_things_and_chapters_that_are_gone() {
        let (dir, mut world) = story();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        state.switch_state(&world, EntityKind::Location, "castle/vault", "open").unwrap();
        state.enter_chapter(&world, Path::new(""), "after").unwrap();
        state.states.entry("object".into()).or_default().insert("ghost".into(), "faint".into());

        world.remove_file(&dir.path().join("castle/vault/open.md"));
        world.remove_file(&dir.path().join("chapters/20-after.md"));
        world.remove_file(&dir.path().join("castle/yard/chapters/after.md"));
        let warnings = state.reconcile(&world);
        assert!(warnings.iter().any(|w| w == "location `castle/vault` has no state `open` any more, so it is back in `closed`"), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("object `ghost` no longer exists")));
        assert!(warnings.iter().any(|w| w == "chapter `after` no longer exists, so the story is back in `before`"));
        assert!(state.states.is_empty());
        assert_eq!(state.chapter.as_ref().unwrap().name(), "before");

        world.remove_file(&dir.path().join("chapters/10-before.md"));
        let warnings = state.reconcile(&world);
        assert_eq!(warnings, vec!["chapter `before` no longer exists, so the story is in no chapter".to_string()]);
        assert!(state.chapter.is_none());
    }

    #[test]
    fn saves_keep_states_and_chapters_and_version_1_still_loads() {
        let (_d, world) = story();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        state.switch_state(&world, EntityKind::Location, "castle/vault", "open").unwrap();
        state.enter_chapter(&world, Path::new(""), "after").unwrap();
        state.values.insert("story.king".into(), "dead".into());
        game.act(&world, &mut state, &PlayAction::Look);
        let saves = tempfile::tempdir().unwrap();
        let path = saves.path().join("s.yaml");
        save_game(&path, &state).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("castle/vault: open") && text.contains("story.king: dead"), "{}", text);
        let (loaded, warnings) = load_game(&path, &world).unwrap();
        assert_eq!(loaded, state);
        assert!(warnings.is_empty(), "{:?}", warnings);

        fs::write(&path, "thwarter_save: 1\nstate:\n  location: castle/hall\n  inventory: [lamp]\n  taken:\n    castle/yard: [lamp]\n  flags: {}\n").unwrap();
        let (old, warnings) = load_game(&path, &world).unwrap();
        assert_eq!(old.inventory, vec!["lamp"]);
        assert!(old.states.is_empty() && old.chapter.is_none() && old.turn == 0);
        assert!(warnings.is_empty());
    }
    fn with_lever(world: &mut WorldEngine, dir: &Path) {
        world.set_file(
            &dir.join("objects/lever.md"),
            "---\nid: lever\ntitle: Iron lever\ntype: object\nobject_type: mechanism\nweight: immovable\nactions:\n  pull:\n    becomes: { castle/vault: open }\n    sets: { story.vault: open, this.pulled: yes }\n    chapter: after\n    requires: { has: lamp }\n  shove:\n    requires: { has: lamp }\n---\nA lever.\n\n<!-- thwarter:action pull -->\nGears grind somewhere below.\n\n<!-- thwarter:action pull refused -->\nIt won't move without light.\n\n<!-- thwarter:action kick -->\nOw.\n",
        );
        world.set_file(
            &dir.join("castle/hall.md"),
            "---\nid: castle/hall\ntitle: Hall\ntype: location\nconnections:\n  south: castle/yard\n  down: castle/vault\nobjects: [lever]\n---\nStone.\n",
        );
    }

    #[test]
    fn an_objects_action_switches_states_sets_values_and_moves_the_story() {
        let (dir, mut world) = story();
        with_lever(&mut world, dir.path());
        assert!(world.current_problems(false).is_empty(), "{:?}", world.current_problems(false));
        let warnings = world.current_warnings();
        for expected in [
            "object `lever`, action `kick`: has a response but no mechanics",
            "object `lever`, action `shove`: has no response",
            "object `lever`, action `shove`: has `requires:` but no refusal",
        ] {
            assert!(warnings.iter().any(|w| w.starts_with(expected)), "{}: {:?}", expected, warnings);
        }
        assert!(!warnings.iter().any(|w| w.contains("action `pull`")), "{:?}", warnings);
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        let pull = PlayAction::Use { verb: "pull".into(), noun: "lever".into() };
        let shove = PlayAction::Use { verb: "shove".into(), noun: "lever".into() };

        game.act(&world, &mut state, &PlayAction::Go("north".into()));
        assert_eq!(game.act(&world, &mut state, &pull).text, "It won't move without light.");
        assert_eq!(game.act(&world, &mut state, &shove).text, "Not yet: you can't shove the iron lever.");
        assert!(state.states.is_empty());

        game.act(&world, &mut state, &PlayAction::Go("south".into()));
        game.act(&world, &mut state, &PlayAction::Take("lamp".into()));
        game.act(&world, &mut state, &PlayAction::Go("north".into()));
        assert_eq!(game.act(&world, &mut state, &shove).text, "You shove the iron lever. Something changes.");
        let r = game.act(&world, &mut state, &pull);
        assert_eq!(r.text, "Gears grind somewhere below.");
        assert!(r.note.is_none(), "{:?}", r.note);
        assert_eq!(state.state_of("location", "castle/vault"), Some("open"));
        assert_eq!(state.values["story.vault"], "open");
        assert_eq!(state.values["lever.pulled"], "yes");
        assert_eq!(state.chapter.as_ref().unwrap().name(), "after");
        assert!(r.changes.contains(&Change::EnteredChapter { name: "after".into() }));

        let kick = |noun: &str| PlayAction::Use { verb: "kick".into(), noun: noun.into() };
        assert_eq!(game.act(&world, &mut state, &kick("iron lever")).text, "Ow.");
        let lick = PlayAction::Use { verb: "lick".into(), noun: "lever".into() };
        assert_eq!(game.act(&world, &mut state, &lick).text, "You can't lick the iron lever.");
        assert_eq!(game.act(&world, &mut state, &kick("dragon")).text, "There's no dragon here.");
        assert_eq!(game.act(&world, &mut state, &kick("")).text, "What do you want to kick?");
    }

    #[test]
    fn validate_reports_what_an_action_names_that_does_not_exist() {
        let (dir, mut world) = story();
        world.set_file(
            &dir.path().join("objects/lever.md"),
            "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: mechanism\nactions:\n  pull:\n    becomes: { castle/vault: ajar, castle/nowhere: open }\n    chapter: never\n    requires: { has: unicorn }\n---\n",
        );
        let problems = world.current_problems(false);
        for expected in [
            "object `lever`, action `pull`: `becomes:` names state `ajar`, but location `castle/vault` has no such state file",
            "object `lever`, action `pull`: `becomes:` nothing has the id `castle/nowhere`",
            "object `lever`, action `pull`: `chapter: never` names no chapter",
            "object `lever`, action `pull`: `requires: { has: unicorn }` names no object",
        ] {
            assert!(problems.iter().any(|p| p.starts_with(expected)), "{}: {:?}", expected, problems);
        }
    }

    #[test]
    fn the_which_reply_can_be_replaced() {
        let (_d, world) = world();
        let mut game = GameEngine::new();
        game.ask_which = |verb, c| format!("Which one do you want to {}? {}", verb, c.join(" or "));
        let mut state = game.start(&world, "castle/yard").unwrap();
        let r = game.act(&world, &mut state, &PlayAction::Take("sword".into()));
        assert_eq!(r.text, "Which one do you want to take? the rusty sword or the old sword");
    }
}
