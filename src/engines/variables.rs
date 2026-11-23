//! Variables in prose (`design/STATES.md` › Variables, REQ-CF-16):
//! `{{ level.name }}`, where the level is `game`, `story`, `player`,
//! `location`, `this` or a thing's id. A thing's id may hold slashes, so a
//! name splits at its last dot (`{{ castle/vault.lid }}`). Hugo's
//! shortcodes (`{{< >}}` and `{{% %}}`) are left alone (REQ-CF-27).

/// The levels that aren't thing ids. They're reserved: no thing may use
/// one as its id.
pub const LEVELS: &[&str] = &["game", "story", "player", "location", "this"];

/// What the engine itself declares at each fixed level.
pub const GAME_VALUES: &[&str] = &["turn", "score"];
pub const STORY_VALUES: &[&str] = &["chapter"];
pub const PLAYER_VALUES: &[&str] = &["title"];
pub const LOCATION_VALUES: &[&str] = &["title", "visits"];
/// What every thing has, besides its `vars:`.
pub const THING_VALUES: &[&str] = &["title"];

/// One `{{ ... }}` in a text: its name as written, trimmed, and the byte
/// range of the whole `{{ ... }}`.
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub name: String,
    pub range: std::ops::Range<usize>,
}

/// Every variable in `text`, in order.
pub fn find(text: &str) -> Vec<Use> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(start) = text[from..].find("{{").map(|i| i + from) {
        let after = &text[start + 2..];
        if after.starts_with('<') || after.starts_with('%') {
            from = start + 2;
            continue;
        }
        let Some(end) = after.find("}}") else { break };
        let name = after[..end].trim().to_string();
        out.push(Use { name, range: start..start + 2 + end + 2 });
        from = start + 2 + end + 2;
    }
    out
}

/// A name split into its level (or thing id) and the value's name:
/// `story.king` → `("story", "king")`. `None` if it has no dot.
pub fn split(name: &str) -> Option<(&str, &str)> {
    let at = name.rfind('.')?;
    let (level, key) = (&name[..at], &name[at + 1..]);
    if level.is_empty() || key.is_empty() {
        return None;
    }
    Some((level, key))
}

/// `text` with each variable replaced by what `value` gives for its name,
/// or by nothing when it gives `None`, so a player never sees raw `{{ }}`.
pub fn substitute(text: &str, value: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for u in find(text) {
        out.push_str(&text[last..u.range.start]);
        out.push_str(&value(&u.name).unwrap_or_default());
        last = u.range.end;
    }
    out.push_str(&text[last..]);
    out
}

use super::bundles::Thing;
use super::layers::Layer;
use super::world_def::read_vars;
use super::world_engine::{At, EntityKind, WorldEngine};
use crate::parsers::ParsedEntity;

/// Where a variable is read: in a game (or not), whose prose it is in
/// (`this`), and which room `location` means.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scope<'a> {
    pub at: At<'a>,
    pub this: Option<(EntityKind, &'a str)>,
    /// The current room; in a game, the player's room unless set.
    pub here: Option<&'a str>,
}

fn title_of(entity: &ParsedEntity) -> String {
    match entity {
        ParsedEntity::Location(l) => l.name.clone(),
        ParsedEntity::Character(c) => c.name.clone(),
        ParsedEntity::Object(o) => o.title.clone(),
    }
}

/// Every name a thing declares in any of its files, plus `title`.
pub fn declared_by(thing: &Thing) -> Vec<String> {
    let mut names: Vec<String> = THING_VALUES.iter().map(|s| s.to_string()).collect();
    let layers = thing.templates.iter().chain(std::iter::once(&thing.index))
        .chain(thing.states.values())
        .chain(thing.overlays.values())
        .chain(thing.area_overlays.iter().map(|o| &o.layer));
    for layer in layers {
        if let Ok(vars) = read_vars(&layer.metadata) {
            names.extend(vars.into_keys());
        }
    }
    names.sort();
    names.dedup();
    names
}

