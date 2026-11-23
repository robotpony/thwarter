//! Text views of characters and objects for edit mode. Rooms use the
//! shared renderer instead (`crate::render::render_location`, REQ-CON-8).
//! Plain text, no colour and no trailing newline, so transcripts can
//! compare it as is.

use crate::engines::world_def::WorldDef;
use crate::models::character::Character;
use crate::models::world::GameObject;

pub fn character(c: &Character, path: &str) -> String {
    let kind = format!("{:?}", c.character_type).to_lowercase();
    let mut out = vec![format!("{} ({})", c.name, c.id), format!("{}.md, {}", path, kind)];
    out.push(String::new());
    out.push(description(&c.description));
    out.push(String::new());
    out.push(format!("Starts in: {}", c.default_location.as_deref().unwrap_or("nowhere")));
    out.push(list("Carries", &c.inventory));
    out.join("\n")
}

pub fn object(o: &GameObject, path: &str) -> String {
    let mut kind = o.object_type.clone();
    if let Some(category) = &o.category {
        kind = format!("{}, {}", kind, category);
    }
    let mut out = vec![format!("{} ({})", o.title, o.id), format!("{}.md, {}", path, kind)];
    out.push(String::new());
    out.push(description(&o.description));
    let mut actions: Vec<&String> = o.actions.keys().collect();
    actions.sort();
    out.push(String::new());
    out.push(list("Actions", &actions.into_iter().cloned().collect::<Vec<_>>()));
    out.join("\n")
}

/// The world's own settings, as edit mode shows any file in `world/`.
pub fn world(def: &WorldDef, path: &str) -> String {
    let mut out = vec![format!("World settings, {}.md", path), String::new()];
    let or_unset = |v: &Option<String>| v.clone().unwrap_or_else(|| "not set".to_string());
    out.push(format!("Title: {}", or_unset(&def.title)));
    if let Some(author) = &def.author {
        out.push(format!("Author: {}", author));
    }
    if let Some(version) = &def.version {
        out.push(format!("Version: {}", version));
    }
    out.push(format!("Starts in: {}", or_unset(&def.start)));
    out.push(String::new());
    out.push(description(&def.intro));
    let story: Vec<String> = def.story.iter().map(|(k, v)| format!("{} = {}", k, v)).collect();
    out.push(String::new());
    out.push(list("Story values", &story));
    if let Some(player) = &def.player {
        let vars: Vec<String> = player.vars.iter().map(|(k, v)| format!("{} = {}", k, v)).collect();
        out.push(format!("Player: {}", player.title));
        out.push(list("Player values", &vars));
    }
    out.join("\n")
}

fn description(text: &str) -> String {
    match text.trim() {
        "" => "(no description yet)".to_string(),
        text => text.to_string(),
    }
}

fn list(heading: &str, items: &[String]) -> String {
    if items.is_empty() {
        format!("{}: none", heading)
    } else {
        format!("{}: {}", heading, items.join(", "))
    }
}
