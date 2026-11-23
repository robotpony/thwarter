//! The console: one interactive client for writing (edit mode) and
//! reading (play mode) a world (`REQUIREMENTS.md` › §4.6).
//!
//! `thwart edit` and `thwart play` hand off to `run` (REQ-CON-1). Tests
//! drive the same loop through `run_script`, which echoes each input line
//! as `> line` instead of printing a prompt, so its output reads as a
//! transcript (REQ-CON-9).

mod session;
mod view;
mod world;

use anyhow::Result;
use session::{Console, Flow};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

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
    let interactive = stdin.is_terminal();
    drive(&mut console, stdin.lock(), io::stdout().lock(), interactive)
}

/// Runs the console over `input` as if typed, and returns everything it
/// printed. Each input line appears in the output as `> line`.
pub fn run_script(world_path: &Path, mode: Mode, input: &str) -> Result<String> {
    let mut console = Console::open(world_path, mode)?;
    let mut out = Vec::new();
    drive(&mut console, input.as_bytes(), &mut out, false)?;
    Ok(String::from_utf8(out)?)
}

fn drive(console: &mut Console, mut input: impl BufRead, mut out: impl Write, interactive: bool) -> Result<()> {
    writeln!(out, "{}", console.greeting())?;
    loop {
        if interactive {
            write!(out, "{}", console.prompt())?;
            out.flush()?;
        }
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            if interactive {
                writeln!(out)?;
            }
            return Ok(());
        }
        let line = line.trim_end_matches(['\n', '\r']);
        if !interactive {
            writeln!(out, "> {}", line)?;
        }
        let (text, flow) = console.handle(line);
        if !text.is_empty() {
            writeln!(out, "{}", text)?;
        }
        if flow == Flow::Quit {
            return Ok(());
        }
    }
}
