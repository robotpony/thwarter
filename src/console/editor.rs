//! The full-screen editor that `/edit room`, `/edit object` and
//! `/edit character` open (REQ-CON-6), drawn with ratatui inside the
//! terminal rather than handed to `$EDITOR`.
//!
//! The editor knows nothing about worlds. It edits a `Document` and hands
//! the text of each section to a save callback, which decides whether the
//! text is acceptable and writes it. A document has one section per file:
//! one for a thing stored as one file, several for a bundle
//! (`design/STATES.md` › Console, not built yet).
//!
//! Keys and drawing are separate from the terminal, so tests drive the
//! editor with a list of keys and ratatui's test backend.

use super::theme::{highlight, is_keyword_heading, line_role, style, Lookup, Role};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
#[cfg(test)]
use ratatui::{backend::Backend, Terminal};
use ratatui_textarea::{CursorMove, Input, Key, TextArea, WrapMode};
#[cfg(test)]
use std::io;

/// What the editor shows: a heading, and one section per file.
#[derive(Debug, Clone)]
pub struct Document {
    /// The top line, such as `/edit room  village/square`.
    pub heading: String,
    pub sections: Vec<Section>,
}

/// One file's text in a document.
#[derive(Debug, Clone)]
pub struct Section {
    pub text: String,
}

impl Document {
    /// A document holding one file's text.
    pub fn single(heading: impl Into<String>, text: impl Into<String>) -> Self {
        Document { heading: heading.into(), sections: vec![Section { text: text.into() }] }
    }

    /// The text the author edits. One section is the file exactly as it is
    /// (REQ-CON-6), with no section header.
    fn combined(&self) -> String {
        match self.sections.as_slice() {
            [one] => one.text.clone(),
            _ => unimplemented!("documents with several sections come with bundles (design/STATES.md)"),
        }
    }

    /// Splits edited text back into one text per section.
    fn split(&self, text: String) -> Vec<String> {
        match self.sections.len() {
            1 => vec![text],
            _ => unimplemented!("documents with several sections come with bundles (design/STATES.md)"),
        }
    }
}

/// What the save callback says about one save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// Written. The report is shown in the status line (its first line)
    /// and printed in full when the editor closes.
    Yes(String),
    /// Not written, and the editor stays open so the author can fix it,
    /// for example text that doesn't parse.
    No(String),
    /// Not written because a file changed on disk since the editor opened
    /// it. A second Ctrl-S overwrites it (the callback gets `force`).
    Conflict(String),
}

/// Where a document's files live, as the editor sees them.
pub trait Files {
    /// Writes each section's text; `force` overwrites a change on disk.
    fn save(&mut self, sections: &[String], force: bool) -> Saved;
    /// Each section's text on disk, if it changed since the editor last
    /// opened, saved or reloaded it. Reports each change once.
    fn changed_on_disk(&mut self) -> Option<Vec<String>>;
}

/// What the editor returns when it closes: each save's report, in order.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Closed {
    pub reports: Vec<String>,
}

/// What the heading bar says about keys. A Mac saves with Command-S, where
/// the terminal passes it on (`screen::input_of`).
const KEYS_HINT: &str = if cfg!(target_os = "macos") { "⌘s or ctrl-s save · esc close" } else { "ctrl-s save · esc close" };

/// The editor's state between keys.
pub struct Editor<'a> {
    document: Document,
    area: TextArea<'a>,
    /// The text as last saved (or opened), to know whether there are
    /// unsaved changes.
    saved_text: String,
    status: String,
    /// Esc was pressed once with unsaved changes; a second Esc discards them.
    discard_armed: bool,
    /// The last save hit a conflict; the next Ctrl-S overwrites.
    force_armed: bool,
    reports: Vec<String>,
}

impl<'a> Editor<'a> {
    /// Opens with the cursor after what's written (Bruce, 2026-11-23): at
    /// the end of the last line of prose, or on a fresh line after the
    /// frontmatter when there's no prose yet.
    pub fn new(document: Document) -> Self {
        let text = document.combined();
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        let last = lines.iter().rposition(|l| !l.trim().is_empty());
        let only_frontmatter = last.is_some_and(|n| lines[n].trim_end() == "---" && n > 0);
        let row = match last {
            Some(n) if only_frontmatter => {
                lines.truncate(n + 1);
                lines.push(String::new());
                n + 1
            }
            Some(n) => n,
            None => 0,
        };
        let col = lines.get(row).map(|l| l.chars().count()).unwrap_or(0);
        let mut area = TextArea::new(lines);
        // Prose wraps at words, as it reads (Bruce, 2026-11-23).
        area.set_wrap_mode(WrapMode::WordOrGlyph);
        area.move_cursor(CursorMove::Jump(row.min(u16::MAX as usize) as u16, col.min(u16::MAX as usize) as u16));
        area.set_line_number_style(style(Role::Chrome));
        area.set_cursor_line_style(Style::default());
        Editor {
            document,
            area,
            saved_text: text,
            status: String::new(),
            discard_armed: false,
            force_armed: false,
            reports: Vec::new(),
        }
    }

