//! The world picker (REQ-CON-62): `thwart edit` or `thwart play` with no
//! world given offers the worlds the engine finds, like the `/` menu.
//! Edit lists the worlds you can write and, with none, offers to make one
//! under `authors/<you>/`; play lists every world, installed ones too.

use super::screen::{enter_terminal, input_of, leave_terminal};
use super::theme::{style, Role};
use super::Mode;
use crate::engines::texts::Texts;
use crate::worlds::{FoundWorld, WorldNotGiven};
use anyhow::Result;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui_textarea::{Input, Key};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Opens the console on `resolved`, or, when no world was given, lets the
/// person pick one on a terminal. Without a terminal the error comes back
/// for the caller to print, as the other commands do.
pub fn run_or_pick(resolved: std::result::Result<PathBuf, WorldNotGiven>, mode: Mode) -> Result<()> {
    let not_given = match resolved {
        Ok(path) => return super::run(&path, mode),
        Err(e) => e,
    };
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        return Err(not_given.into());
    }
    let (texts, _) = super::session::console_texts();
    let mut pick = Pick::new(&not_given, mode, crate::settings::current_user());
    if pick.rows.is_empty() {
        // Play with nothing to play: say how to get a world, and stop.
        println!("{}", texts.text("console", "pick_none_to_play", &[]));
        return Err(super::super::cli::AlreadyReported.into());
    }
    match pick_on_terminal(&mut pick, &texts)? {
        Outcome::Open(path) => super::run(&path, mode),
        Outcome::Make(dir, title) => {
            let path = make_world(&dir, &title, &super::world_index::machine_user())?;
            super::run(&path, mode)
        }
        Outcome::Quit | Outcome::Stay => Ok(()),
    }
}

/// One row of the picker.
#[derive(Debug, Clone, PartialEq)]
enum Row {
    World(FoundWorld),
    /// Make a new world in this folder (`authors/<you>/`).
    Make(PathBuf),
}

/// What a key did.
#[derive(Debug, Clone, PartialEq)]
enum Outcome {
    Stay,
    Open(PathBuf),
    /// Make a world titled `.1` in a new folder under `.0`.
    Make(PathBuf, String),
    Quit,
}

/// The picker's state, apart from the terminal, so tests can drive it.
#[derive(Debug)]
struct Pick {
    rows: Vec<Row>,
    selected: usize,
    /// The new world's name while it's being typed.
    naming: Option<String>,
    from: PathBuf,
}

impl Pick {
    fn new(not_given: &WorldNotGiven, mode: Mode, user: Option<String>) -> Self {
        let mut rows: Vec<Row> = not_given
            .found
            .iter()
            .filter(|w| mode == Mode::Play || w.writable)
            .cloned()
            .map(Row::World)
            .collect();
        if rows.is_empty() && mode == Mode::Edit {
            let user = user.unwrap_or_else(|| "author".to_string());
            rows.push(Row::Make(authors_dir(&not_given.from).join(user)));
        }
        Pick { rows, selected: 0, naming: None, from: not_given.from.clone() }
    }

