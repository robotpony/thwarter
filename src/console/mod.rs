//! The console: one interactive client for writing (edit mode) and
//! reading (play mode) a world (`REQUIREMENTS.md` › §4.6).
//!
//! This is the entry point `thwart edit` and `thwart play` hand off to
//! (REQ-CON-1). The loop itself is not built yet; until it is, `run`
//! loads the world so a bad path still fails clearly, then says the
//! console isn't implemented rather than pretending to start one.

use crate::engines::WorldEngine;
use anyhow::{bail, Result};
use std::path::Path;

/// Which mode the console opens in. `/edit` and `/play` switch between
/// them inside a running console (REQ-CON-2).
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

/// Opens the console on the world at `world_path` in `mode`.
pub fn run(world_path: &Path, mode: Mode) -> Result<()> {
    if !world_path.is_dir() {
        bail!("World directory `{}` not found", world_path.display());
    }
    let mut engine = WorldEngine::new();
    engine.load_world(world_path)?;
    bail!(
        "the console ({} mode) is not implemented yet (PLAN.md › Phase 2)",
        mode.name()
    )
}
