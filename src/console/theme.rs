//! Colour by role, from the terminal's own palette (REQ-CON-38): only the
//! default colours and the 16 named ANSI ones, so whatever theme the
//! terminal has, the console follows it. `NO_COLOR` drops the colours and
//! keeps bold, dim, italic, underline and reverse.
//!
//! Also the editor's highlighting (REQ-CON-39): `highlight` gives each
//! character of a file's lines its role.

use ratatui::style::{Color, Modifier, Style};
use std::sync::OnceLock;

/// What a piece of text is, which decides how it looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The status bar and the editor's heading bar.
    Bar,
    /// The rest of the console's frame: the sidebar's border, line numbers.
    Chrome,
    /// The `/` that starts a console command.
    Slash,
    /// A console command's name, after its `/`.
    Command,
    /// What the author typed, echoed in the output.
    Input,
    /// A room's or thing's title.
    Title,
    /// An exit, a link, an id that resolves, a declared `{{ value }}`.
    Link,
    /// Something not written yet, or not declared.
    Missing,
    Warning,
    Error,
    /// Author notes and other asides.
    Note,
    /// The console saying it did something: Saved, Reloaded, Restarted.
    Message,
    /// The sidebar's headings.
    Heading,
    PromptEdit,
    PromptPlay,
    /// Frontmatter keys.
    Key,
    /// A Markdown link in prose: cyan and underlined.
    Url,
    /// Struck-through text (`~~like this~~`).
    Struck,
    /// A Markdown list marker or quote mark.
    Marker,
    /// Quoted prose, after `>`.
    Quote,
    Tag,
    Code,
    Emphasis,
    Strong,
}

/// How `role` looks in this terminal.
pub fn style(role: Role) -> Style {
    let colour = |c: Color| if no_colour() { Style::default() } else { Style::default().fg(c) };
    match role {
        // Chrome is blue, so the console's frame never looks like its text.
        Role::Bar if no_colour() => Style::default().add_modifier(Modifier::REVERSED),
        Role::Bar => Style::default().bg(Color::Blue).fg(Color::White),
        Role::Chrome if no_colour() => Style::default().add_modifier(Modifier::DIM),
        Role::Chrome => colour(Color::Blue),
        Role::Slash => colour(Color::Yellow).add_modifier(Modifier::BOLD),
        Role::Input | Role::Title | Role::Strong | Role::Command | Role::Heading => {
            Style::default().add_modifier(Modifier::BOLD)
        }
        Role::Link => colour(Color::Cyan),
        Role::Missing | Role::Warning => colour(Color::Yellow),
        Role::Error => colour(Color::Red),
        Role::Note | Role::Code => Style::default().add_modifier(Modifier::DIM),
        Role::Message => colour(Color::Green),
        Role::PromptEdit => colour(Color::Green).add_modifier(Modifier::BOLD),
        Role::PromptPlay => colour(Color::Magenta).add_modifier(Modifier::BOLD),
        Role::Key => colour(Color::Blue),
        Role::Tag => colour(Color::Magenta),
        Role::Url => colour(Color::Cyan).add_modifier(Modifier::UNDERLINED),
        Role::Struck => Style::default().add_modifier(Modifier::CROSSED_OUT),
        Role::Marker if no_colour() => Style::default().add_modifier(Modifier::BOLD),
        Role::Marker => colour(Color::Blue).add_modifier(Modifier::BOLD),
        Role::Quote => Style::default().add_modifier(Modifier::ITALIC),
        Role::Emphasis => Style::default().add_modifier(Modifier::ITALIC),
    }
}

fn no_colour() -> bool {
    static NO_COLOUR: OnceLock<bool> = OnceLock::new();
    *NO_COLOUR.get_or_init(|| std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()))
}

