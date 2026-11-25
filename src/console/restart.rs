//! `/restart` (REQ-CON-30, 31): the console restarts onto the `thwart`
//! binary now on disk, in the same terminal, and puts the author back
//! where they were. The session goes into a resume file, then the new
//! binary replaces this process (`exec`) with the same arguments and the
//! file's path in `THWART_RESUME`. On start, it reads the file and takes
//! back what it can; whatever it can't take starts fresh, with a notice.
//!
//! The running game travels as an ordinary game save, so its format
//! version and its reconciling with the world come from the same code as
//! `/load`.

use super::session::{Console, Place};
use super::Mode;
use crate::engines::{load_game, save_game};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Names the resume file a restarted console reads on start.
pub const RESUME_ENV: &str = "THWART_RESUME";

/// The resume file's format. A build that can't read it starts fresh.
const RESUME_VERSION: u32 = 1;

/// What carries over a restart, besides the game (saved beside it).
#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    thwarter_restart: u32,
    mode: String,
    /// `dir` or `file`, then the path, as `Place` holds it.
    place: (String, String),
    /// The input history, oldest first.
    history: Vec<String>,
    /// The output so far, so the screen looks the same after.
    #[serde(default)]
    scrollback: Vec<String>,
    /// The game save beside this file, if a game was running.
    game: Option<PathBuf>,
}

/// What a resumed console starts with.
pub struct Resumed {
    pub history: Vec<String>,
    pub scrollback: Vec<String>,
    /// Shown in place of the usual greeting.
    pub greeting: String,
}

impl Console {
    /// Writes the session to a resume file in `dir` and returns its path.
    pub(super) fn write_snapshot(&self, history: &[String], scrollback: &[String], dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(dir).with_context(|| format!("can't create {}", dir.display()))?;
        let stem = format!("restart-{}", std::process::id());
        let game = match &self.game {
            Some(state) => {
                let path = dir.join(format!("{}-game.yaml", stem));
                save_game(&path, state)?;
                Some(path)
            }
            None => None,
        };
        let place = match &self.place {
            Place::Dir(dir) => ("dir".to_string(), dir.clone()),
            Place::File(path) => ("file".to_string(), path.clone()),
        };
        let snapshot = Snapshot {
            thwarter_restart: RESUME_VERSION,
            mode: self.mode.name().to_string(),
            place,
            history: history.to_vec(),
            scrollback: scrollback.to_vec(),
            game,
        };
        let path = dir.join(format!("{}.yaml", stem));
        std::fs::write(&path, serde_yaml::to_string(&snapshot)?).with_context(|| format!("can't write {}", path.display()))?;
        Ok(path)
    }

    /// Takes back the session in the resume file at `path`, then removes
    /// it. Whatever can't be taken back is left as `open` set it, and
    /// the greeting says so.
    pub(super) fn resume(&mut self, path: &Path) -> Resumed {
        let mut notes = Vec::new();
        let snapshot = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_yaml::from_str::<Snapshot>(&text).ok())
            .filter(|s| s.thwarter_restart == RESUME_VERSION);
        let _ = std::fs::remove_file(path);
        let Some(snapshot) = snapshot else {
            return Resumed {
                history: Vec::new(),
                scrollback: Vec::new(),
                greeting: format!("{}\n{}", restarted_on(self), self.t("session_not_carried_over", &[])),
            };
        };

        self.mode = if snapshot.mode == "play" { Mode::Play } else { Mode::Edit };
        if let Some(game) = &snapshot.game {
            match load_game(game, &self.world.engine) {
                Ok((state, warnings)) => {
                    self.game = Some(state);
                    notes.extend(warnings.iter().map(|w| self.t("game_note", &[("note", w)])));
                }
                Err(e) => {
                    let fresh = if self.game.is_some() { self.t("game_reset_fresh", &[]) } else { String::new() };
                    notes.push(self.t("game_reset", &[("reason", &format!("{:#}", e)), ("fresh", &fresh)]));
                }
            }
            let _ = std::fs::remove_file(game);
        } else {
            self.game = None;
        }
        if self.mode == Mode::Play && self.game.is_none() {
            self.mode = Mode::Edit;
            notes.push(self.t("no_game_to_resume", &[]));
        }

        let (kind, at) = snapshot.place;
        self.place = if kind == "file" { Place::File(at) } else { Place::Dir(at) };
        if !self.place_exists() {
            let gone = self.pwd();
            self.move_to_existing_folder();
            notes.push(self.t("place_gone", &[("place", &gone), ("now", &self.pwd())]));
        }

        let mut greeting = vec![restarted_on(self), self.t("resumed_where_you_were", &[])];
        greeting.extend(notes);
        Resumed { history: snapshot.history, scrollback: snapshot.scrollback, greeting: greeting.join("\n") }
    }
}