    /// The text in the editor, ending with a newline as content files do.
    fn text(&self) -> String {
        let mut text = self.area.lines().join("\n");
        text.push('\n');
        text
    }

    fn dirty(&self) -> bool {
        normalize(&self.text()) != normalize(&self.saved_text)
    }

    /// After the files changed on disk: takes the new text if nothing here
    /// is unsaved, else warns once, and the next save overwrites the change
    /// (`design/STATES.md` › Engine rules: authors use git, so this is rare).
    fn disk_changed(&mut self, sections: Vec<String>) {
        if self.dirty() {
            self.force_armed = true;
            self.status = "Changed on disk too. Ctrl-s overwrites that change with yours; esc keeps it.".to_string();
            return;
        }
        for (section, text) in self.document.sections.iter_mut().zip(&sections) {
            section.text = text.clone();
        }
        let text = self.document.combined();
        let cursor = self.area.cursor();
        let (row, col) = (cursor.0, cursor.1);
        let mut area = TextArea::new(text.lines().map(str::to_string).collect());
        area.set_wrap_mode(WrapMode::WordOrGlyph);
        area.set_line_number_style(style(Role::Chrome));
        area.set_cursor_line_style(Style::default());
        area.move_cursor(CursorMove::Jump(row.min(u16::MAX as usize) as u16, col.min(u16::MAX as usize) as u16));
        self.area = area;
        self.saved_text = text;
        self.status = "Reloaded: changed on disk.".to_string();
    }

    /// Handles one key. Returns false when the editor should close.
    pub fn key(&mut self, input: Input, files: &mut impl Files) -> bool {
        match input {
            Input { key: Key::Esc, .. } => {
                if !self.dirty() || self.discard_armed {
                    return false;
                }
                self.discard_armed = true;
                self.status = "Unsaved changes. Esc again discards them; ctrl-s saves.".to_string();
            }
            Input { key: Key::Char('s'), ctrl: true, alt: false, .. } => {
                self.discard_armed = false;
                let text = self.text();
                let sections = self.document.split(text.clone());
                match files.save(&sections, self.force_armed) {
                    Saved::Yes(report) => {
                        self.saved_text = text;
                        self.force_armed = false;
                        self.status = report.lines().next().unwrap_or("Saved.").to_string();
                        if report.lines().count() > 1 {
                            self.status.push_str(" (more when you close)");
                        }
                        self.reports.push(report);
                    }
                    Saved::No(why) => self.status = one_line(&why),
                    Saved::Conflict(why) => {
                        self.force_armed = true;
                        self.status = one_line(&why);
                    }
                }
            }
            input => {
                self.discard_armed = false;
                self.area.input(input);
            }
        }
        true
    }

    /// Takes a change on disk to the files, if there is one.
    pub fn check_disk(&mut self, files: &mut impl Files) {
        if let Some(sections) = files.changed_on_disk() {
            self.disk_changed(sections);
        }
    }

    /// What the editor closed with.
    pub fn closed(self) -> Closed {
        Closed { reports: self.reports }
    }

    /// Draws the editor in `area`: its heading, the text, and a status line.
    /// Draws the editor in `area`: its heading, the text highlighted
    /// (REQ-CON-39), and a status line coloured by what it says.
    pub fn draw(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, lookup: &dyn Lookup) {
        let [top, middle, bottom] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)]).areas(area);
        let width = top.width as usize;
        let gap = width.saturating_sub(self.document.heading.chars().count() + KEYS_HINT.chars().count() + 1).max(2);
        let heading = format!(" {}{}{}", self.document.heading, " ".repeat(gap), KEYS_HINT);
        frame.render_widget(Paragraph::new(Line::from(heading)).style(style(Role::Bar)), top);
        frame.render_widget(&self.area, middle);
        let roles = highlight(self.area.lines(), lookup);
        paint(frame.buffer_mut(), middle, self.area.lines(), &roles, self.area.cursor().0);
        let status = if self.status.is_empty() && self.dirty() { "Modified." } else { self.status.as_str() };
        frame.render_widget(Paragraph::new(Line::from(format!(" {}", status))).style(status_style(status)), bottom);
    }
}