    fn key(&mut self, input: Input) -> Outcome {
        if let Some(name) = self.naming.as_mut() {
            match input.key {
                Key::Esc => self.naming = None,
                Key::Backspace => {
                    name.pop();
                }
                Key::Enter if !name.trim().is_empty() => {
                    if let Some(Row::Make(dir)) = self.rows.get(self.selected) {
                        return Outcome::Make(dir.clone(), name.trim().to_string());
                    }
                }
                Key::Char(c) if !input.ctrl && !input.alt => name.push(c),
                _ => {}
            }
            return Outcome::Stay;
        }
        match input.key {
            Key::Esc => Outcome::Quit,
            Key::Char('c') | Key::Char('d') if input.ctrl => Outcome::Quit,
            Key::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(self.rows.len() - 1);
                Outcome::Stay
            }
            Key::Down => {
                self.selected = (self.selected + 1) % self.rows.len();
                Outcome::Stay
            }
            Key::Enter => match &self.rows[self.selected] {
                Row::World(world) => Outcome::Open(world.root.clone()),
                Row::Make(_) => {
                    self.naming = Some(String::new());
                    Outcome::Stay
                }
            },
            _ => Outcome::Stay,
        }
    }

    /// The picker as lines of text, with the row that's chosen.
    fn lines(&self, texts: &Texts) -> Vec<(String, bool, Option<Role>)> {
        let t = |key: &str, args: &[(&str, &str)]| texts.text("console", key, args);
        let mut out = vec![(t("pick_heading", &[]), false, Some(Role::Title))];
        let width = self
            .rows
            .iter()
            .map(|r| match r {
                Row::World(w) => w.title.chars().count(),
                Row::Make(_) => 0,
            })
            .max()
            .unwrap_or(0);
        for (i, row) in self.rows.iter().enumerate() {
            let chosen = i == self.selected;
            let mark = if chosen { ">" } else { " " };
            let text = match row {
                Row::World(w) => {
                    let installed = if w.installed { format!("  {}", t("pick_installed", &[])) } else { String::new() };
                    format!("{} {:<width$}  {}{}", mark, w.title, shown(&w.root, &self.from), installed, width = width)
                }
                Row::Make(dir) => format!("{} {}", mark, t("pick_make", &[("folder", &shown(dir, &self.from))])),
            };
            out.push((text, chosen, None));
        }
        match &self.naming {
            Some(name) => {
                out.push((t("pick_name", &[("name", name)]), false, None));
                out.push((t("pick_naming_keys", &[]), false, Some(Role::Chrome)));
            }
            None => out.push((t("pick_keys", &[]), false, Some(Role::Chrome))),
        }
        out
    }
}

/// Runs the picker on the terminal until a world is chosen or Esc.
fn pick_on_terminal(pick: &mut Pick, texts: &Texts) -> Result<Outcome> {
    use ratatui::crossterm::event::{self, Event, KeyEventKind};
    let mut terminal = enter_terminal()?;
    let result = (|| -> Result<Outcome> {
        loop {
            terminal.draw(|frame| {
                let lines: Vec<Line> = pick
                    .lines(texts)
                    .into_iter()
                    .map(|(text, chosen, role)| {
                        let line = Line::from(Span::raw(format!(" {}", text)));
                        let line = match role {
                            Some(role) => line.style(style(role)),
                            None => line,
                        };
                        if chosen {
                            line.style(Style::default().add_modifier(Modifier::REVERSED))
                        } else {
                            line
                        }
                    })
                    .collect();
                frame.render_widget(Paragraph::new(lines), frame.area());
            })?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                match pick.key(input_of(key)) {
                    Outcome::Stay => {}
                    done => return Ok(done),
                }
            }
        }
    })();
    leave_terminal();
    result
}

