//! Which world a command means when none is named (REQ-CLI-21), and the
//! worlds a person can pick from. There's no built-in default world: a
//! world is `--world`, a configured default, or the one the current folder
//! is in; otherwise the command lists what it can find and stops.

use crate::engines::layers::Layer;
use crate::engines::world_def::WorldDef;
use std::path::{Path, PathBuf};

/// A world a person could pick (REQ-CLI-21, REQ-CON-62).
#[derive(Debug, Clone, PartialEq)]
pub struct FoundWorld {
    pub root: PathBuf,
    /// From `world/index.md` or `world.toml`, else the folder's name.
    pub title: String,
    /// Under the data folder's `worlds/`, so only played (REQ-WS-8).
    pub installed: bool,
    /// The current user may write it: not installed, and either named in
    /// `authors:` or the world names no one yet.
    pub writable: bool,
    pub authors: Vec<String>,
}

/// No world was named and none could be found from where the command ran.
/// `thwart edit` and `play` show a picker from `found`; other commands
/// print it (its `Display`) and exit non-zero.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldNotGiven {
    pub found: Vec<FoundWorld>,
    /// Where it looked, for the message.
    pub from: PathBuf,
}

impl std::fmt::Display for WorldNotGiven {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.found.is_empty() {
            writeln!(f, "No world given, and there are no worlds in an authors/ folder here or installed.")?;
            return write!(
                f,
                "To start one, make a folder for it under authors/<your user name>/ and run `thwart edit <that folder>`, or install one with `thwart install <file>.thwarter`."
            );
        }
        writeln!(f, "No world given. Worlds you can name:")?;
        let shown: Vec<String> = self.found.iter().map(|w| shown_path(&w.root, &self.from)).collect();
        let width = shown.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        for (world, path) in self.found.iter().zip(&shown) {
            let installed = if world.installed { " (installed)" } else { "" };
            let title = crate::render::clean_text(&world.title);
            writeln!(f, "  {:width$}  {}{}", path, title, installed, width = width)?;
        }
        write!(f, "Name a world with --world <path>.")
    }
}

impl std::error::Error for WorldNotGiven {}

/// A path relative to `from` when it's under it, so the list reads short.
fn shown_path(path: &Path, from: &Path) -> String {
    path.strip_prefix(from).map(Path::to_path_buf).unwrap_or_else(|_| path.to_path_buf()).display().to_string()
}

/// Whether `dir` holds a world: a `world/index.md` or a `world.toml`.
pub fn is_world(dir: &Path) -> bool {
    dir.join("world/index.md").is_file() || dir.join("world.toml").is_file()
}

