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

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::{backend::Backend, Terminal};
use ratatui_textarea::{Input, Key, TextArea};
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

/// What the editor returns when it closes: each save's report, in order.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Closed {
    pub reports: Vec<String>,
}

const KEYS_HINT: &str = "ctrl-s save · esc close";

/// The editor's state between keys.
struct Editor<'a> {
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
    fn new(document: Document) -> Self {
        let text = document.combined();
        let mut area = TextArea::new(text.lines().map(str::to_string).collect());
        area.set_line_number_style(Style::default().add_modifier(Modifier::DIM));
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

    /// Handles one key. Returns false when the editor should close.
    fn key(&mut self, input: Input, save: &mut impl FnMut(&[String], bool) -> Saved) -> bool {
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
                match save(&sections, self.force_armed) {
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

    fn draw(&self, frame: &mut ratatui::Frame) {
        let [top, middle, bottom] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
        let width = top.width as usize;
        let gap = width.saturating_sub(self.document.heading.chars().count() + KEYS_HINT.chars().count()).max(2);
        let heading = format!("{}{}{}", self.document.heading, " ".repeat(gap), KEYS_HINT);
        frame.render_widget(Paragraph::new(Line::from(heading)).style(Style::default().add_modifier(Modifier::REVERSED)), top);
        frame.render_widget(&self.area, middle);
        let status = if self.status.is_empty() && self.dirty() { "Modified." } else { self.status.as_str() };
        frame.render_widget(Paragraph::new(Line::from(status.to_string())), bottom);
    }
}

/// Runs the editor on `terminal` until the author closes it. `next_key`
/// waits for the next key (`None` for anything that only needs a redraw,
/// such as a resize). `save` gets each section's text and whether to
/// overwrite a conflict.
pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    document: Document,
    mut next_key: impl FnMut() -> io::Result<Option<Input>>,
    mut save: impl FnMut(&[String], bool) -> Saved,
) -> io::Result<Closed>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut editor = Editor::new(document);
    loop {
        terminal.draw(|frame| editor.draw(frame)).map_err(io::Error::other)?;
        let Some(input) = next_key()? else { continue };
        if !editor.key(input, &mut save) {
            return Ok(Closed { reports: editor.reports });
        }
    }
}

/// Opens the editor full-screen on the real terminal, and puts the
/// terminal back as it was when it closes, even after an error.
pub fn run_in_terminal(document: Document, save: impl FnMut(&[String], bool) -> Saved) -> io::Result<Closed> {
    use ratatui::crossterm::event::{self, Event, KeyEventKind};
    let mut terminal = ratatui::try_init()?;
    let next_key = || -> io::Result<Option<Input>> {
        Ok(match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => Some(Input::from(key)),
            _ => None,
        })
    };
    let result = run(&mut terminal, document, next_key, save);
    ratatui::restore();
    result
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

    /// Runs the editor over `keys` on a small test screen, returning what
    /// it closed with, each save's texts, and the last screen drawn.
    fn drive(text: &str, keys: Vec<Input>, mut answer: impl FnMut(&str, bool) -> Saved) -> (Closed, Vec<(String, bool)>, String) {
        let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
        let mut keys: VecDeque<Input> = keys.into();
        let mut saves = Vec::new();
        let closed = run(
            &mut terminal,
            Document::single("/edit room  village/square", text),
            || Ok(Some(keys.pop_front().expect("the editor wanted more keys than the test gave"))),
            |sections, force| {
                saves.push((sections[0].clone(), force));
                answer(&sections[0], force)
            },
        )
        .unwrap();
        let screen = terminal.backend().buffer().content().iter().map(|c| c.symbol()).collect::<String>();
        (closed, saves, screen)
    }

    #[test]
    fn typing_then_saving_hands_over_the_whole_file() {
        let (closed, saves, _) =
            drive("---\nid: a\n---\nOld.\n", vec![ch('X'), ctrl('s'), esc()], |_, _| Saved::Yes("Saved a.md.".into()));
        assert_eq!(saves, vec![("X---\nid: a\n---\nOld.\n".to_string(), false)]);
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
    fn the_heading_names_the_file_and_the_keys() {
        let (_, _, screen) = drive("text\n", vec![esc()], |_, _| unreachable!());
        assert!(screen.contains("/edit room  village/square"), "{}", screen);
        assert!(screen.contains("ctrl-s save"), "{}", screen);
    }
}