/// A world with nothing in it, for tests that don't need one.
#[cfg(test)]
struct NoLookup;
#[cfg(test)]
impl Lookup for NoLookup {
    fn resolves(&self, _: &str) -> bool {
        false
    }
    fn declared(&self, _: &str) -> bool {
        false
    }
}

/// Runs the editor on its own `terminal` until the author closes it, for
/// tests; the full-screen console hosts `Editor` itself. `next_key`
/// returns `None` when no key came, and the editor then checks the disk.
#[cfg(test)]
pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    document: Document,
    mut next_key: impl FnMut() -> io::Result<Option<Input>>,
    mut files: impl Files,
) -> io::Result<Closed>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut editor = Editor::new(document);
    loop {
        terminal.draw(|frame| editor.draw(frame, frame.area(), &NoLookup)).map_err(io::Error::other)?;
        let Some(input) = next_key()? else {
            editor.check_disk(&mut files);
            continue;
        };
        if !editor.key(input, &mut files) {
            return Ok(editor.closed());
        }
    }
}

/// Colours what the text area drew, from each line's roles. The text area
/// draws each line after its number, so a row's number says which line it
/// starts; the rows after it with no number are the same line wrapped,
/// and pick up where the row before stopped. A row that doesn't match its
/// line's text is left as drawn.
fn paint(buf: &mut Buffer, area: Rect, lines: &[String], roles: &[Vec<Option<Role>>], cursor_row: usize) {
    let digits = lines.len().max(1).to_string().len();
    let gutter = digits + 2;
    // The line being drawn, and how many of its characters rows have shown.
    let mut current: Option<(usize, usize)> = None;
    for y in area.y..area.bottom() {
        let row: String = (area.x..area.right()).map(|x| buf[(x, y)].symbol().to_string()).collect();
        let shown: String = row.chars().skip(gutter).collect();
        let shown = shown.trim_end();
        let number = row.get(..gutter).and_then(|g| g.trim().parse::<usize>().ok());
        let (line_index, from) = match (number, current) {
            (Some(n), _) if n >= 1 && n <= lines.len() => (n - 1, 0),
            (None, Some((n, consumed))) => {
                // A wrapped row starts after the spaces the wrap left behind.
                let chars: Vec<char> = lines[n].chars().collect();
                let mut at = consumed;
                while at < chars.len() && chars[at] == ' ' {
                    at += 1;
                }
                (n, at)
            }
            _ => {
                current = None;
                continue;
            }
        };
        let chars: Vec<char> = lines[line_index].chars().collect();
        let shown_chars: Vec<char> = shown.chars().collect();
        let fits = chars.get(from..from + shown_chars.len()).is_some_and(|part| part == shown_chars.as_slice());
        if !fits && !shown.is_empty() {
            current = None;
            continue;
        }
        current = Some((line_index, from + shown_chars.len()));
        if line_index == cursor_row && number.is_some() {
            for x in area.x..(area.x + gutter as u16).min(area.right()) {
                let cell = &mut buf[(x, y)];
                cell.set_style(cell.style().remove_modifier(Modifier::DIM).add_modifier(Modifier::BOLD));
            }
        }
        // A keyword heading is drawn as a rule across the rest of its row.
        if number.is_some() && is_keyword_heading(&lines[line_index]) {
            for x in (area.x + (gutter + shown_chars.len() + 1) as u16)..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_symbol("─");
                cell.set_style(style(Role::Chrome));
            }
        }
        for i in 0..shown_chars.len() {
            let x = area.x + (gutter + i) as u16;
            if x >= area.right() {
                break;
            }
            if let Some(Some(role)) = roles[line_index].get(from + i) {
                let cell = &mut buf[(x, y)];
                cell.set_style(cell.style().patch(style(*role)));
            }
        }
    }
}

/// The editor's status line: green when saved, red when refused, yellow
/// for anything to watch out for.
fn status_style(status: &str) -> Style {
    let warn = ["Unsaved", "Changed on disk too", "Modified."];
    if warn.iter().any(|w| status.starts_with(w)) || status.contains("changed on disk since") || status.contains("more when you close") {
        return style(Role::Warning);
    }
    line_role(status).map(style).unwrap_or_default()
}

/// Text compared for unsaved changes: line endings and a missing final
/// newline don't count.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

