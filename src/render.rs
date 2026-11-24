//! Text rendering shared by `thwart show` and the console (REQ-CON-8), so
//! the two room views can't drift apart. Output is plain text with no
//! colour and no trailing newline, so it can be printed or compared in
//! transcripts as is.

use crate::engines::{EntityKind, LocationView};
use crate::parsers::link_target;
use pulldown_cmark::{Event, Options, Parser, Tag};

/// `text` without terminal control characters or escape sequences
/// (REQ-CFG-9), so a world's prose and titles can't move the cursor,
/// change colours, rewrite the screen or set the terminal's title. Line
/// breaks and tabs stay; carriage returns, which can overwrite a line,
/// don't. CSI (`ESC [ ...`) and OSC (`ESC ] ... BEL` or `ESC \\`)
/// sequences go whole, so no stray `[31m` is left behind.
pub fn clean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.peek() {
                Some('[') => {
                    chars.next();
                    // Parameters and intermediates, then one final byte.
                    while let Some(&n) = chars.peek() {
                        chars.next();
                        if ('\u{40}'..='\u{7e}').contains(&n) {
                            break;
                        }
                    }
                }
                Some(']') | Some('P') | Some('_') | Some('^') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '\u{7}' {
                            break;
                        }
                        if n == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            },
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

impl Rendered {
    /// The same blocks with every span's text cleaned (REQ-CFG-9).
    fn cleaned(mut self) -> Self {
        for block in &mut self.blocks {
            for span in &mut block.spans {
                span.text = clean_text(&span.text);
                if let Some(link) = span.style.link.as_mut() {
                    link.name = clean_text(&link.name);
                }
            }
        }
        self
    }
}

/// Who a room view is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStyle {
    /// `thwart show` and edit mode: ids, coordinates, and what's missing.
    Author,
    /// Play mode: what the player sees, by name only.
    Player,
}

/// Text as logical blocks of styled spans (REQ-CF-28), for the console
/// to style and wrap, and for `to_plain` to print. Blocks are never
/// wrapped or hard-broken here; roles are semantic, never colours.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rendered {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    pub spans: Vec<Span>,
    /// A blank line comes before this block.
    pub space_before: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Title,
    Heading(u8),
    Paragraph,
    /// A list item, with its depth (1 for the outermost list).
    ListItem(u8),
    Quote,
    Code,
    /// The ways out of a room.
    Exits,
    /// Who and what is here.
    Here,
    /// A message from the engine, such as "(no description yet)".
    Notice,
    /// An author's `<!-- note -->`.
    AuthorNote,
    /// An action's reply.
    Response,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    pub strong: bool,
    pub emphasis: bool,
    pub code: bool,
    pub link: Option<Link>,
    /// A `#tag`, shown without its `#`.
    pub tag: bool,
    /// An exit or link to something not written yet.
    pub missing: bool,
}

/// A link to a thing (REQ-CF-29), with its target if it resolves.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    /// As written: `castle/vault`, `lever`, `objects/lever.md`.
    pub name: String,
    pub target: Option<(EntityKind, String)>,
}

/// What a link resolves to: the thing's kind, id and title.
pub type Resolver<'a> = &'a dyn Fn(&str) -> Option<(EntityKind, String, String)>;

fn no_links(_: &str) -> Option<(EntityKind, String, String)> {
    None
}

impl Span {
    pub fn plain(text: impl Into<String>) -> Self {
        Span { text: text.into(), style: Style::default() }
    }
}

impl Block {
    pub fn new(kind: BlockKind, spans: Vec<Span>, space_before: bool) -> Self {
        Block { kind, spans, space_before }
    }

    fn line(kind: BlockKind, text: impl Into<String>, space_before: bool) -> Self {
        Block::new(kind, vec![Span::plain(text)], space_before)
    }