impl WorldEngine {
    /// The thing a level names: `this`, the current room for `location`,
    /// or a thing by id (or by link).
    fn level_thing(&self, level: &str, scope: &Scope) -> Option<&Thing> {
        match level {
            "this" => scope.this.and_then(|(k, id)| self.thing(k, id)),
            "location" => {
                let here = scope.here.or(scope.at.play.map(|p| p.location.as_str()))?;
                self.thing(EntityKind::Location, here)
            }
            _ => self.thing_by_id(&self.resolve_name(level, None)).ok(),
        }
    }

    /// A variable's value (`story.king`, `this.lit`, `castle/vault.lid`):
    /// a value set in play holds; otherwise the declared starting value,
    /// or for a thing the value in its current version. `None` if it
    /// isn't declared, which play shows as blank.
    pub fn variable(&self, name: &str, scope: &Scope) -> Option<String> {
        let (level, key) = split(name)?;
        let play = scope.at.play;
        let set = |full: String| play.and_then(|p| p.values.get(&full).cloned());
        match level {
            "game" => match key {
                "turn" => Some(play.map_or(0, |p| p.turn).to_string()),
                "score" => Some(set("game.score".into()).unwrap_or_else(|| "0".into())),
                _ => None,
            },
            "story" => match key {
                "chapter" => Some(play.and_then(|p| p.chapter.as_ref()).map(|c| c.name().to_string()).unwrap_or_default()),
                _ => set(format!("story.{}", key)).or_else(|| self.world().story.get(key).cloned()),
            },
            "player" => {
                let player = self.world().player.as_ref();
                set(format!("player.{}", key)).or_else(|| match key {
                    "title" => Some(player.map_or_else(|| "you".to_string(), |p| p.title.clone())),
                    _ => player.and_then(|p| p.vars.get(key).cloned()),
                })
            }
            _ => {
                let thing = self.level_thing(level, scope)?;
                if level == "location" && key == "visits" {
                    return Some(play.map_or(0, |p| p.visits_to(&thing.id)).to_string());
                }
                if let Some(v) = set(format!("{}.{}", thing.id, key)) {
                    return Some(v);
                }
                let state = play
                    .and_then(|p| p.state_of(thing.kind.type_name(), &thing.id))
                    .filter(|s| thing.states.contains_key(*s))
                    .map(String::from)
                    .or_else(|| thing.starting_state.clone());
                let chapter = play.and_then(|p| p.chapter.as_ref()).and_then(|c| self.chapter_key(c));
                let (entity, merged) = thing.build(state.as_deref(), chapter.as_ref()).ok()?;
                if key == "title" {
                    return Some(title_of(&entity));
                }
                read_vars(&merged.metadata).ok()?.get(key).cloned()
            }
        }
    }

    /// `text` with its variables filled in, unknown ones blank (REQ-CF-16).
    pub fn fill(&self, text: &str, scope: &Scope) -> String {
        substitute(text, |name| self.variable(name, scope))
    }

