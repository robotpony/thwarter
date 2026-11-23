//! The full-screen console on a terminal (REQ-CON-22): a status bar on
//! top, the output on the left, the sidebar on the right (GAME, and HERE
//! in edit mode), and the input line across the bottom. The full-screen
//! editor opens over the output and keeps the sidebar.
//!
//! What it shows comes from `Console` (`panels.rs` for the sidebar and
//! status bar); this module only lays it out and handles keys. Scripts and
//! transcripts don't come here: they stay line-based (REQ-CON-9).

use super::editor::Editor;
use super::full_edit::{EditFiles, EditTarget};
use super::panels::{folded, Row};
use super::restart::{self, Resumed};
use super::session::{Console, Flow};
use super::lock;
use super::theme::{line_role, style, Role};
use anyhow::Result;
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use ratatui_textarea::{Input, Key, TextArea};
use std::cell::Cell;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Lines of output kept, and carried over a `/restart`.
const SCROLLBACK: usize = 5000;
/// Lines of input history kept, and carried over a `/restart`.
const HISTORY: usize = 500;
/// The narrowest terminal that gets a sidebar; narrower, it folds into one
/// line above the input (REQ-CON-23, proposed).
const SIDEBAR_FROM: u16 = 100;
/// How often the open editor looks at its file, and the console at its
/// own binary.
const CHECK_EVERY: Duration = Duration::from_millis(500);

pub struct Screen<'a> {
    /// What the console has printed, one logical line each.
    output: Vec<String>,
    /// How far the output is scrolled up from the bottom, in screen rows.
    scroll: usize,
    input: TextArea<'a>,
    history: Vec<String>,
    /// Where Up has got to in `history`, and the line being typed before.
    browsing: Option<(usize, String)>,
    editing: Option<(Editor<'a>, EditTarget)>,
    sidebar_scroll: usize,
    /// The output pane's height at the last draw, for Page Up and Down.
    page: Cell<usize>,
    /// This binary's modification time when it started, to notice a newer build.
    built: Option<SystemTime>,
    newer_build: bool,
    last_check: Instant,
    quit: bool,
    restart: bool,
}

impl<'a> Screen<'a> {
    pub fn new(greeting: &str, resumed: Option<Resumed>) -> Self {
        let mut screen = Screen {
            output: Vec::new(),
            scroll: 0,
            input: input_line(),
            history: Vec::new(),
            browsing: None,
            editing: None,
            sidebar_scroll: 0,
            page: Cell::new(20),
            built: binary_built(),
            newer_build: false,
            last_check: Instant::now(),
            quit: false,
            restart: false,
        };
        match resumed {
            Some(resumed) => {
                screen.output = resumed.scrollback;
                screen.history = resumed.history;
                screen.push(&resumed.greeting);
            }
            None => screen.push(greeting),
        }
        screen
    }

    /// Adds `text` to the output and scrolls to the bottom.
    pub fn push(&mut self, text: &str) {
        self.output.extend(text.lines().map(str::to_string));
        if self.output.len() > SCROLLBACK {
            let extra = self.output.len() - SCROLLBACK;
            self.output.drain(..extra);
        }
        self.scroll = 0;
    }