    /// The block's text, without styling.
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

impl Rendered {
    /// Plain text: blocks one per line, a blank line where a block asks
    /// for one, list items marked, nothing wrapped, no trailing newline.
    pub fn to_plain(&self) -> String {
        let mut out = String::new();
        for (i, block) in self.blocks.iter().enumerate() {
            if i > 0 {
                out.push('\n');
                if block.space_before {
                    out.push('\n');
                }
            }
            let text = block.text();
            match block.kind {
                BlockKind::ListItem(depth) => {
                    out.push_str(&"  ".repeat(depth.saturating_sub(1) as usize));
                    out.push_str("- ");
                    out.push_str(&text);
                }
                BlockKind::Quote => {
                    let quoted: Vec<String> = text.lines().map(|l| format!("  {}", l)).collect();
                    out.push_str(&quoted.join("\n"));
                }
                _ => out.push_str(&text),
            }
        }
        out
    }
}

/// `render_location` in the given style.
pub fn render_location_as(view: &LocationView, style: RenderStyle) -> String {
    location_blocks(view, style, &no_links).to_plain()
}

/// A location as blocks, for the console: the same content
/// `render_location_as` prints. `links` resolves links in the prose.
pub fn location_blocks(view: &LocationView, style: RenderStyle, links: Resolver) -> Rendered {
    match style {
        RenderStyle::Author => author_blocks(view),
        RenderStyle::Player => player_blocks(view, links),
    }
    .cleaned()
}

/// A room as the player sees it: title, description, the ways out, then
/// what and who is here, by name. No ids, coordinates or missing marks.
fn player_blocks(view: &LocationView, links: Resolver) -> Rendered {
    let mut blocks = vec![Block::line(BlockKind::Title, view.title.clone(), false)];
    let body = view.body.trim();
    if body.is_empty() {
        blocks.push(Block::line(BlockKind::Notice, "(no description yet)", true));
    } else {
        let mut prose = prose_blocks(body, links).blocks;
        if let Some(first) = prose.first_mut() {
            first.space_before = true;
        }
        blocks.extend(prose);
    }
    if view.exits.is_empty() {
        blocks.push(Block::line(BlockKind::Notice, "There's no way out.", true));
    } else {
        let dirs: Vec<&str> = view.exits.iter().map(|e| e.direction.as_str()).collect();
        blocks.push(Block::line(BlockKind::Exits, format!("Exits: {}.", dirs.join(", ")), true));
    }
    // Each door's own text by the exits; the room's prose never changes
    // for a door (`design/STATES.md` › Doors).
    for door in view.exits.iter().filter_map(|e| e.door.as_ref()) {
        if !door.description.trim().is_empty() {
            for block in prose_blocks(door.description.trim(), links).blocks {
                blocks.push(Block { kind: BlockKind::Exits, space_before: false, ..block });
            }
        }
        if let Some(beyond) = &door.beyond {
            blocks.push(Block::line(BlockKind::Exits, format!("Through the {} you can see the {}.", door.title.to_lowercase(), beyond), false));
        }
    }
    let names = |refs: &[crate::engines::EntityRef], kind: EntityKind, label: &str| -> Option<Block> {
        if refs.is_empty() {
            return None;
        }
        let mut spans = vec![Span::plain(format!("{}: ", label))];
        for (i, r) in refs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::plain(", "));
            }
            let link = Link { name: r.id.clone(), target: r.name.as_ref().map(|_| (kind, r.id.clone())) };
            spans.push(Span { text: r.display_name(), style: Style { link: Some(link), ..Default::default() } });
        }
        spans.push(Span::plain("."));
        Some(Block::new(BlockKind::Here, spans, false))
    };
    blocks.extend(seen(view));
    blocks.extend(names(&view.characters, EntityKind::Character, "Here"));
    Rendered { blocks }
}

/// "You see:" in play: every visible thing by title, each held thing
/// right after what holds it, "Old photo (on the walls)" (REQ-CF-31), and
/// what the doors hold last.
fn seen(view: &LocationView) -> Option<Block> {
    let mut entries: Vec<(&crate::engines::EntityRef, Option<&crate::engines::Held>)> = Vec::new();
    fn add<'a>(view: &'a LocationView, holder: &str, entries: &mut Vec<(&'a crate::engines::EntityRef, Option<&'a crate::engines::Held>)>) {
        for held in view.held.iter().filter(|h| h.holder == holder) {
            if entries.iter().any(|(r, _)| r.id == held.thing.id) {
                continue;
            }
            entries.push((&held.thing, Some(held)));
            add(view, &held.thing.id, entries);
        }
    }
    for object in &view.objects {
        entries.push((object, None));
        add(view, &object.id, &mut entries);
    }
    for door in view.exits.iter().filter_map(|e| e.door.as_ref()) {
        add(view, &door.id, &mut entries);
    }
    if entries.is_empty() {
        return None;
    }
    let mut spans = vec![Span::plain("You see: ")];
    for (i, (r, holder)) in entries.iter().enumerate() {
        if i > 0 {
            spans.push(Span::plain(", "));
        }
        let link = Link { name: r.id.clone(), target: r.name.as_ref().map(|_| (EntityKind::Object, r.id.clone())) };
        spans.push(Span { text: r.display_name(), style: Style { link: Some(link), ..Default::default() } });
        if let Some(held) = holder {
            spans.push(Span::plain(format!(" ({} the {})", held.word, held.holder_name.to_lowercase())));
        }
    }
    spans.push(Span::plain("."));
    Some(Block::new(BlockKind::Here, spans, false))
}

