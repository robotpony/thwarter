//! Play: applying player actions to a `PlayState` (REQ-ENG-5). Nothing
//! here writes a world file (REQ-CON-3); the world is read-only.

use crate::engines::location_view::{expand_direction, EntityRef, LocationView};
use crate::engines::variables::Scope;
use crate::engines::{At, EntityKind, Visibility, WorldEngine};
use crate::models::game::{ActionResult, Change, PlayAction, PlayState};
use crate::engines::turn::Outcome;
use crate::render::prose_plain;
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
    /// `noun` is what the player typed, without "the"; `turn` lets a
    /// reply vary. Used when the candidates share no template with a
    /// `## When: #ambiguous` section.
    /// `None` uses the world's `[engine] which` (`design/ENGINE_TEXT.md`).
    pub ask_which: Option<fn(verb: &str, noun: &str, candidates: &[String], turn: u64) -> String>,
}

impl Default for GameEngine {
    fn default() -> Self {
        Self { ask_which: None }
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
        // Every reply is cleaned of terminal control sequences (REQ-CFG-9):
        // built-in replies quote titles, which come from the world.
        let mut result = self.act_raw(world, state, action);
        result.text = crate::render::clean_text(&result.text);
        result.note = result.note.map(|n| crate::render::clean_text(&n));
        result
    }

    fn act_raw(&self, world: &WorldEngine, state: &mut PlayState, action: &PlayAction) -> ActionResult {
        // `again` is the last action, run once more (REQ-PC-27).
        let action = match action {
            PlayAction::Again => match state.last_action.clone() {
                Some(last) => last,
                None => return ActionResult::say(self.say(world, state, "nothing_to_repeat", None, &[])),
            },
            other => other.clone(),
        };
        let action = &action;
        state.last_action = Some(action.clone());
        // What `it` means next (REQ-PC-28): the thing this action names.
        let named = match action {
            PlayAction::Take(n) | PlayAction::Examine(n) | PlayAction::Drop(n) | PlayAction::Talk(n) => Some(n.clone()),
            PlayAction::Give { noun, .. } => Some(noun.clone()),
            PlayAction::Use { noun, .. } => Some(noun.clone()),
            _ => None,
        };
        if let Some(n) = named.filter(|n| !n.trim().is_empty()) {
            state.it = Some(n);
        }
        state.turn += 1;
        if world.get_location(&state.location).is_none() {
            return ActionResult::say(self.say(world, state, "nowhere_gone", None, &[]))
                .with_note(format!("location `{}` isn't in the world any more; use /play here to move", state.location));
        }
        match action {
            PlayAction::Look => self.look(world, state),
            PlayAction::Go(direction) => self.go(world, state, direction),
            PlayAction::Take(noun) => self.take(world, state, noun),
            PlayAction::Examine(noun) => self.examine(world, state, noun),
            PlayAction::Inventory => self.inventory(world, state),
            PlayAction::Search => self.search(world, state, false),
            PlayAction::Drop(noun) => self.drop(world, state, noun),
            PlayAction::TakeAll => self.take_all(world, state),
            PlayAction::Wait => ActionResult::say(self.say(world, state, "waited", None, &[])),
            PlayAction::Verb(verb) => self.bare_verb(world, state, verb),
            PlayAction::Again => ActionResult::say(self.say(world, state, "nothing_to_repeat", None, &[])),
            PlayAction::Give { noun, to } => self.give(world, state, noun, to),
            PlayAction::Talk(noun) => self.talk(world, state, noun),
            PlayAction::Ask { person, topic } => self.ask(world, state, person, topic),
            PlayAction::Where => self.where_am_i(world, state),
            PlayAction::LookToward(direction) => self.look_toward(world, state, direction),
            PlayAction::Use { verb, noun, with } => self.use_object(world, state, verb, noun, with.as_deref()),
        }
    }

    /// The current room as play mode shows it: each thing in its current
    /// version, without what's been taken or is carried, so a chapter that
    /// adds an object the player holds never shows a second one.
    pub fn view(&self, world: &WorldEngine, state: &PlayState) -> Option<LocationView> {
        let mut view = self.whole_view(world, state)?;
        view.objects.retain(|o| shown(world, state, EntityKind::Object, &o.id));
        view.characters.retain(|c| shown(world, state, EntityKind::Character, &c.id));
        view.exits.retain(|e| {
            (!e.target_exists || shown(world, state, EntityKind::Location, &e.target))
                && e.door.as_ref().map_or(true, |d| shown(world, state, EntityKind::Door, &d.id))
        });
        // What the room's objects and doors hold, however deep, in this
        // game (REQ-CF-31).
        view.held.clear();
        let mut holders: Vec<(String, String)> = view.objects.iter().map(|o| (o.id.clone(), o.display_name())).collect();
        holders.extend(doors_here(&view).into_iter().map(|d| (d.id.clone(), d.display_name())));
        let mut seen: std::collections::BTreeSet<String> = holders.iter().map(|(id, _)| id.clone()).collect();
        let mut i = 0;
        while i < holders.len() {
            let (holder, holder_name) = holders[i].clone();
            i += 1;
            for id in self.contents(world, state, &holder) {
                if seen.insert(id.clone()) {
                    let thing = EntityRef { name: world.object_at(&id, At::play(state)).map(|o| o.title), state: world.state_at(EntityKind::Object, &id, At::play(state)), id: id.clone() };
                    holders.push((id, thing.display_name()));
                    view.held.push(crate::engines::Held { thing, holder: holder.clone(), holder_name: holder_name.clone(), word: world.holds_word(&holder, At::play(state)) });
                }
            }
        }
        Some(view)
    }

    /// The room with everything in it, however hidden, minus what's been
    /// taken or is carried.
    fn whole_view(&self, world: &WorldEngine, state: &PlayState) -> Option<LocationView> {
        let mut view = world.describe_location_at(&state.location, At::play(state))?;
        // Each description with the sections this turn matches.
        let body = self.describe(world, state, EntityKind::Location, &state.location, &view.body);
        view.body = world.fill(&body, &Scope { at: At::play(state), this: Some((EntityKind::Location, &state.location)), here: None });
        for door in view.exits.iter_mut().filter_map(|e| e.door.as_mut()) {
            let text = self.describe(world, state, EntityKind::Door, &door.id, &door.description);
            door.description = world.fill(&text, &Scope { at: At::play(state), this: Some((EntityKind::Door, &door.id)), here: None });
            door.state = self.current_state(world, state, EntityKind::Door, &door.id);
        }
        view.objects
            .retain(|o| !state.was_taken(&state.location, &o.id) && !state.inventory.contains(&o.id));
        // What the player dropped here.
        for id in state.held.get(&state.location).into_iter().flatten() {
            if !view.objects.iter().any(|o| &o.id == id) {
                let at = At::play(state);
                view.objects.push(EntityRef { id: id.clone(), name: world.object_at(id, at).map(|o| o.title), state: world.state_at(EntityKind::Object, id, at) });
            }
        }
        Some(view)
    }

    /// The reply when `noun` matches several things: the
    /// `## When: #ambiguous` section of the nearest template they all
    /// share, else `ask_which`.
    #[allow(clippy::too_many_arguments)]
    fn which(&self, world: &WorldEngine, state: &PlayState, verb: &str, noun: &str, kind: EntityKind, candidates: &[EntityRef]) -> String {
        let chains: Vec<Vec<&crate::engines::layers::Layer>> = candidates
            .iter()
            .map(|c| world.thing(kind, &c.id).map(|t| t.templates.iter().rev().collect()).unwrap_or_default())
            .collect();
        if let Some(first) = chains.first() {
            for template in first {
                let shared = chains[1..].iter().all(|chain| chain.iter().any(|t| t.path == template.path));
                if let (true, Some(text)) = (shared, template.reply("#ambiguous")) {
                    if !text.trim().is_empty() {
                        return prose(world, &world.fill(text.trim(), &Scope { at: At::play(state), this: None, here: None }));
                    }
                }
            }
        }
        if kind == EntityKind::Character {
            let names: Vec<String> = candidates.iter().map(person).collect();
            return self.say(world, state, "who", None, &[("verb", verb), ("or", &join_or(&names))]);
        }
        let names: Vec<String> = candidates.iter().map(the_object).collect();
        let noun = noun.trim().to_lowercase();
        let noun = noun.strip_prefix("the ").unwrap_or(&noun).trim().to_string();
        match self.ask_which {
            Some(ask) => ask(verb, &noun, &names, state.turn),
            None => self.say(world, state, "which", None, &[("noun", &noun), ("verb", verb), ("or", &join_or(&names))]),
        }
    }

    /// Find the faint things here not yet found, and say what was found.
    /// `closely` is examining the room, which words its finds its own way.
    fn search(&self, world: &WorldEngine, state: &mut PlayState, closely: bool) -> ActionResult {
        let Some(view) = self.whole_view(world, state) else { return ActionResult::say(self.say(world, state, "nowhere", None, &[])) };
        let mut found: Vec<(EntityKind, String, String)> = Vec::new();
        let faint = |kind, id: &str| world.visibility(kind, id, At::play(state)) == Visibility::Faint && !state.has_noticed(kind.type_name(), id);
        for o in &view.objects {
            if faint(EntityKind::Object, &o.id) {
                found.push((EntityKind::Object, o.id.clone(), the_object(o)));
            }
        }
        for c in &view.characters {
            if faint(EntityKind::Character, &c.id) {
                found.push((EntityKind::Character, c.id.clone(), person(c)));
            }
        }
        for e in view.exits.iter().filter(|e| e.target_exists) {
            if faint(EntityKind::Location, &e.target) {
                found.push((EntityKind::Location, e.target.clone(), self.say(world, state, "a_way", None, &[("direction", &e.direction)])));
            }
        }
        // The room's `## When: #searched` section, if any, comes first.
        let said = self.searched(world, state).map(|t| prose(world, &world.fill(&t, &Scope { at: At::play(state), this: Some((EntityKind::Location, &state.location)), here: None })));
        if found.is_empty() {
            return ActionResult::say(match said {
                Some(text) => text,
                None => self.say(world, state, if closely { "look_closely_nothing" } else { "search_nothing" }, None, &[]),
            });
        }
        let mut changes = Vec::new();
        for (kind, id, _) in &found {
            state.notice(kind.type_name(), id);
            changes.push(Change::Noticed { type_name: kind.type_name().to_string(), id: id.clone() });
        }
        let names: Vec<String> = found.into_iter().map(|(_, _, name)| name).collect();
        let text = match said {
            Some(said) => format!("{}\n\n{}", said, self.say(world, state, "found", None, &[("list", &join_and(&names))])),
            None => self.say(world, state, if closely { "look_closely_found" } else { "search_found" }, None, &[("list", &join_and(&names))]),
        };
        ActionResult { text, changes, note: None, topics: Vec::new() }
    }

    fn look(&self, world: &WorldEngine, state: &PlayState) -> ActionResult {
        match self.view(world, state) {
            Some(view) => ActionResult::say(crate::render::play_blocks(&view, &|n| world.link_for(n), world.play_text()).to_plain()),
            None => ActionResult::say(self.say(world, state, "nowhere", None, &[])),
        }
    }

