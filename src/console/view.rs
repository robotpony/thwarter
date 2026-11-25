//! Text views of characters and objects for edit mode. Rooms use the
//! shared renderer instead (`crate::render::render_location`, REQ-CON-8).
//! Plain text, no colour and no trailing newline, so transcripts can
//! compare it as is.

use crate::engines::world_def::WorldDef;
use crate::models::character::Character;
use crate::models::world::GameObject;

/// A character as edit mode shows it. Its file and kind are in the
/// sidebar, not here (Bruce, 2026-11-23).
pub fn character(c: &Character) -> String {
    let mut out = vec![format!("{} ({})", c.name, c.id)];
    out.push(String::new());
    out.push(description(&c.description));
    out.push(String::new());
    out.push(format!("Starts in: {}", c.default_location.as_deref().unwrap_or("nowhere")));
    out.push(list("Carries", &c.inventory));
    tidy(out)
}

/// An object as edit mode shows it; its file and type are in the sidebar.
pub fn object(o: &GameObject) -> String {
    let mut out = vec![format!("{} ({})", o.title, o.id)];
    out.push(String::new());
    out.push(description(&o.description));
    let mut actions: Vec<&String> = o.actions.keys().collect();
    actions.sort();
    out.push(String::new());
    out.push(list("Actions", &actions.into_iter().cloned().collect::<Vec<_>>()));
    tidy(out)
}

/// The world's own settings, as edit mode shows any file in `world/`.
pub fn world(def: &WorldDef, path: &str) -> String {
    let mut out = vec![format!("World settings, {}.md", path), String::new()];
    let or_unset = |v: &Option<String>| v.clone().unwrap_or_else(|| "not set".to_string());
    out.push(format!("Title: {}", or_unset(&def.title)));
    if !def.authors.is_empty() {
        let label = if def.authors.len() == 1 { "Author" } else { "Authors" };
        out.push(format!("{}: {}", label, def.authors.join(", ")));
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
    tidy(out)
}

fn description(text: &str) -> String {
    match text.trim() {
        "" => "(no description yet)".to_string(),
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