/// A location as an author checks it: title and id, coordinates,
/// description (or `(no description yet)`), exits, then who and what is
/// there. An exit to an unknown room says `(missing)`; an object with no
/// card says `(no card)`; a character with no card says `(not found)`.
/// The description is shown as written.
fn author_blocks(view: &LocationView) -> Rendered {
    let mut blocks = vec![Block::line(BlockKind::Title, format!("{} ({})", view.title, view.id), false)];
    if let Some(c) = &view.coordinates {
        blocks.push(Block::line(BlockKind::Notice, format!("at ({}, {}, {})", c.x, c.y, c.z), false));
    }
    let body = view.body.trim();
    // Matches the console's character and object views.
    blocks.push(if body.is_empty() {
        Block::line(BlockKind::Notice, "(no description yet)", true)
    } else {
        Block::line(BlockKind::Paragraph, body, true)
    });

    if view.exits.is_empty() {
        blocks.push(Block::line(BlockKind::Exits, "Exits: none", true));
    } else {
        blocks.push(Block::line(BlockKind::Exits, "Exits:", true));
        let width = view.exits.iter().map(|e| e.direction.len()).max().unwrap_or(0);
        for exit in &view.exits {
            let target = Style {
                link: Some(Link {
                    name: exit.target.clone(),
                    target: exit.target_exists.then(|| (EntityKind::Location, exit.target.clone())),
                }),
                missing: !exit.target_exists,
                ..Default::default()
            };
            let mut spans = vec![
                Span::plain(format!("  {:<width$}  ", exit.direction, width = width)),
                Span { text: exit.target.clone(), style: target },
            ];
            if !exit.target_exists {
                spans.push(Span { text: "  (missing)".into(), style: Style { missing: true, ..Default::default() } });
            }
            if let Some(door) = &exit.door {
                let link = Link { name: door.id.clone(), target: Some((EntityKind::Door, door.id.clone())) };
                spans.push(Span::plain("  (through "));
                spans.push(Span { text: door.id.clone(), style: Style { link: Some(link), ..Default::default() } });
                spans.push(Span::plain(if door.passable { ", passable)" } else { ", not passable)" }));
            }
            blocks.push(Block::new(BlockKind::Exits, spans, false));
        }
    }

    push_refs(&mut blocks, "Characters", &view.characters, "not found", EntityKind::Character);
    push_refs(&mut blocks, "Objects", &view.objects, "no card", EntityKind::Object);
    push_held(&mut blocks, view);
    Rendered { blocks }
}

/// A map as ASCII (`thwart map`): each room a `[Title]` box on its grid
/// cell, `@` for the room it's drawn from and `?` for one not written
/// yet, exits as `---`, `|`, `/` and `\\`, with `+` for a door on the way
/// and `#` for a closed one. Exits that don't fit the grid are listed
/// after it. Titles longer than 18 characters are cut.
pub fn render_map(map: &crate::engines::map::Map) -> String {
    if map.cells.is_empty() {
        return "(no rooms to map)".to_string();
    }
    let label = |c: &crate::engines::map::MapCell| -> String {
        let name = c.title.clone().unwrap_or_else(|| c.room.clone());
        let name: String = if name.chars().count() > 18 { name.chars().take(17).collect::<String>() + "…" } else { name };
        let mark = if c.here { "@ " } else if !c.written { "? " } else { "" };
        format!("{}{}", mark, name)
    };
    let width = map.cells.iter().map(|c| label(c).chars().count()).max().unwrap_or(1) + 2;
    let (min_x, max_x) = (map.cells.iter().map(|c| c.x).min().unwrap(), map.cells.iter().map(|c| c.x).max().unwrap());
    let (min_y, max_y) = (map.cells.iter().map(|c| c.y).min().unwrap(), map.cells.iter().map(|c| c.y).max().unwrap());
    let pitch = width + 3;
    let cols = (max_x - min_x) as usize * pitch + width;
    let rows = (max_y - min_y) as usize * 2 + 1;
    let mut canvas = vec![vec![' '; cols]; rows];
    let at = |x: i32, y: i32| ((x - min_x) as usize * pitch, (y - min_y) as usize * 2);
    let put = |canvas: &mut Vec<Vec<char>>, col: usize, row: usize, text: &str| {
        for (i, ch) in text.chars().enumerate() {
            if let Some(cell) = canvas.get_mut(row).and_then(|r| r.get_mut(col + i)) {
                *cell = ch;
            }
        }
    };
    for c in &map.cells {
        let (col, row) = at(c.x, c.y);
        let text = label(c);
        let pad = width - 2 - text.chars().count();
        put(&mut canvas, col, row, &format!("[{}{}{}]", " ".repeat(pad / 2), text, " ".repeat(pad - pad / 2)));
    }
    for link in &map.links {
        // Upper end first (then left), so every link runs down or right.
        let (a, b) = if (link.from.1, link.from.0) <= (link.to.1, link.to.0) { (link.from, link.to) } else { (link.to, link.from) };
        let mark = match &link.door {
            None => None,
            Some(d) if d.passable => Some('+'),
            Some(_) => Some('#'),
        };
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let (col, row) = at(a.0, a.1);
        match (dx, dy) {
            (1, 0) => put(&mut canvas, col + width, row, &format!("-{}-", mark.unwrap_or('-'))),
            (0, 1) => put(&mut canvas, col + width / 2, row + 1, &mark.unwrap_or('|').to_string()),
            (1, 1) => put(&mut canvas, col + width + 1, row + 1, &mark.unwrap_or('\\').to_string()),
            (-1, 1) => put(&mut canvas, col - 2, row + 1, &mark.unwrap_or('/').to_string()),
            _ => {}
        }
    }
    let mut out: Vec<String> = canvas.into_iter().map(|r| r.into_iter().collect::<String>().trim_end().to_string()).collect();
    if !map.beside.is_empty() {
        out.push(String::new());
        out.push("Also:".to_string());
        for b in &map.beside {
            let from = b.room_title.clone().unwrap_or_else(|| b.room.clone());
            let door = b.door.as_ref().map(|d| format!(" (through the {}{})", d.title.to_lowercase(), if d.passable { "" } else { ", closed" })).unwrap_or_default();
            let missing = if b.written { "" } else { " (not written yet)" };
            out.push(format!("  {}: {} → {}{}{}", from, b.direction, b.target, door, missing));
        }
    }
    clean_text(&out.join("\n"))
}

