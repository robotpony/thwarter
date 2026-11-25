//! `thwart package`, `install`, `installed` (REQ-CLI-12 to 16), run as
//! the real binary, with the data folder in a temporary place.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

struct Setup {
    dir: tempfile::TempDir,
}

impl Setup {
    fn world(&self) -> PathBuf {
        self.dir.path().join("authors/bruce/castle")
    }
    fn data(&self) -> PathBuf {
        self.dir.path().join("data")
    }
    fn out(&self) -> PathBuf {
        self.dir.path().join("out")
    }
    fn run(&self, args: &[&str]) -> (bool, String, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_thwart"))
            .args(args)
            .env("XDG_DATA_HOME", self.data())
            .current_dir(self.out())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "authors/bruce/index.md", "---\ntype: author\ntitle: B. Writer\nemail: b@example.com\n---\nWrites.\n");
    let w = d.join("authors/bruce/castle");
    write(&w, "world/index.md", "---\ntitle: Castle\ntype: world\nauthors: bruce\nversion: 1.2\nstart: hall\n---\nYou wake.\n");
    write(&w, "world/styles/terse.md", "My private style notes.\n");
    write(&w, "world.toml", "[settings]\ndefault_map_style = \"ascii\"\n");
    write(&w, "hall.md", "---\nid: hall\ntitle: Hall\ntype: location\n---\nStone. <!-- the butler did it -->\n\n<!-- spoiler: the vault is under the stairs -->\n");
    write(&w, "objects/lever.md", "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull: { sets: { this.pulled: yes } }\nvars: { pulled: no }\n---\nA lever.\n\n## When: #pulled\nClunk.\n");
    write(&w, ".obsidian/workspace.json", "{}");
    std::fs::create_dir_all(d.join("out")).unwrap();
    Setup { dir }
}

fn entries(package: &Path) -> Vec<String> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(package).unwrap()).unwrap();
    let mut names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().unwrap().to_string()).collect();
    names.sort();
    names
}

fn read_entry(package: &Path, name: &str) -> String {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(package).unwrap()).unwrap();
    let mut text = String::new();
    zip.by_name(name).unwrap().read_to_string(&mut text).unwrap();
    text
}

#[test]
fn a_world_packages_with_its_persona_and_installs_to_play() {
    let s = setup();
    let world = s.world();
    let world = world.to_str().unwrap();

    // Without a terminal, it says what ships and refuses without --yes.
    let (ok, _, err) = s.run(&["package", world]);
    assert!(!ok, "{}", err);
    assert!(err.contains("This package ships as B. Writer") && err.contains("email: b@example.com"), "{}", err);
    assert!(err.contains("2 author comment(s) in 1 file(s)"), "{}", err);
    assert!(err.contains("pass --yes"), "{}", err);
    assert!(std::fs::read_dir(s.out()).unwrap().next().is_none());

    let (ok, out, err) = s.run(&["package", world, "--yes"]);
    assert!(ok, "{}", err);
    let package = s.out().join("bruce-castle-1.2.thwarter");
    assert!(out.contains("Packaged") && package.exists(), "{}", out);
    assert_eq!(
        entries(&package),
        [
            "bruce/castle/hall.md",
            "bruce/castle/objects/lever.md",
            "bruce/castle/world.toml",
            "bruce/castle/world/index.md",
            "bruce/index.md",
            "thwarter-package.toml"
        ]
    );
    let manifest = read_entry(&package, "thwarter-package.toml");
    assert!(manifest.contains("format = 1") && manifest.contains("persona = \"bruce\"") && manifest.contains("version = \"1.2\""), "{}", manifest);
    assert!(read_entry(&package, "bruce/castle/hall.md").contains("the butler did it"));

    // --strip-comments drops author comments, never sections.
    let (ok, _, err) = s.run(&["package", world, "--yes", "--strip-comments", "--output", "stripped.thwarter"]);
    assert!(ok, "{}", err);
    assert!(err.contains("Author comments are left out (2 in 1 file(s))"), "{}", err);
    let stripped = s.out().join("stripped.thwarter");
    let hall = read_entry(&stripped, "bruce/castle/hall.md");
    assert!(!hall.contains("<!--") && hall.contains("Stone."), "{:?}", hall);
    assert!(read_entry(&stripped, "bruce/castle/objects/lever.md").contains("## When: #pulled\nClunk."));
    assert!(std::fs::read_to_string(s.world().join("hall.md")).unwrap().contains("the butler"));

    // Install, refuse a second time, replace, list.
    let package = package.to_str().unwrap();
    let (ok, out, err) = s.run(&["install", package]);
    assert!(ok, "{}", err);
    assert!(out.contains("Installed Castle 1.2") && out.contains("thwart play bruce/castle"), "{}", out);
    let installed = s.data().join("thwarter/worlds/bruce/castle");
    assert!(installed.join("hall.md").exists() && s.data().join("thwarter/worlds/bruce/index.md").exists());
    let (ok, _, err) = s.run(&["install", package]);
    assert!(!ok && err.contains("already installed; --replace"), "{}", err);
    let (ok, out, _) = s.run(&["install", package, "--replace"]);
    assert!(ok && out.contains("Replaced"), "{}", out);
    let (ok, out, _) = s.run(&["installed"]);
    assert!(ok && out.trim() == "bruce/castle  Castle (1.2)", "{}", out);

    // The installed copy loads cleanly.
    let mut engine = thwarter::WorldEngine::new();
    engine.load_world(&installed).unwrap();
    assert!(engine.current_problems(false).is_empty(), "{:?}", engine.current_problems(false));
}