    /// One key, with the console to act on.
    pub fn key(&mut self, input: Input, console: &Mutex<Console>) {
        if let Some((editor, target)) = &mut self.editing {
            let mut files = EditFiles { console, target };
            if !editor.key(input, &mut files) {
                let (editor, target) = self.editing.take().expect("the editor is open");
                let report = lock(console).edit_closed(&target, editor.closed());
                self.push(&report);
            }
            return;
        }
        match input {
            Input { key: Key::Enter, .. } => self.submit(console),
            Input { key: Key::Up, alt: false, .. } => self.browse(-1),
            Input { key: Key::Down, alt: false, .. } => self.browse(1),
            Input { key: Key::Up, alt: true, .. } => self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1),
            Input { key: Key::Down, alt: true, .. } => self.sidebar_scroll += 1,
            Input { key: Key::PageUp, .. } => self.scroll += self.page.get().saturating_sub(2).max(1),
            Input { key: Key::PageDown, .. } => self.scroll = self.scroll.saturating_sub(self.page.get().saturating_sub(2).max(1)),
            Input { key: Key::Char('c'), ctrl: true, .. } => self.set_input(""),
            Input { key: Key::Char('d'), ctrl: true, .. } if self.input_text().is_empty() => self.quit = true,
            input => {
                self.browsing = None;
                self.input.input(input);
            }
        }
    }

    /// Between keys: watcher notices, the open editor's file, a newer build.
    pub fn tick(&mut self, console: &Mutex<Console>, notices: &Receiver<String>) {
        while let Ok(notice) = notices.try_recv() {
            self.output.push(String::new());
            self.push(&notice);
        }
        if self.last_check.elapsed() < CHECK_EVERY {
            return;
        }
        self.last_check = Instant::now();
        if let Some((editor, target)) = &mut self.editing {
            editor.check_disk(&mut EditFiles { console, target });
        }
        self.newer_build = matches!((self.built, binary_built()), (Some(then), Some(now)) if now > then);
    }

    fn submit(&mut self, console: &Mutex<Console>) {
        let line = self.input_text();
        self.set_input("");
        self.browsing = None;
        self.push(&format!("> {}", line));
        if !line.trim().is_empty() {
            self.history.push(line.clone());
            if self.history.len() > HISTORY {
                self.history.remove(0);
            }
        }
        let (text, flow) = lock(console).handle(&line);
        if !text.is_empty() {
            self.push(&text);
        }
        match flow {
            Flow::Quit => self.quit = true,
            Flow::Restart => self.restart = true,
            Flow::Clear => {
                self.output.clear();
                self.scroll = 0;
            }
            Flow::Continue => {}
        }
        let mut console = lock(console);
        if let Some(target) = console.take_pending_edit() {
            let document = console.edit_document(&target);
            self.editing = Some((Editor::new(document), target));
        }
    }

    /// Up and Down through the input history.
    fn browse(&mut self, step: isize) {
        if self.history.is_empty() {
            return;
        }
        let (at, draft) = match self.browsing.take() {
            Some((at, draft)) => (at as isize + step, draft),
            None if step < 0 => (self.history.len() as isize - 1, self.input_text()),
            None => return,
        };
        if at < 0 {
            self.browsing = Some((0, draft));
            return;
        }
        if at as usize >= self.history.len() {
            self.set_input(&draft);
            return;
        }
        let line = self.history[at as usize].clone();
        self.set_input(&line);
        self.browsing = Some((at as usize, draft));
    }

    fn input_text(&self) -> String {
        self.input.lines().join(" ")
    }

    fn set_input(&mut self, text: &str) {
        self.input = input_line();
        self.input.insert_str(text);
    }

    pub fn draw(&self, frame: &mut Frame, console: &Console) {
        let area = frame.area();
        let wide = area.width >= SIDEBAR_FROM;
        let sidebar = console.sidebar();
        let rows = if wide {
            vec![Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)]
        } else {
            vec![Constraint::Length(1), Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)]
        };
        let parts = Layout::vertical(rows).split(area);
        let (status, body, input) = (parts[0], parts[1], parts[parts.len() - 1]);

        let bar = console.status(self.newer_build);
        let gap = (status.width as usize).saturating_sub(bar.left.chars().count() + bar.right.chars().count() + 2).max(1);
        frame.render_widget(
            Paragraph::new(format!(" {}{}{} ", bar.left, " ".repeat(gap), bar.right)).style(style(Role::Bar)),
            status,
        );

        let output_area = if wide {
            let width = (body.width / 3).max(32);
            let [left, right] = Layout::horizontal([Constraint::Min(20), Constraint::Length(width)]).areas(body);
            self.draw_sidebar(frame, right, &sidebar.fixed, &sidebar.rest);
            left
        } else {
            frame.render_widget(Paragraph::new(folded(&sidebar)).style(style(Role::Note)), parts[2]);
            body
        };
        match &self.editing {
            Some((editor, target)) => editor.draw(frame, output_area, &console.lookup_for(target)),
            None => self.draw_output(frame, output_area),
        }

        let prompt = console.prompt();
        let [left, right] =
            Layout::horizontal([Constraint::Length(prompt.chars().count() as u16 + 1), Constraint::Min(1)]).areas(input);
        let role = if console.mode == super::Mode::Play { Role::PromptPlay } else { Role::PromptEdit };
        frame.render_widget(Paragraph::new(format!(" {}", prompt)).style(style(role)), left);
        if self.editing.is_none() {
            frame.render_widget(&self.input, right);
        }
    }

    fn draw_output(&self, frame: &mut Frame, area: Rect) {
        let width = area.width.saturating_sub(1).max(1) as usize;
        let rows: Vec<(String, Style)> =
            self.output.iter().flat_map(|line| wrap(line, width).into_iter().map(move |row| (row, style_of(line)))).collect();
        let height = area.height as usize;
        self.page.set(height);
        let bottom = rows.len().saturating_sub(self.scroll.min(rows.len().saturating_sub(height)));
        let top = bottom.saturating_sub(height);
        let lines: Vec<Line> = rows[top..bottom].iter().map(|(text, style)| Line::styled(format!(" {}", text), *style)).collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_sidebar(&self, frame: &mut Frame, area: Rect, fixed: &[Row], rest: &[Row]) {
        let block = Block::default().borders(Borders::LEFT);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        // Long rows wrap within the sidebar, keeping their indent.
        let width = inner.width.saturating_sub(1).max(1) as usize;
        let wrapped = |rows: &[Row]| -> Vec<Row> { rows.iter().flat_map(|row| wrap_row(row, width)).collect() };
        let (fixed, rest) = (wrapped(fixed), wrapped(rest));
        let fixed_height = (fixed.len() as u16).min(inner.height);
        let [top, bottom] = Layout::vertical([Constraint::Length(fixed_height), Constraint::Min(0)]).areas(inner);
        frame.render_widget(Paragraph::new(fixed.iter().map(row_line).collect::<Vec<_>>()), top);

        let height = bottom.height as usize;
        let start = self.sidebar_scroll.min(rest.len().saturating_sub(height));
        let mut lines: Vec<Line> = rest.iter().skip(start).take(height).map(row_line).collect();
        if rest.len() > start + height && height > 0 {
            lines.pop();
            lines.push(Line::styled(" ↓ more (alt-↓)", style(Role::Note)));
        }
        frame.render_widget(Paragraph::new(lines), bottom);
    }
}