/// An object as an author checks it (`thwart show <object>`): title and
/// id, its type, its description as written, then its fields and
/// actions.
pub fn render_object(o: &crate::models::world::GameObject) -> String {
    let mut out = vec![format!("{} ({})", o.title, o.id)];
    let mut kind = format!("object, {}", o.object_type);
    if let Some(category) = &o.category {
        kind.push_str(&format!(", {}", category));
    }
    out.push(kind);
    out.push(String::new());
    let body = o.description.trim();
    out.push(if body.is_empty() { "(no description yet)".to_string() } else { body.to_string() });
    let mut fields = Vec::new();
    if let Some(w) = &o.weight {
        fields.push(format!("Weight: {}", w));
    }
    if let Some(v) = &o.value {
        fields.push(format!("Value: {}", v));
    }
    if !fields.is_empty() {
        out.push(String::new());
        out.extend(fields);
    }
    out.push(String::new());
    let mut names: Vec<&String> = o.actions.keys().collect();
    names.sort();
    if names.is_empty() {
        out.push("Actions: none".to_string());
    } else {
        out.push("Actions:".to_string());
        for name in names {
            let a = &o.actions[name];
            let mut what = Vec::new();
            for (id, state) in &a.becomes {
                what.push(format!("{} becomes {}", id, state));
            }
            for (var, value) in &a.sets {
                what.push(format!("sets {} = {}", var, value));
            }
            if let Some(c) = &a.chapter {
                what.push(format!("moves to chapter {}", c));
            }
            if !a.requires_has.is_empty() {
                what.push(format!("needs {}", a.requires_has.join(", ")));
            }
            let detail = if what.is_empty() { String::new() } else { format!(": {}", what.join("; ")) };
            out.push(format!("  {}{}", name, detail));
        }
    }
    clean_text(&out.join("\n"))
}

/// A door as an author checks it (`thwart show <door>`): its sides and
/// where each leads, whether it can be passed or seen through now, and
/// its actions.
pub fn render_door(d: &crate::models::world::Door) -> String {
    let mut out = vec![format!("{} ({})", d.title, d.id), if d.portal { "portal".to_string() } else { "door".to_string() }];
    out.push(String::new());
    let body = d.description.trim();
    out.push(if body.is_empty() { "(no description yet)".to_string() } else { body.to_string() });
    out.push(String::new());
    out.push("Sides:".to_string());
    let width = d.sides.iter().map(|s| s.room.len()).max().unwrap_or(0);
    for side in &d.sides {
        let to = d.leads_to(&side.room).map(|t| format!(" to {}", t)).unwrap_or_else(|| " (leads nowhere)".to_string());
        out.push(format!("  {:<width$}  {}{}", side.room, side.direction, to, width = width));
    }
    out.push(format!(
        "{}, {}",
        if d.passable { "Passable" } else { "Not passable" },
        if d.transparent { "see-through" } else { "opaque" }
    ));
    let mut names: Vec<&String> = d.actions.keys().collect();
    names.sort();
    if !names.is_empty() {
        out.push(format!("Actions: {}", names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(", ")));
    }
    clean_text(&out.join("\n"))
}

