//! `thwart show` against a small world (REQ-CLI-5, 9 to 11), run as the
//! real binary.

use std::path::Path;
use std::process::Command;

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A world at `authors/bruce/castle/` in a temporary folder, with the
/// persona beside it. Returns the folder and the world's path.
struct World {
    _dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl World {
    fn path(&self) -> &Path {
        &self.root
    }
}

fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "authors/bruce/index.md", "---\ntype: author\ntitle: B. Alderson\nwebsite: https://example.com\n---\nWrites small worlds.\n");
    let root = dir.path().join("authors/bruce/castle");
    let d = root.as_path();
    write(d, "world/index.md", "---\ntitle: Tiny Castle\ntype: world\nauthors: bruce\nstart: vault\n---\nYou wake.\n");
    write(d, "world/story.md", "---\ntype: story\nvars: { king: alive }\n---\n");
    write(d, "chapters/10-before.md", "---\ntitle: Before\n---\n");
    write(d, "vault/index.md", "---\nid: vault\ntitle: Vault\ntype: location\nstate: shut\ncharacters: [guard]\n---\n");
    write(d, "vault/shut.md", "---\ninherits: index\n---\nShut.\n");
    write(d, "vault/open.md", "---\ninherits: index\n---\nOpen.\n");
    write(d, "vault/chapters/before.md", "---\n---\nQuiet.\n");
    write(d, "objects/templates/book.md", "---\ntype: object\nobject_type: book\n---\nPages.\n");
    write(d, "objects/diary.md", "---\nid: diary\ntitle: Diary\ntype: object\ninherits: book\n---\n");
    write(d, "objects/vault.md", "---\nid: vault\ntitle: Vault key\ntype: object\nobject_type: key\n---\n");
    write(d, "npcs/guard.md", "---\nid: guard\ntitle: Guard\ntype: character\ndefault_location: vault\n---\nStern.\n");
    World { _dir: dir, root }
}

fn show(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_thwart"))
        .arg("show")
        .args(args)
        .arg("--world")
        .arg(dir)
        .output()
        .unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn the_summary_shows_the_world_definition_chapters_and_ids() {
    let dir = world();
    let (ok, out, _) = show(dir.path(), &[]);
    assert!(ok);
    for expected in ["Tiny Castle\nby B. Alderson (bruce)\nStarts in: vault\n\nYou wake.", "Story values: king = alive", "Chapters:\n  10  before  Before", "Locations (1):\n  - vault", "Objects (2):\n  - diary\n  - vault", "Characters (1):\n  - guard"] {
        assert!(out.contains(expected), "missing {:?} in:\n{}", expected, out);
    }
}

#[test]
fn any_thing_shows_in_any_version_and_shared_ids_need_a_kind() {
    let dir = world();
    let (ok, _, err) = show(dir.path(), &["vault"]);
    assert!(!ok && err.contains("`vault` names a location and an object; say which with --kind"), "{}", err);

    let (ok, out, _) = show(dir.path(), &["vault", "--kind", "location", "--state", "open", "--chapter", "before"]);
    assert!(ok && out.contains("Quiet."), "{}", out);
    assert!(out.contains("States: open (shown), shut (starting)") && out.contains("Chapter versions: before") && out.contains("Shown in chapter: before"), "{}", out);

    let (ok, out, _) = show(dir.path(), &["diary"]);
    assert!(ok && out.contains("Diary (diary)\nobject, book\n\nPages."), "{}", out);
    assert!(out.contains("Templates: ") && out.contains("objects/templates/book.md"), "{}", out);

    let (ok, out, _) = show(dir.path(), &["guard"]);
    assert!(ok && out.contains("Guard (guard)\ncharacter, npc\n\nStern.\n\nStarts in: vault"), "{}", out);

    let (ok, out, _) = show(dir.path(), &["diary", "--format", "json"]);
    assert!(ok && out.contains("\"object_type\": \"book\""), "{}", out);
}

#[test]
fn templates_and_built_ins_show_as_written() {
    let dir = world();
    // A bare name is looked up from the root, whose templates/ has no `book`.
    let (ok, _, _) = show(dir.path(), &["--template", "book"]);
    assert!(!ok);
    let (ok, out, _) = show(dir.path(), &["--template", "objects/templates/book"]);
    assert!(ok && out.contains("object_type: book") && out.contains("Pages."), "{}", out);
    let (ok, out, _) = show(dir.path(), &["--template", "scenery"]);
    assert!(ok && out.contains("Template scenery (built in, <built-in>/scenery.md)") && out.contains("object_type: scenery"), "{}", out);
    let (ok, _, err) = show(dir.path(), &["--template", "nope"]);
    assert!(!ok && err.contains("No template `nope`. Templates here: objects/templates/book, scenery"), "{}", err);
}

#[test]
fn warnings_go_to_stderr_as_a_count_and_json_is_one_document() {
    let dir = world();
    let (ok, out, err) = show(dir.path(), &["diary", "--format", "json"]);
    assert!(ok);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap_or_else(|e| panic!("not one JSON document ({}):\n{}", e, out));
    assert_eq!(doc["id"], "diary");
    assert!(!out.contains("WARNING"));
    assert!(err.trim() == "2 warnings; run `thwart validate` to list them", "{}", err);
}

#[test]
fn map_draws_from_the_start_and_unbuilt_options_say_so() {
    let dir = world();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_thwart")).arg("map").args(args).arg("--world").arg(dir.path()).output().unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
    };
    let (ok, out, err) = run(&[]);
    assert!(ok && out.contains("[@ Vault]"), "{} {}", out, err);
    for (args, why) in [(&["--player", "x"][..], "--player"), (&["--style", "unicode"][..], "--style unicode"), (&["--connections"][..], "--connections"), (&["nowhere"][..], "No location `nowhere`")] {
        let (ok, _, err) = run(args);
        assert!(!ok && err.contains(why), "{:?}: {}", args, err);
    }
}

