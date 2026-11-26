//! The full-screen console on a terminal (REQ-CON-22): a status bar on
//! top, the output on the left, the sidebar on the right (GAME, and HERE
//! in edit mode), and the input line across the bottom. The full-screen
//! editor opens over the output and keeps the sidebar.
//!
//! What it shows comes from `Console` (`panels.rs` for the sidebar and
//! status bar); this module only lays it out and handles keys. Scripts and
//! transcripts don't come here: they stay line-based (REQ-CON-9).

use super::asking::{needs, Answer, Asking, Needs};
use super::quick_menu::{Ask, QuickMenu};
use super::commands::Completion;
use crate::engines::questions::Choice;
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
    /// The screen's own lines, from `[console]`.
    words: Words,
    /// What kind each `[console]` line is, for its colour.
    kinds: super::kinds::Kinds,
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
    /// Questions about a thing just made, while they're being asked
    /// (REQ-CON-46); the editor opens on the thing after them.
    picker: Option<Picker<'a>>,
    /// The quick menu, while it's open (REQ-CON-63).
    quick: Option<Quick<'a>>,
    /// The drop-down console over the editor, while it's open (REQ-CON-71).
    dropdown: Option<Dropdown<'a>>,
    /// The line Esc closed the menu on; it stays closed until the line changes.
    menu_dismissed: Option<String>,
    sidebar_scroll: usize,
    /// The output pane's height at the last draw, for Page Up and Down.
    page: Cell<usize>,
    /// Where the sidebar was at the last draw, for the wheel, and what a
    /// click on each of its screen rows does (REQ-CON-69, 70).
    sidebar_area: Cell<Rect>,
    clicks: std::cell::RefCell<Vec<(u16, Click)>>,
    /// This binary's modification time when it started, to notice a newer build.
    built: Option<SystemTime>,
    newer_build: bool,
    /// Where this world's command history is kept between sessions.
    history_file: Option<std::path::PathBuf>,
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
            picker: None,
            quick: None,
            dropdown: None,
            menu_dismissed: None,
            sidebar_scroll: 0,
            page: Cell::new(20),
            sidebar_area: Cell::new(Rect::default()),
            clicks: Default::default(),
            built: binary_built(),
            newer_build: false,
            history_file: None,
            last_check: Instant::now(),
            quit: false,
            restart: false,
            words: Words::from(&crate::engines::texts::Texts::defaults()),
            kinds: Default::default(),
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
        if self.quick.is_some() {
            self.quick_key(input, console);
            return;
        }
        if self.dropdown.is_some() {
            self.dropdown_key(input, console);
            return;
        }
        // `~` on an empty line in the editor slides the console down
        // (REQ-CON-71); anywhere else it's text.
        if let (Some((editor, _)), Input { key: Key::Char('~'), ctrl: false, alt: false, .. }) = (&self.editing, &input) {
            if editor.on_empty_line() {
                self.dropdown = Some(Dropdown { area: input_line(), reply: Vec::new() });
                return;
            }
        }
        // A TO WRITE key switches documents from the editor too: save this
        // one, then make and open that one (REQ-CON-59).
        if let (Some(_), Input { key: Key::Char(c @ '1'..='9'), alt: true, .. }) = (&self.editing, &input) {
            let n = *c as usize - '0' as usize;
            let Some(name) = lock(console).to_write_at(n) else { return };
            let (editor, target) = self.editing.as_mut().expect("the editor is open");
            let mut files = EditFiles { console, target };
            if !editor.save_to_switch(&mut files) {
                return;
            }
            let (editor, target) = self.editing.take().expect("the editor is open");
            let report = lock(console).edit_closed(&target, editor.closed());
            self.push(&report);
            self.set_input(&format!("/write {}", name));
            self.submit(console);
            return;
        }
        // Ctrl-M opens the quick menu from the editor too, before the text
        // area reads it as a new line (REQ-CON-63). The document is saved
        // and closed, the menu acts on its thing, and the editor reopens
        // on it afterwards.
        if self.editing.is_some() && matches!(input, Input { key: Key::Char('m'), ctrl: true, .. }) {
            let (editor, target) = self.editing.as_mut().expect("the editor is open");
            if !editor.save_to_switch(&mut EditFiles { console, target }) {
                return;
            }
            let (editor, target) = self.editing.take().expect("the editor is open");
            let (kind, id, path) = target.thing();
            let (kind, id, path) = (kind, id.to_string(), path.to_string());
            let report = lock(console).edit_closed(&target, editor.closed());
            if !report.is_empty() {
                self.push(&report);
            }
            self.open_quick_for(console, Some(&path));
            match self.quick.as_mut() {
                Some(quick) => {
                quick.reopen = Some((kind, id));
                quick.file = Some(path);
            }
                None => self.reopen(console, kind, &id),
            }
            return;
        }
        if let Some((editor, target)) = &mut self.editing {
            let mut files = EditFiles { console, target };
            if !editor.key(input, &mut files) {
                let (editor, target) = self.editing.take().expect("the editor is open");
                let report = lock(console).edit_closed(&target, editor.closed());
                self.push(&report);
            }
            return;
        }
        if self.picker.is_some() {
            self.picker_key(input, console);
            return;
        }
        // Ctrl-M opens the quick menu (REQ-CON-63). Without the kitty
        // protocol it arrives as Enter, so only /menu reaches it there.
        if matches!(input, Input { key: Key::Char('m'), ctrl: true, .. }) {
            self.open_quick(console);
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
            Input { key: Key::Char(c @ '1'..='9'), alt: true, .. } => {
                let name = lock(console).to_write_at(c as usize - '0' as usize);
                if let Some(name) = name {
                    self.set_input(&format!("/write {}", name));
                    self.submit(console);
                }
            }
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
        self.add_history(&line);
        // A blank line after what was typed, in every mode (Bruce, 2026-11-24).
        self.output.push(String::new());
        let (text, flow) = lock(console).handle(&line);
        if !text.is_empty() {
            self.push(&text);
        }
        // A blank line after every reply too, so each block of output has
        // air before and after it, and never two blank lines together
        // (Bruce, 2026-11-24).
        if self.output.last().is_some_and(|l| !l.is_empty()) {
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
        self.after_command(console);
    }

    /// What a command left for the screen: questions, the editor, or the
    /// quick menu.
    fn after_command(&mut self, console: &Mutex<Console>) {
        if lock(console).take_pending_menu() {
            self.open_quick(console);
            return;
        }
        let mut console = lock(console);
        // Questions come first; the editor opens on the thing after them.
        if let Some(asking) = console.take_pending_questions() {
            let reopen = console.take_pending_edit().is_some();
            self.picker = Some(Picker::new(asking, reopen));
            return;
        }
        if let Some(target) = console.take_pending_edit() {
            let document = console.edit_document(&target);
            self.editing = Some((Editor::new(document).with_texts(console.texts.clone()), target));
        }
    }

    /// The mouse (REQ-CON-69, 70): a left click on a sidebar row does what
    /// its key does; the wheel scrolls what's under the pointer.
    pub fn mouse(&mut self, event: ratatui::crossterm::event::MouseEvent, console: &Mutex<Console>) {
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};
        let side = self.sidebar_area.get();
        let in_side = side.width > 0 && event.column >= side.x && event.column < side.right() && event.row >= side.y && event.row < side.bottom();
        match event.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let up = event.kind == MouseEventKind::ScrollUp;
                if in_side {
                    self.sidebar_scroll = if up { self.sidebar_scroll.saturating_sub(1) } else { self.sidebar_scroll + 1 };
                } else if let Some((editor, target)) = &mut self.editing {
                    let key = if up { Key::MouseScrollUp } else { Key::MouseScrollDown };
                    editor.key(Input { key, ..Input::default() }, &mut EditFiles { console, target });
                } else if up {
                    self.scroll += 3;
                } else {
                    self.scroll = self.scroll.saturating_sub(3);
                }
            }
            MouseEventKind::Down(MouseButton::Left) if self.quick.is_some() && !in_side => {
                let run = self.quick.as_mut().is_some_and(|q| q.click(event.row));
                if run {
                    self.run_quick(console);
                }
            }
            MouseEventKind::Down(MouseButton::Left) if in_side => {
                let click = self.clicks.borrow().iter().find(|(y, _)| *y == event.row).map(|(_, c)| c.clone());
                if let Some(click) = click {
                    self.click(click, console);
                }
            }
            _ => {}
        }
    }

    /// Does what a sidebar row's key or command does.
    fn click(&mut self, click: Click, console: &Mutex<Console>) {
        if self.quick.is_some() || self.picker.is_some() || self.dropdown.is_some() {
            return;
        }
        match click {
            Click::QuickMenu if self.editing.is_some() => self.key(Input { key: Key::Char('m'), ctrl: true, ..Input::default() }, console),
            Click::QuickMenu => self.open_quick(console),
            Click::ToWrite(n @ 1..=9) => {
                let digit = char::from(b'0' + n as u8);
                self.key(Input { key: Key::Char(digit), alt: true, ..Input::default() }, console);
            }
            Click::ToWrite(n) if self.editing.is_none() => {
                let name = lock(console).to_write_at(n);
                if let Some(name) = name {
                    self.set_input(&format!("/write {}", name));
                    self.submit(console);
                }
            }
            Click::Exit(direction) if self.editing.is_none() => {
                self.set_input(&format!("go {}", direction));
                self.submit(console);
            }
            _ => {}
        }
    }

    /// A key while the drop-down console is open over the editor.
    fn dropdown_key(&mut self, input: Input, console: &Mutex<Console>) {
        let Some(dropdown) = self.dropdown.as_mut() else { return };
        let line = dropdown.area.lines().join(" ");
        match input.key {
            Key::Esc => {
                self.dropdown = None;
                self.browsing = None;
            }
            Key::Char('~') if line.trim().is_empty() && !input.ctrl && !input.alt => {
                self.dropdown = None;
                self.browsing = None;
            }
            Key::Enter if !line.trim().is_empty() => {
                self.browsing = None;
                self.add_history(line.trim());
                self.dropdown_run(line.trim(), console);
            }
            Key::Enter => {}
            // Up and Down recall the same history the prompt does.
            Key::Up | Key::Down => {
                let step = if input.key == Key::Up { -1 } else { 1 };
                if let Some(recalled) = self.recall(step, line) {
                    if let Some(dropdown) = self.dropdown.as_mut() {
                        let mut area = input_line();
                        area.insert_str(recalled);
                        dropdown.area = area;
                    }
                }
            }
            _ => {
                self.browsing = None;
                dropdown.area.input(input);
            }
        }
    }

    /// Runs a drop-down command for the open file: saves first, runs it
    /// with edit mode at the file, then puts the place back (REQ-CON-72).
    fn dropdown_run(&mut self, line: &str, console: &Mutex<Console>) {
        let Some((editor, target)) = self.editing.as_mut() else {
            self.dropdown = None;
            return;
        };
        if !editor.save_to_switch(&mut EditFiles { console, target }) {
            if let Some(dropdown) = self.dropdown.as_mut() {
                dropdown.reply = vec![self.words.text("dropdown_not_saved", &[])];
            }
            return;
        }
        let (kind, id, path) = target.thing();
        let (kind, id, path) = (kind, id.to_string(), path.to_string());
        let cursor = editor.cursor();
        self.push(&format!("> {}", line));
        self.output.push(String::new());
        let (text, flow) = {
            let mut guard = lock(console);
            let before = std::mem::replace(&mut guard.place, super::session::Place::File(path.clone()));
            let out = guard.handle(line);
            guard.place = before;
            if !guard.place_exists() {
                guard.move_to_existing_folder();
            }
            out
        };
        if !text.is_empty() {
            self.push(&text);
            self.output.push(String::new());
        }
        if flow == Flow::Quit {
            self.quit = true;
        }
        if let Some(dropdown) = self.dropdown.as_mut() {
            dropdown.area = input_line();
            dropdown.reply = text.lines().map(str::to_string).collect();
        }
        // /menu, questions or another file: the drop-down and the editor
        // close for them, and the menu comes back to this file.
        let (menu, other) = {
            let mut guard = lock(console);
            let menu = guard.take_pending_menu();
            (menu, guard.pending_questions.is_some() || guard.pending_edit.is_some())
        };
        if menu || other {
            self.dropdown = None;
            let (editor, target) = self.editing.take().expect("the editor is open");
            let report = lock(console).edit_closed(&target, editor.closed());
            if !report.is_empty() {
                self.push(&report);
            }
            if menu {
                self.open_quick_for(console, Some(&path));
                match self.quick.as_mut() {
                    Some(quick) => {
                        quick.reopen = Some((kind, id));
                        quick.file = Some(path);
                    }
                    None => self.reopen(console, kind, &id),
                }
            } else {
                self.after_command(console);
                if self.editing.is_none() && self.picker.is_none() {
                    self.reopen(console, kind, &id);
                }
            }
            return;
        }
        // The file moved (a bundle now): reopen it at the same line.
        // Otherwise take what the command wrote into the open text.
        let moved = lock(console).world.path_of(kind, &id).is_some_and(|now| now != path);
        if moved {
            self.reopen(console, kind, &id);
            if let Some((editor, _)) = self.editing.as_mut() {
                editor.jump(cursor);
            }
        } else if let Some((editor, target)) = self.editing.as_mut() {
            editor.check_disk(&mut EditFiles { console, target });
        }
    }

    /// Opens the quick menu for where edit mode is, or says why not.
    fn open_quick(&mut self, console: &Mutex<Console>) {
        self.open_quick_for(console, None);
    }

    /// Opens the quick menu for a file, such as the editor's, leaving edit
    /// mode's place where it is (Bruce, 2026-11-25).
    fn open_quick_for(&mut self, console: &Mutex<Console>, file: Option<&str>) {
        let menu = match file {
            Some(path) => lock(console).quick_menu_for(Some(path)),
            None => lock(console).quick_menu(),
        };
        match menu {
            Ok(menu) if !menu.entries.is_empty() => self.quick = Some(Quick::new(menu)),
            Ok(_) => {}
            Err(why) => {
                self.push(&why);
                self.output.push(String::new());
            }
        }
    }

    /// A key while the quick menu is open.
    fn quick_key(&mut self, input: Input, console: &Mutex<Console>) {
        let Some(quick) = self.quick.as_mut() else { return };
        if input.key == Key::Esc || matches!(input, Input { key: Key::Char('c'), ctrl: true, .. }) {
            if let Some((kind, id)) = self.quick.take().and_then(|q| q.reopen) {
                self.reopen(console, kind, &id);
            }
            return;
        }
        if !quick.key(input) {
            return;
        }
        self.run_quick(console);
    }

    /// Runs the answered quick menu entry's commands.
    fn run_quick(&mut self, console: &Mutex<Console>) {
        let quick = self.quick.take().expect("an answered quick menu");
        let commands = quick.commands();
        // From the editor the commands act on its file, and edit mode's
        // place is put back afterwards (Bruce, 2026-11-25).
        let before = quick.file.as_ref().map(|file| {
            let mut guard = lock(console);
            std::mem::replace(&mut guard.place, super::session::Place::File(file.clone()))
        });
        for command in &commands {
            self.push(&format!("> {}", command));
            self.output.push(String::new());
            let (text, flow) = lock(console).handle(command);
            if !text.is_empty() {
                self.push(&text);
                self.output.push(String::new());
            }
            if flow == Flow::Quit {
                self.quit = true;
            }
        }
        if let Some(before) = before {
            let mut guard = lock(console);
            guard.place = before;
            if !guard.place_exists() {
                guard.move_to_existing_folder();
            }
        }
        let typed = lock(console).quick_menu_typed(&commands);
        self.push(&typed);
        self.output.push(String::new());
        self.after_command(console);
        if let (Some((kind, id)), None, None) = (quick.reopen, &self.editing, &self.picker) {
            self.reopen(console, kind, &id);
        }
    }

    /// Opens the editor on a thing again, at wherever its file now is.
    fn reopen(&mut self, console: &Mutex<Console>, kind: super::world::Kind, id: &str) {
        let guard = lock(console);
        if let Some(path) = guard.world.path_of(kind, id) {
            if let Ok(target) = guard.open_target(kind, id.to_string(), path) {
                let document = guard.edit_document(&target);
                self.editing = Some((Editor::new(document).with_texts(guard.texts.clone()), target));
            }
        }
    }

    /// A key while questions are being asked.
    fn picker_key(&mut self, input: Input, console: &Mutex<Console>) {
        let Some(picker) = self.picker.as_mut() else { return };
        let question = &picker.asking.set.questions[picker.at];
        match &mut picker.step {
            Step::Choose => match input.key {
                Key::Up => picker.selected = (picker.selected + question.options.len() - 1) % question.options.len().max(1),
                Key::Down => picker.selected = (picker.selected + 1) % question.options.len().max(1),
                Key::Enter => {
                    let choice = question.options[picker.selected].clone();
                    match needs(&choice) {
                        None => picker.answer(choice, None),
                        Some(Needs::Text { prompt, default }) => {
                            let mut area = input_line();
                            if let Some(default) = default {
                                area.insert_str(default);
                            }
                            picker.step = Step::Typing { choice, prompt, area };
                        }
                        Some(Needs::Thing) => {
                            let things = lock(console).things_to_pick(&picker.asking.id);
                            if things.is_empty() {
                                picker.answer(choice, None);
                            } else {
                                picker.step = Step::Picking { choice, things, selected: 0 };
                            }
                        }
                    }
                }
                Key::Esc => picker.use_defaults(),
                _ => {}
            },
            Step::Typing { choice, area, .. } => match input.key {
                Key::Enter => {
                    let (choice, given) = (choice.clone(), area.lines().join(" "));
                    picker.answer(choice, Some(given));
                }
                Key::Esc => picker.step = Step::Choose,
                _ => {
                    area.input(input);
                }
            },
            Step::Picking { choice, things, selected } => match input.key {
                Key::Up => *selected = (*selected + things.len() - 1) % things.len(),
                Key::Down => *selected = (*selected + 1) % things.len(),
                Key::Enter => {
                    let (choice, given) = (choice.clone(), things[*selected].clone());
                    picker.answer(choice, Some(given));
                }
                Key::Esc => picker.step = Step::Choose,
                _ => {}
            },
        }
        if self.picker.as_ref().is_some_and(|p| p.done()) {
            let picker = self.picker.take().expect("checked");
            let report = lock(console).run_answers(&picker.asking, &picker.answers);
            if !report.is_empty() {
                self.push(&report);
            }
            if picker.reopen {
                let guard = lock(console);
                let kind = picker.asking.kind;
                if let Some(path) = guard.world.path_of(kind, &picker.asking.id) {
                    if let Ok(target) = guard.open_target(kind, picker.asking.id.clone(), path) {
                        let document = guard.edit_document(&target);
                        self.editing = Some((Editor::new(document).with_texts(guard.texts.clone()), target));
                    }
                }
            }
        }
    }

    /// Keeps the command history in `file` between sessions (Bruce,
    /// 2026-11-24): what's there loads for Up and Down, and nothing is
    /// shown until they're pressed (Bruce, 2026-11-25). After a /restart
    /// the session's own history carries over instead.
    pub fn keep_history(&mut self, file: std::path::PathBuf, resumed: bool) {
        if !resumed {
            let earlier: Vec<String> = std::fs::read_to_string(&file)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_string)
                .collect();
            self.history = earlier[earlier.len().saturating_sub(HISTORY)..].to_vec();
        }
        self.history_file = Some(file);
    }

    /// Adds `line` to the history file. A history that can't be written
    /// is skipped; the session's own history still works.
    fn remember(&self, line: &str) {
        let Some(file) = &self.history_file else { return };
        // Leaving isn't worth recalling, and Up would offer it first.
        if matches!(line.trim(), "/quit" | "/q") {
            return;
        }
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
            let _ = writeln!(f, "{}", line.replace('\n', " "));
        }
    }

    /// Up and Down through the input history.
    fn browse(&mut self, step: isize) {
        let current = self.input_text();
        if let Some(line) = self.recall(step, current) {
            self.set_input(&line);
        }
    }

    /// One step through the shared history from `current`, the line as
    /// typed: the line to show, or `None` to leave it. The prompt and the
    /// drop-down both use it (Bruce, 2026-11-25).
    fn recall(&mut self, step: isize, current: String) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }
        let (at, draft) = match self.browsing.take() {
            Some((at, draft)) => (at as isize + step, draft),
            None if step < 0 => (self.history.len() as isize - 1, current),
            None => return None,
        };
        if at < 0 {
            self.browsing = Some((0, draft));
            return None;
        }
        if at as usize >= self.history.len() {
            return Some(draft);
        }
        self.browsing = Some((at as usize, draft));
        Some(self.history[at as usize].clone())
    }

    /// Adds a command to the shared history, and to the file that keeps
    /// it between sessions.
    fn add_history(&mut self, line: &str) {
        if line.trim().is_empty() {
            return;
        }
        self.history.push(line.to_string());
        if self.history.len() > HISTORY {
            self.history.remove(0);
        }
        self.remember(line);
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
        let editing = self.editing.as_ref().map(|(_, target)| target.thing().2.to_string());
        let sidebar = console.sidebar_editing(editing.as_deref());
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
            self.draw_sidebar(frame, right, &sidebar.fixed, &sidebar.rest, &sidebar.warnings);
            left
        } else {
            frame.render_widget(Paragraph::new(folded(&sidebar)).style(style(Role::Note)), parts[2]);
            body
        };
        match &self.editing {
            Some((editor, target)) => {
                editor.draw(frame, output_area, &console.lookup_for(target));
                if let Some(dropdown) = &self.dropdown {
                    dropdown.draw(frame, output_area, &self.words);
                }
            }
            None if self.quick.is_some() => {
                self.draw_output(frame, output_area);
                if let Some(quick) = &self.quick {
                    quick.draw(frame, output_area, &self.words);
                }
            }
            None => {
                self.draw_output(frame, output_area);
                if let Some(picker) = &self.picker {
                    picker.draw(frame, output_area, &self.words);
                } else if let Some(quick) = &self.quick {
                    quick.draw(frame, output_area, &self.words);
                } else if let Some(menu) = &self.menu {
                    draw_menu(frame, output_area, menu, &self.words);
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
            self.output.iter().flat_map(|line| wrap(line, width).into_iter().map(move |row| (row, style_of(line, &self.kinds)))).collect();
        let height = area.height as usize;
        self.page.set(height);
        let bottom = rows.len().saturating_sub(self.scroll.min(rows.len().saturating_sub(height)));
        let top = bottom.saturating_sub(height);
        let lines: Vec<Line> = rows[top..bottom].iter().map(|(text, style)| output_line(text, *style)).collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_sidebar(&self, frame: &mut Frame, area: Rect, fixed: &[Row], rest: &[Row], warnings: &[Row]) {
        self.sidebar_area.set(area);
        let block = Block::default().borders(Borders::LEFT).border_style(style(Role::Chrome));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        // Problems and warnings at the foot, under a line, at most a third
        // of the height; the panels above take the rest.
        let width = inner.width.saturating_sub(1).max(1) as usize;
        let foot: Vec<Row> = warnings.iter().flat_map(|row| wrap_row(row, width)).collect();
        let foot_height = ((foot.len() as u16) + 1).min(inner.height / 3).max(2.min(inner.height));
        let [inner, bottom] = Layout::vertical([Constraint::Min(0), Constraint::Length(foot_height)]).areas(inner);
        let foot_block = Block::default().borders(Borders::TOP).border_style(style(Role::Chrome));
        let foot_inner = foot_block.inner(bottom);
        frame.render_widget(foot_block, bottom);
        let shown = foot_inner.height as usize;
        let mut lines: Vec<Line> = foot.iter().take(shown).map(row_line).collect();
        if foot.len() > shown && shown > 0 {
            lines.pop();
            lines.push(Line::styled(self.words.text("side_foot_more", &[("count", &(foot.len() - shown + 1).to_string())]), style(Role::Note)));
        }
        frame.render_widget(Paragraph::new(lines), foot_inner);
        // Long rows wrap within the sidebar, keeping their indent.
        let width = inner.width.saturating_sub(1).max(1) as usize;
        let wrapped = |rows: &[Row]| -> Vec<Row> { rows.iter().flat_map(|row| wrap_row(row, width)).collect() };
        // What a click on each of rest's rows does, wrapped rows included.
        let rest_clicks: Vec<Option<Click>> = clicks_of(rest, &self.words)
            .into_iter()
            .zip(rest)
            .flat_map(|(click, row)| std::iter::repeat_n(click, wrap_row(row, width).len()))
            .collect();
        let (fixed, rest) = (wrapped(fixed), wrapped(rest));
        let fixed_height = (fixed.len() as u16).min(inner.height);
        let [top, bottom] = Layout::vertical([Constraint::Length(fixed_height), Constraint::Min(0)]).areas(inner);
        frame.render_widget(Paragraph::new(fixed.iter().map(row_line).collect::<Vec<_>>()), top);

        let height = bottom.height as usize;
        let start = self.sidebar_scroll.min(rest.len().saturating_sub(height));
        let mut clicks = self.clicks.borrow_mut();
        clicks.clear();
        for (i, click) in rest_clicks.iter().enumerate().skip(start).take(height) {
            if let Some(click) = click {
                clicks.push((bottom.y + (i - start) as u16, click.clone()));
            }
        }
        let mut lines: Vec<Line> = rest.iter().skip(start).take(height).map(row_line).collect();
        if rest.len() > start + height && height > 0 {
            lines.pop();
            lines.push(Line::styled(self.words.text("side_more", &[]), style(Role::Note)));
        }
        frame.render_widget(Paragraph::new(lines), bottom);
    }
}

/// The drop-down console over the editor (REQ-CON-71): a command line,
/// and the last reply under it.
struct Dropdown<'a> {
    area: TextArea<'a>,
    reply: Vec<String>,
}

/// The most reply lines the drop-down shows; the output keeps them all.
const DROPDOWN_ROWS: usize = 10;

impl Dropdown<'_> {
    /// Drawn over the top of the editor, between two rules.
    fn draw(&self, frame: &mut Frame, area: Rect, words: &Words) {
        let mut lines = vec![Line::from(format!(" ~ {}▏", self.area.lines().join(" ")))];
        let shown = self.reply.len().min(DROPDOWN_ROWS);
        lines.extend(self.reply[self.reply.len() - shown..].iter().map(|l| Line::from(format!("   {}", l))));
        let height = (lines.len() as u16 + 2).min(area.height);
        let top = Rect { x: area.x, y: area.y, width: area.width, height };
        frame.render_widget(ratatui::widgets::Clear, top);
        let block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(style(Role::Chrome))
            .title_bottom(Line::styled(words.text("dropdown_keys", &[]), style(Role::Chrome)).right_aligned());
        frame.render_widget(Paragraph::new(lines).block(block), top);
    }
}