/// The role of a whole line of output, from what it says. Rooms and
/// things get finer styling once the renderer gives styled spans
/// (REQ-CF-28).
pub fn line_role(line: &str) -> Option<Role> {
    const MESSAGES: &[&str] = &[
        "Saved", "Reloaded", "Restarted", "Created", "Loaded", "Set ", "Linked", "Removed the", "Moved", "Play mode.",
        "Edit mode,",
    ];
    const ERRORS: &[&str] =
        &["Not ", "Reload failed", "Can't ", "Unknown command", "The editor failed", "File watching failed"];
    if line.starts_with("> ") {
        Some(Role::Input)
    } else if line.starts_with("Author note:") || line.starts_with("Game:") {
        Some(Role::Note)
    } else if ERRORS.iter().any(|p| line.starts_with(p)) {
        Some(Role::Error)
    } else if line.starts_with("This adds") || line.contains("warning(s)") || line.contains("isn't written yet") {
        Some(Role::Warning)
    } else if line.contains("(missing)") || line.contains("(no card)") {
        Some(Role::Missing)
    } else if MESSAGES.iter().any(|p| line.starts_with(p)) || line.starts_with("Changed on disk") {
        Some(Role::Message)
    } else if is_arrival(line) {
        Some(Role::Title)
    } else {
        None
    }
}

/// `The Well (room)` or `The Elder (character), npcs/elder.md`: edit
/// mode arriving somewhere.
fn is_arrival(line: &str) -> bool {
    ["(room)", "(character)", "(object)", "(world settings)", "(state)", "(chapter)", "(template)"]
        .iter()
        .any(|kind| line.ends_with(kind) || line.contains(&format!("{}, ", kind)))
}

/// What the editor needs to know about the world to colour a file.
pub trait Lookup {
    /// Whether `id` names a room, character, object or door in the world.
    fn resolves(&self, id: &str) -> bool;
    /// Whether `{{ name }}` is declared where this file is.
    fn declared(&self, name: &str) -> bool;
}

/// `## When: ...` or `## Unless: ...`, a section heading (SECTIONS.md).
pub fn is_keyword_heading(line: &str) -> bool {
    line.starts_with("## When:") || line.starts_with("## Unless:")
}

/// Frontmatter keys whose values are ids, coloured by whether they resolve.
const ID_KEYS: &[&str] = &["connections", "characters", "objects", "inventory", "default_location", "start"];

/// Each line's characters with their roles, `None` for plain text.
pub fn highlight(lines: &[String], lookup: &dyn Lookup) -> Vec<Vec<Option<Role>>> {
    let mut out = Vec::with_capacity(lines.len());
    let mut in_frontmatter = false;
    let mut frontmatter_done = false;
    let mut in_code = false;
    let mut in_data = false;
    let mut was_data = false;
    let mut id_key: Option<String> = None;
    for (n, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut roles = vec![None; chars.len()];
        let fence = line.trim_end() == "---";
        if fence && (n == 0 || in_frontmatter) && !frontmatter_done {
            fill(&mut roles, 0, chars.len(), Role::Note);
            if in_frontmatter {
                frontmatter_done = true;
            }
            in_frontmatter = !in_frontmatter;
        } else if in_frontmatter {
            frontmatter_line(&chars, &mut roles, &mut id_key, lookup);
        } else if line.trim_start().starts_with("```") {
            // A ```thwarter block holds a section's data, read like
            // frontmatter (`design/SECTIONS.md`).
            in_data = !in_code && line.trim_start().trim_start_matches('`').trim() == "thwarter";
            in_code = !in_code;
            fill(&mut roles, 0, chars.len(), if in_data || (!in_code && was_data) { Role::Marker } else { Role::Code });
            was_data = in_data;
            id_key = None;
        } else if in_code && in_data {
            frontmatter_line(&chars, &mut roles, &mut id_key, lookup);
        } else if in_code {
            fill(&mut roles, 0, chars.len(), Role::Code);
        } else if is_keyword_heading(line) {
            // `## When: #locked`: a section heading, drawn as a rule.
            let colon = chars.iter().position(|c| *c == ':').unwrap_or(0);
            fill(&mut roles, 0, colon + 1, Role::Marker);
            prose_line(&chars[colon + 1..], &mut roles[colon + 1..], lookup);
        } else {
            prose_line(&chars, &mut roles, lookup);
        }
        out.push(roles);
    }
    out
}

