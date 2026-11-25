//! Text views of characters and objects for edit mode. Rooms use the
//! shared renderer instead (`crate::render::render_location`, REQ-CON-8).
//! Plain text, no colour and no trailing newline, so transcripts can
//! compare it as is.

use crate::engines::world_def::WorldDef;
use crate::models::character::Character;
use crate::models::world::GameObject;
use crate::engines::texts::Texts;

fn t(texts: &Texts, key: &str, args: &[(&str, &str)]) -> String {
    texts.text("console", key, args)
}

/// A character as edit mode shows it. Its file and kind are in the
/// sidebar, not here (Bruce, 2026-11-23).
pub fn character(c: &Character, texts: &Texts) -> String {
    let mut out = vec![format!("{} ({})", c.name, c.id)];
    out.push(String::new());
    out.push(description(&c.description, texts));
    out.push(String::new());
    let nowhere = t(texts, "nowhere", &[]);
    out.push(t(texts, "starts_in", &[("room", c.default_location.as_deref().unwrap_or(&nowhere))]));
    out.push(list(&t(texts, "view_carries", &[]), &c.inventory));
    tidy(out)
}

/// An object as edit mode shows it; its file and type are in the sidebar.
pub fn object(o: &GameObject, texts: &Texts) -> String {
    let mut out = vec![format!("{} ({})", o.title, o.id)];
    out.push(String::new());
    out.push(description(&o.description, texts));
    let mut actions: Vec<&String> = o.actions.keys().collect();
    actions.sort();
    out.push(String::new());
    out.push(list(&t(texts, "view_actions", &[]), &actions.into_iter().cloned().collect::<Vec<_>>()));
    tidy(out)
}

/// The world's own settings, as edit mode shows any file in `world/`.
pub fn world(def: &WorldDef, path: &str, texts: &Texts) -> String {
    let mut out = vec![t(texts, "view_world_heading", &[("path", path)]), String::new()];
    let or_unset = |v: &Option<String>| v.clone().unwrap_or_else(|| t(texts, "view_not_set", &[]));
    out.push(t(texts, "view_title", &[("title", &or_unset(&def.title))]));
    if !def.authors.is_empty() {
        let key = if def.authors.len() == 1 { "view_author" } else { "view_authors" };
        out.push(t(texts, key, &[("authors", &def.authors.join(", "))]));
    }
    if let Some(version) = &def.version {
        out.push(t(texts, "view_version", &[("version", version)]));
    }
    out.push(t(texts, "starts_in", &[("room", &or_unset(&def.start))]));
    out.push(String::new());
    out.push(description(&def.intro, texts));
    let story: Vec<String> = def.story.iter().map(|(k, v)| format!("{} = {}", k, v)).collect();
    out.push(String::new());
    out.push(list(&t(texts, "view_story_values", &[]), &story));
    if let Some(player) = &def.player {
        let vars: Vec<String> = player.vars.iter().map(|(k, v)| format!("{} = {}", k, v)).collect();
        out.push(t(texts, "view_player", &[("player", &player.title)]));
        out.push(list(&t(texts, "view_player_values", &[]), &vars));
    }
    tidy(out)
}

fn description(text: &str, texts: &Texts) -> String {
    match text.trim() {
        "" => t(texts, "view_no_description", &[]),
        text => text.to_string(),
    }
}

/// The lines joined, with the blanks an empty list leaves behind folded
/// away: no two blank lines together, none at the end.
fn tidy(lines: Vec<String>) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        if line.is_empty() && out.last().is_some_and(|l: &String| l.is_empty()) {
            continue;
        }
        out.push(line);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// "Actions: light, open", or nothing when there are none (Bruce,
/// 2026-11-24: an empty list isn't worth a line).
fn list(heading: &str, items: &[String]) -> String {
    if items.is_empty() {
        String::new()
    } else {
        format!("{}: {}", heading, items.join(", "))
    }
}
