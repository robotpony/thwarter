//! Personas (REQ-WS-4, 5): who writes a world. A persona is
//! `authors/<id>/index.md` (`type: author`, a `title` that is the pen name,
//! contact details, and a bio as its prose), and a world's
//! `world/index.md` names one by id (`author: bob`). A persona folder
//! without `index.md` still works, with its folder name as the pen name.

use super::layers::Layer;
use serde::Serialize;
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Persona {
    /// The folder's name, which worlds use in `author:`.
    pub id: String,
    /// The pen name readers see.
    pub title: String,
    /// Contact details (`email`, `website` and any others), which travel
    /// with every packaged world (REQ-WS-6).
    pub contacts: BTreeMap<String, String>,
    pub bio: String,
    /// The writing style the persona uses by default, if any.
    pub style: Option<String>,
    /// The world the persona opens by default, if any.
    pub world: Option<String>,
    /// Its `index.md`, if it has one.
    pub path: Option<PathBuf>,
}

/// Keys a persona file reads that aren't contact details.
const OWN_KEYS: &[&str] = &["type", "title", "style", "world"];

/// The persona in `folder` (`authors/<id>/`), or why its file can't be
/// read. `Ok(None)` if there's no such folder.
pub fn read_persona(folder: &Path) -> Result<Option<Persona>, String> {
    if !folder.is_dir() {
        return Ok(None);
    }
    let id = folder.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let index = folder.join("index.md");
    let Ok(text) = std::fs::read_to_string(&index) else {
        return Ok(Some(Persona { title: id.clone(), id, contacts: BTreeMap::new(), bio: String::new(), style: None, world: None, path: None }));
    };
    let layer = Layer::parse(&index, &text).map_err(|e| format!("{}: {}", index.display(), e))?;
    match layer.str_key("type") {
        Some("author") => {}
        Some(other) => return Err(format!("{}: a persona is `type: author`, not `{}`", index.display(), other)),
        None => return Err(format!("{}: a persona needs `type: author`", index.display())),
    }
    let text_of = |v: &Value| crate::engines::world_def::value_text(v);
    let mut contacts = BTreeMap::new();
    for (key, value) in &layer.metadata {
        let Some(key) = key.as_str() else { continue };
        if OWN_KEYS.contains(&key) {
            continue;
        }
        match text_of(value) {
            Some(v) => {
                contacts.insert(key.to_string(), v);
            }
            None => return Err(format!("{}: `{}` must be a single value", index.display(), key)),
        }
    }
    Ok(Some(Persona {
        title: layer.str_key("title").unwrap_or(&id).to_string(),
        id,
        contacts,
        bio: layer.body.clone(),
        style: layer.str_key("style").map(str::to_string),
        world: layer.str_key("world").map(str::to_string),
        path: Some(index),
    }))
}

/// Every persona in an `authors/` folder, by id. A folder whose
/// `index.md` can't be read is left out; `read_persona` says why.
pub fn list_personas(authors: &Path) -> Vec<Persona> {
    let Ok(entries) = std::fs::read_dir(authors) else { return Vec::new() };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir() && !p.file_name().map_or(true, |n| n.to_string_lossy().starts_with('.')))
        .collect();
    folders.sort();
    folders.into_iter().filter_map(|f| read_persona(&f).ok().flatten()).collect()
}

/// The folder a world's persona lives in: the world's parent, if it's
/// named for the persona (`authors/bob/castle/`), else a sibling of that
/// folder (`authors/bob/` beside `authors/alice/castle/`).
pub fn persona_folder(world_root: &Path, id: &str) -> Option<PathBuf> {
    let world = std::fs::canonicalize(world_root).unwrap_or_else(|_| world_root.to_path_buf());
    let parent = world.parent()?;
    if parent.file_name().map_or(false, |n| n == id) {
        return Some(parent.to_path_buf());
    }
    Some(parent.parent()?.join(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_persona_is_read_from_its_file_or_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let authors = dir.path().join("authors");
        std::fs::create_dir_all(authors.join("bob/castle")).unwrap();
        std::fs::create_dir_all(authors.join("ann")).unwrap();
        std::fs::write(
            authors.join("bob/index.md"),
            "---\ntype: author\ntitle: B. Writer\nemail: bob@example.com\nwebsite: https://example.com\nmastodon: \"@bob@example.social\"\nstyle: terse\n---\nWrites about castles.\n",
        )
        .unwrap();
        let bob = read_persona(&authors.join("bob")).unwrap().unwrap();
        assert_eq!(bob.title, "B. Writer");
        assert_eq!(bob.contacts.keys().collect::<Vec<_>>(), ["email", "mastodon", "website"]);
        assert_eq!(bob.bio, "Writes about castles.");
        assert_eq!(bob.style.as_deref(), Some("terse"));

        let ann = read_persona(&authors.join("ann")).unwrap().unwrap();
        assert_eq!((ann.title.as_str(), ann.path.is_none()), ("ann", true));
        assert_eq!(read_persona(&authors.join("nobody")).unwrap(), None);
        assert_eq!(list_personas(&authors).iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["ann", "bob"]);

        std::fs::write(authors.join("ann/index.md"), "---\ntype: world\n---\n").unwrap();
        assert!(read_persona(&authors.join("ann")).unwrap_err().contains("a persona is `type: author`"));
        assert_eq!(persona_folder(&authors.join("bob/castle"), "bob").unwrap(), std::fs::canonicalize(authors.join("bob")).unwrap());
        assert_eq!(persona_folder(&authors.join("bob/castle"), "ann").unwrap(), std::fs::canonicalize(&authors).unwrap().join("ann"));
    }
}