fn fill(roles: &mut [Option<Role>], from: usize, to: usize, role: Role) {
    let len = roles.len();
    for r in &mut roles[from.min(len)..to.min(len)] {
        *r = Some(role);
    }
}

/// `key: value`, with ids in the value checked when the key holds ids.
fn frontmatter_line(chars: &[char], roles: &mut [Option<Role>], id_key: &mut Option<String>, lookup: &dyn Lookup) {
    let indent = chars.iter().take_while(|c| **c == ' ').count();
    let colon = chars.iter().position(|c| *c == ':');
    let (value_from, key) = match colon {
        Some(at) if !chars[indent..at].contains(&' ') && !chars[indent..at].is_empty() => {
            fill(roles, indent, at, Role::Key);
            let key: String = chars[indent..at].iter().collect();
            (at + 1, Some(key))
        }
        _ => (indent, None),
    };
    // A top-level key starts a new value; an indented one (`north:`) is
    // inside the last top-level key's map.
    if indent == 0 {
        *id_key = key.clone().filter(|k| ID_KEYS.contains(&k.as_str()));
    }
    if id_key.is_none() {
        return;
    }
    let mut at = value_from;
    while at < chars.len() {
        if chars[at].is_alphanumeric() {
            let start = at;
            while at < chars.len() && (chars[at].is_alphanumeric() || matches!(chars[at], '/' | '-' | '_' | '.')) {
                at += 1;
            }
            // In `connections: { north: village/well }`, `north` is a key.
            if chars.get(at) == Some(&':') {
                fill(roles, start, at, Role::Key);
                continue;
            }
            let id: String = chars[start..at].iter().collect();
            fill(roles, start, at, if lookup.resolves(&id) { Role::Link } else { Role::Missing });
        } else {
            at += 1;
        }
    }
}