/// Runs the full-screen console until `/quit` or Ctrl-D.
pub fn run(console: Arc<Mutex<Console>>, notices: Receiver<String>, resumed: Option<Resumed>) -> Result<()> {
    use ratatui::crossterm::event::{self, Event, KeyEventKind};
    let greeting = lock(&console).greeting();
    let mut screen = Screen::new(&greeting, resumed);
    let mut terminal = ratatui::try_init()?;
    // After a /restart, the screen still shows the last build's frame.
    clear_screen()?;
    let result = (|| -> Result<()> {
        loop {
            draw(&mut terminal, &screen, &console)?;
            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Release {
                        screen.key(Input::from(key), &console);
                    }
                }
            }
            screen.tick(&console, &notices);
            if screen.quit {
                return Ok(());
            }
            if screen.restart {
                screen.restart = false;
                // The new build takes the terminal over from here; if it
                // can't, this one takes it back.
                ratatui::restore();
                let why = restart::restart(&lock(&console), &screen.history, &screen.output);
                terminal = ratatui::try_init()?;
                clear_screen()?;
                screen.push(&why);
            }
        }
    })();
    ratatui::restore();
    result
}

fn draw<B: Backend>(terminal: &mut Terminal<B>, screen: &Screen, console: &Mutex<Console>) -> Result<()>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let console = lock(console);
    terminal.draw(|frame| screen.draw(frame, &console))?;
    Ok(())
}

