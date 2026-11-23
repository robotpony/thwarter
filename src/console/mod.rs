//! The console: one interactive client for writing (edit mode) and
//! reading (play mode) a world (`design/CONSOLE_REQUIREMENTS.md`).
//!
//! `thwart edit` and `thwart play` hand off to `run` (REQ-CON-1). Tests
//! drive the same loop through `run_script`, which echoes each input line
//! as `> line` instead of printing a prompt, so its output reads as a
//! transcript (REQ-CON-9).

mod author;
mod play;
mod saves;
mod session;
mod verbs;
mod view;
mod watch;
mod world;

use anyhow::Result;
use session::{Console, Flow};
use reedline::{
    ExternalPrinter, FileBackedHistory, Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, Reedline, Signal,
};
use std::borrow::Cow;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Which mode the console is in. `/edit` and `/play` switch between them
/// inside a running console (REQ-CON-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Sees everything and changes content (REQ-CON-3).
    Edit,
    /// In-game verbs only; never changes content files (REQ-CON-3).
    Play,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Edit => "edit",
            Mode::Play => "play",
        }
    }
}

/// Opens the console on the world at `world_path` in `mode`, reading
/// from stdin until `/quit` or end of input.
pub fn run(world_path: &Path, mode: Mode) -> Result<()> {
    let mut console = Console::open(world_path, mode)?;
    let stdin = io::stdin();
    if stdin.is_terminal() && io::stdout().is_terminal() {
        drive_terminal(console, world_path)
    } else {
        drive(&mut console, stdin.lock(), io::stdout().lock())
    }
}

/// The interactive loop: a line editor with history (arrow keys,
/// Ctrl-R search) on a real terminal. Ctrl-C clears the line; Ctrl-D
/// leaves, like `/quit`. World files are watched, and a change reloads
/// the world and prints what happened above the prompt (REQ-CON-7).
fn drive_terminal(console: Console, world_path: &Path) -> Result<()> {
    let console = Arc::new(Mutex::new(console));
    let notices = ExternalPrinter::<String>::default();
    let _watching = match watch::watch(world_path, console.clone(), notices.sender()) {
        Ok(handle) => Some(handle),
        Err(e) => {
            println!("Can't watch the world's files ({}); use /reload after changes.", e);
            None
        }
    };
    let mut editor = Reedline::create()
        .with_history(Box::new(FileBackedHistory::new(HISTORY_LINES)?))
        .with_external_printer(notices);
    println!("{}", lock(&console).greeting());
    loop {
        let prompt = ConsolePrompt(lock(&console).prompt());
        let line = match editor.read_line(&prompt)? {
            Signal::Success(line) => line,
            Signal::CtrlC(_) => continue,
            Signal::CtrlD => return Ok(()),
            _ => continue,
        };
        let (text, flow) = lock(&console).handle(&line);
        if !text.is_empty() {
            println!("{}", text);
        }
        if flow == Flow::Quit {
            return Ok(());
        }
    }
}

/// The console, even if a watcher reload panicked while holding it.
fn lock(console: &Mutex<Console>) -> std::sync::MutexGuard<'_, Console> {
    console.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lines of history kept for the session. History isn't saved between
/// sessions yet.
const HISTORY_LINES: usize = 500;

/// The console's prompt (`edit castle/courtyard> `) in reedline's terms.
struct ConsolePrompt(String);

impl Prompt for ConsolePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("... ")
    }
    fn render_prompt_history_search_indicator(&self, search: PromptHistorySearch) -> Cow<'_, str> {
        let status = match search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!("({}search: {}) ", status, search.term))
    }
}

/// Runs the console over `input` as if typed, and returns everything it
/// printed. Each input line appears in the output as `> line`.
pub fn run_script(world_path: &Path, mode: Mode, input: &str) -> Result<String> {
    let mut console = Console::open(world_path, mode)?;
    let mut out = Vec::new();
    drive(&mut console, input.as_bytes(), &mut out)?;
    Ok(String::from_utf8(out)?)
}

/// The scripted loop: reads lines from `input` and echoes each as
/// `> line`, so the output reads as a transcript.
fn drive(console: &mut Console, mut input: impl BufRead, mut out: impl Write) -> Result<()> {
    writeln!(out, "{}", console.greeting())?;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let line = line.trim_end_matches(['\n', '\r']);
        writeln!(out, "> {}", line)?;
        let (text, flow) = console.handle(line);
        if !text.is_empty() {
            writeln!(out, "{}", text)?;
        }
        if flow == Flow::Quit {
            return Ok(());
        }
    }
}
