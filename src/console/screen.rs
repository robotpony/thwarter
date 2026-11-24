//! The full-screen console on a terminal (REQ-CON-22): a status bar on
//! top, the output on the left, the sidebar on the right (GAME, and HERE
//! in edit mode), and the input line across the bottom. The full-screen
//! editor opens over the output and keeps the sidebar.
//!
//! What it shows comes from `Console` (`panels.rs` for the sidebar and
//! status bar); this module only lays it out and handles keys. Scripts and
//! transcripts don't come here: they stay line-based (REQ-CON-9).

use super::commands::Completion;
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
    /// The `/` menu, while it's open (REQ-CON-33).
    menu: Option<Menu>,
    /// The line Esc closed the menu on; it stays closed until the line changes.
    menu_dismissed: Option<String>,
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
            menu: None,
            menu_dismissed: None,
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
        if self.menu.is_some() && self.menu_key(&input, console) {
            return;
        }
        match input {
            Input { key: Key::Tab, .. } => self.refresh_menu(console, true),
            Input { key: Key::Enter, .. } => self.submit(console),
            Input { key: Key::Up, alt: false, .. } => self.browse(-1),
            Input { key: Key::Down, alt: false, .. } => self.browse(1),
            Input { key: Key::Up, alt: true, .. } => self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1),
            Input { key: Key::Down, alt: true, .. } => self.sidebar_scroll += 1,
            Input { key: Key::PageUp, .. } => self.scroll += self.page.get().saturating_sub(2).max(1),
            Input { key: Key::PageDown, .. } => self.scroll = self.scroll.saturating_sub(self.page.get().saturating_sub(2).max(1)),
            Input { key: Key::Char('c'), ctrl: true, .. } => {
                self.set_input("");
                self.menu = None;
            }
            Input { key: Key::Char('d'), ctrl: true, .. } if self.input_text().is_empty() => self.quit = true,
            input => {
                self.browsing = None;
                self.input.input(input);
                self.refresh_menu(console, false);
            }
        }
    }

    /// A key while the menu is open. Returns whether the menu took it.
    fn menu_key(&mut self, input: &Input, console: &Mutex<Console>) -> bool {
        let naming_a_command = !self.input_text().contains(' ');
        let Some(menu) = self.menu.as_mut() else { return false };
        let count = menu.completion.suggestions.len();
        match input.key {
            Key::Up if count > 0 => {
                menu.selected = (menu.selected + count - 1) % count;
                menu.chosen = true;
            }
            Key::Down if count > 0 => {
                menu.selected = (menu.selected + 1) % count;
                menu.chosen = true;
            }
            Key::Esc => {
                self.menu = None;
                self.menu_dismissed = Some(self.input_text());
            }
            Key::Tab if count > 0 => {
                let (completion, selected) = (menu.completion.clone(), menu.selected);
                let line = completion.apply(&self.input_text(), selected);
                self.set_input(&line);
                self.refresh_menu(console, true);
            }
            // Enter picks the command being typed, or an argument chosen with
            // Up or Down, then runs the line unless more has to be typed.
            // Otherwise it runs the line as typed.
            Key::Enter if count > 0 && (menu.chosen || naming_a_command) => {
                let (completion, selected) = (menu.completion.clone(), menu.selected);
                let more = completion.suggestions[selected].more;
                let line = completion.apply(&self.input_text(), selected);
                self.set_input(&line);
                self.menu = None;
                if more {
                    self.refresh_menu(console, true);
                } else {
                    self.submit(console);
                }
            }
            _ => return false,
        }
        true
    }

    /// Opens, updates or closes the menu for the line as it is now. It opens
    /// by itself for a `/` command or once a word is followed by a space;
    /// Tab (`asked`) opens it for anything.
    fn refresh_menu(&mut self, console: &Mutex<Console>, asked: bool) {
        let line = self.input_text();
        if !asked && self.menu_dismissed.as_deref() == Some(line.as_str()) {
            return;
        }
        self.menu_dismissed = None;
        let wanted = asked || line.starts_with('/') || line.contains(' ');
        let completion = if wanted && !line.is_empty() { lock(console).complete(&line) } else { Completion::default() };
        self.menu = (!completion.suggestions.is_empty() || completion.hint.is_some())
            .then(|| Menu { completion, selected: 0, chosen: false });
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
        self.menu = None;
        self.menu_dismissed = None;
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
        // Play reads as a story: a blank line after what was typed and after
        // each reply (Bruce, 2026-11-23).
        let playing = lock(console).mode == super::Mode::Play && !line.trim_start().starts_with('/');
        if playing {
            self.output.push(String::new());
        }
        let (text, flow) = lock(console).handle(&line);
        if !text.is_empty() {
            self.push(&text);
        }
        if playing {
            self.output.push(String::new());
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
            None => {
                self.draw_output(frame, output_area);
                if let Some(menu) = &self.menu {
                    draw_menu(frame, output_area, menu);
                }
            }
        }

        let prompt = console.prompt();
        let [left, right] =
            Layout::horizontal([Constraint::Length(prompt.chars().count() as u16 + 1), Constraint::Min(1)]).areas(input);
        let role = if console.mode == super::Mode::Play { Role::PromptPlay } else { Role::PromptEdit };
        frame.render_widget(Paragraph::new(format!(" {}", prompt)).style(style(role)), left);
        if self.editing.is_none() {
            frame.render_widget(&self.input, right);
            paint_command(frame.buffer_mut(), right, &self.input_text());
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
        let lines: Vec<Line> = rows[top..bottom].iter().map(|(text, style)| output_line(text, *style)).collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_sidebar(&self, frame: &mut Frame, area: Rect, fixed: &[Row], rest: &[Row]) {
        let block = Block::default().borders(Borders::LEFT).border_style(style(Role::Chrome));
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

/// The `/` menu: what it offers, and which one is chosen.
struct Menu {
    completion: Completion,
    selected: usize,
    /// Whether the author chose with Up or Down, so Enter takes the choice.
    chosen: bool,
}

/// How many suggestions the menu shows at once.
const MENU_ROWS: usize = 8;

/// The menu, drawn over the bottom of the output, just above the input.
fn draw_menu(frame: &mut Frame, area: Rect, menu: &Menu) {
    let items = &menu.completion.suggestions;
    let shown = items.len().min(MENU_ROWS);
    let start = menu.selected.saturating_sub(MENU_ROWS - 1).min(items.len().saturating_sub(shown));
    let label_width = items.iter().map(|s| s.label.chars().count()).max().unwrap_or(0).min(32);
    let mut lines: Vec<Line> = Vec::new();
    if let Some(hint) = &menu.completion.hint {
        lines.push(Line::styled(format!(" {}", hint), style(Role::Chrome)));
    }
    for (i, s) in items.iter().enumerate().skip(start).take(shown) {
        let chosen = i == menu.selected;
        let mut spans = vec![Span::raw(" ")];
        match s.label.strip_prefix('/') {
            Some(name) => {
                spans.push(Span::styled("/", style(Role::Slash)));
                spans.push(Span::styled(format!("{:<w$}", name, w = label_width.saturating_sub(1)), style(Role::Command)));
            }
            None => spans.push(Span::raw(format!("{:<w$}", s.label, w = label_width))),
        }
        spans.push(Span::styled(format!("  {}", s.detail), style(Role::Note)));
        let mut line = Line::from(spans);
        if chosen {
            line = line.style(Style::default().add_modifier(ratatui::style::Modifier::REVERSED));
        }
        lines.push(line);
    }
    if items.len() > start + shown {
        lines.push(Line::styled(format!(" ↓ {} more", items.len() - start - shown), style(Role::Note)));
    }
    let keys = if items.is_empty() { " esc close" } else { " ↑↓ choose · tab or enter pick · esc close" };
    lines.push(Line::styled(keys, style(Role::Chrome)));
    let height = (lines.len() as u16).min(area.height);
    let width = area.width.min(90);
    let popup = Rect { x: area.x, y: area.bottom().saturating_sub(height), width, height };
    frame.render_widget(ratatui::widgets::Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::TOP).border_style(style(Role::Chrome))),
        Rect { y: popup.y.saturating_sub(1), height: popup.height + 1, ..popup },
    );
}

/// Runs the full-screen console until `/quit` or Ctrl-D.
pub fn run(console: Arc<Mutex<Console>>, notices: Receiver<String>, resumed: Option<Resumed>) -> Result<()> {
    use ratatui::crossterm::event::{self, Event, KeyEventKind};
    let greeting = lock(&console).greeting();
    let mut screen = Screen::new(&greeting, resumed);
    let mut terminal = enter_terminal()?;
    let result = (|| -> Result<()> {
        loop {
            draw(&mut terminal, &screen, &console)?;
            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Release {
                        screen.key(input_of(key), &console);
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
                leave_terminal();
                let why = restart::restart(&lock(&console), &screen.history, &screen.output);
                terminal = enter_terminal()?;
                screen.push(&why);
            }
        }
    })();
    leave_terminal();
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

/// Takes the terminal over: raw mode and the alternate screen (ratatui),
/// keys reported with their modifiers so a Mac's Command key reaches the
/// console in terminals that can send it (the kitty keyboard protocol:
/// kitty, WezTerm, Ghostty, iTerm2 3.5+; others ignore the request), and a
/// clear screen, since after a /restart it still shows the last build's frame.
fn enter_terminal() -> std::io::Result<ratatui::DefaultTerminal> {
    use ratatui::crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
    let terminal = ratatui::try_init()?;
    let _ = ratatui::crossterm::execute!(
        std::io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );
    clear_screen()?;
    Ok(terminal)
}

/// Gives the terminal back as it was.
fn leave_terminal() {
    let _ = ratatui::crossterm::execute!(std::io::stdout(), ratatui::crossterm::event::PopKeyboardEnhancementFlags);
    ratatui::restore();
}

/// A key as the console takes it. Command-S, which a Mac user saves with,
/// is Ctrl-S; other Command keys stay with the terminal (copy, paste).
fn input_of(key: ratatui::crossterm::event::KeyEvent) -> Input {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};
    if key.modifiers.contains(KeyModifiers::SUPER) && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S')) {
        return Input { key: Key::Char('s'), ctrl: true, ..Input::default() };
    }
    Input::from(key)
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
        // A map row keeps its shape: cut at the edge, never wrapped.
        Row::Map(t) => return vec![Row::Map(t.chars().take(width).collect())],
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
        Row::Map(text) => styled(text, Role::Link),
        Row::Blank => Line::from(""),
    }
}

