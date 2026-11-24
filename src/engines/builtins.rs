//! Templates the engine supplies (REQ-CF-21, `design/STATES.md` ›
//! Built-in mechanics). Each is an ordinary Markdown template, compiled
//! in, so an author can read one and copy it to change it: one file, or
//! a bundle (an index and its states). A world template of the same name
//! overrides one.

use super::layers::Layer;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Every built-in, by name, with its files: `index.md` (or the name's one
/// file) first, then any state files.
pub const BUILTINS: &[(&str, &[(&str, &str)])] = &[
    ("scenery", &[("scenery.md", include_str!("../builtins/scenery.md"))]),
    (
        "door",
        &[
            ("door/index.md", include_str!("../builtins/door/index.md")),
            ("door/closed.md", include_str!("../builtins/door/closed.md")),
            ("door/open.md", include_str!("../builtins/door/open.md")),
            ("door/actions.rhai", include_str!("../builtins/door/actions.rhai")),
        ],
    ),
    (
        "locked-door",
        &[
            ("locked-door/index.md", include_str!("../builtins/locked-door/index.md")),
            ("locked-door/locked.md", include_str!("../builtins/locked-door/locked.md")),
            ("locked-door/closed.md", include_str!("../builtins/locked-door/closed.md")),
            ("locked-door/open.md", include_str!("../builtins/locked-door/open.md")),
            ("locked-door/actions.rhai", include_str!("../builtins/locked-door/actions.rhai")),
        ],
    ),
    (
        "portal",
        &[
            ("portal/index.md", include_str!("../builtins/portal/index.md")),
            ("portal/closed.md", include_str!("../builtins/portal/closed.md")),
            ("portal/open.md", include_str!("../builtins/portal/open.md")),
            ("portal/actions.rhai", include_str!("../builtins/portal/actions.rhai")),
        ],
    ),
];

fn files(name: &str) -> Option<&'static [(&'static str, &'static str)]> {
    BUILTINS.iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
}

fn path(file: &str) -> PathBuf {
    Path::new("<built-in>").join(file)
}

/// The built-in template `name`, as its index layer, whose path reads
/// `<built-in>/door/index.md` in messages.
pub fn builtin(name: &str) -> Option<Layer> {
    let (file, text) = files(name)?.first()?;
    Layer::parse(&path(file), text).ok()
}

/// The states a built-in bundle has, by name.
pub fn builtin_states(name: &str) -> BTreeMap<String, Layer> {
    let mut states = BTreeMap::new();
    for (file, text) in files(name).unwrap_or_default().iter().skip(1).filter(|(f, _)| f.ends_with(".md")) {
        let state = Path::new(file).file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        if let Ok(layer) = Layer::parse(&path(file), text) {
            states.insert(state, layer);
        }
    }
    states
}

/// A built-in's script (`actions.rhai`), if it has one.
pub fn builtin_script(name: &str) -> Option<super::scripts::Script> {
    files(name)?.iter().find(|(f, _)| f.ends_with(".rhai")).map(|(f, t)| super::scripts::Script { path: path(f), source: t.to_string() })
}

/// A built-in's files as written, for showing it: each file's path and
/// text.
pub fn builtin_files(name: &str) -> Vec<(PathBuf, &'static str)> {
    files(name).unwrap_or_default().iter().map(|(f, t)| (path(f), *t)).collect()
}

/// The built-in a layer from `builtin` came from, if it is one.
pub fn builtin_name(layer_path: &Path) -> Option<&'static str> {
    BUILTINS.iter().find(|(_, files)| files.first().map_or(false, |(f, _)| path(f) == layer_path)).map(|(n, _)| *n)
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_builtin_parses_and_is_a_template_or_a_state() {
        for (name, files) in super::BUILTINS {
            let index = super::builtin(name).unwrap_or_else(|| panic!("{} doesn't parse", name));
            assert!(index.metadata.get("type").is_some(), "{}", name);
            let states = files.iter().skip(1).filter(|(f, _)| f.ends_with(".md")).count();
            assert_eq!(super::builtin_states(name).len(), states, "{}", name);
            if let Some(script) = super::builtin_script(name) {
                assert!(crate::engines::scripts::compile(&[script]).is_ok(), "{}", name);
            }
            assert_eq!(super::builtin_name(&index.path), Some(*name));
        }
        assert!(super::builtin_states("door").contains_key("open"));
    }
}