/// A message for the one-line status bar.
fn one_line(text: &str) -> String {
    text.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use std::collections::VecDeque;

    fn ch(c: char) -> Input {
        Input { key: Key::Char(c), ..Input::default() }
    }
    fn ctrl(c: char) -> Input {
        Input { key: Key::Char(c), ctrl: true, ..Input::default() }
    }
    fn esc() -> Input {
        Input { key: Key::Esc, ..Input::default() }
    }

    /// Files for tests: answers saves with `answer`, and reports a change
    /// on disk at each `tick()` that has one queued.
    struct TestFiles<'a, F: FnMut(&str, bool) -> Saved> {
        answer: F,
        saves: &'a mut Vec<(String, bool)>,
        disk: VecDeque<Option<String>>,
    }

    impl<F: FnMut(&str, bool) -> Saved> Files for TestFiles<'_, F> {
        fn save(&mut self, sections: &[String], force: bool) -> Saved {
            self.saves.push((sections[0].clone(), force));
            (self.answer)(&sections[0], force)
        }
        fn changed_on_disk(&mut self) -> Option<Vec<String>> {
            self.disk.pop_front().flatten().map(|text| vec![text])
        }
    }

    /// No key this time round, so the editor checks the disk.
    fn tick() -> Option<Input> {
        None
    }

    /// Runs the editor over `keys` on a small test screen, returning what
    /// it closed with, each save's texts, and the last screen drawn.
    fn drive(text: &str, keys: Vec<Input>, answer: impl FnMut(&str, bool) -> Saved) -> (Closed, Vec<(String, bool)>, String) {
        drive_with_disk(text, keys.into_iter().map(Some).collect(), Vec::new(), answer)
    }

    /// `drive`, with ticks among the keys and a change on disk queued for
    /// each tick (`None` for no change).
    fn drive_with_disk(
        text: &str,
        keys: Vec<Option<Input>>,
        disk: Vec<Option<String>>,
        answer: impl FnMut(&str, bool) -> Saved,
    ) -> (Closed, Vec<(String, bool)>, String) {
        let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
        let mut keys: VecDeque<Option<Input>> = keys.into();
        let mut saves = Vec::new();
        let files = TestFiles { answer, saves: &mut saves, disk: disk.into() };
        let closed = run(
            &mut terminal,
            Document::single("/edit room  village/square", text),
            || Ok(keys.pop_front().expect("the editor wanted more keys than the test gave")),
            files,
        )
        .unwrap();
        let screen = terminal.backend().buffer().content().iter().map(|c| c.symbol()).collect::<String>();
        (closed, saves, screen)
    }

    #[test]
    fn typing_then_saving_hands_over_the_whole_file() {
        let (closed, saves, _) =
            drive("---\nid: a\n---\nOld.\n", vec![ch('X'), ctrl('s'), esc()], |_, _| Saved::Yes("Saved a.md.".into()));
        assert_eq!(saves, vec![("---\nid: a\n---\nOld.X\n".to_string(), false)], "typed where the cursor starts: after the prose");
        assert_eq!(closed.reports, vec!["Saved a.md.".to_string()]);
    }

    #[test]
    fn esc_with_unsaved_changes_asks_once() {
        let (closed, saves, screen) = drive("text\n", vec![ch('X'), esc(), esc()], |_, _| unreachable!());
        assert!(saves.is_empty());
        assert!(closed.reports.is_empty());
        assert!(screen.contains("Esc again discards"), "{}", screen);
    }

    #[test]
    fn esc_with_nothing_changed_closes_at_once() {
        let (closed, _, _) = drive("text\n", vec![esc()], |_, _| unreachable!());
        assert_eq!(closed, Closed::default());
    }

    #[test]
    fn a_refused_save_keeps_the_editor_open_with_the_reason() {
        let (closed, saves, screen) = drive(
            "text\n",
            vec![ch('X'), ctrl('s'), esc(), esc()],
            |_, _| Saved::No("Not saved: a.md doesn't parse:\n  bad YAML".into()),
        );
        assert_eq!(saves.len(), 1);
        assert!(closed.reports.is_empty());
        // The first esc after a refused save warns that the change isn't
        // saved; the second discards it.
        assert!(screen.contains("Unsaved changes"), "{}", screen);
    }

    #[test]
    fn a_conflict_needs_a_second_save_to_overwrite() {
        let (closed, saves, _) = drive("text\n", vec![ch('X'), ctrl('s'), ctrl('s'), esc()], |_, force| {
            if force {
                Saved::Yes("Saved a.md.".into())
            } else {
                Saved::Conflict("a.md changed on disk.".into())
            }
        });
        assert_eq!(saves.iter().map(|(_, force)| *force).collect::<Vec<_>>(), vec![false, true]);
        assert_eq!(closed.reports.len(), 1);
    }

    #[test]
    fn a_reload_says_so_on_the_status_line() {
        let (_, _, screen) = drive_with_disk("old\n", vec![tick(), Some(esc())], vec![Some("new\n".into())], |_, _| unreachable!());
        assert!(screen.contains("Reloaded: changed on disk."), "{}", screen);
    }

    #[test]
    fn a_change_on_disk_reloads_when_nothing_is_unsaved() {
        let (closed, saves, screen) = drive_with_disk(
            "old\n",
            vec![tick(), Some(ch('X')), Some(ctrl('s')), Some(esc())],
            vec![Some("new from git\n".into())],
            |_, _| Saved::Yes("Saved.".into()),
        );
        assert_eq!(saves, vec![("newX from git\n".to_string(), false)], "the reload keeps the cursor where it was");
        assert_eq!(closed.reports.len(), 1);
        assert!(!screen.contains("Changed on disk too"), "{}", screen);
    }

    #[test]
    fn a_change_on_disk_with_unsaved_edits_warns_once_then_saving_overwrites() {
        let (_, saves, _) = drive_with_disk(
            "old\n",
            vec![Some(ch('X')), tick(), tick(), Some(ctrl('s')), Some(esc())],
            vec![Some("new from git\n".into()), None],
            |_, _| Saved::Yes("Saved.".into()),
        );
        // The author's text wins, and the save is told to overwrite.
        assert_eq!(saves, vec![("oldX\n".to_string(), true)]);
    }

    #[test]
    fn the_disk_warning_shows_on_the_status_line() {
        let mut editor = Editor::new(Document::single("h", "old\n"));
        editor.area.input(ch('X'));
        editor.disk_changed(vec!["new\n".into()]);
        assert!(editor.status.starts_with("Changed on disk too."), "{}", editor.status);
        assert_eq!(editor.text(), "oldX\n");

        let mut clean = Editor::new(Document::single("h", "old\n"));
        clean.disk_changed(vec!["new\n".into()]);
        assert_eq!((clean.text().as_str(), clean.status.as_str()), ("new\n", "Reloaded: changed on disk."));
        assert!(!clean.dirty());
    }

    #[test]
    fn the_cursor_starts_after_whats_written() {
        let prose = Editor::new(Document::single("h", "---\nid: a\n---\n\nSome prose.\n\n"));
        assert_eq!(prose.area.cursor(), (4, "Some prose.".len()));
        let card = Editor::new(Document::single("h", "---\nid: bench\ntype: object\n---\n"));
        assert_eq!(card.area.cursor(), (4, 0), "a fresh line after the frontmatter");
        assert!(!card.dirty(), "that line isn't an unsaved change");
        let empty = Editor::new(Document::single("h", ""));
        assert_eq!(empty.area.cursor(), (0, 0));
    }

    #[test]
    fn long_prose_wraps_and_keeps_its_colours_on_every_row() {
        use ratatui::style::Color;
        let text = "---\nid: a\n---\nThe old well stands in the square, and beside it sits the [[elder]] all day.\n";
        let editor = Editor::new(Document::single("h", text));
        let mut terminal = Terminal::new(TestBackend::new(36, 10)).unwrap();
        terminal.draw(|frame| editor.draw(frame, frame.area(), &NoLookup)).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..10).map(|y| (0..36).map(|x| buffer[(x, y)].symbol().to_string()).collect()).collect();
        let (y, row) = rows.iter().enumerate().find(|(_, r)| r.contains("[[elder")).unwrap_or_else(|| panic!("{:#?}", rows));
        assert!(!row.trim_start().starts_with('4'), "it's on a wrapped row, not line 4's first: {:?}", rows);
        let x = row.find("[[elder").unwrap() as u16;
        assert_eq!(buffer[(x + 2, y as u16)].fg, Color::Cyan, "{:?}", rows);
        assert!(rows.iter().all(|r| r.chars().count() <= 36), "nothing runs off the edge");
    }

    #[test]
    fn a_keyword_heading_is_drawn_as_a_rule() {
        let editor = Editor::new(Document::single("h", "---\nid: d\n---\nA door.\n\n## When: #open\nIt stands open.\n"));
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal.draw(|frame| editor.draw(frame, frame.area(), &NoLookup)).unwrap();
        let buffer = terminal.backend().buffer();
        let row: String = (0..40).map(|x| buffer[(x, 6)].symbol().to_string()).collect();
        assert!(row.contains("## When: #open ───"), "{}", row);
    }

    #[test]
    fn the_heading_names_the_file_and_the_keys() {
        let (_, _, screen) = drive("text\n", vec![esc()], |_, _| unreachable!());
        assert!(screen.contains("/edit room  village/square"), "{}", screen);
        assert!(screen.contains("ctrl-s save"), "{}", screen);
    }
}
