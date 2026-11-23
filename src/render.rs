//! Text rendering shared by `thwart show` and the console (REQ-CON-8), so
//! the two room views can't drift apart. Output is plain text with no
//! colour and no trailing newline, so it can be printed or compared in
//! transcripts as is.

use crate::engines::LocationView;

/// Who a room view is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStyle {
    /// `thwart show` and edit mode: ids, coordinates, and what's missing.
    Author,
    /// Play mode: what the player sees, by name only.
    Player,
}

/// `render_location` in the given style.
pub fn render_location_as(view: &LocationView, style: RenderStyle) -> String {
    match style {
        RenderStyle::Author => render_location(view),
        RenderStyle::Player => render_for_player(view),
    }
}

/// A room as the player sees it: title, description, the ways out, then
/// what and who is here, by name. No ids, coordinates or missing marks.
fn render_for_player(view: &LocationView) -> String {
    let mut out = vec![view.title.clone(), String::new()];
    let body = view.body.trim();
    out.push(if body.is_empty() { "(no description yet)".to_string() } else { body.to_string() });
    out.push(String::new());
    if view.exits.is_empty() {
        out.push("There's no way out.".to_string());
    } else {
        let dirs: Vec<&str> = view.exits.iter().map(|e| e.direction.as_str()).collect();
        out.push(format!("Exits: {}.", dirs.join(", ")));
    }
    if !view.objects.is_empty() {
        let names: Vec<String> = view.objects.iter().map(|o| o.display_name()).collect();
        out.push(format!("You see: {}.", names.join(", ")));
    }
    if !view.characters.is_empty() {
        let names: Vec<String> = view.characters.iter().map(|c| c.display_name()).collect();
        out.push(format!("Here: {}.", names.join(", ")));
    }
    out.join("\n")
}

/// A location as an author checks it: title and id, coordinates,
/// description (or `(no description yet)`), exits, then who and what is there. An exit to an unknown
/// room says `(missing)`; an object with no card says `(no card)`; a
/// character with no card says `(not found)`.
pub fn render_location(view: &LocationView) -> String {
    let mut out = Vec::new();
    out.push(format!("{} ({})", view.title, view.id));
    if let Some(c) = &view.coordinates {
        out.push(format!("at ({}, {}, {})", c.x, c.y, c.z));
    }
    out.push(String::new());
    let body = view.body.trim();
    // Matches the console's character and object views.
    out.push(if body.is_empty() { "(no description yet)".to_string() } else { body.to_string() });

    out.push(String::new());
    if view.exits.is_empty() {
        out.push("Exits: none".to_string());
    } else {
        out.push("Exits:".to_string());
        let width = view.exits.iter().map(|e| e.direction.len()).max().unwrap_or(0);
        for exit in &view.exits {
            let missing = if exit.target_exists { "" } else { "  (missing)" };
            out.push(format!("  {:<width$}  {}{}", exit.direction, exit.target, missing, width = width));
        }
    }

    push_refs(&mut out, "Characters", &view.characters, "not found");
    push_refs(&mut out, "Objects", &view.objects, "no card");
    out.join("\n")
}

fn push_refs(out: &mut Vec<String>, heading: &str, refs: &[crate::engines::EntityRef], missing_label: &str) {
    out.push(String::new());
    if refs.is_empty() {
        out.push(format!("{}: none", heading));
        return;
    }
    out.push(format!("{}:", heading));
    for r in refs {
        match &r.name {
            Some(name) => out.push(format!("  {} ({})", name, r.id)),
            None => out.push(format!("  {}  ({})", r.id, missing_label)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::{EntityRef, ExitView};
    use crate::models::world::Coordinate;

    fn view() -> LocationView {
        LocationView {
            id: "castle/courtyard".into(),
            title: "Castle Courtyard".into(),
            coordinates: Some(Coordinate::new(0, 1, 0)),
            body: "  A dry fountain.\n".into(),
            exits: vec![
                ExitView { direction: "north".into(), target: "castle/great-hall".into(), target_exists: true },
                ExitView { direction: "down".into(), target: "castle/cellar".into(), target_exists: false },
            ],
            characters: vec![
                EntityRef { id: "guard-captain".into(), name: Some("Captain Marcus".into()) },
                EntityRef { id: "ghost".into(), name: None },
            ],
            objects: vec![EntityRef { id: "fountain".into(), name: None }],
        }
    }

    #[test]
    fn renders_full_room() {
        let expected = "\
Castle Courtyard (castle/courtyard)
at (0, 1, 0)

A dry fountain.

Exits:
  north  castle/great-hall
  down   castle/cellar  (missing)

Characters:
  Captain Marcus (guard-captain)
  ghost  (not found)

Objects:
  fountain  (no card)";
        assert_eq!(render_location(&view()), expected);
    }

    #[test]
    fn renders_empty_room() {
        let mut v = view();
        v.coordinates = None;
        v.exits.clear();
        v.characters.clear();
        v.objects.clear();
        let text = render_location(&v);
        assert!(!text.contains("at ("));
        assert!(text.contains("Exits: none"));
        assert!(text.contains("Characters: none"));
        assert!(text.ends_with("Objects: none"));
    }

    #[test]
    fn renders_for_the_player_by_name_only() {
        let expected = "\
Castle Courtyard

A dry fountain.

Exits: north, down.
You see: Fountain.
Here: Captain Marcus, Ghost.";
        assert_eq!(render_location_as(&view(), RenderStyle::Player), expected);
        let mut empty = view();
        empty.exits.clear();
        empty.objects.clear();
        empty.characters.clear();
        assert!(render_location_as(&empty, RenderStyle::Player).ends_with("There's no way out."));
        assert_eq!(render_location_as(&view(), RenderStyle::Author), render_location(&view()));
    }

    #[test]
    fn an_empty_body_says_so() {
        let mut v = view();
        v.coordinates = None;
        v.body = "  \n".into();
        let text = render_location(&v);
        assert!(text.starts_with("Castle Courtyard (castle/courtyard)\n\n(no description yet)\n\nExits:\n"), "{}", text);
    }
}