/// One row of output. A console command echoed as `> /move ...` shows its
/// slash and name the way the input line does.
fn output_line(text: &str, row_style: Style) -> Line<'static> {
    if let Some(command) = text.strip_prefix("> /") {
        let name_end = command.find(' ').unwrap_or(command.len());
        return Line::from(vec![
            Span::styled(" > ", row_style),
            Span::styled("/", style(Role::Slash)),
            Span::styled(command[..name_end].to_string(), style(Role::Command)),
            Span::styled(command[name_end..].to_string(), Style::default()),
        ]);
    }
    Line::styled(format!(" {}", text), row_style)
}

/// Colours a console command being typed: its `/` and its name.
fn paint_command(buf: &mut ratatui::buffer::Buffer, area: Rect, text: &str) {
    if !text.starts_with('/') || text.chars().count() >= area.width as usize {
        return;
    }
    let name_end = text.find(' ').unwrap_or(text.len());
    for (i, _) in text[..name_end].chars().enumerate() {
        let cell = &mut buf[(area.x + i as u16, area.y)];
        let role = if i == 0 { Role::Slash } else { Role::Command };
        cell.set_style(cell.style().patch(style(role)));
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
        assert!(lines[23].contains("in village/square >"), "{}", text);
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
    fn chrome_is_blue_and_commands_show_their_slash() {
        use ratatui::style::Color;
        let console = tiny();
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "/status");
        for c in "/move lantern".chars() {
            screen.key(Input { key: Key::Char(c), ..Input::default() }, &console);
        }
        let mut terminal = Terminal::new(TestBackend::new(110, 12)).unwrap();
        draw(&mut terminal, &screen, &console).unwrap();
        let buffer = terminal.backend().buffer();
        let row = |y: u16| (0..110).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>();
        assert_eq!(buffer[(0, 0)].bg, Color::Blue, "the status bar");
        let border = row(1).chars().position(|c| c == '│').unwrap() as u16;
        assert_eq!(buffer[(border, 1)].fg, Color::Blue, "the sidebar's border");
        // The echoed command and the one being typed.
        let echo = (0..12).find(|&y| row(y).contains("> /status")).unwrap();
        let slash = row(echo).find('/').unwrap() as u16;
        assert_eq!(buffer[(slash, echo)].fg, Color::Yellow);
        let input = row(11).find("/move").unwrap() as u16;
        assert_eq!(buffer[(input, 11)].fg, Color::Yellow);
        assert!(buffer[(input + 1, 11)].modifier.contains(ratatui::style::Modifier::BOLD));
        assert_ne!(buffer[(input + 7, 11)].fg, Color::Yellow, "arguments stay plain");
    }

    fn keys(screen: &mut Screen, console: &Mutex<Console>, text: &str) {
        for c in text.chars() {
            screen.key(Input { key: Key::Char(c), ..Input::default() }, console);
        }
    }

    fn press(screen: &mut Screen, console: &Mutex<Console>, key: Key) {
        screen.key(Input { key, ..Input::default() }, console);
    }

    #[test]
    fn a_slash_opens_the_menu_and_typing_narrows_it() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        keys(&mut screen, &console, "/");
        assert!(screen.menu.as_ref().unwrap().completion.suggestions.len() > 10);
        keys(&mut screen, &console, "re");
        let text = screen_text(&screen, &console, 110, 24);
        assert!(text.contains("/reload") && text.contains("/restart"), "{}", text);
        assert!(text.contains("tab or enter pick"), "{}", text);
        press(&mut screen, &console, Key::Down);
        press(&mut screen, &console, Key::Tab);
        assert_eq!(screen.input_text(), "/restart");
    }

    #[test]
    fn enter_picks_and_runs_unless_more_is_needed() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        keys(&mut screen, &console, "/stat");
        press(&mut screen, &console, Key::Enter);
        assert!(screen.output.iter().any(|l| l.starts_with("Tiny World, at")), "/status ran: {:?}", screen.output);
        keys(&mut screen, &console, "/lin");
        press(&mut screen, &console, Key::Enter);
        assert_eq!(screen.input_text(), "/link ", "it needs a direction next, so it waits");
        assert!(screen.menu.is_some(), "and offers the directions");
    }

    #[test]
    fn go_offers_exits_and_esc_closes_the_menu_until_the_line_changes() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "go /village/square");
        keys(&mut screen, &console, "go n");
        assert_eq!(screen.menu.as_ref().unwrap().completion.suggestions[0].text, "north");
        press(&mut screen, &console, Key::Esc);
        assert!(screen.menu.is_none());
        press(&mut screen, &console, Key::Enter);
        assert!(screen.output.iter().any(|l| l == "The Well (room)"), "go n ran as typed: {:?}", screen.output);
        // With the menu closed, Up is history again.
        press(&mut screen, &console, Key::Up);
        assert_eq!(screen.input_text(), "go n");
    }

    #[test]
    fn command_s_saves_like_control_s() {
        use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let command_s = input_of(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::SUPER));
        assert_eq!((command_s.key, command_s.ctrl), (Key::Char('s'), true));
        let command_c = input_of(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SUPER));
        assert!(!command_c.ctrl, "only Command-S is taken over");
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