/// A character as an author checks it (`thwart show <character>`).
pub fn render_character(c: &crate::models::character::Character) -> String {
    let mut out = vec![format!("{} ({})", c.name, c.id), format!("character, {:?}", c.character_type).to_lowercase()];
    out.push(String::new());
    let body = c.description.trim();
    out.push(if body.is_empty() { "(no description yet)".to_string() } else { body.to_string() });
    out.push(String::new());
    out.push(format!("Starts in: {}", c.default_location.as_deref().unwrap_or("nowhere")));
    out.push(if c.inventory.is_empty() { "Carries: nothing".to_string() } else { format!("Carries: {}", c.inventory.join(", ")) });
    if !c.attributes.is_empty() {
        let mut attrs: Vec<String> = c.attributes.iter().map(|(k, v)| format!("{} {}", k, v)).collect();
        attrs.sort();
        out.push(format!("Attributes: {}", attrs.join(", ")));
    }
    if !c.relationships.is_empty() {
        let mut rels: Vec<String> = c.relationships.iter().map(|(k, v)| format!("{} ({})", k, v)).collect();
        rels.sort();
        out.push(format!("Relationships: {}", rels.join(", ")));
    }
    clean_text(&out.join("\n"))
}

/// Where a thing's version comes from, for `thwart show`: its states,
/// chapter versions and templates. Empty for a one-file thing with none.
pub fn render_thing_sources(thing: &crate::engines::bundles::Thing, shown_state: Option<&str>, shown_chapter: Option<&str>) -> String {
    let mut out = Vec::new();
    let names = thing.state_names();
    if !names.is_empty() {
        let states: Vec<String> = names
            .iter()
            .map(|s| {
                let mut tags = Vec::new();
                if thing.starting_state.as_deref() == Some(s.as_str()) {
                    tags.push("starting");
                }
                if shown_state == Some(s.as_str()) {
                    tags.push("shown");
                }
                if tags.is_empty() { s.clone() } else { format!("{} ({})", s, tags.join(", ")) }
            })
            .collect();
        out.push(format!("States: {}", states.join(", ")));
    }
    if !thing.overlays.is_empty() || !thing.area_overlays.is_empty() {
        let mut names: Vec<String> = thing.overlays.keys().map(|k| k.path.join("/")).collect();
        names.extend(thing.area_overlays.iter().map(|o| format!("{} (from {}/)", o.chapter.path.join("/"), o.area.display())));
        out.push(format!("Chapter versions: {}", names.join(", ")));
    }
    if let Some(c) = shown_chapter {
        out.push(format!("Shown in chapter: {}", c));
    }
    if !thing.templates.is_empty() {
        let names: Vec<String> = thing.templates.iter().rev().map(|t| t.path.display().to_string()).collect();
        out.push(format!("Templates: {}", names.join(", then ")));
    }
    clean_text(&out.join("\n"))
}

/// `author_blocks` as text: what `thwart show` prints.
pub fn render_location(view: &LocationView) -> String {
    author_blocks(view).cleaned().to_plain()
}

