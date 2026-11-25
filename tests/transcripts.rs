//! The console's acceptance tests (REQ-CON-9): every `.txt` file in
//! `tests/transcripts/` is a console session, written exactly as the
//! console prints it, input lines included as `> line`.
//!
//! A transcript starts with a header of `# key: value` lines:
//!
//! ```text
//! # world: tests/worlds/tiny  (relative to the repository root)
//! # mode: edit             (or play)
//! # copy: yes              (optional: run on a throwaway copy of the world)
//! ```
//!
//! Game saves go to a temp folder for the run (`XDG_STATE_HOME`).
//!
//! A transcript that changes files (`/new`, `/set`, ...) MUST use
//! `# copy: yes`, so the fixture world never changes. The copy's path is
//! shown as the world's own path in the output, so the transcript is the
//! same on every machine.
//!
//! Everything after the header is the expected output. The runner feeds
//! the `> ` lines to the console as input and compares what it prints with
//! the file. To accept new output after a deliberate change, run
//! `THWART_BLESS=1 cargo test --test transcripts` and review the diff.

use std::fs;
use std::path::{Path, PathBuf};
use thwarter::console::{run_script, Mode};

const DIR: &str = "tests/transcripts";

/// Commands that write content files. A transcript using any of them must
/// run on a copy, or it would change the fixture world.
/// (`/edit` alone only switches mode; `/edit room` and the like will
/// write once full editing exists.)

struct Transcript {
    header: String,
    world: PathBuf,
    mode: Mode,
    copy: bool,
    expected: String,
}

fn parse(path: &Path) -> Result<Transcript, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut header = String::new();
    let mut world = None;
    let mut mode = Mode::Edit;
    let mut copy = false;
    let mut lines = text.split_inclusive('\n').peekable();
    while let Some(line) = lines.peek() {
        let Some(directive) = line.strip_prefix("# ") else { break };
        let (key, value) = directive.split_once(':').ok_or(format!("bad header line: {}", line.trim()))?;
        match (key.trim(), value.trim()) {
            ("world", value) => world = Some(PathBuf::from(value)),
            ("mode", "edit") => mode = Mode::Edit,
            ("mode", "play") => mode = Mode::Play,
            ("copy", "yes") => copy = true,
            ("copy", "no") => copy = false,
            (key, value) => return Err(format!("unknown header `{}: {}`", key, value)),
        }
        header.push_str(line);
        lines.next();
    }
    let world = world.ok_or("missing `# world:` header")?;
    Ok(Transcript { header, world, mode, copy, expected: lines.collect() })
}

fn input_of(expected: &str) -> String {
    expected.lines().filter_map(|l| l.strip_prefix("> ").or(if l == ">" { Some("") } else { None })).map(|l| format!("{}\n", l)).collect()
}

/// The first line where `actual` and `expected` differ, for a readable failure.
fn first_difference(expected: &str, actual: &str) -> String {
    let (e, a): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), actual.lines().collect());
    for i in 0..e.len().max(a.len()) {
        let (el, al) = (e.get(i).copied(), a.get(i).copied());
        if el != al {
            return format!(
                "line {} after the header\n  expected: {}\n  actual:   {}",
                i + 1,
                el.unwrap_or("<end of transcript>"),
                al.unwrap_or("<end of output>")
            );
        }
    }
    "trailing newline differs".to_string()
}

fn copy_world(from: &Path) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    for entry in walkdir::WalkDir::new(from).into_iter().filter_map(|e| e.ok()) {
        let to = dir.path().join(entry.path().strip_prefix(from).unwrap());
        if entry.file_type().is_dir() {
            fs::create_dir_all(&to).unwrap();
        } else {
            fs::copy(entry.path(), &to).unwrap();
        }
    }
    dir
}

#[test]
fn transcripts() {
    // Game saves (`/save`) go to a temp folder, never the real state folder.
    // Safe here: this is the only test in this binary.
    let state = tempfile::tempdir().expect("temp dir");
    // The author is the machine user (REQ-CON-34); fix it so transcripts
    // read the same on every machine.
    std::env::set_var("THWART_USER", "author");
    std::env::set_var("XDG_STATE_HOME", state.path());
    let bless = std::env::var_os("THWART_BLESS").is_some();
    let mut paths: Vec<PathBuf> = fs::read_dir(DIR)
        .expect("tests/transcripts exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("txt"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no transcripts in {}", DIR);

    let mut failures = Vec::new();
    for path in &paths {
        let name = path.display();
        let t = match parse(path) {
            Ok(t) => t,
            Err(e) => {
                failures.push(format!("{}: {}", name, e));
                continue;
            }
        };
        let input = input_of(&t.expected);
        // The commands that change content, with their aliases, from the
        // registry (REQ-CON-50).
        let changing: Vec<String> = thwarter::console::registry::changing().iter().map(|c| format!("/{}", c)).collect();
        if !t.copy {
            if let Some(line) = input.lines().find(|l| changing.iter().any(|c| l.split_whitespace().next() == Some(c.as_str()))) {
                failures.push(format!("{}: `{}` changes files, so this transcript needs `# copy: yes`", name, line));
                continue;
            }
        }
        let scratch = t.copy.then(|| copy_world(&t.world));
        let world = scratch.as_ref().map(|d| d.path().to_path_buf()).unwrap_or_else(|| t.world.clone());
        let actual = match run_script(&world, t.mode, &input) {
            // The build's version changes with every release, so it reads
            // as `<version>` in transcripts.
            Ok(out) => out
                .replace(&world.display().to_string(), &t.world.display().to_string())
                .replace(&format!("thwart {}", env!("CARGO_PKG_VERSION")), "thwart <version>"),
            Err(e) => {
                failures.push(format!("{}: console failed: {:#}", name, e));
                continue;
            }
        };
        if actual == t.expected {
            continue;
        }
        if bless {
            fs::write(path, format!("{}{}", t.header, actual)).expect("write transcript");
            eprintln!("blessed {}", name);
        } else {
            failures.push(format!("{}: {}", name, first_difference(&t.expected, &actual)));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} transcript(s) failed (THWART_BLESS=1 accepts new output):\n\n{}",
        failures.len(),
        paths.len(),
        failures.join("\n\n")
    );
}