/// The quick menu: choosing an entry, then asking its questions one at a
/// time (REQ-CON-65).
struct Quick<'a> {
    menu: QuickMenu,
    /// The entry chosen in the list, or being asked.
    selected: usize,
    /// Whether the entry's questions are being asked.
    asking: bool,
    /// Which question, and the answers so far.
    at: usize,
    answers: Vec<String>,
    /// The option chosen in a choice question.
    choice: usize,
    /// Whether "a new state…" was chosen, so the name is being typed.
    naming: bool,
    area: TextArea<'a>,
    /// The thing to open the editor on again afterwards, when the menu
    /// was opened from the editor.
    reopen: Option<(super::world::Kind, String)>,
    /// The editor's file the commands act on, if it was opened there.
    file: Option<String>,
    /// Which screen row picks which entry or option, from the last draw.
    rows: std::cell::RefCell<Vec<(u16, usize)>>,
}

impl<'a> Quick<'a> {
    fn new(menu: QuickMenu) -> Self {
        Quick { menu, selected: 0, asking: false, at: 0, answers: Vec::new(), choice: 0, area: input_line(), reopen: None, file: None, naming: false, rows: Default::default() }
    }

    fn entry(&self) -> &super::quick_menu::Entry {
        &self.menu.entries[self.selected]
    }