/// `path` as the person would type it from `from`: relative when it's
/// under `from`, with `~` for the home folder otherwise.
fn shown(path: &Path, from: &Path) -> String {
    if let Ok(rest) = path.strip_prefix(from) {
        return rest.display().to_string();
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        if let Ok(rest) = path.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

/// The `authors/` folder new worlds go in: the nearest one from `from`
/// up, else a new one in `from`.
fn authors_dir(from: &Path) -> PathBuf {
    from.ancestors().map(|d| d.join("authors")).find(|d| d.is_dir()).unwrap_or_else(|| from.join("authors"))
}

/// Makes a world titled `title` in a new folder under `dir`, with a
/// `world/index.md` naming `user` as its author, and returns its folder.
fn make_world(dir: &Path, title: &str, user: &str) -> Result<PathBuf> {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.is_empty() { "world".to_string() } else { slug };
    let root = dir.join(&slug);
    if root.exists() {
        anyhow::bail!("{} already exists", root.display());
    }
    std::fs::create_dir_all(root.join("world"))?;
    let title = crate::writer::content_file::emit_scalar(&serde_yaml::Value::String(title.to_string()), false)
        .unwrap_or_else(|_| title.to_string());
    std::fs::write(root.join("world/index.md"), format!("---\ntitle: {}\ntype: world\nauthors: {}\n---\n", title, user))?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(root: &str, title: &str, installed: bool, writable: bool) -> FoundWorld {
        FoundWorld { root: PathBuf::from(root), title: title.into(), installed, writable, authors: vec![] }
    }

    fn not_given() -> WorldNotGiven {
        WorldNotGiven {
            found: vec![
                world("/home/a/authors/mx/tiny", "Tiny World", false, true),
                world("/home/a/authors/kim/light", "The Lighthouse", false, false),
                world("/data/worlds/castle", "Castle Adventure", true, false),
            ],
            from: PathBuf::from("/home/a"),
        }
    }

    fn key(k: Key) -> Input {
        Input { key: k, ..Input::default() }
    }

    #[test]
    fn edit_lists_only_what_you_can_write_and_play_lists_everything() {
        let edit = Pick::new(&not_given(), Mode::Edit, Some("mx".into()));
        assert_eq!(edit.rows.len(), 1);
        let play = Pick::new(&not_given(), Mode::Play, Some("mx".into()));
        assert_eq!(play.rows.len(), 3);
        let lines: Vec<String> = play.lines(&Texts::defaults()).into_iter().map(|l| l.0).collect();
        assert_eq!(lines[0], "Which world?");
        assert_eq!(lines[1], "> Tiny World        authors/mx/tiny");
        assert_eq!(lines[3], "  Castle Adventure  /data/worlds/castle  installed");
        assert_eq!(lines[4], "↑↓ choose · enter opens · esc quits");
    }

    #[test]
    fn arrows_choose_enter_opens_and_esc_quits() {
        let mut pick = Pick::new(&not_given(), Mode::Play, None);
        assert_eq!(pick.key(key(Key::Up)), Outcome::Stay);
        assert_eq!(pick.selected, 2, "up from the top wraps to the bottom");
        pick.key(key(Key::Down));
        pick.key(key(Key::Down));
        assert_eq!(pick.key(key(Key::Enter)), Outcome::Open(PathBuf::from("/home/a/authors/kim/light")));
        assert_eq!(pick.key(key(Key::Esc)), Outcome::Quit);
    }

    #[test]
    fn with_nothing_to_write_edit_offers_to_make_a_world() {
        let none = WorldNotGiven { found: vec![world("/data/worlds/castle", "Castle", true, false)], from: PathBuf::from("/nowhere") };
        let mut pick = Pick::new(&none, Mode::Edit, Some("mx".into()));
        assert_eq!(pick.rows, vec![Row::Make(PathBuf::from("/nowhere/authors/mx"))]);
        pick.key(key(Key::Enter));
        for c in "My Tale".chars() {
            pick.key(key(Key::Char(c)));
        }
        assert!(pick.lines(&Texts::defaults()).iter().any(|l| l.0 == "Name the new world: My Tale"));
        assert_eq!(pick.key(key(Key::Enter)), Outcome::Make(PathBuf::from("/nowhere/authors/mx"), "My Tale".into()));
        assert!(Pick::new(&none, Mode::Play, None).rows.len() == 1, "play still lists installed worlds");
    }

    #[test]
    fn a_made_world_is_a_world_its_maker_can_write() {
        let dir = tempfile::tempdir().unwrap();
        let root = make_world(&dir.path().join("authors/mx"), "My Tale: Part 1", "mx").unwrap();
        assert!(root.ends_with("authors/mx/my-tale-part-1"));
        assert!(crate::worlds::is_world(&root));
        let found = crate::worlds::find_worlds(dir.path());
        assert!(found.iter().any(|w| w.root == root && w.title == "My Tale: Part 1"), "{:?}", found);
        assert!(make_world(&dir.path().join("authors/mx"), "My Tale: Part 1", "mx").is_err(), "never over an existing folder");
    }
}