/// Markdown prose and Thwarter's own marks within it.
fn prose_line(chars: &[char], roles: &mut [Option<Role>], lookup: &dyn Lookup) {
    let text: String = chars.iter().collect();
    if text.trim_start().starts_with("<!--") {
        fill(roles, 0, chars.len(), Role::Note);
        return;
    }
    // A heading: its marks step back, its words stand out.
    let hashes = chars.iter().take_while(|c| **c == '#').count();
    if hashes > 0 && chars.get(hashes) == Some(&' ') {
        fill(roles, 0, hashes, Role::Note);
        fill(roles, hashes, chars.len(), Role::Strong);
        return;
    }
    // A horizontal rule.
    let trimmed = text.trim();
    if trimmed.len() >= 3 && (trimmed.chars().all(|c| c == '*') || trimmed.chars().all(|c| c == '_')) {
        fill(roles, 0, chars.len(), Role::Note);
        return;
    }
    let indent = chars.iter().take_while(|c| **c == ' ').count();
    let mut at = indent;
    // A quote: the marks, then the quoted words in italics.
    if chars.get(at) == Some(&'>') {
        while chars.get(at) == Some(&'>') || chars.get(at) == Some(&' ') {
            if chars[at] == '>' {
                fill(roles, at, at + 1, Role::Marker);
            }
            at += 1;
        }
        fill(roles, at, chars.len(), Role::Quote);
    }
    // A list item's marker: `- `, `* `, `+ ` or `1. `.
    let digits = chars[at..].iter().take_while(|c| c.is_ascii_digit()).count();
    if matches!(chars.get(at), Some('-' | '*' | '+')) && chars.get(at + 1) == Some(&' ') {
        fill(roles, at, at + 1, Role::Marker);
        at += 2;
    } else if digits > 0 && matches!(chars.get(at + digits), Some('.' | ')')) && chars.get(at + digits + 1) == Some(&' ') {
        fill(roles, at, at + digits + 1, Role::Marker);
        at += digits + 2;
    }
    while at < chars.len() {
        let rest = &chars[at..];
        let starts = |s: &str| rest.iter().zip(s.chars()).filter(|(a, b)| **a == *b).count() == s.chars().count();
        if starts("{{") {
            if let Some(end) = find(chars, at + 2, "}}") {
                let name: String = chars[at + 2..end].iter().collect::<String>().trim().to_string();
                fill(roles, at, end + 2, if lookup.declared(&name) { Role::Link } else { Role::Missing });
                at = end + 2;
                continue;
            }
        }
        if starts("[[") {
            if let Some(end) = find(chars, at + 2, "]]") {
                fill(roles, at, end + 2, Role::Url);
                at = end + 2;
                continue;
            }
        }
        if chars[at] == '[' {
            if let Some(close) = find(chars, at + 1, "](") {
                if let Some(end) = find(chars, close + 2, ")") {
                    fill(roles, at, end + 1, Role::Url);
                    at = end + 1;
                    continue;
                }
            }
        }
        if starts("~~") {
            if let Some(end) = find(chars, at + 2, "~~") {
                fill(roles, at, end + 2, Role::Struck);
                at = end + 2;
                continue;
            }
        }
        if chars[at] == '`' {
            if let Some(end) = find(chars, at + 1, "`") {
                fill(roles, at, end + 1, Role::Code);
                at = end + 1;
                continue;
            }
        }
        if starts("**") {
            if let Some(end) = find(chars, at + 2, "**") {
                fill(roles, at, end + 2, Role::Strong);
                at = end + 2;
                continue;
            }
        }
        if matches!(chars[at], '*' | '_') && chars.get(at + 1).is_some_and(|c| !c.is_whitespace()) {
            let mark = chars[at].to_string();
            let word_start = at == 0 || !chars[at - 1].is_alphanumeric();
            if word_start {
                if let Some(end) = find(chars, at + 1, &mark) {
                    fill(roles, at, end + 1, Role::Emphasis);
                    at = end + 1;
                    continue;
                }
            }
        }
        // `@guard-captain`: a character, by id.
        if chars[at] == '@' && (at == 0 || chars[at - 1].is_whitespace()) && chars.get(at + 1).is_some_and(|c| c.is_alphanumeric()) {
            let start = at;
            at += 1;
            while at < chars.len() && (chars[at].is_alphanumeric() || matches!(chars[at], '-' | '_' | '.')) {
                at += 1;
            }
            while at > start + 1 && chars[at - 1] == '.' {
                at -= 1;
            }
            let id: String = chars[start + 1..at].iter().collect();
            let id = id.split('.').next().unwrap_or("").to_string();
            fill(roles, start, at, if lookup.resolves(&id) { Role::Link } else { Role::Missing });
            continue;
        }
        if chars[at] == '#' && (at == 0 || chars[at - 1].is_whitespace()) && chars.get(at + 1).is_some_and(|c| c.is_alphanumeric()) {
            let start = at;
            at += 1;
            while at < chars.len() && (chars[at].is_alphanumeric() || matches!(chars[at], '-' | '_' | '/')) {
                at += 1;
            }
            fill(roles, start, at, Role::Tag);
            continue;
        }
        at += 1;
    }
}

