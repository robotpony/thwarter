//! `thwart show` against a small world (REQ-CLI-5, 9 to 11), run as the
//! real binary.

use std::path::Path;
use std::process::Command;

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn world() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "world/index.md", "---\ntitle: Tiny Castle\ntype: world\nauthor: Bruce\nstart: vault\n---\nYou wake.\n");
    write(d, "world/story.md", "---\ntype: story\nvars: { king: alive }\n---\n");
    write(d, "chapters/10-before.md", "---\ntitle: Before\n---\n");
    write(d, "vault/index.md", "---\nid: vault\ntitle: Vault\ntype: location\nstate: shut\n---\n");
    write(d, "vault/shut.md", "---\ninherits: index\n---\nShut.\n");
    write(d, "vault/open.md", "---\ninherits: index\n---\nOpen.\n");
    write(d, "vault/chapters/before.md", "---\n---\nQuiet.\n");
    write(d, "objects/templates/book.md", "---\ntype: object\nobject_type: book\n---\nPages.\n");
    write(d, "objects/diary.md", "---\nid: diary\ntitle: Diary\ntype: object\ninherits: book\n---\n");
    write(d, "objects/vault.md", "---\nid: vault\ntitle: Vault key\ntype: object\nobject_type: key\n---\n");
    write(d, "npcs/guard.md", "---\nid: guard\ntitle: Guard\ntype: character\ndefault_location: vault\n---\nStern.\n");
    dir
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
    for expected in ["Tiny Castle\nby Bruce\nStarts in: vault\n\nYou wake.", "Story values: king = alive", "Chapters:\n  10  before  Before", "Locations (1):\n  - vault", "Objects (2):\n  - diary\n  - vault", "Characters (1):\n  - guard"] {
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