/// The world a command means (REQ-CLI-21), in order: `flag` (`--world`
/// or the command's own world argument), the configured default
/// (`thwart.toml`'s `[world] path` in `from`, then `config.toml`'s), then
/// the world `from` is in or under. Otherwise `WorldNotGiven`, with every
/// world `find_worlds` sees.
pub fn resolve_world(flag: Option<&str>, from: &Path) -> Result<PathBuf, WorldNotGiven> {
    if let Some(path) = flag.filter(|p| !p.trim().is_empty()) {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = configured_world(from) {
        return Ok(path);
    }
    if let Some(root) = enclosing_world(from) {
        return Ok(root);
    }
    Err(WorldNotGiven { found: find_worlds(from), from: from.to_path_buf() })
}

/// The default world a person set: `[world] path` in `thwart.toml` in
/// `from`, relative to it, else in their `config.toml`.
pub fn configured_world(from: &Path) -> Option<PathBuf> {
    let path_in = |table: &toml::Table| -> Option<String> {
        table.get("world")?.get("path")?.as_str().map(str::trim).filter(|p| !p.is_empty()).map(str::to_string)
    };
    let local = std::fs::read_to_string(from.join("thwart.toml")).ok().and_then(|t| t.parse::<toml::Table>().ok());
    if let Some(path) = local.as_ref().and_then(path_in) {
        return Some(from.join(path));
    }
    let personal = crate::settings::PersonalConfig::load().ok()?;
    path_in(personal.table()).map(PathBuf::from)
}

/// The world `from` is in or under: the nearest folder, `from` itself or
/// above it, that holds one.
pub fn enclosing_world(from: &Path) -> Option<PathBuf> {
    from.ancestors().find(|dir| is_world(dir)).map(Path::to_path_buf)
}

/// Every world a person could pick from `from` (REQ-CON-62): those in
/// `authors/*/*` of the nearest folder above it with an `authors/`, by
/// path, then the installed ones.
pub fn find_worlds(from: &Path) -> Vec<FoundWorld> {
    let user = crate::settings::current_user();
    let mut out = Vec::new();
    if let Some(repo) = from.ancestors().find(|d| d.join("authors").is_dir()) {
        let mut roots: Vec<PathBuf> = subfolders(&repo.join("authors")).iter().flat_map(|persona| subfolders(persona)).filter(|d| is_world(d)).collect();
        roots.sort();
        for root in roots {
            out.push(describe(root, false, user.as_deref()));
        }
    }
    if let Some(data) = crate::settings::data_dir() {
        for w in crate::packaging::installed(&data) {
            out.push(describe(w.path.clone(), true, user.as_deref()));
        }
    }
    out
}

/// A folder name from a title: lower case, with every run of other
/// characters as one `-` ("Tiny Castle!" is `tiny-castle`).
pub fn slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// The title a new world gets from the name it was made with: a folder
/// name reads as words, first letter capitalized (`tiny-castle` is
/// "Tiny castle"); a name with spaces is kept as typed.
pub fn title_from(name: &str) -> String {
    let words = name.trim().replace(['-', '_'], " ");
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = words.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

/// Where `thwart new <name>` makes a world (REQ-CLI-22): a name with a
/// `/` (or `.` first) is a path from `from`; a bare name goes in
/// `authors/<user>/`, under the nearest `authors/` folder from `from` up,
/// else a new one in `from`, as its slug.
pub fn new_world_folder(name: &str, from: &Path, user: &str) -> PathBuf {
    let name = name.trim();
    if name.contains('/') || name.contains(std::path::MAIN_SEPARATOR) || name.starts_with('.') {
        return from.join(name);
    }
    let authors = from.ancestors().map(|d| d.join("authors")).find(|d| d.is_dir()).unwrap_or_else(|| from.join("authors"));
    authors.join(user).join(slug(name))
}

/// Makes a world in `root`, which must not exist or be empty: a
/// `world/index.md` with its `title`, `type: world` and `authors: user`
/// (REQ-WS-5, 7).
pub fn create_world(root: &Path, title: &str, user: &str) -> anyhow::Result<()> {
    if root.exists() && std::fs::read_dir(root).map_or(true, |mut d| d.next().is_some()) {
        anyhow::bail!("{} already exists and isn't empty, so no world was made there", root.display());
    }
    std::fs::create_dir_all(root.join("world"))?;
    let title = crate::writer::content_file::emit_scalar(&serde_yaml::Value::String(title.to_string()), false).unwrap_or_else(|_| title.to_string());
    let user = crate::writer::content_file::emit_scalar(&serde_yaml::Value::String(user.to_string()), false).unwrap_or_else(|_| user.to_string());
    std::fs::write(root.join("world/index.md"), format!("---\ntitle: {}\ntype: world\nauthors: {}\n---\n", title, user))?;
    Ok(())
}

fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map_or(false, |t| t.is_dir()))
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

/// A world's title and authors, read from its definition alone, without
/// loading the world.
fn describe(root: PathBuf, installed: bool, user: Option<&str>) -> FoundWorld {
    let index = root.join("world/index.md");
    let def = match std::fs::read_to_string(&index).ok().and_then(|text| Layer::parse(&index, &text).ok()) {
        Some(layer) => {
            let mut def = WorldDef::default();
            def.read_index(&layer, &mut Vec::new(), &mut Vec::new());
            def
        }
        None => std::fs::read_to_string(root.join("world.toml"))
            .ok()
            .and_then(|t| t.parse::<toml::Value>().ok())
            .map(|v| WorldDef::from_world_toml(&root, &v))
            .unwrap_or_default(),
    };
    let title = def
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    let writable = !installed && (def.authors.is_empty() || user.map_or(false, |u| def.authors.iter().any(|a| a == u)));
    FoundWorld { root, title, installed, writable, authors: def.authors }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn a_world_is_named_found_from_the_folder_or_listed() {
        let dir = tempfile::tempdir().unwrap();
        let d = std::fs::canonicalize(dir.path()).unwrap();
        write(&d, "authors/ann/castle/world/index.md", "---\ntype: world\ntitle: Castle\nauthors: [ann]\n---\n");
        write(&d, "authors/ann/castle/hall/index.md", "---\nid: hall\ntype: location\n---\n");
        write(&d, "authors/ann/forest/world.toml", "[settings]\nstarting_location = \"glade\"\n");
        write(&d, "authors/ann/notes/readme.md", "not a world\n");
        write(&d, "authors/ann/index.md", "---\ntype: author\n---\n");

        // Named wins over everything.
        assert_eq!(resolve_world(Some("elsewhere"), &d), Ok(PathBuf::from("elsewhere")));
        // From inside a world, that world.
        assert_eq!(enclosing_world(&d.join("authors/ann/castle/hall")), Some(d.join("authors/ann/castle")));
        // A thwart.toml here names the default.
        write(&d, "authors/ann/thwart.toml", "[world]\npath = \"forest\"\n");
        assert_eq!(resolve_world(None, &d.join("authors/ann")), Ok(d.join("authors/ann/forest")));

        // Otherwise, the list, and no world. (Built by hand: a default in
        // the person's own config.toml would stop resolve_world first.)
        let err = WorldNotGiven { found: find_worlds(&d), from: d.clone() };
        let roots: Vec<&Path> = err.found.iter().filter(|w| !w.installed).map(|w| w.root.as_path()).collect();
        assert_eq!(roots, [d.join("authors/ann/castle"), d.join("authors/ann/forest")]);
        let castle = &err.found[0];
        assert_eq!((castle.title.as_str(), castle.authors.clone()), ("Castle", vec!["ann".to_string()]));
        // The forest names no authors yet, so anyone may write it.
        assert!(err.found[1].writable);
        assert_eq!(err.found[1].title, "forest");
        let text = err.to_string();
        assert!(text.starts_with("No world given. Worlds you can name:\n"), "{}", text);
        assert!(text.contains("\n  authors/ann/castle  Castle\n  authors/ann/forest  forest\n"), "{}", text);
        assert!(text.ends_with("Name a world with --world <path>."), "{}", text);
    }

    #[test]
    fn a_new_world_goes_in_the_authors_folder_with_its_author() {
        let dir = tempfile::tempdir().unwrap();
        let d = std::fs::canonicalize(dir.path()).unwrap();
        assert_eq!((slug("Tiny Castle!"), title_from("tiny-castle"), title_from("The Sunken Bell")), ("tiny-castle".into(), "Tiny castle".into(), "The Sunken Bell".into()));
        // A bare name goes in authors/<user>/ above, or a new authors/ here.
        assert_eq!(new_world_folder("Tiny Castle", &d, "ann"), d.join("authors/ann/tiny-castle"));
        std::fs::create_dir_all(d.join("authors/ann/notes")).unwrap();
        assert_eq!(new_world_folder("bell", &d.join("authors/ann/notes"), "ann"), d.join("authors/ann/bell"));
        // A path is a path.
        assert_eq!(new_world_folder("./mine", &d, "ann"), d.join("./mine"));
        assert_eq!(new_world_folder("worlds/bell", &d, "ann"), d.join("worlds/bell"));

        let root = d.join("authors/ann/bell");
        create_world(&root, "The Bell: a tale", "ann").unwrap();
        let text = std::fs::read_to_string(root.join("world/index.md")).unwrap();
        assert_eq!(text, "---\ntitle: \"The Bell: a tale\"\ntype: world\nauthors: ann\n---\n");
        assert!(is_world(&root));
        let found = find_worlds(&d);
        let bell = found.iter().find(|w| w.root == root).unwrap();
        assert_eq!((bell.title.as_str(), bell.authors.clone()), ("The Bell: a tale", vec!["ann".to_string()]));
        // Never over a folder with something in it.
        assert!(create_world(&root, "Again", "ann").unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn with_no_worlds_it_says_how_to_make_one() {
        let err = WorldNotGiven { found: Vec::new(), from: PathBuf::from("/x") };
        assert!(err.to_string().contains("there are no worlds") && err.to_string().contains("thwart edit <that folder>"));
    }
}