    /// Starts the chosen entry's questions; returns whether it has none left.
    fn start(&mut self) -> bool {
        self.asking = true;
        self.at = 0;
        self.ready()
    }

    /// Gets the next question ready; returns whether they're all answered.
    fn ready(&mut self) -> bool {
        self.area = input_line();
        self.naming = false;
        match self.entry().asks.get(self.at) {
            None => true,
            Some(Ask::Choose { default, .. }) => {
                self.choice = *default;
                false
            }
            Some(Ask::Text { .. }) => false,
        }
    }

    /// One key. Returns whether the entry is answered and should run.
    fn key(&mut self, input: Input) -> bool {
        let count = self.menu.entries.len();
        if !self.asking {
            return match input.key {
                Key::Up => {
                    self.selected = (self.selected + count - 1) % count;
                    false
                }
                Key::Down => {
                    self.selected = (self.selected + 1) % count;
                    false
                }
                Key::Enter => self.start(),
                Key::Char(c) if !input.ctrl && !input.alt => match self.menu.entries.iter().position(|e| e.letter == c) {
                    Some(i) => {
                        self.selected = i;
                        self.start()
                    }
                    None => false,
                },
                _ => false,
            };
        }
        let ask = self.entry().asks[self.at].clone();
        let answer = match ask {
            // "A new state…" asks for its name as a typed answer.
            Ask::Choose { .. } if self.naming => match input.key {
                Key::Enter => {
                    let typed = self.area.lines().join(" ").trim().to_string();
                    (!typed.is_empty()).then_some(typed)
                }
                _ => {
                    self.area.input(input);
                    None
                }
            },
            Ask::Choose { options, new, .. } => {
                let count = options.len() + usize::from(new);
                match input.key {
                    Key::Up => {
                        self.choice = (self.choice + count - 1) % count;
                        None
                    }
                    Key::Down => {
                        self.choice = (self.choice + 1) % count;
                        None
                    }
                    Key::Enter if self.choice == options.len() => {
                        self.naming = true;
                        None
                    }
                    Key::Enter => Some(options[self.choice].clone()),
                    _ => None,
                }
            }
            Ask::Text { required, .. } => match input.key {
                Key::Enter => {
                    let typed = self.area.lines().join(" ").trim().to_string();
                    (!required || !typed.is_empty()).then_some(typed)
                }
                _ => {
                    self.area.input(input);
                    None
                }
            },
        };
        let Some(answer) = answer else { return false };
        self.answers.push(answer);
        self.at += 1;
        self.ready()
    }