#[test]
fn search_finds_text_and_broken_exits() {
    let dir = world();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_thwart")).arg("search").args(args).arg("--world").arg(dir.path()).output().unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
    };
    let (ok, out, err) = run(&["stern"]);
    assert!(ok && out.contains("character guard  Guard\n    Stern.") && err.contains("1 match"), "{} {}", out, err);
    let (ok, out, _) = run(&["VAULT", "--type", "object"]);
    assert!(ok && out.trim() == "object vault  Vault key", "{}", out);
    let (ok, out, _) = run(&["nothing-like-this"]);
    assert!(ok && out.trim() == "No matches.", "{}", out);
    let (ok, out, _) = run(&["--missing-connections"]);
    assert!(ok && out.trim() == "No broken exits.", "{}", out);
    let (ok, _, err) = run(&["(", "--regex"]);
    assert!(!ok && err.contains("isn't a regular expression"), "{}", err);
    let (ok, _, err) = run(&[]);
    assert!(!ok && err.contains("needs a query"), "{}", err);
    // Listing: a kind, a folder, or both, with no query.
    let (ok, out, _) = run(&["--type", "character"]);
    assert!(ok && out.trim() == "character guard  Guard", "{}", out);
    let (ok, out, _) = run(&["--type", "object", "objects/"]);
    assert!(ok && out.contains("object diary  Diary") && out.contains("object vault  Vault key"), "{}", out);
    let (ok, out, _) = run(&["--type", "object", ""]);
    assert!(ok && out.lines().count() == 2, "{}", out);
    let (ok, _, err) = run(&["--type", "text"]);
    assert!(!ok && err.contains("--type text needs a query"), "{}", err);
}

#[test]
fn a_door_shows_its_state_not_whether_it_is_passable() {
    let w = world();
    let d = w.path();
    write(d, "hall/index.md", "---\nid: hall\ntitle: Hall\ntype: location\nconnections: { down: trapdoor, east: grate }\n---\n");
    write(d, "doors/trapdoor.md", "---\nid: trapdoor\ntitle: Trapdoor\ntype: door\ninherits: door\nsides:\n  hall: { direction: down }\n  vault: { direction: up }\n---\n");
    write(d, "doors/grate.md", "---\nid: grate\ntitle: Grate\ntype: door\npassable: no\ntransparent: yes\nsides:\n  hall: { direction: east }\n  vault: { direction: west }\n---\n");
    let (ok, out, err) = show(d, &["trapdoor"]);
    assert!(ok, "{}", err);
    assert!(out.contains("\nStarts closed, opaque\n"), "{}", out);
    let (_, out, _) = show(d, &["trapdoor", "--state", "open"]);
    assert!(out.contains("\nIn state open, opaque\n"), "{}", out);
    let (_, out, _) = show(d, &["grate"]);
    assert!(out.contains("\nAlways closed, see-through"), "{}", out);
    assert!(!out.to_lowercase().contains("passable"), "{}", out);
}
