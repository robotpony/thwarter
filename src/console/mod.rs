//! The console: one interactive client for writing (edit mode) and
//! reading (play mode) a world (`design/CONSOLE_REQUIREMENTS.md`).
//!
//! `thwart edit` and `thwart play` hand off to `run` (REQ-CON-1). Tests
//! drive the same loop through `run_script`, which echoes each input line
//! as `> line` instead of printing a prompt, so its output reads as a
//! transcript (REQ-CON-9).

mod asking;
mod author;
mod commands;
mod doors;
mod editor;
mod full_edit;
mod inspect;
mod kinds;
pub mod registry;
mod removing;
mod to_write;
mod world_index;
mod panels;
mod picker;
pub use picker::run_or_pick;
mod play;
mod restart;
mod screen;
mod session;
mod states;
mod theme;
mod view;
mod watch;
mod world;

use anyhow::Result;
use session::{Console, Flow};
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
        // After a /restart, the session the last build saved (REQ-CON-30).
        let resumed = std::env::var_os(restart::RESUME_ENV).map(|path| console.resume(Path::new(&path)));
        drive_terminal(console, world_path, resumed)
    } else {
        drive(&mut console, stdin.lock(), io::stdout().lock())
    }
}

/// The full-screen console on a real terminal (REQ-CON-22). World files
/// are watched, and a change reloads the world and says what happened in
/// the output (REQ-CON-7).
fn drive_terminal(mut console: Console, world_path: &Path, resumed: Option<restart::Resumed>) -> Result<()> {
    console.interactive = true;
    console.unknown_log = crate::settings::state_dir().map(|dir| dir.join("unknown-commands.log"));
    let console = Arc::new(Mutex::new(console));
    let (send, notices) = std::sync::mpsc::sync_channel(256);
    let _watching = match watch::watch(world_path, console.clone(), send.clone()) {
        Ok(handle) => Some(handle),
        Err(e) => {
            let text = console.lock().map(|c| c.t("watch_failed", &[("reason", &e.to_string())])).unwrap_or_default();
            let _ = send.try_send(text);
            None
        }
    };
    screen::run(console, notices, resumed)
}

/// The console, even if a watcher reload panicked while holding it.
fn lock(console: &Mutex<Console>) -> std::sync::MutexGuard<'_, Console> {
    console.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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