/// What the room's objects hold, each under its holder and indented one
/// step further, as `push_refs` lists the objects (REQ-CF-31). Lines go
/// right after their holder's.
fn push_held(blocks: &mut Vec<Block>, view: &LocationView) {
    if view.held.is_empty() {
        return;
    }
    fn line(r: &crate::engines::EntityRef, depth: usize) -> Block {
        let pad = "  ".repeat(depth + 1);
        let link = |missing: bool| Style { link: Some(Link { name: r.id.clone(), target: (!missing).then(|| (EntityKind::Object, r.id.clone())) }), missing, ..Default::default() };
        let spans = match &r.name {
            Some(name) => vec![Span::plain(format!("{}{} (", pad, name)), Span { text: r.id.clone(), style: link(false) }, Span::plain(")")],
            None => vec![Span::plain(pad), Span { text: r.id.clone(), style: link(true) }, Span { text: "  (no card)".into(), style: Style { missing: true, ..Default::default() } }],
        };
        Block::new(BlockKind::Here, spans, false)
    }
    fn under(view: &LocationView, holder: &str, depth: usize, out: &mut Vec<(String, Block)>, seen: &mut Vec<String>) {
        for h in view.held.iter().filter(|h| h.holder == holder) {
            if seen.contains(&h.thing.id) {
                continue;
            }
            seen.push(h.thing.id.clone());
            out.push((holder.to_string(), line(&h.thing, depth)));
            under(view, &h.thing.id, depth + 1, out, seen);
        }
    }
    // Each object's lines, then what doors hold under a heading of their own.
    let mut seen = Vec::new();
    let mut result: Vec<Block> = Vec::new();
    let objects: Vec<&crate::engines::EntityRef> = view.objects.iter().collect();
    let start = blocks.len() - objects.len();
    let (head, rows) = blocks.split_at(start);
    let mut rebuilt: Vec<Block> = head.to_vec();
    for (object, row) in objects.iter().zip(rows.iter()) {
        rebuilt.push(row.clone());
        let mut lines = Vec::new();
        under(view, &object.id, 1, &mut lines, &mut seen);
        rebuilt.extend(lines.into_iter().map(|(_, b)| b));
    }
    for door in view.exits.iter().filter_map(|e| e.door.as_ref()) {
        let mut lines = Vec::new();
        under(view, &door.id, 1, &mut lines, &mut seen);
        if !lines.is_empty() {
            result.push(Block::line(BlockKind::Here, format!("{} the {}:", capitalize(&view.held.iter().find(|h| h.holder == door.id).map_or("on".to_string(), |h| h.word.clone())), door.title.to_lowercase()), false));
            result.extend(lines.into_iter().map(|(_, b)| b));
        }
    }
    *blocks = rebuilt;
    blocks.extend(result);
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

fn push_refs(blocks: &mut Vec<Block>, heading: &str, refs: &[crate::engines::EntityRef], missing_label: &str, kind: EntityKind) {
    if refs.is_empty() {
        blocks.push(Block::line(BlockKind::Here, format!("{}: none", heading), true));
        return;
    }
    blocks.push(Block::line(BlockKind::Here, format!("{}:", heading), true));
    for r in refs {
        let link = |missing: bool| Style {
            link: Some(Link { name: r.id.clone(), target: (!missing).then(|| (kind, r.id.clone())) }),
            missing,
            ..Default::default()
        };
        let spans = match &r.name {
            Some(name) => vec![Span::plain(format!("  {} (", name)), Span { text: r.id.clone(), style: link(false) }, Span::plain(")")],
            None => vec![
                Span::plain("  "),
                Span { text: r.id.clone(), style: link(true) },
                Span { text: format!("  ({})", missing_label), style: Style { missing: true, ..Default::default() } },
            ],
        };
        blocks.push(Block::new(BlockKind::Here, spans, false));
    }
}

/// Prose as the player reads it (REQ-CF-28 to 30): Markdown's headings,
/// emphasis, lists, quotes and code become blocks and styles, never raw
/// syntax; `[[links]]` and `[text](file.md)` links become link spans
/// showing their text (an alias, else the thing's title, else the name
/// as written); `#tags` become tag spans without the `#`. An author note
/// (`<!-- -->`) on its own is an `AuthorNote` block, kept as written.
pub fn prose_blocks(text: &str, links: Resolver) -> Rendered {
    let mut blocks: Vec<Block> = Vec::new();
    let mut current: Option<Block> = None;
    let mut style = Style::default();
    let mut list_depth: u8 = 0;
    let mut in_quote = false;
    let mut item_open = false;

    let open = |blocks: &mut Vec<Block>, current: &mut Option<Block>, kind: BlockKind, space: bool| {
        if let Some(done) = current.take() {
            if !done.spans.is_empty() {
                blocks.push(done);
            }
        }
        *current = Some(Block::new(kind, Vec::new(), space));
    };
    let close = |blocks: &mut Vec<Block>, current: &mut Option<Block>| {
        if let Some(done) = current.take() {
            if !done.spans.is_empty() {
                blocks.push(done);
            }
        }
    };
    let push = |current: &mut Option<Block>, text: &str, style: &Style| {
        let block = current.get_or_insert_with(|| Block::new(BlockKind::Paragraph, Vec::new(), false));
        block.spans.push(Span { text: text.to_string(), style: style.clone() });
    };

    for event in Parser::new_ext(text, Options::empty()) {
        match event {
            Event::Start(Tag::Paragraph) => {
                if !item_open {
                    let kind = if in_quote { BlockKind::Quote } else { BlockKind::Paragraph };
                    open(&mut blocks, &mut current, kind, true);
                }
            }
            Event::End(Tag::Paragraph) => {
                if item_open {
                    push(&mut current, "\n", &Style::default());
                }
            }
            Event::Start(Tag::Heading(level, _, _)) => open(&mut blocks, &mut current, BlockKind::Heading(level as u8), true),
            Event::End(Tag::Heading(..)) => close(&mut blocks, &mut current),
            Event::Start(Tag::BlockQuote) => in_quote = true,
            Event::End(Tag::BlockQuote) => {
                in_quote = false;
                close(&mut blocks, &mut current);
            }
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(Tag::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                if list_depth == 0 {
                    close(&mut blocks, &mut current);
                }
            }
            Event::Start(Tag::Item) => {
                let first_in_list = !matches!(current.as_ref().map(|b| b.kind), Some(BlockKind::ListItem(_)));
                open(&mut blocks, &mut current, BlockKind::ListItem(list_depth), first_in_list);
                item_open = true;
            }
            Event::End(Tag::Item) => {
                item_open = false;
                if let Some(b) = current.as_mut() {
                    while b.spans.last().map_or(false, |s| s.text == "\n") {
                        b.spans.pop();
                    }
                }
            }
            Event::Start(Tag::CodeBlock(_)) => open(&mut blocks, &mut current, BlockKind::Code, true),
            Event::End(Tag::CodeBlock(_)) => {
                if let Some(b) = current.as_mut() {
                    if let Some(last) = b.spans.last_mut() {
                        last.text = last.text.trim_end_matches('\n').to_string();
                    }
                }
                close(&mut blocks, &mut current);
            }
            Event::Start(Tag::Strong) => style.strong = true,
            Event::End(Tag::Strong) => style.strong = false,
            Event::Start(Tag::Emphasis) => style.emphasis = true,
            Event::End(Tag::Emphasis) => style.emphasis = false,
            Event::Start(Tag::Link(_, dest, _)) => {
                let name = link_target(&dest);
                let target = links(&name).map(|(k, id, _)| (k, id));
                style.missing = target.is_none() && !dest.contains("://");
                style.link = Some(Link { name, target });
            }
            Event::End(Tag::Link(..)) => {
                style.link = None;
                style.missing = false;
            }
            Event::Text(t) => push(&mut current, &t, &style),
            Event::Code(t) => push(&mut current, &t, &Style { code: true, ..style.clone() }),
            Event::SoftBreak | Event::HardBreak => push(&mut current, "\n", &style),
            Event::Html(h) => {
                let note = h.trim_end_matches('\n');
                if current.as_ref().map_or(true, |b| b.spans.is_empty()) && note.trim_start().starts_with("<!--") {
                    open(&mut blocks, &mut current, BlockKind::AuthorNote, true);
                }
                push(&mut current, note, &style);
            }
            _ => {}
        }
    }
    if let Some(done) = current.take() {
        if !done.spans.is_empty() {
            blocks.push(done);
        }
    }
    for block in blocks.iter_mut() {
        block.spans = links_and_tags(std::mem::take(&mut block.spans), links, block.kind == BlockKind::Code);
    }
    if let Some(first) = blocks.first_mut() {
        first.space_before = false;
    }
    Rendered { blocks }.cleaned()
}

/// Prose as plain text, as play shows it.
pub fn prose_plain(text: &str, links: Resolver) -> String {
    prose_blocks(text, links).to_plain()
}

/// Join adjacent spans of the same style (the parser splits text at
/// brackets), then find `[[links]]` and `#tags` in plain text spans.
fn links_and_tags(spans: Vec<Span>, links: Resolver, code: bool) -> Vec<Span> {
    let mut joined: Vec<Span> = Vec::new();
    for span in spans {
        match joined.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            _ => joined.push(span),
        }
    }
    if code {
        return joined;
    }
    let mut out = Vec::new();
    for span in joined {
        if span.style.code || span.style.link.is_some() {
            out.push(span);
            continue;
        }
        let mut rest = span.text.as_str();
        while !rest.is_empty() {
            let link_at = rest.find("[[").filter(|&i| rest[i..].contains("]]"));
            let tag_at = tag_start(rest);
            match (link_at, tag_at) {
                (Some(l), t) if t.map_or(true, |t| l < t) => {
                    push_text(&mut out, &rest[..l], &span.style);
                    let end = l + rest[l..].find("]]").unwrap() + 2;
                    let inner = &rest[l + 2..end - 2];
                    let alias = inner.split_once('|').map(|(_, a)| a.trim().to_string());
                    let name = link_target(&format!("[[{}]]", inner));
                    let found = links(&name);
                    let text = alias.or_else(|| found.as_ref().map(|(_, _, t)| t.clone())).unwrap_or_else(|| name.clone());
                    let style = Style {
                        missing: found.is_none(),
                        link: Some(Link { name, target: found.map(|(k, id, _)| (k, id)) }),
                        ..span.style.clone()
                    };
                    out.push(Span { text, style });
                    rest = &rest[end..];
                }
                (_, Some(t)) => {
                    push_text(&mut out, &rest[..t], &span.style);
                    let len = rest[t + 1..].find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '/')).unwrap_or(rest.len() - t - 1);
                    out.push(Span { text: rest[t + 1..t + 1 + len].to_string(), style: Style { tag: true, ..span.style.clone() } });
                    rest = &rest[t + 1 + len..];
                }
                _ => {
                    push_text(&mut out, rest, &span.style);
                    rest = "";
                }
            }
        }
    }
    out
}