/// Clears the screen without asking the terminal where its cursor is, as
/// `Terminal::clear` does; a terminal that doesn't answer would stop the
/// console starting. A new `Terminal`'s buffer is blank already.
fn clear_screen() -> std::io::Result<()> {
    use ratatui::crossterm::{execute, terminal::{Clear, ClearType}};
    execute!(std::io::stdout(), Clear(ClearType::All))
}

fn input_line<'a>() -> TextArea<'a> {
    let mut area = TextArea::default();
    area.set_cursor_line_style(Style::default());
    area
}

/// A panel row as one or more rows of at most `width` characters.
fn wrap_row(row: &Row, width: usize) -> Vec<Row> {
    let (text, make): (&str, fn(String) -> Row) = match row {
        Row::Heading(t) => (t, Row::Heading),
        Row::Text(t) => (t, Row::Text),
        Row::Dim(t) => (t, Row::Dim),
        Row::Missing(t) => (t, Row::Missing),
        Row::Warning(t) => (t, Row::Warning),
        Row::Blank => return vec![Row::Blank],
    };
    let indent = text.chars().take_while(|c| *c == ' ').count();
    wrap(text, width)
        .into_iter()
        .enumerate()
        .map(|(i, line)| make(if i == 0 { line } else { format!("{}  {}", " ".repeat(indent), line.trim_start()) }))
        .collect()
}

fn row_line(row: &Row) -> Line<'static> {
    let styled = |text: &str, role: Role| Line::from(Span::styled(format!(" {}", text), style(role)));
    match row {
        Row::Heading(text) => styled(text, Role::Heading),
        Row::Text(text) => Line::from(format!(" {}", text)),
        Row::Dim(text) => styled(text, Role::Note),
        Row::Missing(text) => styled(text, Role::Missing),
        Row::Warning(text) => styled(text, Role::Warning),
        Row::Blank => Line::from(""),
    }
}

/// How a line of output is drawn, from what it says (REQ-CON-38).
fn style_of(line: &str) -> Style {
    line_role(line).map(style).unwrap_or_default()
}

/// `line` broken into rows of at most `width` characters, at spaces where
/// it can be, keeping its indent on every row.
fn wrap(line: &str, width: usize) -> Vec<String> {
    if line.chars().count() <= width {
        return vec![line.to_string()];
    }
    let indent: String = line.chars().take_while(|c| *c == ' ').collect();
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in line.trim_start().split(' ') {
        let len = row.chars().count();
        let add = word.chars().count() + usize::from(len > indent.len());
        if len > indent.len() && len + add > width {
            rows.push(std::mem::take(&mut row));
        }
        if row.is_empty() {
            row.push_str(&indent);
        } else if row.chars().count() > indent.len() {
            row.push(' ');
        }
        row.push_str(word);
        while row.chars().count() > width {
            let head: String = row.chars().take(width).collect();
            let tail: String = row.chars().skip(width).collect();
            rows.push(head);
            row = format!("{}{}", indent, tail);
        }
    }
    if !row.trim().is_empty() {
        rows.push(row);
    }
    rows
}

