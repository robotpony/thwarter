//! Reloads the world when its files change on disk (REQ-CON-7), so an
//! author can keep the console open while changing files elsewhere.
//! Only the terminal loop watches; scripted runs and transcripts don't.

use super::session::Console;
use anyhow::Result;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventBatch, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Long enough that an editor's save (often a write, then a rename) is
/// one change, short enough to feel immediate.
const SETTLE: Duration = Duration::from_millis(300);

/// Watches `root` and, after each burst of changes to content files,
/// reloads `console` and sends what the reload said to `notices`. The
/// watching stops when the returned handle is dropped.
pub fn watch(root: &Path, console: Arc<Mutex<Console>>, notices: SyncSender<String>) -> Result<Debouncer<RecommendedWatcher>> {
    let shown_root = root.to_path_buf();
    let mut debouncer = new_debouncer(SETTLE, move |result: DebounceEventResult| {
        let changed = match result {
            Ok(DebouncedEventBatch::Events(events)) => {
                let paths: Vec<PathBuf> = events.into_iter().map(|e| e.path).collect();
                content_files(&paths, &shown_root)
            }
            Ok(DebouncedEventBatch::Rescan) => vec!["(changes were missed)".to_string()],
            Ok(_) => return,
            Err(e) => {
                let _ = notices.send(format!("File watching failed: {}. /reload still works.", e));
                return;
            }
        };
        let (changed, report) = {
            let Ok(mut console) = console.lock() else { return };
            // What the loader ignores (dot-names, Hugo's folders, the
            // world's `ignore` list) doesn't reload the world.
            let changed: Vec<String> = changed
                .into_iter()
                .filter(|p| p.starts_with('(') || !console.world.engine.is_ignored(std::path::Path::new(p)))
                .collect();
            let changed = console.not_own_writes(changed);
            if changed.is_empty() {
                return;
            }
            let report = console.reload();
            (changed, report)
        };
        // Sent with the console unlocked: while the full-screen editor is
        // open, nothing reads notices, and a full channel must not stop
        // the editor's saves from taking the lock.
        let _ = notices.send(format!("Changed on disk: {}.\n{}", changed.join(", "), report));
    })?;
    debouncer.watcher().watch(root, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// The changed paths that are world content (`.md` files and
/// `world.toml`), shown relative to the world root, without repeats.
/// Editor swap files, backups and the like are ignored.
fn content_files(paths: &[PathBuf], root: &Path) -> Vec<String> {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut shown: Vec<String> = Vec::new();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_content = path.extension().and_then(|e| e.to_str()) == Some("md") || name == "world.toml";
        if !is_content || name.starts_with('.') {
            continue;
        }
        let relative = path
            .strip_prefix(&canonical_root)
            .or_else(|_| path.strip_prefix(root))
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| name.to_string());
        if !shown.contains(&relative) {
            shown.push(relative);
        }
    }
    shown.sort();
    shown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::Mode;
    use std::sync::mpsc::sync_channel;

    #[test]
    fn only_content_files_count_as_changes() {
        let root = Path::new("/w");
        let paths: Vec<PathBuf> = ["/w/castle/hall.md", "/w/castle/.hall.md.swp", "/w/castle/hall.md~", "/w/world.toml", "/w/notes.txt", "/w/castle/hall.md"]
            .iter()
            .map(PathBuf::from)
            .collect();
        assert_eq!(content_files(&paths, root), vec!["castle/hall.md", "world.toml"]);
    }

    #[test]
    fn a_changed_file_reloads_the_console() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("hall.md"),
            "---\nid: hall\ntitle: Hall\ntype: location\n---\n\nA hall.\n",
        )
        .unwrap();
        let console = Arc::new(Mutex::new(Console::open(dir.path(), Mode::Edit).unwrap()));
        let (send, receive) = sync_channel(8);
        let _watching = watch(dir.path(), console.clone(), send).unwrap();
        // Give the platform watcher a moment to start before changing anything.
        std::thread::sleep(Duration::from_millis(200));

        std::fs::write(
            dir.path().join("cellar.md"),
            "---\nid: cellar\ntitle: Cellar\ntype: location\n---\n\nDamp.\n",
        )
        .unwrap();

        // macOS can also report `hall.md`, written just before watching
        // began, in a notice of its own; wait for the one about the cellar.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let notice = loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let notice = receive.recv_timeout(left).expect("a reload notice naming cellar.md");
            if notice.contains("cellar.md") {
                break notice;
            }
        };
        assert!(notice.starts_with("Changed on disk:"), "{}", notice);
        assert!(notice.contains("Reloaded: 2 room(s)"), "{}", notice);
    }
}