    /// Why `name` isn't declared where it's used, or `None` if it is.
    /// `this` is the thing whose file holds the text, if any.
    pub fn undeclared(&self, name: &str, this: Option<&Thing>) -> Option<String> {
        let Some((level, key)) = split(name) else {
            return Some("needs a level, like `story.` or `player.`".to_string());
        };
        let ok = match level {
            "game" => GAME_VALUES.contains(&key),
            "story" => STORY_VALUES.contains(&key) || self.world().story.contains_key(key),
            "player" => PLAYER_VALUES.contains(&key) || self.world().player.as_ref().map_or(false, |p| p.vars.contains_key(key)),
            "location" => {
                LOCATION_VALUES.contains(&key)
                    || match this.filter(|t| t.kind == EntityKind::Location) {
                        Some(room) => declared_by(room).iter().any(|n| n == key),
                        None => self.locations_declaring(key),
                    }
            }
            "this" => match this {
                Some(thing) => declared_by(thing).iter().any(|n| n == key),
                None => return Some("`this.` only means something in a thing's own file".to_string()),
            },
            _ => match self.thing_by_id(&self.resolve_name(level, None)) {
                Ok(thing) => declared_by(thing).iter().any(|n| n == key),
                Err(e) => return Some(e.to_string()),
            },
        };
        if ok {
            return None;
        }
        Some(match level {
            "game" => format!("the game has only {}", GAME_VALUES.join(" and ")),
            "story" => "story values are declared in world/story.md's `vars:`".to_string(),
            "player" => "player values are declared in world/player.md's `vars:`".to_string(),
            _ => format!("`{}` isn't in the `vars:` it names", key),
        })
    }

    fn locations_declaring(&self, key: &str) -> bool {
        self.loaded_locations
            .keys()
            .filter_map(|id| self.thing(EntityKind::Location, id))
            .any(|t| declared_by(t).iter().any(|n| n == key))
    }

    /// Warnings for each undeclared variable in prose and in `sets:`.
    pub(crate) fn variable_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let check = |text: &str, path: &std::path::Path, this: Option<&Thing>, out: &mut Vec<String>| {
            for u in find(text) {
                if let Some(why) = self.undeclared(&u.name, this) {
                    out.push(format!("{}: `{{{{ {} }}}}` isn't declared: {}", path.display(), u.name, why));
                }
            }
        };
        let mut things: Vec<&Thing> = self.things_iter().collect();
        things.sort_by(|a, b| a.index.path.cmp(&b.index.path));
        for thing in &things {
            let own: Vec<&Layer> = std::iter::once(&thing.index).chain(thing.states.values()).chain(thing.overlays.values()).collect();
            for layer in own {
                check(&layer.body, &layer.path, Some(thing), &mut warnings);
            }
        }
        for chapter in self.chapters() {
            check(&chapter.layer.body, &chapter.layer.path, None, &mut warnings);
        }
        let world = self.world();
        if let Some(source) = &world.source {
            check(&world.intro, source, None, &mut warnings);
        }
        if let Some(player) = &world.player {
            check(&player.description, &player.path, None, &mut warnings);
        }
        let mut ids: Vec<&String> = self.objects.keys().collect();
        ids.sort();
        for id in ids {
            let object = &self.objects[id];
            let this = self.thing(EntityKind::Object, id);
            let mut names: Vec<&String> = object.actions.keys().collect();
            names.sort();
            for name in names {
                for var in object.actions[name].sets.keys() {
                    if let Some(why) = self.undeclared(var, this) {
                        warnings.push(format!("object `{}`, action `{}`: `sets:` names `{}`, which isn't declared: {}", id, name, var, why));
                    }
                }
            }
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables_are_found_and_substituted_and_shortcodes_left_alone() {
        let text = "Hi {{ player.title }}, it's turn {{game.turn}}. {{< figure >}} {{% note %}} {{ castle/vault.lid }}";
        let names: Vec<String> = find(text).into_iter().map(|u| u.name).collect();
        assert_eq!(names, ["player.title", "game.turn", "castle/vault.lid"]);
        let out = substitute(text, |n| (n == "player.title").then(|| "Ana".to_string()));
        assert_eq!(out, "Hi Ana, it's turn . {{< figure >}} {{% note %}} ");
        assert_eq!(split("castle/vault.lid"), Some(("castle/vault", "lid")));
        assert_eq!(split("king"), None);
        assert_eq!(find("an unclosed {{ name"), vec![]);
    }

    use crate::engines::{EntityKind, GameEngine, WorldEngine};
    use crate::models::game::PlayAction;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn world() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "world/index.md", "---\ntitle: T\ntype: world\nstart: hall\n---\n");
        write(d, "world/story.md", "---\ntype: story\nvars: { king: alive }\n---\n");
        write(d, "world/player.md", "---\ntitle: Ana\ntype: character\ncharacter_type: player\nvars: { health: 100 }\n---\n");
        write(d, "hall/index.md", "---\nid: hall\ntitle: Hall\ntype: location\nstate: dim\nobjects: [lamp]\nvars: { mood: calm }\n---\n{{ player.title }} stands in the {{ location.title }} (visit {{ location.visits }}, turn {{ game.turn }}). The king is {{ story.king }}. The lamp is {{ lamp.lit }}. {{ story.nope }}{{< figure >}}\n");
        write(d, "hall/dim.md", "---\ninherits: index\n---\n");
        write(d, "hall/bright.md", "---\ninherits: index\nvars: { mood: cheerful }\n---\nA {{ this.mood }} hall.\n");
        write(d, "objects/lamp.md", "---\nid: lamp\ntitle: Lamp\ntype: object\nobject_type: tool\nvars: { lit: unlit }\nactions:\n  light:\n    sets: { this.lit: lit, story.king: dead, story.queen: sad }\n    becomes: { hall: bright }\n---\nA {{ this.lit }} lamp.\n\n<!-- thwarter:action light -->\nThe lamp is {{ this.lit }} now; health {{ player.health }}.\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        (dir, engine)
    }