    fn commands(&self) -> Vec<String> {
        self.entry().commands(&self.answers)
    }

    /// A click on screen row `y`: picks the entry or option there, as
    /// moving to it and pressing Enter would. Returns whether to run.
    fn click(&mut self, y: u16) -> bool {
        let Some(pick) = self.rows.borrow().iter().find(|(row, _)| *row == y).map(|(_, p)| *p) else { return false };
        if self.asking {
            self.choice = pick;
        } else {
            self.selected = pick;
        }
        self.key(Input { key: Key::Enter, ..Input::default() })
    }

    /// Drawn on the output's last rows, just above the input.
    fn draw(&self, frame: &mut Frame, area: Rect, words: &Words) {
        let heading = words.text("qm_heading", &[("id", &self.menu.id), ("kind", self.menu.kind.label())]);
        let mut lines = vec![Line::styled(heading, style(Role::Chrome))];
        let reversed = Style::default().add_modifier(ratatui::style::Modifier::REVERSED);
        // (line index, what it picks), turned into screen rows below.
        let mut picks: Vec<(usize, usize)> = Vec::new();
        let keys;
        if !self.asking {
            let width = self.menu.entries.iter().map(|e| words.text(e.label, &[]).chars().count()).max().unwrap_or(0);
            for (i, e) in self.menu.entries.iter().enumerate() {
                let text = format!(" {}  {:<width$}  {}", e.letter, words.text(e.label, &[]), e.stands_for, width = width);
                let line = Line::from(text);
                picks.push((lines.len(), i));
                lines.push(if i == self.selected { line.style(reversed) } else { line });
            }
            keys = words.text("qm_keys_menu", &[]);
        } else {
            let entry = self.entry();
            lines.push(Line::styled(format!(" {}", words.text(entry.label, &[])), style(Role::Title)));
            for (ask, answer) in entry.asks.iter().zip(&self.answers) {
                lines.push(Line::styled(format!(" {} {}", words.text(ask.key(), &[]), answer), style(Role::Note)));
            }
            let ask = &entry.asks[self.at];
            let facts: Vec<(&str, &str)> = self.menu.facts.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let hint = Line::styled(format!(" {}", words.text(ask.hint(), &facts)), style(Role::Note));
            match ask {
                Ask::Choose { key, .. } if self.naming => {
                    lines.push(Line::from(format!(" {}", words.text(key, &[]))));
                    lines.push(hint);
                    lines.push(Line::from(format!(" {} {}▏", words.text("qm_ask_new_state", &[]), self.area.lines().join(" "))));
                    keys = words.text("qm_keys_typing", &[]);
                }
                Ask::Choose { key, options, new, describe, .. } => {
                    lines.push(Line::from(format!(" {}", words.text(key, &[]))));
                    lines.push(hint);
                    let shown = |o: &str| if o == "any" { words.text("qm_any_state", &[]) } else { o.to_string() };
                    let width = options.iter().map(|o| shown(o).chars().count()).max().unwrap_or(0);
                    let mut rows: Vec<String> = options
                        .iter()
                        .map(|o| match describe {
                            Some(prefix) => format!("   {:<width$}  {}", shown(o), words.text(&format!("{}_{}", prefix, o.replace('-', "_")), &[]), width = width),
                            None => format!("   {}", shown(o)),
                        })
                        .collect();
                    if *new {
                        rows.push(format!("   {}", words.text("qm_new_state", &[])));
                    }
                    for (i, row) in rows.into_iter().enumerate() {
                        let line = Line::from(row);
                        picks.push((lines.len(), i));
                        lines.push(if i == self.choice { line.style(reversed) } else { line });
                    }
                    keys = words.text("qm_keys_choose", &[]);
                }
                Ask::Text { key, .. } => {
                    lines.push(Line::from(format!(" {}", words.text(key, &[]))));
                    lines.push(hint);
                    lines.push(Line::from(format!(" > {}▏", self.area.lines().join(" "))));
                    keys = words.text("qm_keys_typing", &[]);
                }
            }
        }
        let height = (lines.len() as u16 + 2).min(area.height);
        let popup = Rect { x: area.x, y: area.bottom().saturating_sub(height), width: area.width, height };
        // Under the top rule; rows cut off by a short screen aren't clickable.
        *self.rows.borrow_mut() = picks.into_iter().filter(|(line, _)| (*line as u16 + 1) < height.saturating_sub(1)).map(|(line, pick)| (popup.y + 1 + line as u16, pick)).collect();
        frame.render_widget(ratatui::widgets::Clear, popup);
        let block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(style(Role::Chrome))
            .title_bottom(Line::styled(keys, style(Role::Chrome)).right_aligned());
        frame.render_widget(Paragraph::new(lines).block(block), popup);
    }
}