    fn go(&self, world: &WorldEngine, state: &mut PlayState, direction: &str) -> ActionResult {
        let direction = expand_direction(direction);
        if direction.is_empty() {
            return ActionResult::say(self.say(world, state, "go_where", None, &[]));
        }
        let Some(here) = world.location_at(&state.location, At::play(state)) else {
            return ActionResult::say(self.say(world, state, "nowhere", None, &[]));
        };
        // `go to the river`: the whole phrase as an exit first, then
        // without the `to` and an article (REQ-PC-20).
        let direction = match here.connections.contains_key(&direction) {
            true => direction,
            false => {
                let words: Vec<&str> = direction.split_whitespace().collect();
                let rest = match words.as_slice() {
                    ["to", "the" | "a" | "an", rest @ ..] | ["to", rest @ ..] if !rest.is_empty() => expand_direction(&rest.join(" ")),
                    _ => direction.clone(),
                };
                if here.connections.contains_key(&rest) { rest } else { self.exit_to_place(world, state, &rest).unwrap_or(direction) }
            }
        };
        let Some(named) = here.connections.get(&direction) else {
            return ActionResult::say(self.say(world, state, "no_exit", None, &[("direction", &direction)]));
        };
        // An exit names a room, or a door that says where it leads.
        let mut target = named.clone();
        if world.get_location(named).is_none() {
            if let Some(door) = world.door_at(named, At::play(state)) {
                if !shown(world, state, EntityKind::Door, &door.id) {
                    return ActionResult::say(self.say(world, state, "no_exit", None, &[("direction", &direction)]));
                }
                if !door.passable {
                    // The door's `## When: #go` section says why, else built in.
                    // A built-in verb leaves the door where it is: that's its
                    // outcome, so a state file's `## When: #go` answers.
                    let stays = Outcome(self.current_state(world, state, EntityKind::Door, &door.id).unwrap_or_else(|| "no-action".into()));
                    let replies = self.replies(world, state, "go", (EntityKind::Door, &door.id), None, &stays);
                    return ActionResult::say(match replies.is_empty() {
                        true => {
                            let locked = self.current_state(world, state, EntityKind::Door, &door.id).as_deref() == Some("locked");
                            self.say(world, state, if locked { "door_locked" } else { "door_closed" }, Some((EntityKind::Door, &door.id)), &[("door", &door.title.to_lowercase())])
                        }
                        false => self.say_replies(world, state, &replies),
                    });
                }
                match door.leads_to(&state.location) {
                    Some(to) => target = to.to_string(),
                    None => {
                        return ActionResult::say(self.say(world, state, "no_exit", None, &[("direction", &direction)]))
                            .with_note(format!("door `{}` doesn't say where it leads from `{}`", door.id, state.location))
                    }
                }
            }
        }
        let target = &target;
        if world.get_location(target).is_some() && !shown(world, state, EntityKind::Location, target) {
            return ActionResult::say(self.say(world, state, "no_exit", None, &[("direction", &direction)]));
        }
        if world.get_location(target).is_none() {
            return ActionResult::say(self.say(world, state, "no_exit", None, &[("direction", &direction)]))
                .with_note(format!("the {} exit leads to `{}`, which doesn't exist yet", direction, target));
        }
        let from = std::mem::replace(&mut state.location, target.clone());
        *state.visits.entry(target.clone()).or_default() += 1;
        let mut result = self.look(world, state);
        result.changes.push(Change::Moved { from, to: target.clone() });
        result
    }

    /// The exit that leads to the room `place` names, by its title, id or
    /// the id's last part: `go closet`, `enter the closet` (REQ-PC-22).
    /// None when no exit or more than one does.
    fn exit_to_place(&self, world: &WorldEngine, state: &PlayState, place: &str) -> Option<String> {
        let words: Vec<&str> = place.split_whitespace().collect();
        let place = match words.as_slice() {
            ["the" | "a" | "an", rest @ ..] if !rest.is_empty() => rest.join(" "),
            _ => words.join(" "),
        };
        let here = world.location_at(&state.location, At::play(state))?;
        let mut found: Vec<String> = Vec::new();
        for (direction, named) in &here.connections {
            let target = match world.door_at(named, At::play(state)) {
                Some(door) if world.get_location(named).is_none() => match door.leads_to(&state.location) {
                    Some(to) => to.to_string(),
                    None => continue,
                },
                _ => named.clone(),
            };
            let Some(room) = world.location_at(&target, At::play(state)) else { continue };
            let last = target.rsplit('/').next().unwrap_or(&target).to_lowercase();
            if room.name.to_lowercase() == place || target.to_lowercase() == place || last == place {
                found.push(direction.clone());
            }
        }
        found.sort();
        found.dedup();
        match found.as_slice() {
            [one] => Some(one.clone()),
            _ => None,
        }
    }

    /// `take all` (REQ-PC-26): each loose thing here, one reply each, in
    /// the room's order. Scenery, immovable things and people stay.
    fn take_all(&self, world: &WorldEngine, state: &mut PlayState) -> ActionResult {
        let Some(view) = self.view(world, state) else {
            return ActionResult::say(self.say(world, state, "nowhere", None, &[]));
        };
        let loose: Vec<String> = view
            .objects
            .iter()
            .filter(|o| world.object_at(&o.id, At::play(state)).map_or(false, |c| !c.weight.as_deref().map_or(false, |w| w.eq_ignore_ascii_case("immovable")) && !c.object_type.eq_ignore_ascii_case("scenery")))
            .map(|o| o.id.clone())
            .collect();
        if loose.is_empty() {
            return ActionResult::say(self.say(world, state, "take_all_nothing", None, &[]));
        }
        let mut result = ActionResult::say(String::new());
        let mut lines = Vec::new();
        for id in loose {
            let taken = self.take(world, state, &id);
            lines.push(taken.text);
            result.changes.extend(taken.changes);
        }
        result.text = lines.join("\n");
        result
    }

    /// `drop <thing>`: a carried thing goes down in this room, back where
    /// it was if it was taken from here. A `## When: #drop` section
    /// rewords the reply.
    fn drop(&self, world: &WorldEngine, state: &mut PlayState, noun: &str) -> ActionResult {
        if noun.trim().is_empty() {
            return ActionResult::say(self.say(world, state, "drop_what", None, &[]));
        }
        let object = match resolve(noun, &self.carried(world, state), the_object) {
            Ok(Some(o)) => o,
            Ok(None) => return ActionResult::say(self.say(world, state, "not_carried", None, &[("noun", noun.trim())])),
            Err(which) => return ActionResult::say(self.which(world, state, "drop", noun, EntityKind::Object, &which)),
        };
        let name = object_name(&object);
        let room = self.put_down(state, &object.id);
        let stays = Outcome(self.current_state(world, state, EntityKind::Object, &object.id).unwrap_or_else(|| "no-action".into()));
        let replies = self.replies(world, state, "drop", (EntityKind::Object, &object.id), None, &stays);
        let text = match replies.is_empty() {
            true => self.say(world, state, "dropped", Some((EntityKind::Object, &object.id)), &[("name", &name)]),
            false => self.say_replies(world, state, &replies),
        };
        ActionResult { text, changes: vec![Change::Dropped { object: object.id.clone(), room }], note: None, topics: Vec::new() }
    }

    /// A carried thing put down in the player's room: back as written if
    /// it was taken from here, else the room holds it. Returns the room.
    fn put_down(&self, state: &mut PlayState, id: &str) -> String {
        state.inventory.retain(|x| x != id);
        let room = state.location.clone();
        let back_home = state.taken.get_mut(&room).and_then(|list| list.iter().position(|x| x == id).map(|i| list.remove(i))).is_some();
        state.taken.retain(|_, l| !l.is_empty());
        if !back_home {
            state.held.entry(room.clone()).or_default().push(id.to_string());
        }
        room
    }

    /// A story verb with no thing (REQ-PC-29): the room's `## When: #jump`
    /// section, else `[engine]`'s reply for the verb.
    fn bare_verb(&self, world: &WorldEngine, state: &mut PlayState, verb: &str) -> ActionResult {
        let room = state.location.clone();
        let replies = self.replies(world, state, verb, (EntityKind::Location, &room), None, &Outcome("no-action".into()));
        if !replies.is_empty() {
            return ActionResult::say(self.say_replies(world, state, &replies));
        }
        ActionResult::say(self.say(world, state, verb, None, &[]))
    }

    /// What a story verb does to a thing when nothing in the story answers
    /// it (REQ-PC-29, 30): `throw` puts it down here; `smash` and `break`
    /// move it to its `broken` state; `stack` puts it on a thing that
    /// holds things; `push` and `pull` move a `pushable: true` thing
    /// through an exit, the player with it. `jump`, `run`, `kick` and
    /// `hit` only say so.
    fn story_effect(&self, world: &WorldEngine, state: &mut PlayState, verb: &str, target: (EntityKind, &EntityRef), used: Option<(EntityKind, &str)>, toward: Option<&str>) -> ActionResult {
        let (kind, object) = target;
        let about = Some((kind, object.id.as_str()));
        let name = object_name(object);
        let carried = state.inventory.contains(&object.id);
        match verb {
            "throw" => {
                if !carried {
                    return ActionResult::say(self.say(world, state, "not_carried", None, &[("noun", &name)]));
                }
                let room = self.put_down(state, &object.id);
                ActionResult { text: self.say(world, state, "thrown", about, &[("name", &name)]), changes: vec![Change::Dropped { object: object.id.clone(), room }], note: None, topics: Vec::new() }
            }
            "smash" | "break" => {
                let has_broken = world.thing(kind, &object.id).map_or(false, |t| t.has_state("broken"));
                if !has_broken {
                    return ActionResult::say(self.say(world, state, "wont_break", about, &[("name", &name)]));
                }
                if self.current_state(world, state, kind, &object.id).as_deref() == Some("broken") {
                    return ActionResult::say(self.say(world, state, "already_broken", about, &[("name", &name)]));
                }
                match state.switch_state(world, kind, &object.id, "broken") {
                    Ok(()) => ActionResult {
                        text: self.say(world, state, "breaks", about, &[("name", &name)]),
                        changes: vec![Change::Became { type_name: kind.type_name().to_string(), id: object.id.clone(), state: "broken".into() }],
                        note: None,
                        topics: Vec::new(),
                    },
                    Err(e) => ActionResult::say(self.say(world, state, "wont_break", about, &[("name", &name)])).with_note(e.to_string()),
                }
            }
            "stack" => {
                let Some((_, holder)) = used else {
                    return ActionResult::say(self.say(world, state, "stack_on_what", None, &[("name", &name)]));
                };
                let holder_name = world.object_at(holder, At::play(state)).map(|o| o.title.to_lowercase()).or_else(|| world.door_at(holder, At::play(state)).map(|d| d.title.to_lowercase())).unwrap_or_else(|| holder.to_string());
                let holds = world.object_at(holder, At::play(state)).map_or(false, |o| o.holds.is_some() || !o.objects.is_empty());
                if !holds || holder == object.id {
                    return ActionResult::say(self.say(world, state, "cant_stack", Some((EntityKind::Object, holder)), &[("holder", &holder_name)]));
                }
                if !carried {
                    return ActionResult::say(self.say(world, state, "not_carried", None, &[("noun", &name)]));
                }
                state.inventory.retain(|x| x != &object.id);
                state.held.entry(holder.to_string()).or_default().push(object.id.clone());
                let word = world.holds_word(holder, At::play(state));
                ActionResult {
                    text: self.say(world, state, "stacked", about, &[("name", &name), ("word", &word), ("holder", &holder_name)]),
                    changes: vec![Change::Kept { object: object.id.clone(), by: holder.to_string() }],
                    note: None,
                    topics: Vec::new(),
                }
            }
            "push" | "pull" => {
                let pushable = world.object_at(&object.id, At::play(state)).and_then(|o| o.extra.get("pushable").cloned()).map_or(false, |v| v.as_bool() == Some(true) || v.as_str() == Some("yes"));
                let Some(direction) = toward.filter(|_| pushable && !carried) else {
                    return ActionResult::say(self.say(world, state, "wont_move", about, &[("verb", verb), ("name", &name)]));
                };
                let from = state.location.clone();
                let mut moved = self.go(world, state, direction);
                let Some(Change::Moved { to, .. }) = moved.changes.iter().find(|c| matches!(c, Change::Moved { .. })).cloned() else {
                    return moved;
                };
                // The thing goes too: out of the room it was in, into this one.
                let held_there = state.held.get_mut(&from).map(|l| {
                    let had = l.iter().any(|x| x == &object.id);
                    l.retain(|x| x != &object.id);
                    had
                });
                state.held.retain(|_, l| !l.is_empty());
                if held_there != Some(true) {
                    let taken = state.taken.entry(from.clone()).or_default();
                    if !taken.contains(&object.id) {
                        taken.push(object.id.clone());
                    }
                }
                let back_home = state.taken.get_mut(&to).and_then(|l| l.iter().position(|x| x == &object.id).map(|i| l.remove(i))).is_some();
                state.taken.retain(|_, l| !l.is_empty());
                if !back_home {
                    state.held.entry(to.clone()).or_default().push(object.id.clone());
                }
                let said = self.say(world, state, "pushed", about, &[("verb", verb), ("name", &name), ("direction", &expand_direction(direction))]);
                // The room's view again, now with the thing in it.
                let look = self.look(world, state).text;
                moved.text = format!("{}\n\n{}", said, look);
                moved.changes.push(Change::Dropped { object: object.id.clone(), room: to });
                moved
            }
            _ => {
                // The thing's own wording for nothing happening wins.
                let args = [("verb", verb), ("name", name.as_str())];
                match self.story_reply(world, state, "nothing", about, &args) {
                    Some(text) => ActionResult::say(prose(world, crate::engines::layers::strip_comments(&text).trim())),
                    None => ActionResult::say(self.say(world, state, &format!("{}_thing", verb), about, &[("name", &name)])),
                }
            }
        }
    }