/// When this binary was last built, to notice a newer one.
fn binary_built() -> Option<SystemTime> {
    std::env::current_exe().ok().and_then(|exe| std::fs::metadata(exe).ok()).and_then(|m| m.modified().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use std::path::Path;

    fn tiny() -> Mutex<Console> {
        let mut console =
            Console::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/transcripts/worlds/tiny"), super::super::Mode::Edit)
                .unwrap();
        console.interactive = true;
        Mutex::new(console)
    }

    fn typed(screen: &mut Screen, console: &Mutex<Console>, line: &str) {
        for c in line.chars() {
            screen.key(Input { key: Key::Char(c), ..Input::default() }, console);
        }
        screen.key(Input { key: Key::Enter, ..Input::default() }, console);
    }

    fn screen_text(screen: &Screen, console: &Mutex<Console>, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        draw(&mut terminal, screen, console).unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_layout_has_status_output_sidebar_and_input() {
        let console = tiny();
        let mut screen = Screen::new("Editing Tiny World.", None);
        typed(&mut screen, &console, "go /village/square");
        let text = screen_text(&screen, &console, 110, 24);
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].contains("Tiny World · edit · 1 warning"), "{}", text);
        assert!(lines[0].contains("thwart "), "{}", text);
        assert!(text.contains("> go /village/square"), "{}", text);
        assert!(text.contains("│ GAME"), "{}", text);
        assert!(text.contains("HERE  village/square"), "{}", text);
        assert!(lines[23].contains("edit village/square>"), "{}", text);
    }

    #[test]
    fn a_narrow_terminal_folds_the_sidebar_into_one_line() {
        let console = tiny();
        let screen = Screen::new("Editing Tiny World.", None);
        let text = screen_text(&screen, &console, 80, 12);
        assert!(!text.contains("│ GAME"), "{}", text);
        assert!(text.lines().nth(10).unwrap().starts_with("GAME  no game yet"), "{}", text);
    }

    #[test]
    fn up_recalls_what_was_typed() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "look");
        typed(&mut screen, &console, "go village");
        screen.key(Input { key: Key::Up, ..Input::default() }, &console);
        assert_eq!(screen.input_text(), "go village");
        screen.key(Input { key: Key::Up, ..Input::default() }, &console);
        assert_eq!(screen.input_text(), "look");
        screen.key(Input { key: Key::Down, ..Input::default() }, &console);
        screen.key(Input { key: Key::Down, ..Input::default() }, &console);
        assert_eq!(screen.input_text(), "");
    }

    #[test]
    fn edit_opens_the_editor_over_the_output_and_keeps_the_sidebar() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "go /village/square");
        typed(&mut screen, &console, "/edit room");
        let text = screen_text(&screen, &console, 110, 24);
        assert!(text.contains("/edit room  village/square.md"), "{}", text);
        assert!(text.contains("id: village/square"), "{}", text);
        assert!(text.contains("HERE  village/square"), "{}", text);
        screen.key(Input { key: Key::Esc, ..Input::default() }, &console);
        assert!(screen.editing.is_none());
        assert!(screen.output.last().unwrap().starts_with("Closed village/square.md"));
    }

    #[test]
    fn the_editor_colours_ids_by_whether_they_resolve() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "go /village/square");
        typed(&mut screen, &console, "/edit");
        let mut terminal = Terminal::new(TestBackend::new(110, 24)).unwrap();
        draw(&mut terminal, &screen, &console).unwrap();
        let buffer = terminal.backend().buffer();
        let find = |text: &str| -> (u16, u16) {
            for y in 0..24 {
                let row: String = (0..110).map(|x| buffer[(x, y)].symbol().to_string()).collect();
                if let Some(at) = row.find(text) {
                    return (row[..at].chars().count() as u16, y);
                }
            }
            panic!("{} isn't on screen", text)
        };
        use ratatui::style::Color;
        let (x, y) = find("village/well");
        assert_eq!(buffer[(x, y)].fg, Color::Cyan, "an exit to a written room");
        let (x, y) = find("village/inn");
        assert_eq!(buffer[(x, y)].fg, Color::Yellow, "an exit to a room not written yet");
        let (x, y) = find("connections");
        assert_eq!(buffer[(x, y)].fg, Color::Blue, "a key");
    }

    #[test]
    fn long_lines_wrap_at_spaces_and_keep_their_indent() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("  aa bb cc", 7), vec!["  aa bb", "  cc"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn quit_and_restart_are_left_to_the_loop() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "/restart");
        assert!(screen.restart);
        typed(&mut screen, &console, "/quit");
        assert!(screen.quit);
    }
}