/// Saves the session and replaces this process with the `thwart` on disk.
/// Returns only if that couldn't happen, with the reason; the console then
/// carries on as it was.
pub fn restart(console: &Console, history: &[String], scrollback: &[String]) -> String {
    let exe = match current_exe() {
        Ok(exe) => exe,
        Err(e) => return console.t("cant_restart", &[("reason", &format!("{:#}", e))]),
    };
    // A build that can't even print its version would leave the terminal
    // with nothing running, so check it starts first.
    match std::process::Command::new(&exe).arg("--version").output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            return console.t(
                "restart_doesnt_start",
                &[("exe", &exe.display().to_string()), ("reason", String::from_utf8_lossy(&out.stderr).trim())],
            )
        }
        Err(e) => return console.t("restart_cant_run", &[("exe", &exe.display().to_string()), ("reason", &e.to_string())]),
    }
    let Some(dir) = crate::settings::state_dir().map(|dir| dir.join("restart")) else {
        return console.t("restart_no_state_dir", &[]);
    };
    let snapshot = match console.write_snapshot(history, scrollback, &dir) {
        Ok(path) => path,
        Err(e) => return console.t("restart_cant_save", &[("reason", &format!("{:#}", e))]),
    };
    let error = exec(&exe, &snapshot);
    let _ = std::fs::remove_file(&snapshot);
    console.t("restart_failed", &[("reason", &error.to_string())])
}

#[cfg(unix)]
fn exec(exe: &Path, snapshot: &Path) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    std::process::Command::new(exe).args(std::env::args_os().skip(1)).env(RESUME_ENV, snapshot).exec()
}

#[cfg(not(unix))]
fn exec(_exe: &Path, _snapshot: &Path) -> std::io::Error {
    std::io::Error::other("restarting in place needs a Unix terminal")
}

/// This binary's path. After a rebuild replaced it, Linux reports the old
/// file as `... (deleted)`; the new one is at the path without that.
fn current_exe() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("can't find the running thwart")?;
    let shown = exe.to_string_lossy();
    Ok(match shown.strip_suffix(" (deleted)") {
        Some(path) => PathBuf::from(path),
        None => exe,
    })
}

/// "Restarted on thwart 0.12.1 (built 14:02)."
fn restarted_on(console: &Console) -> String {
    let built = current_exe()
        .ok()
        .and_then(|exe| std::fs::metadata(exe).ok())
        .and_then(|m| m.modified().ok())
        .map(|time| chrono::DateTime::<chrono::Local>::from(time).format(" (built %H:%M)").to_string())
        .unwrap_or_default();
    console.t("restarted_on", &[("version", env!("CARGO_PKG_VERSION")), ("built", &built)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tiny_copy() -> (tempfile::TempDir, PathBuf) {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/worlds/tiny");
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tiny");
        for entry in walkdir::WalkDir::new(&fixture) {
            let entry = entry.unwrap();
            let to = root.join(entry.path().strip_prefix(&fixture).unwrap());
            if entry.file_type().is_dir() {
                fs::create_dir_all(&to).unwrap();
            } else {
                fs::copy(entry.path(), &to).unwrap();
            }
        }
        (dir, root)
    }

    #[test]
    fn a_session_comes_back_where_it_was() {
        let (dir, root) = tiny_copy();
        let mut before = Console::open(&root, Mode::Play).unwrap();
        before.handle("take lantern");
        before.handle("n");
        before.handle("/edit");
        before.handle("go /npcs/elder");
        let history = vec!["take lantern".to_string(), "n".to_string()];
        let snapshot = before.write_snapshot(&history, &["before".to_string()], &dir.path().join("restart")).unwrap();

        let mut after = Console::open(&root, Mode::Edit).unwrap();
        let resumed = after.resume(&snapshot);
        assert!(resumed.greeting.contains("\nYou are precisely where you were when you left."), "{}", resumed.greeting);
        assert_eq!(resumed.history, history);
        assert_eq!(resumed.scrollback, vec!["before".to_string()]);
        assert_eq!(after.mode, Mode::Edit);
        assert_eq!(after.pwd(), "npcs/elder");
        let game = after.game.as_ref().unwrap();
        assert_eq!(game.location, "village/well");
        assert_eq!(game.inventory, vec!["lantern".to_string()]);
        assert!(!snapshot.exists(), "the resume file is used once");
    }

    #[test]
    fn what_no_longer_exists_starts_fresh_with_a_notice() {
        let (dir, root) = tiny_copy();
        let mut before = Console::open(&root, Mode::Edit).unwrap();
        before.handle("go /village/well");
        let snapshot = before.write_snapshot(&[], &[], &dir.path().join("restart")).unwrap();
        fs::remove_file(root.join("village/well.md")).unwrap();

        let mut after = Console::open(&root, Mode::Edit).unwrap();
        let resumed = after.resume(&snapshot);
        assert!(resumed.greeting.contains("village/well is gone; now at village/."), "{}", resumed.greeting);
    }

    #[test]
    fn an_unreadable_resume_file_is_a_fresh_start() {
        let (dir, root) = tiny_copy();
        let path = dir.path().join("bad.yaml");
        fs::write(&path, "thwarter_restart: 99\n").unwrap();
        let mut console = Console::open(&root, Mode::Edit).unwrap();
        let resumed = console.resume(&path);
        assert!(resumed.greeting.contains("fresh start"), "{}", resumed.greeting);
        assert!(resumed.history.is_empty());
    }
}