#[test]
fn a_world_with_problems_is_not_packaged() {
    let s = setup();
    write(&s.world(), "broken.md", "no frontmatter\n");
    let (ok, _, err) = s.run(&["package", s.world().to_str().unwrap(), "--yes"]);
    assert!(!ok && err.contains("1 problem(s), so it can't be packaged"), "{}", err);
}

/// A zip with the given entries, each a name and text, plus a valid
/// manifest unless `manifest` is false.
fn hostile(path: &Path, entries: &[(&str, &str)], manifest: bool) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    if manifest {
        zip.start_file("thwarter-package.toml", options).unwrap();
        zip.write_all(b"format = 1\npersona = \"eve\"\nworld = \"trap\"\nthwart = \"0\"\npackaged = 0\n").unwrap();
    }
    for (name, text) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn hostile_packages_are_refused_before_anything_is_written() {
    let s = setup();
    let cases: Vec<(&str, Vec<(&str, &str)>, bool, &str)> = vec![
        ("escape.thwarter", vec![("eve/trap/../../../escape.md", "x")], true, "points outside the package"),
        ("absolute.thwarter", vec![("/etc/evil.md", "x")], true, "points outside the package"),
        ("elsewhere.thwarter", vec![("mallory/other/a.md", "x")], true, "is outside eve/trap/"),
        ("bare.thwarter", vec![("eve/trap/a.md", "x")], false, "has no thwarter-package.toml"),
    ];
    for (name, entries, manifest, why) in cases {
        let path = s.out().join(name);
        hostile(&path, &entries, manifest);
        let (ok, _, err) = s.run(&["install", path.to_str().unwrap()]);
        assert!(!ok && err.contains(why), "{}: {}", name, err);
    }
    // A link entry.
    let path = s.out().join("link.thwarter");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("thwarter-package.toml", options).unwrap();
    zip.write_all(b"format = 1\npersona = \"eve\"\nworld = \"trap\"\nthwart = \"0\"\npackaged = 0\n").unwrap();
    zip.add_symlink("eve/trap/link.md", "/etc/passwd", options).unwrap();
    zip.finish().unwrap();
    let (ok, _, err) = s.run(&["install", path.to_str().unwrap()]);
    assert!(!ok && err.contains("is a link"), "{}", err);

    assert!(!s.data().join("thwarter/worlds/eve").exists() || std::fs::read_dir(s.data().join("thwarter/worlds/eve")).unwrap().next().is_none());
}

#[test]
fn every_author_with_a_persona_file_ships_and_installs() {
    let s = setup();
    let d = s.dir.path();
    write(d, "authors/alice/index.md", "---\ntype: author\ntitle: A. Writer\nwebsite: https://example.org\n---\nAlso writes.\n");
    let w = s.world();
    write(&w, "world/index.md", "---\ntitle: Castle\ntype: world\nauthors: [bruce, alice, nobody]\nversion: 1.2\nstart: hall\n---\nYou wake.\n");
    let world = w.to_str().unwrap();
    let (ok, _, err) = s.run(&["package", world, "--yes"]);
    assert!(ok, "{}", err);
    assert!(err.contains("It also ships A. Writer's details (alice):") && err.contains("website: https://example.org"), "{}", err);
    let package = s.out().join("bruce-castle-1.2.thwarter");
    let names = entries(&package);
    assert!(names.contains(&"bruce/index.md".to_string()) && names.contains(&"alice/index.md".to_string()), "{:?}", names);
    // `nobody` has no persona file, so nothing ships for it.
    assert!(!names.iter().any(|n| n.starts_with("nobody/")), "{:?}", names);
    let manifest: toml::Value = toml::from_str(&read_entry(&package, "thwarter-package.toml")).unwrap();
    assert_eq!(manifest["persona"].as_str(), Some("bruce"));
    let authors: Vec<&str> = manifest["authors"].as_array().unwrap().iter().filter_map(|a| a.as_str()).collect();
    assert_eq!(authors, ["bruce", "alice", "nobody"]);

    let (ok, out, err) = s.run(&["install", package.to_str().unwrap()]);
    assert!(ok, "{}{}", out, err);
    assert!(s.data().join("thwarter/worlds/alice/index.md").exists());
    assert!(s.data().join("thwarter/worlds/bruce/castle/world/index.md").exists());
}