/// Where a `#tag` starts: a `#` at the start or after whitespace,
/// followed by a letter.
fn tag_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    (0..bytes.len()).find(|&i| {
        bytes[i] == b'#'
            && (i == 0 || (bytes[i - 1] as char).is_whitespace())
            && text[i + 1..].chars().next().map_or(false, char::is_alphabetic)
    })
}

fn push_text(out: &mut Vec<Span>, text: &str, style: &Style) {
    if !text.is_empty() {
        out.push(Span { text: text.to_string(), style: style.clone() });
    }
}

/// Every tag in prose (`#tag`, outside code), without the `#`.
pub fn prose_tags(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = prose_blocks(text, &no_links)
        .blocks
        .iter()
        .flat_map(|b| b.spans.iter().filter(|s| s.style.tag).map(|s| s.text.clone()))
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

/// Every link in prose, by name as written: `[[name]]` and `[text](name)`,
/// web links left out.
pub fn prose_links(text: &str) -> Vec<String> {
    prose_blocks(text, &no_links)
        .blocks
        .iter()
        .flat_map(|b| b.spans.iter().filter_map(|s| s.style.link.as_ref().filter(|_| s.style.missing).map(|l| l.name.clone())))
        .collect()
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
                ExitView { direction: "north".into(), target: "castle/great-hall".into(), target_exists: true, door: None },
                ExitView { direction: "down".into(), target: "castle/cellar".into(), target_exists: false, door: None },
            ],
            held: Vec::new(),
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

    /// The player view as it was before blocks, kept to prove `to_plain`
    /// prints the same bytes for prose with no Markdown in it.
    fn legacy_player(view: &LocationView) -> String {
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

    #[test]
    fn plain_text_is_unchanged_for_every_room_in_the_sample_worlds() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for world in ["tests/worlds/sample", "tests/transcripts/worlds/tiny"] {
            let mut engine = crate::engines::WorldEngine::new();
            engine.load_world(root.join(world)).unwrap();
            let mut ids: Vec<&String> = engine.loaded_locations.keys().collect();
            ids.sort();
            assert!(!ids.is_empty());
            for id in ids {
                let view = engine.describe_location(id).unwrap();
                assert_eq!(render_location_as(&view, RenderStyle::Player), legacy_player(&view), "{} in {}", id, world);
            }
        }
    }

    #[test]
    fn markdown_renders_as_blocks_and_styles_never_raw_syntax() {
        let links = |name: &str| match name {
            "lever" | "objects/lever" => Some((EntityKind::Object, "lever".to_string(), "Iron lever".to_string())),
            _ => None,
        };
        let text = "# The vault\n\nA **heavy** door and an *old* [[lever]], or [the lever](objects/lever.md), or [[gone|a ghost]].\nSee #secret and `code`.\n\n- one\n- two\n  - deeper\n\n> Said the guard.\n\n<!-- note to self -->";
        let r = prose_blocks(text, &links);
        let kinds: Vec<BlockKind> = r.blocks.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, [BlockKind::Heading(1), BlockKind::Paragraph, BlockKind::ListItem(1), BlockKind::ListItem(1), BlockKind::ListItem(2), BlockKind::Quote, BlockKind::AuthorNote]);
        let para = &r.blocks[1];
        assert!(para.spans.iter().any(|s| s.text == "heavy" && s.style.strong));
        assert!(para.spans.iter().any(|s| s.text == "old" && s.style.emphasis));
        let lever: Vec<&Span> = para.spans.iter().filter(|s| s.style.link.as_ref().map_or(false, |l| l.target.is_some())).collect();
        assert_eq!(lever.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["Iron lever", "the lever"]);
        assert!(para.spans.iter().any(|s| s.text == "a ghost" && s.style.missing));
        assert!(para.spans.iter().any(|s| s.text == "secret" && s.style.tag));
        assert!(para.spans.iter().any(|s| s.text == "code" && s.style.code));
        assert_eq!(
            r.to_plain(),
            "The vault\n\nA heavy door and an old Iron lever, or the lever, or a ghost.\nSee secret and code.\n\n- one\n- two\n  - deeper\n\n  Said the guard.\n\n<!-- note to self -->"
        );
        assert_eq!(prose_tags(text), ["secret"]);
        assert_eq!(prose_links(text), ["lever", "objects/lever", "gone"]);
        assert_eq!(prose_links("See [the site](https://example.com)."), Vec::<String>::new());
    }

    #[test]
    fn terminal_control_sequences_are_stripped_from_everything_shown() {
        let nasty = "Hall\u{1b}[2J\u{1b}[31m red\u{1b}[0m\u{1b}]0;owned\u{7} and\u{1b}]2;x\u{1b}\\ more\r\u{8}\u{9b}31m\tend\nnext";
        assert_eq!(clean_text(nasty), "Hall red and more31m\tend\nnext");
        let mut v = view();
        v.title = "Court\u{1b}[1;1Hyard".into();
        v.body = "A \u{1b}]0;pwned\u{7}fountain.".into();
        v.objects = vec![EntityRef { id: "x\u{1b}[5m".into(), name: Some("Bell\u{7}".into()) }];
        for text in [render_location(&v), render_location_as(&v, RenderStyle::Player)] {
            assert!(!text.chars().any(|c| c.is_control() && c != '\n'), "{:?}", text);
            assert!(text.contains("Courtyard") && text.contains("A fountain."), "{}", text);
        }
        let blocks = prose_blocks("**Bold\u{1b}[8m** [[x\u{1b}[1m]]", &no_links);
        assert!(blocks.blocks.iter().flat_map(|b| &b.spans).all(|s| !s.text.contains('\u{1b}')));
    }
}