/// Where `needle` next starts in `chars`, from `from`.
fn find(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (from..chars.len()).find(|&i| chars[i..].starts_with(&needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tiny;
    impl Lookup for Tiny {
        fn resolves(&self, id: &str) -> bool {
            matches!(id, "village/well" | "elder" | "lantern")
        }
        fn declared(&self, name: &str) -> bool {
            name == "lantern.lit"
        }
    }

    /// The text of each run with a role, as `role:text`.
    fn runs(line: &str) -> Vec<String> {
        runs_in(&[line.to_string()], 0)
    }

    fn runs_in(lines: &[String], n: usize) -> Vec<String> {
        let roles = &highlight(lines, &Tiny)[n];
        let chars: Vec<char> = lines[n].chars().collect();
        let mut out: Vec<String> = Vec::new();
        let mut last: Option<Role> = None;
        for (c, role) in chars.iter().zip(roles) {
            match role {
                Some(r) if Some(*r) == last => out.last_mut().unwrap().push(*c),
                Some(r) => out.push(format!("{:?}:{}", r, c)),
                None => {}
            }
            last = *role;
        }
        out
    }

    #[test]
    fn frontmatter_keys_and_ids() {
        let lines: Vec<String> =
            ["---", "id: village/square", "connections:", "  north: village/well", "  east: village/inn", "characters: [elder]", "---", "Prose."]
                .iter()
                .map(|s| s.to_string())
                .collect();
        assert_eq!(runs_in(&lines, 0), vec!["Note:---"]);
        assert_eq!(runs_in(&lines, 1), vec!["Key:id"]);
        assert_eq!(runs_in(&lines, 3), vec!["Key:north", "Link:village/well"]);
        assert_eq!(runs_in(&lines, 4), vec!["Key:east", "Missing:village/inn"]);
        assert_eq!(runs_in(&lines, 5), vec!["Key:characters", "Link:elder"]);
        assert!(runs_in(&lines, 7).is_empty());
    }

    #[test]
    fn a_thwarter_block_reads_like_frontmatter() {
        let lines: Vec<String> = ["---", "id: d", "---", "## When: #open", "```thwarter", "passable: yes", "```", "Prose."]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(runs_in(&lines, 4), vec!["Marker:```thwarter"]);
        assert_eq!(runs_in(&lines, 5), vec!["Key:passable"]);
        assert_eq!(runs_in(&lines, 6), vec!["Marker:```"]);
        assert!(runs_in(&lines, 7).is_empty());
    }

    #[test]
    fn prose_marks() {
        assert_eq!(runs("## The old well"), vec!["Note:##", "Strong: The old well"]);
        assert_eq!(runs("- a *bright* lamp"), vec!["Marker:-", "Emphasis:*bright*"]);
        assert_eq!(runs("2. second"), vec!["Marker:2."]);
        assert_eq!(runs("> said the elder"), vec!["Marker:>", "Quote:said the elder"]);
        assert_eq!(runs("an ~~old~~ new well"), vec!["Struck:~~old~~"]);
        assert_eq!(runs("***"), vec!["Note:***"]);
        assert_eq!(runs("@elder waves; @nobody doesn't."), vec!["Link:@elder", "Missing:@nobody"]);
        assert_eq!(runs("## When: #locked"), vec!["Marker:## When:", "Tag:#locked"]);
        assert_eq!(runs("An *old* well"), vec!["Emphasis:*old*"]);
        assert_eq!(runs("very **dark** here"), vec!["Strong:**dark**"]);
        assert_eq!(runs("the [[elder]] sits #village"), vec!["Url:[[elder]]", "Tag:#village"]);
        assert_eq!(runs("lit: {{ lantern.lit }}, oil: {{ lantern.oil }}"), vec!["Link:{{ lantern.lit }}", "Missing:{{ lantern.oil }}"]);
        assert_eq!(runs("<!-- thwarter:action light -->"), vec!["Note:<!-- thwarter:action light -->"]);
        assert_eq!(runs("use `x` here"), vec!["Code:`x`"]);
        assert!(runs("snake_case_word and 3*4").is_empty());
    }

    #[test]
    fn output_lines_by_what_they_say() {
        assert_eq!(line_role("> look"), Some(Role::Input));
        assert_eq!(line_role("Saved village/well.md."), Some(Role::Message));
        assert_eq!(line_role("Not saved: bad YAML"), Some(Role::Error));
        assert_eq!(line_role("Author note: the inn isn't written"), Some(Role::Note));
        assert_eq!(line_role("The Well (room)"), Some(Role::Title));
        assert_eq!(line_role("The Elder (character), npcs/elder.md"), Some(Role::Title));
        assert_eq!(line_role("A mossy stone well."), None);
    }
}