    /// `give <thing> to <person>`: the person takes it only when the story
    /// says they do, with a `## When: #give @thing` section in them, the
    /// thing or the room; else they refuse (`give_refused`, which a
    /// `## When: #give #refused` section rewords) and the player keeps it.
    fn give(&self, world: &WorldEngine, state: &mut PlayState, noun: &str, to: &str) -> ActionResult {
        let Some(view) = self.view(world, state) else {
            return ActionResult::say(self.say(world, state, "nowhere", None, &[]));
        };
        let object = match resolve(noun, &self.carried(world, state), the_object) {
            Ok(Some(o)) => o,
            Ok(None) => return ActionResult::say(self.say(world, state, "not_carried", None, &[("noun", noun.trim())])),
            Err(which) => return ActionResult::say(self.which(world, state, "give", noun, EntityKind::Object, &which)),
        };
        let someone = match resolve(to, &view.characters, person) {
            Ok(Some(p)) => p,
            Ok(None) => return ActionResult::say(self.say(world, state, "not_here", None, &[("noun", to.trim())])),
            Err(which) => return ActionResult::say(self.which(world, state, "give", to, EntityKind::Character, &which)),
        };
        // Matched with only the given thing in hand, so `@lantern` means
        // what's given, not anything else the player carries.
        let mut after = state.clone();
        after.inventory.clear();
        let target = (EntityKind::Character, someone.id.as_str());
        let with = Some((EntityKind::Object, object.id.as_str()));
        let taken = self.replies(world, &after, "give", target, with, &Outcome("given".into()));
        if taken.is_empty() {
            // A `## When: #give #refused` section rewords this one (`say`).
            return ActionResult::say(self.say(world, state, "give_refused", Some(target), &[("person", &person_mid(&someone)), ("name", &object_name(&object))]));
        }
        state.inventory.retain(|id| id != &object.id);
        state.held.entry(someone.id.clone()).or_default().push(object.id.clone());
        let text = self.say_replies(world, state, &taken);
        ActionResult { text, changes: vec![Change::Gave { object: object.id.clone(), to: someone.id.clone() }], note: None, topics: Vec::new() }
    }

    /// `talk to <person>` (REQ-PC-31): the person's `## When: #talk`
    /// sections that match, run as replies (with their blocks' mechanics),
    /// and any topics they end with. With none, `talk_nothing`. A thing
    /// that isn't a person answers as its own action, `talk`.
    fn talk(&self, world: &WorldEngine, state: &mut PlayState, noun: &str) -> ActionResult {
        if noun.trim().is_empty() {
            return ActionResult::say(self.say(world, state, "talk_to_whom", None, &[]));
        }
        let Some(view) = self.view(world, state) else {
            return ActionResult::say(self.say(world, state, "nowhere", None, &[]));
        };
        let someone = match resolve(noun, &view.characters, person) {
            Ok(Some(p)) => p,
            Ok(None) => return self.use_object(world, state, "talk", noun, None),
            Err(which) => return ActionResult::say(self.which(world, state, "talk", noun, EntityKind::Character, &which)),
        };
        let target = (EntityKind::Character, someone.id.as_str());
        let now = Outcome(self.current_state(world, state, EntityKind::Character, &someone.id).unwrap_or_else(|| "no-action".into()));
        let replies = self.reply_sections(world, state, "talk", target, None, &now);
        if replies.is_empty() {
            return ActionResult::say(self.say(world, state, "talk_nothing", Some(target), &[("person", &person(&someone))]));
        }
        self.run_replies(world, state, "talk", &someone.id, replies)
    }

    /// `ask <person> about <topic>` (REQ-PC-32): a topic that names a
    /// thing matches `## When: #ask @thing`, and any topic matches
    /// `## When: #ask #<its slug>`. Sections are matched with nothing else
    /// in hand, so `@lantern` means the lantern is what's asked about.
    fn ask(&self, world: &WorldEngine, state: &mut PlayState, who: &str, topic: &str) -> ActionResult {
        if who.trim().is_empty() {
            return ActionResult::say(self.say(world, state, "ask_whom", None, &[]));
        }
        let Some(view) = self.view(world, state) else {
            return ActionResult::say(self.say(world, state, "nowhere", None, &[]));
        };
        let someone = match resolve(who, &view.characters, person) {
            Ok(Some(p)) => p,
            Ok(None) => return ActionResult::say(self.say(world, state, "not_here", None, &[("noun", who.trim())])),
            Err(which) => return ActionResult::say(self.which(world, state, "ask", who, EntityKind::Character, &which)),
        };
        let target = (EntityKind::Character, someone.id.as_str());
        let topic = topic.trim();
        if topic.is_empty() {
            return ActionResult::say(self.say(world, state, "ask_about_what", Some(target), &[("person", &person_mid(&someone))]));
        }
        // The thing a topic names: `@id` exactly, else a thing here or
        // carried by name, else any thing whose id is the topic's slug.
        let slug = crate::worlds::slug(topic.trim_start_matches(['@', '#']));
        let thing: Option<(EntityKind, String)> = if let Some(id) = topic.strip_prefix('@') {
            world.thing_by_id(id).ok().map(|t| (t.kind, t.id.clone()))
        } else if topic.starts_with('#') {
            None
        } else {
            let mut things = self.carried(world, state);
            things.extend(view.objects.iter().cloned());
            things.extend(doors_here(&view));
            let people: Vec<EntityRef> = view.characters.iter().cloned().collect();
            match resolve(topic, &things, the_object).ok().flatten() {
                Some(t) => world.thing_by_id(&t.id).ok().map(|t| (t.kind, t.id.clone())),
                None => match resolve(topic, &people, person).ok().flatten() {
                    Some(p) => Some((EntityKind::Character, p.id.clone())),
                    None => world.thing_by_id(&slug).ok().map(|t| (t.kind, t.id.clone())),
                },
            }
        };
        let mut alone = state.clone();
        alone.inventory.clear();
        let now = Outcome(self.current_state(world, state, EntityKind::Character, &someone.id).unwrap_or_else(|| "no-action".into()));
        let with = thing.as_ref().map(|(k, id)| (*k, id.as_str()));
        // The topic's slugs, with and without a leading `the` (`war` and
        // `the war` both ask `#the-war` and `#war`), and a thing's by its
        // id's last part.
        let mut extra = vec![format!("#{}", slug)];
        match ["the-", "a-", "an-"].iter().find_map(|a| slug.strip_prefix(a)) {
            Some(rest) => extra.push(format!("#{}", rest)),
            None => extra.push(format!("#the-{}", slug)),
        }
        if let Some((_, id)) = &thing {
            extra.push(format!("#{}", id.rsplit('/').next().unwrap_or(id)));
        }
        let replies = self.reply_sections_with(world, &alone, "ask", target, with, &now, &extra);
        if replies.is_empty() {
            return ActionResult::say(self.say(world, state, "ask_nothing", Some(target), &[("person", &person(&someone))]));
        }
        self.run_replies(world, state, "ask", &someone.id, replies)
    }

    /// Talk and ask replies: each section's block runs (`becomes:`,
    /// `sets:`, `chapter:`, with `this` its own thing), then its prose is
    /// the reply, with a closing list of `- Label -> ask <topic>` lines
    /// read as topics and shown as their labels.
    fn run_replies(&self, world: &WorldEngine, state: &mut PlayState, verb: &str, person_id: &str, replies: Vec<crate::engines::turn::Reply>) -> ActionResult {
        let mut changes = Vec::new();
        let mut notes = Vec::new();
        let mut shown = Vec::new();
        let mut topics = Vec::new();
        for reply in replies {
            if !reply.block.is_empty() {
                match crate::parsers::markdown_parser::parse_action(verb, &serde_yaml::Value::Mapping(reply.block.clone()), &reply.path) {
                    Ok(mut action) => {
                        if let Some(to) = action.becomes.remove("this") {
                            action.becomes.insert(reply.id.clone(), to);
                        }
                        self.apply_mechanics(world, state, (reply.kind, &reply.id), &action, &mut changes, &mut notes);
                    }
                    Err(e) => notes.push(format!("{:#}", e)),
                }
            }
            let mut lines = Vec::new();
            for line in reply.text.lines() {
                let item = line.trim_start().strip_prefix("- ");
                match item.and_then(|i| i.split_once("->")).map(|(l, a)| (l.trim(), a.trim())) {
                    Some((label, act)) if act.starts_with("ask ") => {
                        topics.push(crate::models::game::Topic { label: label.to_string(), person: person_id.to_string(), topic: act["ask ".len()..].trim().to_string() });
                        lines.push(format!("- {}", label));
                    }
                    _ => lines.push(line.to_string()),
                }
            }
            shown.push((reply.kind, reply.id, lines.join("\n")));
        }
        let text = self.say_replies(world, state, &shown);
        let mut result = ActionResult { text, changes, note: None, topics };
        if !notes.is_empty() {
            result.note = Some(notes.join("; "));
        }
        result
    }

    /// `where`: the room the player is in (REQ-PC-21).
    fn where_am_i(&self, world: &WorldEngine, state: &PlayState) -> ActionResult {
        match world.location_at(&state.location, At::play(state)) {
            Some(here) => ActionResult::say(self.say(world, state, "where", None, &[("room", &here.name.to_lowercase())])),
            None => ActionResult::say(self.say(world, state, "nowhere", None, &[])),
        }
    }

    /// `look north`: what's that way, without going (REQ-PC-23). A room
    /// is named by its title, a door by its name and state ("a closed
    /// door"); a hidden or missing way is no way.
    fn look_toward(&self, world: &WorldEngine, state: &PlayState, direction: &str) -> ActionResult {
        let direction = expand_direction(direction);
        let Some(here) = world.location_at(&state.location, At::play(state)) else {
            return ActionResult::say(self.say(world, state, "nowhere", None, &[]));
        };
        let none = || ActionResult::say(self.say(world, state, "look_toward_none", None, &[("direction", &direction)]));
        let Some(named) = here.connections.get(&direction) else { return none() };
        if world.get_location(named).is_some() {
            let Some(room) = world.location_at(named, At::play(state)).filter(|_| shown(world, state, EntityKind::Location, named)) else {
                return none();
            };
            return ActionResult::say(self.say(world, state, "look_toward", None, &[("direction", &direction), ("room", &room.name)]));
        }
        match world.door_at(named, At::play(state)) {
            Some(door) if shown(world, state, EntityKind::Door, &door.id) => {
                let name = match self.current_state(world, state, EntityKind::Door, &door.id).as_deref() {
                    Some(s @ ("open" | "closed" | "locked")) => format!("{} {}", s, door.title),
                    _ => door.title.clone(),
                };
                let door_name = crate::engines::play_text::with_article(&name);
                ActionResult::say(self.say(world, state, "look_toward_door", Some((EntityKind::Door, &door.id)), &[("direction", &direction), ("door", &door_name)]))
            }
            _ => none(),
        }
    }