/// Questions being asked about one new thing.
struct Picker<'a> {
    asking: Asking,
    /// Which question.
    at: usize,
    selected: usize,
    step: Step<'a>,
    answers: Vec<Answer>,
    /// Whether to open the editor on the thing afterwards.
    reopen: bool,
}

/// Where a question is: choosing an option, or giving what it needs.
enum Step<'a> {
    Choose,
    Typing { choice: Choice, prompt: String, area: TextArea<'a> },
    Picking { choice: Choice, things: Vec<String>, selected: usize },
}

impl<'a> Picker<'a> {
    fn new(asking: Asking, reopen: bool) -> Self {
        let selected = default_index(&asking.set.questions[0]);
        Picker { asking, at: 0, selected, step: Step::Choose, answers: Vec::new(), reopen }
    }

    fn done(&self) -> bool {
        self.at >= self.asking.set.questions.len()
    }

    /// Records an answer and moves to the next question.
    fn answer(&mut self, choice: Choice, given: Option<String>) {
        self.answers.push(Answer { choice, given });
        self.at += 1;
        self.step = Step::Choose;
        if let Some(next) = self.asking.set.questions.get(self.at) {
            self.selected = default_index(next);
        }
    }

    /// Esc: the defaults for this question and the rest. A default that
    /// needs typing takes its own default value, or is left out.
    fn use_defaults(&mut self) {
        while let Some(question) = self.asking.set.questions.get(self.at) {
            let Some(choice) = question.default_choice().cloned() else {
                self.at += 1;
                continue;
            };
            let given = match needs(&choice) {
                Some(Needs::Text { default, .. }) => default,
                _ => None,
            };
            self.answer(choice, given);
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect, words: &Words) {
        let set = &self.asking.set;
        let Some(question) = set.questions.get(self.at) else { return };
        let mut lines = vec![
            Line::styled(
                words.text(
                    "asking_heading",
                    &[
                        ("id", &self.asking.id),
                        ("path", &self.asking.shown_path),
                        ("at", &(self.at + 1).to_string()),
                        ("of", &set.questions.len().to_string()),
                    ],
                ),
                style(Role::Chrome),
            ),
            Line::from(""),
            Line::styled(format!(" {}", question.text), style(Role::Title)),
        ];
        if !question.hint.is_empty() {
            lines.push(Line::styled(format!(" {}", question.hint), style(Role::Note)));
        }
        lines.push(Line::from(""));
        match &self.step {
            Step::Choose => {
                for (i, option) in question.options.iter().enumerate() {
                    let mark = if i == self.selected { "▸" } else { " " };
                    let mut line = Line::from(format!(" {} {}. {}", mark, i + 1, option.label));
                    if i == self.selected {
                        line = line.style(Style::default().add_modifier(ratatui::style::Modifier::REVERSED));
                    }
                    lines.push(line);
                }
            }
            Step::Typing { prompt, area, .. } => {
                lines.push(Line::from(format!(" {} {}", prompt, area.lines().join(" "))));
            }
            Step::Picking { things, selected, .. } => {
                lines.push(Line::styled(format!(" {}", words.text("which_one", &[])), style(Role::Note)));
                for (i, thing) in things.iter().enumerate() {
                    let mark = if i == *selected { "▸" } else { " " };
                    let mut line = Line::from(format!(" {} {}", mark, thing));
                    if i == *selected {
                        line = line.style(Style::default().add_modifier(ratatui::style::Modifier::REVERSED));
                    }
                    lines.push(line);
                }
            }
        }
        let keys = match self.step {
            Step::Choose => words.text("asking_keys_choose", &[]),
            Step::Typing { .. } => words.text("asking_keys_typing", &[]),
            Step::Picking { .. } => words.text("asking_keys_picking", &[]),
        };
        let height = (lines.len() as u16 + 2).min(area.height);
        let popup = Rect { x: area.x, y: area.bottom().saturating_sub(height), width: area.width, height };
        frame.render_widget(ratatui::widgets::Clear, popup);
        let block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(style(Role::Chrome))
            .title_bottom(Line::styled(keys, style(Role::Chrome)).right_aligned());
        frame.render_widget(Paragraph::new(lines).block(block), popup);
    }
}

fn default_index(question: &crate::engines::questions::Question) -> usize {
    question.options.iter().position(|o| o.default).unwrap_or(0)
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
fn draw_menu(frame: &mut Frame, area: Rect, menu: &Menu, words: &Words) {
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
        lines.push(Line::styled(words.text("menu_more", &[("count", &(items.len() - start - shown).to_string())]), style(Role::Note)));
    }
    // The menu sits on the output's last rows, between two rules; the keys
    // ride on the lower rule at the right, just above the prompt.
    let height = (lines.len() as u16 + 2).min(area.height);
    let popup = Rect { x: area.x, y: area.bottom().saturating_sub(height), width: area.width, height };
    frame.render_widget(ratatui::widgets::Clear, popup);
    let block = Block::default().borders(Borders::TOP | Borders::BOTTOM).border_style(style(Role::Chrome));
    let keys = if items.is_empty() { words.text("menu_keys_empty", &[]) } else { words.text("menu_keys", &[]) };
    let block = block.title_bottom(Line::styled(keys, style(Role::Chrome)).right_aligned());
    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

/// The screen's own lines, read from `[console]` when the screen opens.
struct Words {
    texts: crate::engines::texts::Texts,
}

impl Words {
    /// `key`'s `[console]` text with `args` filled.
    fn text(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.texts.text("console", key, args)
    }
}

impl From<&crate::engines::texts::Texts> for Words {
    fn from(texts: &crate::engines::texts::Texts) -> Self {
        Words { texts: texts.clone() }
    }
}

/// Runs the full-screen console until `/quit` or Ctrl-D.
pub fn run(console: Arc<Mutex<Console>>, notices: Receiver<String>, resumed: Option<Resumed>) -> Result<()> {
    use ratatui::crossterm::event::{self, Event, KeyEventKind};
    let greeting = lock(&console).greeting();
    let was_resumed = resumed.is_some();
    let mut screen = Screen::new(&greeting, resumed);
    screen.words = Words::from(&lock(&console).texts);
    screen.kinds = super::kinds::Kinds::new(&lock(&console).texts);
    if let Some(file) = lock(&console).history_file() {
        screen.keep_history(file, was_resumed);
    }
    let mut terminal = enter_terminal()?;
    let result = (|| -> Result<()> {
        loop {
            draw(&mut terminal, &screen, &console)?;
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key) if key.kind != KeyEventKind::Release => screen.key(input_of(key), &console),
                    Event::Mouse(mouse) => screen.mouse(mouse, &console),
                    _ => {}
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
pub(super) fn enter_terminal() -> std::io::Result<ratatui::DefaultTerminal> {
    use ratatui::crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
    let terminal = ratatui::try_init()?;
    let _ = ratatui::crossterm::execute!(
        std::io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        // Clicks in the sidebar (REQ-CON-69); Shift-drag still selects.
        ratatui::crossterm::event::EnableMouseCapture
    );
    clear_screen()?;
    Ok(terminal)
}

/// Gives the terminal back as it was.
pub(super) fn leave_terminal() {
    let _ = ratatui::crossterm::execute!(
        std::io::stdout(),
        ratatui::crossterm::event::DisableMouseCapture,
        ratatui::crossterm::event::PopKeyboardEnhancementFlags
    );
    ratatui::restore();
}

/// A key as the console takes it. Command-S, which a Mac user saves with,
/// is Ctrl-S; other Command keys stay with the terminal (copy, paste).
pub(super) fn input_of(key: ratatui::crossterm::event::KeyEvent) -> Input {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};
    if key.modifiers.contains(KeyModifiers::SUPER) && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S')) {
        return Input { key: Key::Char('s'), ctrl: true, ..Input::default() };
    }
    // Ctrl-1 to 9 pick a TO WRITE entry, as Alt-1 to 9 do (REQ-CON-59,
    // Bruce, 2026-11-24). Only a real Ctrl-digit from the kitty protocol
    // counts: a terminal without it sends NUL, Esc or a control code,
    // which crossterm never reports as a digit, so Esc stays Esc.
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c @ '1'..='9') = key.code {
            return Input { key: Key::Char(c), alt: true, ..Input::default() };
        }
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
/// What a click on a sidebar row does (REQ-CON-69).
#[derive(Debug, Clone, PartialEq)]
enum Click {
    QuickMenu,
    /// The nth TO WRITE entry, from 1.
    ToWrite(usize),
    /// Go through this exit.
    Exit(String),
}

/// The click for each row: exits by their direction, TO WRITE entries by
/// their place in the list, and the quick menu's hint.
fn clicks_of(rows: &[Row], words: &Words) -> Vec<Option<Click>> {
    let to_write = words.text("side_to_write", &[]);
    let hints = [words.text("side_quick_menu", &[]), words.text("side_quick_menu_editor", &[])];
    let mut in_to_write = None;
    rows.iter()
        .map(|row| match row {
            Row::Heading(t) => {
                in_to_write = (t.trim() == to_write.trim()).then_some(0);
                None
            }
            Row::Blank => {
                in_to_write = None;
                None
            }
            Row::Exit(t, _) => {
                let direction = t.trim().split(" -").next().unwrap_or("").trim().to_string();
                (!direction.is_empty()).then_some(Click::Exit(direction))
            }
            Row::Dim(t) if hints.contains(t) => Some(Click::QuickMenu),
            Row::Text(_) if in_to_write.is_some() => {
                let n = in_to_write.map(|n| n + 1).unwrap_or(1);
                in_to_write = Some(n);
                Some(Click::ToWrite(n))
            }
            _ => None,
        })
        .collect()
}

fn wrap_row(row: &Row, width: usize) -> Vec<Row> {
    let (text, make): (&str, fn(String) -> Row) = match row {
        Row::Heading(t) => (t, Row::Heading),
        Row::Text(t) => (t, Row::Text),
        Row::Dim(t) => (t, Row::Dim),
        Row::Warning(t) => (t, Row::Warning),
        Row::Error(t) => (t, Row::Error),
        // A map row keeps its shape: cut at the edge, never wrapped.
        Row::Map(t) => return vec![Row::Map(t.chars().take(width).collect())],
        Row::Exit(t, missing) => {
            let cut = if t.chars().count() > width { format!("{}…", t.chars().take(width.saturating_sub(1)).collect::<String>()) } else { t.clone() };
            return vec![Row::Exit(cut, *missing)];
        }
        Row::Blank => return vec![Row::Blank],
    };
    // The first row as it is; the rest indented two more, so they're
    // wrapped two narrower, or the edge would cut their last characters.
    let indent = text.chars().take_while(|c| *c == ' ').count();
    let rows = wrap(text, width);
    let Some(first) = rows.first().cloned() else { return vec![Row::Blank] };
    let rest: String = rows[1..].iter().map(|r| r.trim()).collect::<Vec<_>>().join(" ");
    let mut out = vec![make(first)];
    if !rest.is_empty() {
        let pad = " ".repeat(indent + 2);
        for line in wrap(&rest, width.saturating_sub(indent + 2).max(1)) {
            out.push(make(format!("{}{}", pad, line)));
        }
    }
    out
}

fn row_line(row: &Row) -> Line<'static> {
    let styled = |text: &str, role: Role| Line::from(code_spans(&format!(" {}", text), style(role)));
    match row {
        Row::Heading(text) => styled(text, Role::Heading),
        Row::Text(text) => Line::from(code_spans(&format!(" {}", text), Style::default())),
        Row::Dim(text) => styled(text, Role::Note),
        Row::Warning(text) => styled(text, Role::Warning),
        Row::Error(text) => styled(text, Role::Error),
        Row::Map(text) => styled(text, Role::Link),
        Row::Exit(text, true) => styled(text, Role::Missing),
        Row::Exit(text, false) => Line::from(format!(" {}", text)),
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
    Line::from(code_spans(&format!(" {}", text), row_style))
}

/// `text` as spans, with each `code` part in the code style and its
/// backticks dropped, as a terminal shows them (Bruce, 2026-11-24).
fn code_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, part) in text.split('`').enumerate() {
        if part.is_empty() {
            continue;
        }
        let style = if i % 2 == 1 { base.patch(style(Role::Code)) } else { base };
        spans.push(Span::styled(part.to_string(), style));
    }
    // An odd number of backticks: show the text as it was.
    if text.matches('`').count() % 2 == 1 {
        return vec![Span::styled(text.to_string(), base)];
    }
    spans
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
fn style_of(line: &str, kinds: &super::kinds::Kinds) -> Style {
    line_role(line, kinds).map(style).unwrap_or_default()
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
            Console::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny"), super::super::Mode::Edit)
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
        assert!(text.contains("HERE  Village Square"), "{}", text);
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
        assert!(text.contains("HERE  Village Square"), "{}", text);
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
        let mut terminal = Terminal::new(TestBackend::new(110, 30)).unwrap();
        draw(&mut terminal, &screen, &console).unwrap();
        let buffer = terminal.backend().buffer();
        let row = |y: u16| (0..110).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>();
        assert_eq!(buffer[(0, 0)].bg, Color::Blue, "the status bar");
        let border = row(1).chars().position(|c| c == '│').unwrap() as u16;
        assert_eq!(buffer[(border, 1)].fg, Color::Blue, "the sidebar's border");
        // The echoed command and the one being typed.
        let echo = (0..30).find(|&y| row(y).contains("> /status")).unwrap();
        let slash = row(echo).find('/').unwrap() as u16;
        assert_eq!(buffer[(slash, echo)].fg, Color::Yellow);
        let input = row(29).find("/move").unwrap() as u16;
        assert_eq!(buffer[(input, 29)].fg, Color::Yellow);
        assert!(buffer[(input + 1, 29)].modifier.contains(ratatui::style::Modifier::BOLD));
        assert_ne!(buffer[(input + 7, 29)].fg, Color::Yellow, "arguments stay plain");
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
        keys(&mut screen, &console, "res");
        let text = screen_text(&screen, &console, 110, 24);
        assert!(text.contains("/restart"), "{}", text);
        assert!(text.contains("tab or enter pick"), "{}", text);
        press(&mut screen, &console, Key::Tab);
        assert_eq!(screen.input_text(), "/restart");
    }

    #[test]
    fn enter_picks_and_runs_unless_more_is_needed() {
        let console = tiny();
        let mut screen = Screen::new("", None);
        keys(&mut screen, &console, "/statu");
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
    fn history_is_kept_between_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history/tiny.txt");
        let console = tiny();
        let mut first = Screen::new("", None);
        first.keep_history(file.clone(), false);
        typed(&mut first, &console, "go /village/square");
        typed(&mut first, &console, "look");

        let mut second = Screen::new("", None);
        second.keep_history(file, false);
        assert!(second.output.is_empty(), "nothing shown until Up: {:?}", second.output);
        press(&mut second, &console, Key::Up);
        assert_eq!(second.input_text(), "look", "Up reaches last session");
    }

    #[test]
    fn a_new_thing_is_asked_its_questions_and_esc_takes_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tiny");
        for entry in walkdir::WalkDir::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny")) {
            let entry = entry.unwrap();
            let to = root.join(entry.path().strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny")).unwrap());
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&to).unwrap();
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
        let mut console = Console::open(&root, super::super::Mode::Edit).unwrap();
        console.interactive = true;
        let console = Mutex::new(console);
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "go /village/square");
        typed(&mut screen, &console, "/add rock");
        assert!(screen.picker.is_some(), "the built-in object questions are asked");
        let text = screen_text(&screen, &console, 110, 24);
        assert!(text.contains("Where does it start?"), "{}", text);
        assert!(text.contains("<built-in>"), "{}", text);
        press(&mut screen, &console, Key::Esc);
        assert!(screen.picker.is_none());
        assert!(screen.output.iter().any(|l| l.starts_with("Made rock's card.")), "{:?}", screen.output);
        assert!(screen.editing.is_some(), "then the editor opens on it");
    }

    #[test]
    fn every_reply_has_air_before_and_after_and_never_two_blanks() {
        let console = tiny();
        let mut screen = Screen::new("Hello.", None);
        typed(&mut screen, &console, "go /village/square");
        typed(&mut screen, &console, "look");
        let at = screen.output.iter().position(|l| l == "> look").unwrap();
        assert_eq!(screen.output[at - 1], "", "a blank line before the echo");
        assert_eq!(screen.output[at + 1], "", "and after it");
        assert_eq!(screen.output.last().unwrap(), "", "and after the reply, before the prompt");
        assert!(!screen.output.windows(2).any(|w| w[0].is_empty() && w[1].is_empty()), "{:?}", screen.output);
    }

    #[test]
    fn backticked_code_drops_its_backticks_and_takes_the_code_style() {
        let spans = code_spans("leads to `village/inn`, see", Style::default());
        let text: String = spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(text, "leads to village/inn, see");
        assert_eq!(spans[1].style, style(Role::Code));
    }

    #[test]
    fn wrapped_sidebar_rows_keep_every_character() {
        let row = Row::Warning("  `tshirt` starts in more than one place: room `hallway`; play shows it in each".into());
        for width in [20, 28, 33, 40] {
            let rows = wrap_row(&row, width);
            let text: String = rows
                .iter()
                .map(|r| match r {
                    Row::Warning(t) => t.trim().to_string(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            assert!(text.ends_with("play shows it in each"), "width {}: {:?}", width, rows);
            assert!(rows.iter().all(|r| matches!(r, Row::Warning(t) if t.chars().count() <= width)), "width {}: {:?}", width, rows);
        }
    }

    #[test]
    fn adding_a_new_thing_elsewhere_puts_it_in_one_room_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tiny");
        for entry in walkdir::WalkDir::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny")) {
            let entry = entry.unwrap();
            let to = root.join(entry.path().strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny")).unwrap());
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&to).unwrap();
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
        let mut console = Console::open(&root, super::super::Mode::Edit).unwrap();
        console.interactive = true;
        let console = Mutex::new(console);
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "go /village/square");
        typed(&mut screen, &console, "/add desk village/well");
        press(&mut screen, &console, Key::Esc);
        let square = std::fs::read_to_string(root.join("village/square.md")).unwrap();
        let well = std::fs::read_to_string(root.join("village/well.md")).unwrap();
        assert!(well.contains("desk"), "{}", well);
        assert!(!square.contains("desk"), "not in edit mode's room too: {}", square);
    }

    #[test]
    fn the_quick_menu_asks_at_the_bottom_then_runs_and_shows_the_command() {
        let dir = tempfile::tempdir().unwrap();
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        let root = dir.path().join("tiny");
        for entry in walkdir::WalkDir::new(&from) {
            let entry = entry.unwrap();
            let to = root.join(entry.path().strip_prefix(&from).unwrap());
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&to).unwrap();
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
        let mut console = Console::open(&root, super::super::Mode::Edit).unwrap();
        console.interactive = true;
        let console = Mutex::new(console);
        let mut screen = Screen::new("", None);
        let ctrl_m = Input { key: Key::Char('m'), ctrl: true, ..Input::default() };
        // Nowhere to be yet: it says so rather than opening.
        screen.key(ctrl_m.clone(), &console);
        assert!(screen.quick.is_none());
        typed(&mut screen, &console, "go /village/square");
        // Ctrl-M opens it; Ctrl-Space doesn't, since macOS takes it.
        screen.key(Input { key: Key::Char(' '), ctrl: true, ..Input::default() }, &console);
        assert!(screen.quick.is_none(), "Ctrl-Space is retired");
        screen.key(ctrl_m.clone(), &console);
        let text = screen_text(&screen, &console, 110, 30);
        assert!(text.contains("Quick menu: village/square") && text.contains("add a door here"), "{}", text);
        press(&mut screen, &console, Key::Esc);
        // From the editor: it saves, asks about the thing being edited, and
        // the editor comes back afterwards, even after Esc.
        typed(&mut screen, &console, "/edit");
        assert!(screen.editing.is_some());
        screen.key(ctrl_m.clone(), &console);
        assert!(screen.editing.is_none() && screen.quick.is_some(), "Ctrl-M isn't a new line in the editor");
        press(&mut screen, &console, Key::Esc);
        assert!(screen.editing.is_some(), "Esc goes back to the editor");
        press(&mut screen, &console, Key::Esc);
        assert!(screen.editing.is_none());
        // An editor opened from the world root: the hint follows the open
        // file and shows only the key the editor has; Ctrl-M leaves edit
        // mode's place alone.
        typed(&mut screen, &console, "go /");
        let place = lock(&console).pwd();
        typed(&mut screen, &console, "/edit lantern");
        assert!(screen.editing.is_some());
        let text = screen_text(&screen, &console, 120, 40);
        assert!(text.contains("⌃M  quick menu") && !text.contains("/menu  quick menu"), "{}", text);
        screen.key(ctrl_m.clone(), &console);
        assert!(screen.quick.as_ref().is_some_and(|q| q.menu.id == "lantern"));
        press(&mut screen, &console, Key::Esc);
        assert_eq!(lock(&console).pwd(), place, "place didn't move");
        press(&mut screen, &console, Key::Esc);
        assert!(screen.editing.is_none());
        typed(&mut screen, &console, "go /village/square");
        typed(&mut screen, &console, "/menu");
        assert!(screen.quick.is_some(), "/menu opens it");
        // l: link to, then a direction and a room, one question at a time.
        keys(&mut screen, &console, "l");
        assert!(screen_text(&screen, &console, 110, 30).contains("Which direction?"));
        press(&mut screen, &console, Key::Enter);
        assert!(screen.quick.is_some(), "a required answer can't be blank");
        keys(&mut screen, &console, "south");
        press(&mut screen, &console, Key::Enter);
        keys(&mut screen, &console, "well");
        press(&mut screen, &console, Key::Enter);
        assert!(screen.quick.is_none());
        let square = std::fs::read_to_string(root.join("village/square.md")).unwrap();
        assert!(square.contains("south"), "{}", square);
        let out = screen.output.join("\n");
        assert!(out.contains("Typed, that was:") && out.contains("  /link south well"), "{}", out);
    }

    #[test]
    fn tilde_on_an_empty_line_drops_a_console_down_over_the_editor() {
        let dir = tempfile::tempdir().unwrap();
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        let root = dir.path().join("tiny");
        for entry in walkdir::WalkDir::new(&from) {
            let entry = entry.unwrap();
            let to = root.join(entry.path().strip_prefix(&from).unwrap());
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&to).unwrap();
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
        let mut console = Console::open(&root, super::super::Mode::Edit).unwrap();
        console.interactive = true;
        let console = Mutex::new(console);
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "/edit lantern");
        let place = lock(&console).pwd();
        // ~ after text is text.
        keys(&mut screen, &console, "x~");
        assert!(screen.dropdown.is_none());
        press(&mut screen, &console, Key::Backspace);
        press(&mut screen, &console, Key::Backspace);
        // On an empty line it opens; the unsaved new line is saved first.
        press(&mut screen, &console, Key::Enter);
        keys(&mut screen, &console, "~");
        assert!(screen.dropdown.is_some(), "on an empty line");
        assert!(screen_text(&screen, &console, 120, 40).contains("Esc or ~ closes"));
        keys(&mut screen, &console, "/state lit");
        press(&mut screen, &console, Key::Enter);
        assert!(root.join("objects/lantern/lit.md").exists() || root.join("objects/lantern/index.md").exists(), "a bundle now");
        assert!(screen.editing.is_some(), "the editor reopened on the bundle");
        assert!(screen.dropdown.as_ref().is_some_and(|d| !d.reply.is_empty()), "the reply shows under the line");
        assert_eq!(lock(&console).pwd(), place, "edit mode's place didn't move");
        // ~ on the empty command line closes it.
        keys(&mut screen, &console, "~");
        assert!(screen.dropdown.is_none());
        assert!(screen.editing.is_some());
        // One history: Up in the drop-down reaches the prompt's commands,
        // and the drop-down's commands are there for the prompt.
        keys(&mut screen, &console, "~");
        press(&mut screen, &console, Key::Up);
        assert_eq!(screen.dropdown.as_ref().unwrap().area.lines().join(""), "/state lit");
        press(&mut screen, &console, Key::Up);
        assert_eq!(screen.dropdown.as_ref().unwrap().area.lines().join(""), "/edit lantern");
        press(&mut screen, &console, Key::Esc);
        press(&mut screen, &console, Key::Esc);
        assert!(screen.editing.is_none());
        press(&mut screen, &console, Key::Up);
        assert_eq!(screen.input_text(), "/state lit");
    }

    #[test]
    fn a_click_in_the_sidebar_does_what_its_key_does() {
        use ratatui::crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let dir = tempfile::tempdir().unwrap();
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        let root = dir.path().join("tiny");
        for entry in walkdir::WalkDir::new(&from) {
            let entry = entry.unwrap();
            let to = root.join(entry.path().strip_prefix(&from).unwrap());
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&to).unwrap();
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
        let mut console = Console::open(&root, super::super::Mode::Edit).unwrap();
        console.interactive = true;
        let console = Mutex::new(console);
        let mut screen = Screen::new("", None);
        typed(&mut screen, &console, "go /village/square");
        // Clicks the first sidebar row showing `text`.
        let click = |screen: &mut Screen, text: &str| {
            let shown = screen_text(screen, &console, 140, 50);
            let side = screen.sidebar_area.get();
            let row = shown.lines().position(|l| l.chars().skip(side.x as usize).collect::<String>().contains(text)).unwrap_or_else(|| panic!("{} in {}", text, shown));
            let event = MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: side.x + 2, row: row as u16, modifiers: KeyModifiers::NONE };
            screen.mouse(event, &console);
        };
        click(&mut screen, "quick menu");
        assert!(screen.quick.is_some(), "the hint opens the menu");
        press(&mut screen, &console, Key::Esc);
        click(&mut screen, "north");
        assert!(lock(&console).pwd().contains("well"), "the exit is gone through: {}", lock(&console).pwd());
        typed(&mut screen, &console, "go /village/square");
        click(&mut screen, "⌃1");
        assert!(screen.editing.is_some(), "the TO WRITE entry is written and opened");
        press(&mut screen, &console, Key::Esc);
        // In the menu's popup a click picks the entry, then the option.
        let click_at = |screen: &mut Screen, text: &str| {
            let shown = screen_text(screen, &console, 140, 50);
            let row = shown.lines().position(|l| l.contains(text)).unwrap_or_else(|| panic!("{} in {}", text, shown));
            let event = MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 3, row: row as u16, modifiers: KeyModifiers::NONE };
            screen.mouse(event, &console);
        };
        typed(&mut screen, &console, "go /village/square");
        typed(&mut screen, &console, "/menu");
        click_at(&mut screen, "add state");
        assert!(screen.quick.as_ref().is_some_and(|q| q.asking && q.entry().label == "qm_state"));
        keys(&mut screen, &console, "lit");
        press(&mut screen, &console, Key::Enter);
        click_at(&mut screen, "   start");
        assert!(screen.quick.is_none(), "the option ran the entry");
        assert!(screen.output.join("\n").contains("/state lit start"));
        // The wheel over the sidebar scrolls it, not the output.
        let side = screen.sidebar_area.get();
        let wheel = MouseEvent { kind: MouseEventKind::ScrollDown, column: side.x + 2, row: side.y + 2, modifiers: KeyModifiers::NONE };
        screen.mouse(wheel, &console);
        assert_eq!(screen.sidebar_scroll, 1);
    }

    #[test]
    fn command_number_writes_that_entry_and_opens_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("hall.md"), "---\nid: hall\ntitle: Hall\ntype: location\n---\nPeeling [[wallpaper]], and @wallpaper again. A [[hole]].\n").unwrap();
        let mut console = Console::open(root, super::super::Mode::Edit).unwrap();
        console.interactive = true;
        console.handle("go hall");
        let rows: Vec<String> = console.to_write_panel().iter().map(|r| format!("{:?}", r)).collect();
        assert!(rows.iter().any(|r| r.contains("wallpaper") && r.contains("×2")), "{:?}", rows);
        let console = Mutex::new(console);
        let mut screen = Screen::new("", None);
        use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        screen.key(input_of(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::CONTROL)), &console);
        assert!(screen.editing.is_some(), "the new card opens: {:?}", screen.output);
        assert!(root.join("objects/hole.md").exists());
        let hall = std::fs::read_to_string(root.join("hall.md")).unwrap();
        assert!(hall.contains("hole"), "listed in the room that names it: {}", hall);
        assert!(lock(&console).to_write().iter().all(|e| e.name != "hole"));
        // From the editor: an unsaved change is saved, then the next opens.
        screen.key(Input { key: Key::Char('!'), ..Input::default() }, &console);
        screen.key(input_of(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL)), &console);
        assert!(std::fs::read_to_string(root.join("objects/hole.md")).unwrap().contains('!'), "saved before switching");
        assert!(root.join("objects/wallpaper.md").exists());
        assert!(screen.editing.is_some());
        // Cmd-digits are the terminal's again, and Esc is never a key here.
        assert_eq!(input_of(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::SUPER)).alt, false);
        assert_eq!(input_of(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).key, Key::Esc);
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
