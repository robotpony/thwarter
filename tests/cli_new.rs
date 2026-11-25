//! `thwart new <world>` (REQ-CLI-22), run as the real binary.

use std::process::Command;

fn new(dir: &std::path::Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_thwart")).arg("new").args(args).current_dir(dir).env("THWART_USER", "ann").output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn new_makes_a_world_in_the_authors_folder_that_validates() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let (ok, out, err) = new(d, &["Tiny Castle"]);
    assert!(ok, "{}", err);
    assert_eq!(out, "Made authors/ann/tiny-castle, titled \"Tiny Castle\", with you (ann) as its author.\nNext: thwart edit authors/ann/tiny-castle\n");
    let index = std::fs::read_to_string(d.join("authors/ann/tiny-castle/world/index.md")).unwrap();
    assert_eq!(index, "---\ntitle: Tiny Castle\ntype: world\nauthors: ann\n---\n");
    let v = Command::new(env!("CARGO_BIN_EXE_thwart")).args(["validate", "--world", "authors/ann/tiny-castle"]).current_dir(d).output().unwrap();
    assert!(v.status.success(), "{}", String::from_utf8_lossy(&v.stdout));

    // A second time, it refuses, and changes nothing.
    let (ok, _, err) = new(d, &["tiny-castle", "--title", "Other"]);
    assert!(!ok && err.contains("authors/ann/tiny-castle already exists"), "{}", err);
    assert_eq!(std::fs::read_to_string(d.join("authors/ann/tiny-castle/world/index.md")).unwrap(), index);

    // A path is made where it says, titled from its last part.
    let (ok, out, _) = new(d, &["worlds/sunken-bell"]);
    assert!(ok && out.contains("titled \"Sunken bell\""), "{}", out);
    assert!(d.join("worlds/sunken-bell/world/index.md").is_file());
}