    fn take(&self, world: &WorldEngine, state: &mut PlayState, noun: &str) -> ActionResult {
        if noun.trim().is_empty() {
            return ActionResult::say(self.say(world, state, "take_what", None, &[]));
        }
        let view = match self.view(world, state) {
            Some(v) => v,
            None => return ActionResult::say(self.say(world, state, "nowhere", None, &[])),
        };
        if let Some(carried) = resolve(noun, &self.carried(world, state), the_object).ok().flatten() {
            return ActionResult::say(self.say(world, state, "already_have", Some((EntityKind::Object, &carried.id)), &[("thing", &the_object(&carried))]));
        }
        // What's in the room, and what the things here hold (photos on a
        // wall, a key in its lock), each with its holder.
        let mut candidates: Vec<EntityRef> = view.objects.clone();
        let inside = self.inside_here(world, state, &view);
        candidates.extend(inside.iter().map(|(t, _)| t.clone()));
        let object = match resolve(noun, &candidates, the_object) {
            Ok(Some(o)) => o,
            Ok(None) => {
                if let Some(p) = resolve(noun, &view.characters, person).ok().flatten() {
                    return ActionResult::say(self.say(world, state, "take_person", Some((EntityKind::Character, &p.id)), &[("person", &person(&p))]));
                }
                return ActionResult::say(self.say(world, state, "not_here", None, &[("noun", noun.trim())]));
            }
            Err(which) => return ActionResult::say(self.which(world, state, "take", noun, EntityKind::Object, &which)),
        };
        let name = object_name(&object);
        let Some(card) = world.object_at(&object.id, At::play(state)) else {
            return ActionResult::say(self.say(world, state, "take_scenery", Some((EntityKind::Object, &object.id)), &[("name", &name)]))
                .with_note(format!("`{}` is scenery with no card, so it can't be taken", object.id));
        };
        if card.weight.as_deref().map_or(false, |w| w.eq_ignore_ascii_case("immovable")) {
            return ActionResult::say(self.say(world, state, "immovable", Some((EntityKind::Object, &object.id)), &[("name", &name)]));
        }
        let from = self.detach(world, state, &object.id);
        state.inventory.push(object.id.clone());
        ActionResult {
            text: self.take_reply(world, state, &object.id).unwrap_or_else(|| self.say(world, state, "taken", Some((EntityKind::Object, &object.id)), &[("name", &name)])),
            changes: vec![Change::Took { object: object.id.clone(), from }],
            note: None,
            topics: Vec::new(),
        }
    }

    fn examine(&self, world: &WorldEngine, state: &mut PlayState, noun: &str) -> ActionResult {
        if noun.trim().is_empty() {
            return ActionResult::say(self.say(world, state, "examine_what", None, &[]));
        }
        let view = match self.view(world, state) {
            Some(v) => v,
            None => return ActionResult::say(self.say(world, state, "nowhere", None, &[])),
        };
        // Examining the room itself looks closely, finding faint things.
        let wanted = noun.trim().to_lowercase();
        let wanted = wanted.strip_prefix("the ").unwrap_or(&wanted);
        if ["room", "here", "around"].contains(&wanted) || wanted == view.title.to_lowercase() || wanted == view.id.to_lowercase() {
            let look = crate::render::play_blocks(&view, &|n| world.link_for(n), world.play_text()).to_plain();
            let mut found = self.search(world, state, true);
            found.text = format!("{}\n\n{}", look, found.text);
            return found;
        }
        let mut things = self.carried(world, state);
        things.extend(view.objects.iter().cloned());
        things.extend(doors_here(&view));
        things.extend(self.inside_here(world, state, &view).into_iter().map(|(t, _)| t).collect::<Vec<_>>());
        match resolve(noun, &things, the_object) {
            Ok(Some(object)) if world.door_at(&object.id, At::play(state)).is_some() && world.object_at(&object.id, At::play(state)).is_none() => {
                let door = world.door_at(&object.id, At::play(state)).unwrap();
                let scope = Scope { at: At::play(state), this: Some((EntityKind::Door, &door.id)), here: None };
                let text = self.describe(world, state, EntityKind::Door, &door.id, &door.description);
                let text = self.with_contents(world, state, &door.id, &text);
                let text = text.trim();
                return if text.is_empty() {
                    ActionResult::say(self.say(world, state, "nothing_special", Some((EntityKind::Door, &door.id)), &[("thing", &the_object(&object))]))
                } else {
                    ActionResult::say(prose(world, &world.fill(text, &scope)))
                };
            }
            Ok(Some(object)) => {
                // The description, with what its state adds.
                let text = world
                    .object_at(&object.id, At::play(state))
                    .map(|card| self.describe(world, state, EntityKind::Object, &object.id, &card.description))
                    .map(|text| self.with_contents(world, state, &object.id, &text));
                return match text {
                    Some(text) if !text.trim().is_empty() => {
                        let scope = Scope { at: At::play(state), this: Some((EntityKind::Object, &object.id)), here: None };
                        ActionResult::say(prose(world, &world.fill(text.trim(), &scope)))
                    }
                    _ => ActionResult::say(self.say(world, state, "nothing_special", Some((EntityKind::Object, &object.id)), &[("thing", &the_object(&object))])),
                }
            }
            Err(which) => return ActionResult::say(self.which(world, state, "examine", noun, EntityKind::Object, &which)),
            Ok(None) => {}
        }
        match resolve(noun, &view.characters, person) {
            Ok(Some(person)) => match world.character_at(&person.id, At::play(state)).map(|card| self.describe(world, state, EntityKind::Character, &person.id, &card.description)) {
                Some(text) if !text.trim().is_empty() => {
                    let scope = Scope { at: At::play(state), this: Some((EntityKind::Character, &person.id)), here: None };
                    ActionResult::say(prose(world, &world.fill(text.trim(), &scope)))
                }
                _ => ActionResult::say(self.say(world, state, "nothing_special", Some((EntityKind::Character, &person.id)), &[("thing", &person.display_name())])),
            },
            Err(which) => ActionResult::say(self.which(world, state, "examine", noun, EntityKind::Character, &which)),
            Ok(None) => ActionResult::say(self.say(world, state, "not_here", None, &[("noun", noun.trim())])),
        }
    }

    /// Run an object's named action: check `requires`, then switch states
    /// (`becomes`), set values (`sets`) and move the story (`chapter`).
    /// A target that can't be found is skipped with an author note, which
    /// `validate` also reports.
    fn use_object(&self, world: &WorldEngine, state: &mut PlayState, verb: &str, noun: &str, with: Option<&str>) -> ActionResult {
        let verb = verb.trim().to_lowercase();
        if noun.trim().is_empty() {
            return ActionResult::say(self.say(world, state, "no_noun", None, &[("verb", &verb)]));
        }
        let view = match self.view(world, state) {
            Some(v) => v,
            None => return ActionResult::say(self.say(world, state, "nowhere", None, &[])),
        };
        let mut things = self.carried(world, state);
        things.extend(view.objects.iter().cloned());
        things.extend(doors_here(&view));
        things.extend(self.inside_here(world, state, &view).into_iter().map(|(t, _)| t).collect::<Vec<_>>());
        // A hidden object can't be seen, but an action may name it exactly.
        let hidden = || -> Option<EntityRef> {
            let whole = self.whole_view(world, state)?;
            let wanted = noun.trim().to_lowercase();
            whole.objects.into_iter().find(|o| {
                world.visibility(EntityKind::Object, &o.id, At::play(state)) == Visibility::Hidden
                    && (o.id.to_lowercase() == wanted || o.display_name().to_lowercase() == wanted)
            })
        };
        let mut named_hidden = false;
        let object = match resolve(noun, &things, the_object) {
            Ok(Some(o)) => o,
            Err(which) => return ActionResult::say(self.which(world, state, &verb, noun, EntityKind::Object, &which)),
            Ok(None) if hidden().is_some() => {
                named_hidden = true;
                hidden().unwrap()
            }
            Ok(None) => {
                return match resolve(noun, &view.characters, person) {
                    Ok(Some(p)) if matches!(verb.as_str(), "talk" | "speak" | "chat" | "ask") => {
                        ActionResult::say(self.say(world, state, "talk_nothing", Some((EntityKind::Character, &p.id)), &[("person", &person(&p))]))
                    }
                    Ok(Some(p)) => ActionResult::say(self.say(world, state, "cant_verb_person", Some((EntityKind::Character, &p.id)), &[("verb", &verb), ("person", &person_mid(&p))])),
                    Err(which) => ActionResult::say(self.which(world, state, &verb, noun, EntityKind::Character, &which)),
                    Ok(None) => ActionResult::say(self.say(world, state, "not_here", None, &[("noun", noun.trim())])),
                }
            }
        };
        // An object's actions, or a door's (`open door`, `unlock door`).
        let kind_of = |id: &str| if world.object_at(id, At::play(state)).is_none() && world.door_at(id, At::play(state)).is_some() { EntityKind::Door } else { EntityKind::Object };
        let this_kind = kind_of(&object.id);
        let actions = match this_kind {
            EntityKind::Door => world.door_at(&object.id, At::play(state)).map(|d| d.actions),
            _ => world.object_at(&object.id, At::play(state)).map(|o| o.actions),
        }
        .unwrap_or_default();
        // `push cart north`: the direction, not a thing used.
        let toward = with.filter(|w| matches!(verb.as_str(), "push" | "pull") && crate::engines::player_commands::is_direction(w)).map(str::to_string);
        let with = if toward.is_some() { None } else { with };
        // What it's used with (`hit window with stick`).
        let used = match with.map(str::trim).filter(|w| !w.is_empty()) {
            None => None,
            Some(w) => match resolve(w, &things, the_object) {
                Ok(Some(u)) => Some(u),
                Err(which) => return ActionResult::say(self.which(world, state, &verb, w, EntityKind::Object, &which)),
                Ok(None) => return ActionResult::say(self.say(world, state, "not_here", None, &[("noun", w)])),
            },
        };
        let used_kind = used.as_ref().map(|u| kind_of(&u.id));
        let used_ref = used.as_ref().zip(used_kind).map(|(u, k)| (k, u.id.clone()));
        let name = object_name(&object);
        let key = verb.replace(' ', "-");
        let mut changes = Vec::new();
        let mut notes = Vec::new();

        // What happens: the thing's script for this verb, else its
        // frontmatter action, else nothing (`design/SECTIONS.md` ›
        // Scripts). The outcome is the state the thing moved into, or why
        // it didn't move.
        // The thing's own script wins; a template's script yields to the
        // thing's own frontmatter action of the same name.
        let function = super::scripts::function_name(&verb);
        let frontmatter = actions.keys().any(|name| name.eq_ignore_ascii_case(&key));
        let script = world
            .thing(this_kind, &object.id)
            .filter(|t| t.own_script_actions.contains(&function) || (t.script_actions.contains(&function) && !frontmatter));
        // Whether an action ran: then, with no reply, the reply is built in.
        let mut acted = false;
        let outcome = if let Some(thing) = script {
            match self.run_script(world, state, thing, &verb, used_ref.as_ref(), &mut changes) {
                Ok(outcome) => outcome,
                Err(e) => {
                    notes.push(format!("{}'s `{}` script: {}", object.id, verb, e));
                    Outcome("no-action".into())
                }
            }
        } else if let Some(action) = actions.iter().find(|(name, _)| name.eq_ignore_ascii_case(&key)).map(|(_, a)| a.clone()) {
            // A hidden thing answers to its exact name only for an action
            // with no `requires:`; otherwise it is as good as absent
            // (Bruce, 2026-11-23).
            if named_hidden && !action.requires_has.is_empty() {
                return ActionResult::say(self.say(world, state, "not_here", None, &[("noun", noun.trim())]));
            }
            if action.requires_has.iter().any(|id| !state.inventory.contains(id)) {
                return ActionResult::say(self.say(world, state, "refused", Some((this_kind, &object.id)), &[("verb", &verb), ("name", &name)]));
            }
            acted = true;
            let before = self.current_state(world, state, this_kind, &object.id);
            self.apply_mechanics(world, state, (this_kind, &object.id), &action, &mut changes, &mut notes);
            let after = self.current_state(world, state, this_kind, &object.id);
            match after.filter(|a| Some(a) != before.as_ref()) {
                Some(moved) => Outcome(moved),
                None => Outcome("no-action".into()),
            }
        } else {
            Outcome("no-action".into())
        };

        // The replies match after the action, so they can show it; a
        // thing's own replace its template's. With none, a built-in reply
        // if something changed, else nothing happened.
        let replies = self.replies(world, state, &verb, (this_kind, &object.id), used_ref.as_ref().map(|(k, id)| (*k, id.as_str())), &outcome);
        // A story verb nothing answered has its built-in effect (REQ-PC-30).
        if replies.is_empty() && changes.is_empty() && !acted && crate::engines::verbs::STORY.contains(&verb.as_str()) {
            let mut result = self.story_effect(world, state, &verb, (this_kind, &object), used_ref.as_ref().map(|(k, id)| (*k, id.as_str())), toward.as_deref());
            if !notes.is_empty() {
                result.note = Some(notes.join("; "));
            }
            return result;
        }
        let text = if replies.is_empty() {
            if changes.is_empty() && !acted {
                self.say(world, state, "nothing", Some((this_kind, &object.id)), &[("verb", &verb), ("name", &name)])
            } else {
                self.say(world, state, "acted", Some((this_kind, &object.id)), &[("verb", &verb), ("name", &name)])
            }
        } else {
            self.say_replies(world, state, &replies)
        };
        let mut result = ActionResult { text, changes, note: None, topics: Vec::new() };
        if !notes.is_empty() {
            result.note = Some(notes.join("; "));
        }
        result
    }