    #[test]
    fn play_fills_variables_from_every_level() {
        let (_d, world) = world();
        let game = GameEngine::new();
        let mut state = game.start(&world, "hall").unwrap();
        let look = game.act(&world, &mut state, &PlayAction::Look).text;
        assert!(look.contains("Ana stands in the Hall (visit 1, turn 1). The king is alive. The lamp is unlit. {{< figure >}}"), "{}", look);
        assert_eq!(game.act(&world, &mut state, &PlayAction::Examine("lamp".into())).text, "A unlit lamp.");
        let r = game.act(&world, &mut state, &PlayAction::Use { verb: "light".into(), noun: "lamp".into() });
        assert_eq!(r.text, "The lamp is lit now; health 100.");
        // Values hold once set; the room's state switched, so `this.` reads the new version.
        let look = game.act(&world, &mut state, &PlayAction::Look).text;
        assert!(look.contains("A cheerful hall."), "{}", look);
        assert_eq!(state.values["story.king"], "dead");
    }

    #[test]
    fn undeclared_variables_warn_and_reserved_ids_are_problems() {
        let (dir, mut world) = world();
        let warnings = world.current_warnings();
        assert!(warnings.iter().any(|w| w.contains("hall/index.md: `{{ story.nope }}` isn't declared: story values are declared in world/story.md")), "{:?}", warnings);
        assert!(warnings.iter().any(|w| w.contains("`sets:` names `story.queen`, which isn't declared")), "{:?}", warnings);
        assert!(!warnings.iter().any(|w| w.contains("lamp.lit") || w.contains("this.mood") || w.contains("player.health")), "{:?}", warnings);
        world.set_file(&dir.path().join("objects/story.md"), "---\nid: story\ntitle: A story\ntype: object\nobject_type: book\n---\n");
        world.set_file(&dir.path().join("objects/v.md"), "---\nid: v1.2\ntitle: V\ntype: object\nobject_type: book\n---\n");
        let problems = world.current_problems(false);
        assert!(problems.iter().any(|p| p == "object `story` can't be an id: it's reserved as a variable level"), "{:?}", problems);
        assert!(problems.iter().any(|p| p.starts_with("object `v1.2` can't be an id: ids can't contain a dot")));
        assert_eq!(world.variable("lamp.lit", &super::Scope::default()).as_deref(), Some("unlit"));
        assert_eq!(world.thing(EntityKind::Object, "lamp").map(|t| super::declared_by(t)), Some(vec!["lit".to_string(), "title".to_string()]));
    }
}