    /// An action's mechanics, from frontmatter or a reply section's
    /// ` ```thwarter ` block: `becomes:`, `sets:` (`this.` is `owner`'s)
    /// and `chapter:` (seen from `owner`'s scope).
    fn apply_mechanics(&self, world: &WorldEngine, state: &mut PlayState, owner: (EntityKind, &str), action: &crate::models::world::ObjectAction, changes: &mut Vec<Change>, notes: &mut Vec<String>) {
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
                Some(rest) => format!("{}.{}", owner.1, rest),
                None => name.clone(),
            };
            state.values.insert(name.clone(), value.clone());
            changes.push(Change::Set { name, value: value.clone() });
        }
        if let Some(chapter) = &action.chapter {
            let scope = world.thing(owner.0, owner.1).map(|t| t.scope.clone()).unwrap_or_default();
            match state.enter_chapter(world, &scope, chapter) {
                Ok(()) => changes.push(Change::EnteredChapter { name: state.chapter.as_ref().map(|c| c.name().to_string()).unwrap_or_default() }),
                Err(e) => notes.push(e.to_string()),
            }
        }
    }

    /// A `## When: #take` reply, which replaces the built-in one where it
    /// exists (`design/STATES.md` › Built-in verbs).
    fn take_reply(&self, world: &WorldEngine, state: &PlayState, id: &str) -> Option<String> {
        let stays = Outcome(self.current_state(world, state, EntityKind::Object, id).unwrap_or_else(|| "no-action".into()));
        let replies = self.replies(world, state, "take", (EntityKind::Object, id), None, &stays);
        (!replies.is_empty()).then(|| self.say_replies(world, state, &replies))
    }

    /// Replies as the player reads them, each filled from its own thing.
    /// One-paragraph replies run together as one paragraph ("It swings
    /// open. Cold air rises from the dark below."); longer ones keep a
    /// blank line between them.
    /// The built-in reply `key`: a story's own wording for it first
    /// (`story_reply`, along the reply's chain from `about`), else
    /// `[engine]`'s, one of a list picked by turn.
    fn say(&self, world: &WorldEngine, state: &PlayState, key: &str, about: Option<(EntityKind, &str)>, args: &[(&str, &str)]) -> String {
        match self.story_reply(world, state, key, about, args) {
            Some(text) => prose(world, &text),
            None => world.texts().text_at("engine", key, args, state.turn),
        }
    }

    fn say_replies(&self, world: &WorldEngine, state: &PlayState, replies: &[(EntityKind, String, String)]) -> String {
        let said: Vec<String> = replies
            .iter()
            .map(|(kind, id, text)| prose(world, &world.fill(text, &Scope { at: At::play(state), this: Some((*kind, id)), here: None })))
            .filter(|t| !t.trim().is_empty())
            .collect();
        let short = said.iter().all(|t| !t.trim().contains('\n'));
        said.join(if short { " " } else { "\n\n" })
    }

    /// A thing as its script sees it: id, title, state, values, and what
    /// it holds.
    fn script_view(&self, world: &WorldEngine, state: &PlayState, kind: EntityKind, id: &str) -> rhai::Map {
        let current = self.current_state(world, state, kind, id);
        let title = world.current(kind, id, At::play(state)).map(|e| e.title().to_string()).unwrap_or_else(|| id.to_string());
        let mut vars: Vec<(String, String)> = Vec::new();
        if let Some(thing) = world.thing(kind, id) {
            let chapter = state.chapter.as_ref().and_then(|c| world.chapter_key(c));
            if let Ok((_, merged)) = thing.build(current.as_deref(), chapter.as_ref()) {
                if let Some(serde_yaml::Value::Mapping(map)) = merged.metadata.get("vars") {
                    for (k, v) in map {
                        if let Some(k) = k.as_str() {
                            vars.push((k.to_string(), crate::engines::world_def::value_text(v).unwrap_or_default()));
                        }
                    }
                }
            }
        }
        for (k, v) in vars.iter_mut() {
            if let Some(set) = state.values.get(&format!("{}.{}", id, k)) {
                *v = set.clone();
            }
        }
        let held = state
            .held
            .get(id)
            .into_iter()
            .flatten()
            .map(|h| self.script_view(world, state, EntityKind::Object, h))
            .collect();
        super::scripts::thing_map(id, &title, current.as_deref(), &vars, held)
    }

    /// Run a thing's script for `verb` and apply what it returns. The
    /// outcome is the state it moved into, else the reason it gave.
    fn run_script(
        &self,
        world: &WorldEngine,
        state: &mut PlayState,
        thing: &crate::engines::bundles::Thing,
        verb: &str,
        used: Option<&(EntityKind, String)>,
        changes: &mut Vec<Change>,
    ) -> std::result::Result<Outcome, String> {
        use super::scripts::{ScriptChange, ScriptResult};
        let ast = super::scripts::compile(&thing.scripts)?.ok_or("it has no script")?;
        let this = self.script_view(world, state, thing.kind, &thing.id);
        let mut turn = rhai::Map::new();
        turn.insert("verb".into(), verb.into());
        turn.insert("used".into(), used.map_or(rhai::Dynamic::UNIT, |(k, id)| rhai::Dynamic::from_map(self.script_view(world, state, *k, id))));
        let result = super::scripts::run(&ast, verb, this, turn)?;
        let list = match result {
            ScriptResult::Reason(why) => return Ok(Outcome(why)),
            ScriptResult::Changes(list) => list,
        };
        let mut outcome = Outcome("no-action".into());
        for change in list {
            match change {
                ScriptChange::Become(to) => {
                    state.switch_state(world, thing.kind, &thing.id, &to).map_err(|e| e.to_string())?;
                    changes.push(Change::Became { type_name: thing.kind.type_name().to_string(), id: thing.id.clone(), state: to.clone() });
                    outcome = Outcome(to);
                }
                ScriptChange::Keep(id) => {
                    // Only what it was used with can be kept.
                    if used.map(|(_, u)| u.as_str()) != Some(id.as_str()) {
                        return Err(format!("keep(`{}`): a thing keeps only what it's used with", id));
                    }
                    self.detach(world, state, &id);
                    state.held.entry(thing.id.clone()).or_default().push(id.clone());
                    changes.push(Change::Kept { object: id, by: thing.id.clone() });
                }
                ScriptChange::Change { id, state: to } => {
                    if !connected(world, thing, &id) {
                        return Err(format!("change(`{}`): it isn't connected; add `connects: [{}]`", id, id));
                    }
                    let other = world.thing_by_id(&id).map_err(|e| e.to_string())?;
                    let kind = other.kind;
                    state.switch_state(world, kind, &id, &to).map_err(|e| e.to_string())?;
                    changes.push(Change::Became { type_name: kind.type_name().to_string(), id, state: to });
                }
            }
        }
        Ok(outcome)
    }

    fn inventory(&self, world: &WorldEngine, state: &PlayState) -> ActionResult {
        let text = world.play_text();
        let carried = self.carried(world, state);
        if carried.is_empty() {
            return ActionResult::say(text.empty_inventory.clone());
        }
        let names: Vec<String> = carried.iter().map(|o| crate::engines::play_text::with_article(&o.display_name())).collect();
        ActionResult::say(text.fill(&text.inventory, &names))
    }

    /// Whether `noun` names anything the player can see or carries,
    /// including the room itself (REQ-PC-10, REQ-PC-14). A noun several
    /// things answer to names something.
    pub(crate) fn names_anything(&self, world: &WorldEngine, state: &PlayState, noun: &str) -> bool {
        let Some(view) = self.view(world, state) else { return false };
        let wanted = noun.trim().to_lowercase();
        if wanted == view.title.to_lowercase() || wanted == view.id.to_lowercase() {
            return true;
        }
        let mut things = self.carried(world, state);
        things.extend(view.objects.iter().cloned());
        things.extend(doors_here(&view));
        things.extend(view.characters.iter().cloned());
        things.extend(self.inside_here(world, state, &view).into_iter().map(|(t, _)| t).collect::<Vec<_>>());
        !matches!(resolve(noun, &things, the_object), Ok(None))
    }

    /// The verbs the things in scope answer to (REQ-ENG-52, 54): what the
    /// player carries, the room's objects, doors and people, and what
    /// they hold. A thing's verbs are its actions, its script's functions
    /// and the `#verb` tags in its section headings, less its own states,
    /// the reserved tags and the chapters. Sorted, each once.
    pub fn verbs_in_scope(&self, world: &WorldEngine, state: &PlayState) -> Vec<String> {
        let Some(view) = self.view(world, state) else { return Vec::new() };
        let mut things: Vec<(EntityKind, String)> = Vec::new();
        for o in self.carried(world, state).into_iter().chain(view.objects.iter().cloned()).chain(self.inside_here(world, state, &view).into_iter().map(|(t, _)| t)) {
            things.push((EntityKind::Object, o.id));
        }
        things.extend(doors_here(&view).into_iter().map(|d| (EntityKind::Door, d.id)));
        things.extend(view.characters.iter().map(|c| (EntityKind::Character, c.id.clone())));
        let chapters: Vec<String> = world.chapters().iter().map(|c| format!("#{}", c.name())).collect();
        let mut verbs: Vec<String> = Vec::new();
        for (kind, id) in things {
            let actions = match kind {
                EntityKind::Object => world.object_at(&id, At::play(state)).map(|o| o.actions),
                EntityKind::Door => world.door_at(&id, At::play(state)).map(|d| d.actions),
                _ => None,
            };
            verbs.extend(actions.into_iter().flat_map(|a| a.into_keys()).map(|k| k.to_lowercase()));
            let Some(thing) = world.thing(kind, &id) else { continue };
            verbs.extend(thing.script_actions.iter().cloned());
            let states: Vec<String> = thing.state_names().into_iter().map(|s| format!("#{}", s)).collect();
            let layers = thing.templates.iter().chain(std::iter::once(&thing.index)).chain(thing.states.values());
            for section in layers.flat_map(|l| l.sections.iter()).filter(|s| !s.unless) {
                for tag in section.tags.iter().filter(|t| t.starts_with('#')) {
                    let tag = tag.to_lowercase();
                    if !states.contains(&tag) && !chapters.contains(&tag) && !crate::engines::layers::RESERVED_TAGS.contains(&tag.as_str()) {
                        verbs.push(tag[1..].to_string());
                    }
                }
            }
        }
        verbs.retain(|v| !v.is_empty() && !crate::engines::verbs::BUILT_IN.contains(&v.as_str()));
        verbs.sort();
        verbs.dedup();
        verbs
    }

    /// Every verb the player could type here (REQ-ENG-52): the built-in
    /// verbs, the aliases, and the verbs the things in scope answer to,
    /// sorted. For a client's completion or a verb list.
    pub fn verbs(&self, world: &WorldEngine, state: &PlayState) -> Vec<String> {
        let mut out: Vec<String> = crate::engines::verbs::BUILT_IN.iter().map(|v| v.to_string()).collect();
        out.extend(world.verbs().aliases().map(|(a, _)| a.to_string()));
        out.extend(self.verbs_in_scope(world, state));
        out.sort();
        out.dedup();
        out
    }

    /// What an object holds now (REQ-CF-31): what its file lists, less
    /// what's been taken from it or kept elsewhere, plus what it has kept
    /// in play (`keep()`), each as play shows it.
    pub(crate) fn contents(&self, world: &WorldEngine, state: &PlayState, holder: &str) -> Vec<String> {
        let kept_elsewhere = |id: &str| state.held.iter().any(|(h, l)| h != holder && l.iter().any(|x| x == id));
        let mut out: Vec<String> = world
            .object_at(holder, At::play(state))
            .map(|o| o.objects)
            .unwrap_or_default()
            .into_iter()
            .filter(|id| !state.was_taken(holder, id) && !state.inventory.contains(id) && !kept_elsewhere(id))
            .collect();
        for id in state.held.get(holder).into_iter().flatten() {
            if !out.contains(id) {
                out.push(id.clone());
            }
        }
        out.retain(|id| shown(world, state, EntityKind::Object, id));
        out
    }

    /// `text`, then what `holder` holds. How a holder's contents read is
    /// still Bruce's to choose (titles, grouping, or prose alone), so it's
    /// made here and nowhere else (REQ-CF-31).
    fn with_contents(&self, world: &WorldEngine, state: &PlayState, holder: &str, text: &str) -> String {
        let names: Vec<String> = self
            .contents(world, state, holder)
            .iter()
            .map(|id| crate::engines::play_text::with_article(&world.object_at(id, At::play(state)).map(|o| o.title).unwrap_or_else(|| crate::engines::title_from_id(id))))
            .collect();
        if names.is_empty() {
            return text.to_string();
        }
        // "There's an old photo on the walls.", as the room says it.
        let holder_name = world.current(EntityKind::Object, holder, At::play(state)).or_else(|| world.current(EntityKind::Door, holder, At::play(state))).map(|e| e.title().to_string()).unwrap_or_else(|| crate::engines::title_from_id(holder));
        let text_ = world.play_text();
        let phrase = format!("{} {} the {}", text_.list(&names), world.holds_word(holder, At::play(state)), holder_name.to_lowercase());
        let line = text_.fill(&text_.held, &[phrase]);
        match text.trim().is_empty() {
            true => line,
            false => format!("{}\n{}", text.trim(), line),
        }
    }

    /// Everything the things here hold, however deep, each with its
    /// holder: what the room's objects, its doors and the player's things
    /// hold, and what those hold. A cycle is followed once.
    fn inside_here(&self, world: &WorldEngine, state: &PlayState, view: &LocationView) -> Vec<(EntityRef, String)> {
        let mut todo: Vec<String> = state.inventory.clone();
        todo.extend(view.objects.iter().map(|o| o.id.clone()));
        todo.extend(doors_here(view).into_iter().map(|d| d.id));
        let mut seen: std::collections::BTreeSet<String> = todo.iter().cloned().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < todo.len() {
            let holder = todo[i].clone();
            i += 1;
            for id in self.contents(world, state, &holder) {
                if seen.insert(id.clone()) {
                    out.push((EntityRef { name: world.object_at(&id, At::play(state)).map(|o| o.title), state: world.state_at(EntityKind::Object, &id, At::play(state)), id: id.clone() }, holder.clone()));
                    todo.push(id);
                }
            }
        }
        out
    }

    /// Take a thing out of wherever it is now, ready to move: the
    /// player's hands, a thing that kept it, an object that holds it, or
    /// the room. Returns where it was.
    fn detach(&self, world: &WorldEngine, state: &mut PlayState, id: &str) -> String {
        if let Some(i) = state.inventory.iter().position(|c| c == id) {
            state.inventory.remove(i);
            return "player".to_string();
        }
        if let Some(holder) = state.held.iter().find(|(_, l)| l.iter().any(|x| x == id)).map(|(h, _)| h.clone()) {
            if let Some(list) = state.held.get_mut(&holder) {
                list.retain(|x| x != id);
            }
            state.held.retain(|_, l| !l.is_empty());
            return holder;
        }
        let holder = self
            .view(world, state)
            .and_then(|view| self.inside_here(world, state, &view).into_iter().find(|(t, _)| t.id == id).map(|(_, h)| h))
            .unwrap_or_else(|| state.location.clone());
        let taken = state.taken.entry(holder.clone()).or_default();
        if !taken.iter().any(|x| x == id) {
            taken.push(id.to_string());
        }
        holder
    }

    fn carried(&self, world: &WorldEngine, state: &PlayState) -> Vec<EntityRef> {
        state
            .inventory
            .iter()
            .map(|id| EntityRef { id: id.clone(), name: world.object_at(id, At::play(state)).map(|o| o.title), state: world.state_at(EntityKind::Object, id, At::play(state)) })
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
        // What things held, if the holder or the held thing is gone.
        for (holder, list) in self.held.iter_mut() {
            let holder_gone = world.thing_by_id(holder).is_err();
            list.retain(|id| {
                let keep = !holder_gone && world.objects.contains_key(id);
                if !keep {
                    warnings.push(format!("`{}` was held by `{}`, but one of them no longer exists, so it's gone", id, holder));
                }
                keep
            });
        }
        self.held.retain(|_, l| !l.is_empty());
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
                if thing.has_state(state.as_str()) {
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
        if !thing.has_state(state) {
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
fn resolve(noun: &str, things: &[EntityRef], _phrase: fn(&EntityRef) -> String) -> std::result::Result<Option<EntityRef>, Vec<EntityRef>> {
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
        _ => Err(unique.into_iter().cloned().collect()),
    }
}

/// The world's `[engine]` reply `key` for `turn`, with `args` filled
/// (`design/ENGINE_TEXT.md`).
/// A key with several wordings picks one by turn, never at random.


/// Prose as play shows it: Markdown rendered, links shown as their text.
/// Prose for the player: author comments never show (`design/SECTIONS.md`
/// › Bruce's principles).
fn prose(world: &WorldEngine, text: &str) -> String {
    prose_plain(crate::engines::layers::strip_comments(text).trim(), &|n| world.link_for(n))
}


/// Whether a thing's script may change `other`: either one names the
/// other in frontmatter `connects:` (`design/STATES.md` › One thing
/// affecting another).
fn connected(world: &WorldEngine, thing: &crate::engines::bundles::Thing, other: &str) -> bool {
    let names = |t: &crate::engines::bundles::Thing, id: &str| {
        t.index.metadata.get("connects").and_then(|v| v.as_sequence()).map_or(false, |list| list.iter().any(|v| v.as_str() == Some(id)))
    };
    names(thing, other) || world.thing_by_id(other).map_or(false, |o| names(o, &thing.id))
}

/// The doors on the room's exits, as things the player can name.
fn doors_here(view: &LocationView) -> Vec<EntityRef> {
    let mut doors: Vec<EntityRef> = Vec::new();
    for door in view.exits.iter().filter_map(|e| e.door.as_ref()) {
        if !doors.iter().any(|d| d.id == door.id) {
            doors.push(EntityRef { id: door.id.clone(), name: Some(door.title.clone()), state: door.state.clone() });
        }
    }
    doors
}

/// Whether play shows a thing: visible, or faint and already found.
fn shown(world: &WorldEngine, state: &PlayState, kind: EntityKind, id: &str) -> bool {
    match world.visibility(kind, id, At::play(state)) {
        Visibility::Visible => true,
        Visibility::Faint => state.has_noticed(kind.type_name(), id),
        Visibility::Hidden | Visibility::Absent => false,
    }
}

fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {}", init.join(", "), last),
    }
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

/// A person's name mid-sentence: "the cook" for "The cook", but "King
/// Aldric" as written.
fn person_mid(character: &EntityRef) -> String {
    let name = character.display_name();
    match name.strip_prefix("The ") {
        Some(rest) => format!("the {}", rest),
        None => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::engines::Parsed;

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
        // No room name by default; who's here follows the prose, then
        // after a blank line what's here, then the ways out (world.toml
        // [play]). The bench is scenery, so the prose has it.
        assert_eq!(r.text, "Grass.\nKing Aldric is here.\n\nYou see a rusty sword, an old sword, and a well.\n\nYou can go north or west.");
        assert!(r.changes.is_empty());
    }

    #[test]
    fn go_moves_through_real_exits_only() {
        let (_d, world) = world();
        let (game, mut state) = play(&world);
        let r = game.act(&world, &mut state, &PlayAction::Go("n".into()));
        assert_eq!(state.location, "castle/hall");
        assert_eq!(r.changes, vec![Change::Moved { from: "castle/yard".into(), to: "castle/hall".into() }]);
        assert!(r.text.starts_with("Stone."), "{}", r.text);

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
        assert_eq!(game.act(&world, &mut state, &PlayAction::Inventory).text, "You have a rusty sword.");
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
            ("sword", "Which sword do you want to take, the rusty sword or the old sword?"),
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
            "Who do you want to examine, King Aldric or Queen Aldric?"
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
        assert!(look.contains("You see a well.") && !look.contains("lamp"), "{}", look);

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
            "---\nid: lever\ntitle: Iron lever\ntype: object\nobject_type: mechanism\nweight: immovable\nactions:\n  pull:\n    becomes: { castle/vault: open }\n    sets: { story.vault: open, this.pulled: yes }\n    chapter: after\n    requires: { has: lamp }\n  shove:\n    requires: { has: lamp }\n  kick:\n---\nA lever.\n",
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
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        let pull = PlayAction::Use { verb: "pull".into(), noun: "lever".into(), with: None };
        let shove = PlayAction::Use { verb: "shove".into(), noun: "lever".into(), with: None };

        game.act(&world, &mut state, &PlayAction::Go("north".into()));
        // Replies are built in until reply sections are (design/SECTIONS.md).
        assert_eq!(game.act(&world, &mut state, &pull).text, "You can't pull the iron lever yet.");
        assert_eq!(game.act(&world, &mut state, &shove).text, "Not yet: you can't shove the iron lever.");
        assert!(state.states.is_empty());

        game.act(&world, &mut state, &PlayAction::Go("south".into()));
        game.act(&world, &mut state, &PlayAction::Take("lamp".into()));
        game.act(&world, &mut state, &PlayAction::Go("north".into()));
        assert_eq!(game.act(&world, &mut state, &shove).text, "You shove the iron lever. Something changes.");
        let r = game.act(&world, &mut state, &pull);
        assert_eq!(r.text, "You pull the iron lever.");
        assert!(r.note.is_none(), "{:?}", r.note);
        assert_eq!(state.state_of("location", "castle/vault"), Some("open"));
        assert_eq!(state.values["story.vault"], "open");
        assert_eq!(state.values["lever.pulled"], "yes");
        assert_eq!(state.chapter.as_ref().unwrap().name(), "after");
        assert!(r.changes.contains(&Change::EnteredChapter { name: "after".into() }));

        let kick = |noun: &str| PlayAction::Use { verb: "kick".into(), noun: noun.into(), with: None };
        assert_eq!(game.act(&world, &mut state, &kick("iron lever")).text, "You kick the iron lever. Something changes.");
        let lick = PlayAction::Use { verb: "lick".into(), noun: "lever".into(), with: None };
        assert_eq!(game.act(&world, &mut state, &lick).text, "Nothing happens.");
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
        game.ask_which = Some(|verb, _noun, c, _turn| format!("Which one do you want to {}? {}", verb, c.join(" or ")));
        let mut state = game.start(&world, "castle/yard").unwrap();
        let r = game.act(&world, &mut state, &PlayAction::Take("sword".into()));
        assert_eq!(r.text, "Which one do you want to take? the rusty sword or the old sword");
    }

    #[test]
    fn faint_things_are_found_hidden_ones_revealed_and_absent_ones_unnameable() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let w = |rel: &str, text: &str| {
            let p = d.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        };
        w("cellar.md", "---\nid: cellar\ntitle: Cellar\ntype: location\nconnections: { north: passage }\nobjects: [key, lever, ghost, crate]\n---\nDamp.\n");
        w("passage.md", "---\nid: passage\ntitle: Passage\ntype: location\nvisibility: faint\n---\n");
        w("objects/key.md", "---\nid: key\ntitle: Brass key\ntype: object\nobject_type: key\nvisibility: faint\n---\n");
        w("objects/crate.md", "---\nid: crate\ntitle: Crate\ntype: object\nobject_type: box\n---\n");
        w("objects/lever/index.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nstate: secret\nactions:\n  pull: { becomes: { ghost: here } }\n  turn: { requires: { has: key } }\n---\n");
        w("objects/lever/secret.md", "---\ninherits: index\nvisibility: hidden\n---\n");
        w("objects/ghost/index.md", "---\nid: ghost\ntitle: Ghost\ntype: object\nobject_type: spirit\nstate: gone\n---\nPale.\n");
        w("objects/ghost/gone.md", "---\ninherits: index\nvisibility: absent\n---\n");
        w("objects/ghost/here.md", "---\ninherits: index\n---\n");
        let mut world = WorldEngine::new();
        world.load_world(d).unwrap();
        assert!(world.current_problems(false).is_empty(), "{:?}", world.current_problems(false));
        let game = GameEngine::new();
        let mut state = game.start(&world, "cellar").unwrap();
        let act = |state: &mut PlayState, a: PlayAction| game.act(&world, state, &a).text;

        let look = act(&mut state, PlayAction::Look);
        assert!(look.contains("You see a crate.") && look.contains("There's no way out."), "{}", look);
        assert_eq!(act(&mut state, PlayAction::Take("key".into())), "There's no key here.");
        assert_eq!(act(&mut state, PlayAction::Go("north".into())), "You can't go north from here.");

        assert_eq!(act(&mut state, PlayAction::Search), "You search carefully and find the brass key and a way north.");
        assert_eq!(act(&mut state, PlayAction::Search), "You search carefully, but find nothing new.");
        let look = act(&mut state, PlayAction::Look);
        assert!(look.contains("You see a brass key and a crate.") && look.contains("You can go north."), "{}", look);
        assert_eq!(act(&mut state, PlayAction::Take("key".into())), "You take the brass key.");

        // Hidden: never listed or found, but an action can name it exactly.
        assert_eq!(act(&mut state, PlayAction::Examine("lever".into())), "There's no lever here.");
        assert_eq!(act(&mut state, PlayAction::Examine("ghost".into())), "There's no ghost here.");
        let pull = |noun: &str| PlayAction::Use { verb: "pull".into(), noun: noun.into(), with: None };
        assert_eq!(act(&mut state, pull("lev")), "There's no lev here.");
        // An action with `requires:` can't reach a hidden thing, even with the key in hand.
        assert_eq!(act(&mut state, PlayAction::Use { verb: "turn".into(), noun: "lever".into(), with: None }), "There's no lever here.");
        assert!(act(&mut state, pull("lever")).starts_with("You pull the lever."));
        assert_eq!(act(&mut state, PlayAction::Examine("ghost".into())), "Pale.");
    }

    #[test]
    fn examining_the_room_looks_closely() {
        let (dir, mut world) = world();
        world.set_file(&dir.path().join("objects/old-sword.md"), "---\nid: old-sword\ntitle: Old sword\ntype: object\nobject_type: weapon\nvisibility: faint\n---\n");
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        let r = game.act(&world, &mut state, &PlayAction::Examine("room".into()));
        assert!(r.text.starts_with("Grass."), "{}", r.text);
        assert!(r.text.ends_with("You look closely and find the old sword."), "{}", r.text);
        assert!(state.has_noticed("object", "old-sword"));
    }

    #[test]
    fn a_bad_visibility_is_a_problem() {
        let (dir, mut world) = world();
        world.set_file(&dir.path().join("objects/well.md"), "---\nid: well\ntitle: Well\ntype: object\nobject_type: x\nvisibility: invisible\n---\n");
        assert!(world.current_problems(false).iter().any(|p| p.contains("`visibility: invisible` must be one of visible, faint, hidden, absent")));
    }

    #[test]
    fn a_shared_template_can_say_its_own_ambiguity_reply() {
        let (dir, mut world) = world();
        let d = dir.path();
        world.set_file(&d.join("objects/templates/sword.md"), "---\ntype: object\nobject_type: weapon\n---\n## When: #ambiguous\nTwo blades glint. You'll have to say which.\n");
        world.set_file(&d.join("objects/rusty-sword.md"), "---\nid: rusty-sword\ntitle: Rusty sword\ntype: object\ninherits: sword\n---\n");
        world.set_file(&d.join("objects/old-sword.md"), "---\nid: old-sword\ntitle: Old sword\ntype: object\ninherits: sword\n---\n");
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        assert_eq!(game.act(&world, &mut state, &PlayAction::Take("sword".into())).text, "Two blades glint. You'll have to say which.");
        // Without a shared template, the built-in question.
        world.set_file(&d.join("objects/old-sword.md"), "---\nid: old-sword\ntitle: Old sword\ntype: object\nobject_type: weapon\n---\n");
        assert_eq!(game.act(&world, &mut state, &PlayAction::Take("sword".into())).text, "Which sword do you want to take, the rusty sword or the old sword?");
    }

    #[test]
    fn prose_renders_markdown_links_and_tags_in_play() {
        let (dir, mut world) = world();
        let d = dir.path();
        world.set_file(&d.join("castle/hall.md"), "---\nid: castle/hall\ntitle: Hall\ntype: location\ntags: [indoors]\nconnections:\n  south: castle/yard\n---\nA **long** hall. The [[king]] sits by [the well](objects/well.md). #royal\n\nA door to [[castle/nowhere]].\n");
        world.set_file(&d.join("objects/well.md"), "---\nid: well\ntitle: Well\ntype: object\nobject_type: environmental\nweight: immovable\n---\nDeep, and *very* old.\n");
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        let look = game.act(&world, &mut state, &PlayAction::Look).text;
        assert!(look.contains("A long hall. The King Aldric sits by the well. royal\n\nA door to castle/nowhere."), "{}", look);
        let warnings = world.current_warnings();
        // A link to nothing is not written yet, not a warning (REQ-ENG-55).
        assert!(!warnings.iter().any(|w| w.contains("castle/nowhere")), "{:?}", warnings);
        let unwritten = world.not_written();
        let nowhere = unwritten.iter().find(|u| u.name == "castle/nowhere").expect("castle/nowhere is not written");
        assert_eq!(nowhere.mentions.len(), 1, "{:?}", unwritten);
        assert_eq!(nowhere.mentions[0].line, Some(11));
        assert!(!warnings.iter().any(|w| w.contains("`king`") || w.contains("objects/well")), "{:?}", warnings);
        let tags = world.tags();
        assert_eq!(tags["royal"], vec![(EntityKind::Location, "castle/hall".to_string())]);
        assert_eq!(tags["indoors"], vec![(EntityKind::Location, "castle/hall".to_string())]);

        game.act(&world, &mut state, &PlayAction::Go("south".into()));
        assert_eq!(game.act(&world, &mut state, &PlayAction::Examine("well".into())).text, "Deep, and very old.");
    }

    fn door_world() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let w = |rel: &str, text: &str| {
            let p = d.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        };
        w("castle/hall.md", "---\nid: castle/hall\ntitle: Hall\ntype: location\nconnections: { down: vault-door, east: grate }\nobjects: [lever]\n---\nStone.\n");
        w("castle/vault.md", "---\nid: castle/vault\ntitle: Vault\ntype: location\nconnections: { up: vault-door }\n---\nGold.\n");
        w("castle/cell.md", "---\nid: castle/cell\ntitle: Cell\ntype: location\nconnections: { west: grate }\n---\nDamp.\n");
        // A door from the built-in: closed (not passable) until opened. Its
        // own closed state adds prose over the template's.
        w("castle/doors/vault-door/index.md", "---\nid: vault-door\ntitle: Iron door\ntype: door\ninherits: door\nsides:\n  castle/hall: { direction: down }\n  castle/vault: { direction: up }\n---\n");
        w("castle/doors/vault-door/closed.md", "---\ninherits: index\n---\nAn iron door is set in the floor, shut.\n");
        w("castle/doors/grate.md", "---\nid: grate\ntitle: Iron grate\ntype: door\npassable: no\ntransparent: yes\nsides:\n  castle/hall: { direction: east }\n  castle/cell: { direction: west }\n---\n");
        w("objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nweight: immovable\nactions:\n  pull: { becomes: { vault-door: open } }\n---\n");
        let mut world = WorldEngine::new();
        world.load_world(d).unwrap();
        (dir, world)
    }

    #[test]
    fn doors_join_rooms_and_open_with_a_state_from_their_template() {
        let (_d, world) = door_world();
        assert!(world.current_problems(true).is_empty(), "{:?}", world.current_problems(true));
        let thing = world.thing(EntityKind::Door, "vault-door").unwrap();
        assert_eq!(thing.starting_state.as_deref(), Some("closed")); // from the built-in
        assert_eq!(thing.state_names(), ["closed", "open"]);
        assert!(!world.doors["vault-door"].passable);
        assert_eq!(world.doors["vault-door"].description, "An iron door is set in the floor, shut.");

        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        let look = game.act(&world, &mut state, &PlayAction::Look).text;
        assert!(look.contains("You can go east through an iron grate or down through an iron door.\nThrough the iron grate you can see the Cell."), "{}", look);
        assert!(!look.contains("shut"), "a door's own text shows when it's examined: {}", look);
        assert_eq!(game.act(&world, &mut state, &PlayAction::Go("down".into())).text, "The iron door is closed.");
        assert_eq!(game.act(&world, &mut state, &PlayAction::Go("east".into())).text, "The iron grate is closed.");
        assert_eq!(game.act(&world, &mut state, &PlayAction::Examine("iron door".into())).text, "An iron door is set in the floor, shut.");

        assert!(game.act(&world, &mut state, &PlayAction::Use { verb: "pull".into(), noun: "lever".into(), with: None }).text.starts_with("You pull the lever."));
        assert_eq!(state.state_of("door", "vault-door"), Some("open"));
        let look = game.act(&world, &mut state, &PlayAction::Look).text;
        assert!(!look.contains("shut"), "{}", look); // the open state has no prose, and the index none either
        let r = game.act(&world, &mut state, &PlayAction::Go("down".into()));
        assert!(r.text.starts_with("Gold."), "{}", r.text);
        game.act(&world, &mut state, &PlayAction::Go("up".into()));
        assert_eq!(state.location, "castle/hall");
    }

    #[test]
    fn story_verbs_have_replies_and_effects_when_nothing_answers() {
        let (dir, mut world) = world();
        let w = |rel: &str, text: &str| {
            let p = dir.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, text).unwrap();
        };
        w("castle/yard.md", "---\nid: castle/yard\ntitle: Yard\ntype: location\nconnections:\n  north: castle/hall\n  west: castle/gone\ncharacters: [king]\nobjects: [rusty-sword, old-sword, stone-bench, well, vase, cart, table]\n---\nGrass.\n");
        w("objects/vase/index.md", "---\nid: vase\ntitle: Vase\ntype: object\nobject_type: decor\nstate: whole\n---\nBlue.\n");
        w("objects/vase/whole.md", "---\ninherits: index\n---\n");
        w("objects/vase/broken.md", "---\ninherits: index\n---\nShards.\n");
        w("objects/cart.md", "---\nid: cart\ntitle: Cart\ntype: object\nobject_type: vehicle\nweight: heavy\npushable: true\n---\n");
        w("objects/table.md", "---\nid: table\ntitle: Table\ntype: object\nobject_type: furniture\nweight: immovable\nholds: on\n---\n");
        world.load_world(dir.path()).unwrap();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        let play = |state: &mut PlayState, line: &str| match game.parse_command(&world, line, state) {
            Parsed::Action(a) => game.act(&world, state, &a).text,
            Parsed::Reply(r) | Parsed::Unknown(r) => r,
        };
        assert_eq!(play(&mut state, "jump"), "You jump on the spot.");
        assert_eq!(play(&mut state, "run"), "You run on the spot for a moment.");
        assert_eq!(play(&mut state, "kick well"), "Kicking the well does nothing.");
        assert_eq!(play(&mut state, "hit king"), "You can't hit King Aldric.");
        assert_eq!(play(&mut state, "kick the dragon"), "There's no dragon here.");
        // throw: carried, then down here.
        assert_eq!(play(&mut state, "throw rusty sword"), "You aren't carrying any rusty sword.");
        play(&mut state, "take rusty sword");
        assert_eq!(play(&mut state, "throw rusty sword"), "You throw the rusty sword. It lands on the floor.");
        assert!(state.inventory.is_empty());
        // smash and break: to `broken` if it has one.
        assert_eq!(play(&mut state, "break well"), "The well won't break.");
        assert_eq!(play(&mut state, "smash vase"), "The vase breaks.");
        assert_eq!(state.state_of("object", "vase"), Some("broken"));
        assert_eq!(play(&mut state, "break vase"), "The vase is already broken.");
        // stack: onto a thing that holds things.
        play(&mut state, "take old sword");
        assert_eq!(play(&mut state, "stack old sword"), "Stack the old sword on what?");
        assert_eq!(play(&mut state, "stack old sword on well"), "You can't stack anything on the well.");
        assert_eq!(play(&mut state, "stack the old sword on the table"), "You put the old sword on the table.");
        assert_eq!(state.held["table"], ["old-sword"]);
        // push: a pushable thing goes through the exit, and the player with it.
        assert_eq!(play(&mut state, "push well north"), "The well won't move.");
        assert_eq!(play(&mut state, "push cart"), "The cart won't move.");
        assert_eq!(play(&mut state, "push cart west"), "You can't go west from here.");
        let pushed = play(&mut state, "push the cart north");
        assert!(pushed.starts_with("You push the cart north.\n\nStone."), "{}", pushed);
        assert!(pushed.contains("You see a cart."), "{}", pushed);
        assert_eq!(state.location, "castle/hall");
        assert!(play(&mut state, "pull cart south").starts_with("You pull the cart south."));
        assert!(state.held.get("castle/hall").is_none() && !state.was_taken("castle/yard", "cart"), "{:?}", state);
    }

    #[test]
    fn take_all_wait_again_and_it() {
        let (_dir, world) = world();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        let play = |state: &mut PlayState, line: &str| match game.parse_command(&world, line, state) {
            Parsed::Action(a) => game.act(&world, state, &a).text,
            Parsed::Reply(r) | Parsed::Unknown(r) => r,
        };
        assert_eq!(play(&mut state, "again"), "There's nothing to repeat.");
        assert_eq!(play(&mut state, "take it"), "I'm not sure what \"it\" means.");
        // The swords are loose; the bench is scenery and the well won't budge.
        assert_eq!(play(&mut state, "take all"), "You take the rusty sword.\nYou take the old sword.");
        assert_eq!(state.inventory, ["rusty-sword", "old-sword"]);
        assert_eq!(play(&mut state, "take all"), "There's nothing here to take.");
        let turn = state.turn;
        assert_eq!(play(&mut state, "z"), "Time passes.");
        assert_eq!(state.turn, turn + 1);
        assert_eq!(play(&mut state, "g"), "Time passes.");
        assert_eq!(state.turn, turn + 2);
        // `it` is the last thing named.
        play(&mut state, "drop rusty sword");
        assert_eq!(play(&mut state, "take it"), "You take the rusty sword.");
        assert_eq!(play(&mut state, "examine it"), "Pitted with rust.");
        assert_eq!(play(&mut state, "again"), "Pitted with rust.");
    }

    #[test]
    fn talk_shows_what_a_person_says_and_ask_matches_a_thing_or_a_topic() {
        let (dir, mut world) = world();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        let play = |world: &WorldEngine, state: &mut PlayState, line: &str| match game.parse_command(world, line, state) {
            Parsed::Action(a) => game.act(world, state, &a),
            Parsed::Reply(r) | Parsed::Unknown(r) => ActionResult::say(r),
        };
        assert_eq!(play(&world, &mut state, "talk").text, "Talk to whom?");
        assert_eq!(play(&world, &mut state, "talk to the king").text, "King Aldric has nothing to say.");
        assert_eq!(play(&world, &mut state, "ask").text, "Who do you want to ask?");
        assert_eq!(play(&world, &mut state, "ask king").text, "Ask King Aldric about what?");
        assert_eq!(play(&world, &mut state, "ask king about the war").text, "King Aldric doesn't know anything about that.");
        world.set_file(
            &dir.path().join("npcs/king.md"),
            "---\nid: king\ntitle: King Aldric\ntype: character\n---\nAn old king.\n\n## When: #talk\n\"Speak, traveller.\"\n\n- The sword -> ask @rusty-sword\n- The war -> ask #the-war\n\n## When: #ask @rusty-sword\n```thwarter\nsets: { story.asked: \"yes\" }\n```\n\"A fine blade, once.\"\n\n## When: #ask #the-war\n\"Long over.\"\n",
        );
        let talk = play(&world, &mut state, "chat with king");
        assert_eq!(talk.text, "\"Speak, traveller.\"\n\n- The sword\n- The war");
        assert_eq!(talk.topics.iter().map(|t| (t.label.as_str(), t.person.as_str(), t.topic.as_str())).collect::<Vec<_>>(), [("The sword", "king", "@rusty-sword"), ("The war", "king", "#the-war")]);
        // A topic by name, by a menu's id, or by its slug with or without
        // its article.
        assert_eq!(play(&world, &mut state, "ask king about rusty sword").text, "\"A fine blade, once.\"");
        assert_eq!(state.values.get("story.asked").map(String::as_str), Some("yes"));
        let menu = game.act(&world, &mut state, &PlayAction::Ask { person: "king".into(), topic: "#the-war".into() });
        assert_eq!(menu.text, "\"Long over.\"");
        assert_eq!(play(&world, &mut state, "ask king about war").text, "\"Long over.\"");
        // Carrying the sword doesn't make every question about it.
        play(&world, &mut state, "take rusty sword");
        assert_eq!(play(&world, &mut state, "ask king about the well").text, "King Aldric doesn't know anything about that.");
    }

    #[test]
    fn drop_puts_a_thing_down_here_and_give_needs_the_story_to_say_yes() {
        let (dir, mut world) = world();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/yard").unwrap();
        let play = |world: &WorldEngine, state: &mut PlayState, line: &str| match game.parse_command(world, line, state) {
            Parsed::Action(a) => game.act(world, state, &a).text,
            Parsed::Reply(r) | Parsed::Unknown(r) => r,
        };
        assert_eq!(play(&world, &mut state, "drop"), "Drop what?");
        assert_eq!(play(&world, &mut state, "drop the lamp"), "You aren't carrying any lamp.");
        play(&world, &mut state, "take rusty sword");
        // Dropped where it was taken, it's back as written.
        assert_eq!(play(&world, &mut state, "drop sword"), "You drop the rusty sword.");
        assert!(state.inventory.is_empty() && state.taken.is_empty() && state.held.is_empty(), "{:?}", state);
        // Dropped elsewhere, it's there and not here.
        play(&world, &mut state, "take rusty sword");
        play(&world, &mut state, "north");
        play(&world, &mut state, "drop sword");
        assert_eq!(state.held["castle/hall"], ["rusty-sword"]);
        assert!(play(&world, &mut state, "look").contains("You see a rusty sword."));
        assert!(play(&world, &mut state, "take sword").starts_with("You take"));
        assert!(state.held.is_empty());
        play(&world, &mut state, "south");
        assert!(!play(&world, &mut state, "look").contains("rusty sword"));

        // Give: the king refuses until the story says he takes it.
        assert_eq!(play(&world, &mut state, "give"), "Give what?");
        assert_eq!(play(&world, &mut state, "give sword"), "Give the sword to whom?");
        assert_eq!(play(&world, &mut state, "give sword to the cook"), "There's no cook here.");
        assert_eq!(play(&world, &mut state, "give sword to king"), "King Aldric doesn't want the rusty sword.");
        assert_eq!(state.inventory, ["rusty-sword"]);
        play(&world, &mut state, "take old sword");
        world.set_file(&dir.path().join("npcs/king.md"), "---\nid: king\ntitle: King Aldric\ntype: character\n---\nAn old king.\n\n## When: #give @rusty-sword\nHe takes it gravely.\n\n## When: #give #refused\nHe waves it away.\n");
        // Carrying the rusty sword doesn't make the old one welcome.
        assert_eq!(play(&world, &mut state, "give old sword to king"), "He waves it away.");
        assert_eq!(play(&world, &mut state, "give rusty sword to king"), "He takes it gravely.");
        assert_eq!(state.inventory, ["old-sword"]);
        assert_eq!(state.held["king"], ["rusty-sword"]);
    }

    #[test]
    fn where_look_toward_and_enter_say_what_is_where() {
        // REQ-PC-21 to 23.
        let (_d, world) = door_world();
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        let mut play = |line: &str| match game.parse_command(&world, line, &state) {
            Parsed::Action(a) => game.act(&world, &mut state, &a).text,
            Parsed::Reply(r) | Parsed::Unknown(r) => r,
        };
        assert_eq!(play("where am i?"), "You're in the hall.");
        assert_eq!(play("where"), "You're in the hall.");
        assert_eq!(play("look down"), "Down: a closed iron door.");
        assert_eq!(play("what\u{2019}s east?"), "East: an iron grate.");
        assert_eq!(play("what is north"), "You see no way north.");
        assert_eq!(play("look lever"), "You see nothing special about the lever.");
        play("pull lever");
        assert_eq!(play("look d"), "Down: an open iron door.");
        // `enter` is `go`, and a room's name finds the exit to it.
        assert!(play("enter the vault").starts_with("Gold."));
        assert_eq!(play("where"), "You're in the vault.");
        assert!(play("go hall").starts_with("Stone."));
        assert_eq!(play("look up"), "You see no way up.");
        assert_eq!(play("where is it"), "I don't understand \"where is it\".");
    }

    #[test]
    fn a_door_can_have_its_own_actions() {
        let (dir, mut world) = door_world();
        world.set_file(&dir.path().join("castle/doors/vault-door/index.md"), "---\nid: vault-door\ntitle: Iron door\ntype: door\ninherits: door\nsides:\n  castle/hall: { direction: down }\n  castle/vault: { direction: up }\nactions:\n  open: { becomes: { this: open } }\n---\n");
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/hall").unwrap();
        // Its own frontmatter action runs (not the built-in's script), and
        // the built-in's reply for ending up open answers.
        assert_eq!(game.act(&world, &mut state, &PlayAction::Use { verb: "open".into(), noun: "door".into(), with: None }).text, "It swings open.");
        assert_eq!(state.state_of("door", "vault-door"), Some("open"), "`this` is the door");
        game.act(&world, &mut state, &PlayAction::Go("down".into()));
        assert_eq!(state.location, "castle/vault");
    }

    #[test]
    fn portals_lead_where_each_side_says_and_door_rules_are_checked() {
        let (dir, mut world) = door_world();
        let d = dir.path();
        world.set_file(&d.join("castle/tower.md"), "---\nid: castle/tower\ntitle: Tower\ntype: location\nconnections: { in: ring }\n---\n");
        world.set_file(&d.join("castle/cell.md"), "---\nid: castle/cell\ntitle: Cell\ntype: location\nconnections: { west: grate, in: ring }\n---\n");
        world.set_file(&d.join("doors/ring.md"), "---\nid: ring\ntitle: Stone ring\ntype: door\ninherits: portal\nstate: open\nsides:\n  castle/tower: { direction: in, to: castle/cell }\n  castle/cell: { direction: in, to: castle/vault }\n  castle/vault: { direction: in, to: castle/tower }\n---\n");
        let problems = world.current_problems(false);
        assert!(problems.is_empty(), "{:?}", problems);
        let warnings = world.current_warnings();
        assert!(warnings.iter().any(|w| w == "door `ring`: castle/vault has no exit through it"), "{:?}", warnings);
        let game = GameEngine::new();
        let mut state = game.start(&world, "castle/tower").unwrap();
        game.act(&world, &mut state, &PlayAction::Go("in".into()));
        assert_eq!(state.location, "castle/cell");

        world.set_file(&d.join("doors/bad.md"), "---\nid: bad\ntitle: Bad\ntype: door\nsides:\n  castle/hall: { direction: north }\n---\n");
        world.set_file(&d.join("doors/ring.md"), "---\nid: ring\ntitle: Ring\ntype: door\nportal: yes\nsides:\n  castle/tower: { direction: in }\n  castle/cell: { direction: in }\n  castle/nowhere: { direction: in }\n---\n");
        world.set_file(&d.join("castle/vault.md"), "---\nid: castle/vault\ntitle: Vault\ntype: location\nconnections: { up: vault-door, east: grate }\n---\n");
        world.set_file(&d.join("doors/none.md"), "---\nid: none\ntitle: None\ntype: door\n---\n");
        let problems = world.current_problems(false);
        // A door in one room so far is half-built: a warning, and going
        // through it leads nowhere.
        assert!(!problems.iter().any(|p| p.contains("door `bad`")), "{:?}", problems);
        assert!(world.current_warnings().iter().any(|w| w.starts_with("door `bad`: it's only in `castle/hall` so far")), "{:?}", world.current_warnings());
        for expected in [
            "door `ring`: the side in `castle/tower` needs `to:`",
            "door `ring`: side `castle/nowhere` is not a known location",
            "castle/vault: connection `east` goes through door `grate`, which doesn't join this room",
            "door `none`: a door needs a side in at least one room",
        ] {
            assert!(problems.iter().any(|p| p.starts_with(expected)), "{}: {:?}", expected, problems);
        }
    }
}
