//! Layers and merging (`design/STATES.md` › What applies, Inheritance).
//!
//! A thing's version is built from files stacked in order: area overlays,
//! its `index.md` (or its one file), its current state, its own chapter
//! overlays. Each `Layer` is one file split into raw YAML and prose; a
//! `Merged` is the result of stacking them, before it is typed as a
//! location, object or character. Every key remembers the file it came
//! from, for error messages and the console's "(from index)" labels.

use crate::parsers::{MarkdownParser, ParsedDocument};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One file's frontmatter and prose, untyped.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub path: PathBuf,
    pub metadata: Mapping,
    pub body: String,
}

impl Layer {
    /// Split `text`, the file at `path`. The error is the reason it can't
    /// be split, without the path.
    pub fn parse(path: &Path, text: &str) -> Result<Layer, String> {
        let doc = MarkdownParser::new().split_document(text).map_err(|e| e.to_string())?;
        let metadata = match doc.metadata {
            Value::Null => Mapping::new(),
            Value::Mapping(map) => map,
            _ => return Err("frontmatter must be a map of keys and values".to_string()),
        };
        Ok(Layer { path: path.to_path_buf(), metadata, body: doc.body })
    }

    /// The value of a top-level key, if it is a string.
    pub fn str_key(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(Value::as_str)
    }

    pub fn has_key(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }

    /// Whether this layer's prose replaces the prose below it: only if
    /// something is left once author comments are removed, so an overlay
    /// holding only a note never blanks a room.
    pub fn has_prose(&self) -> bool {
        !strip_comments(&self.body).trim().is_empty()
    }
}

/// `text` without its `<!-- ... -->` author comments. An unclosed comment
/// runs to the end, as in HTML.
pub fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Keys that direct the layering and are not part of the thing.
const DIRECTIVES: &[&str] = &["inherits"];

/// Layers stacked into one document, with where each part came from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Merged {
    pub metadata: Mapping,
    /// The file each key was last set by, by dotted path
    /// (`connections.north`). A map set whole is one entry; use
    /// `source_of` to look up a key inside it.
    pub sources: BTreeMap<String, PathBuf>,
    pub body: String,
    /// The file the prose came from; `None` if no layer had any.
    pub body_source: Option<PathBuf>,
    /// Each list a layer replaced and each key a `null` removed. These
    /// warn, never fail (Bruce, 2026-11-23).
    pub warnings: Vec<String>,
}

impl Merged {
    /// Stack `layers`, first at the bottom.
    pub fn of<'a>(layers: impl IntoIterator<Item = &'a Layer>) -> Merged {
        let mut merged = Merged::default();
        for layer in layers {
            merged.apply(layer);
        }
        merged
    }

    /// Put `layer` on top. Scalars and lists replace what's below, maps
    /// merge key by key all the way down, and `null` removes an inherited
    /// key. Prose replaces the prose below only if the layer has some.
    pub fn apply(&mut self, layer: &Layer) {
        let mut over = layer.metadata.clone();
        for key in DIRECTIVES {
            over.remove(*key);
        }
        let mut metadata = std::mem::take(&mut self.metadata);
        self.merge_map(&mut metadata, &over, "", &layer.path);
        self.metadata = metadata;
        if layer.has_prose() {
            self.body = layer.body.clone();
            self.body_source = Some(layer.path.clone());
        }
    }

    fn merge_map(&mut self, base: &mut Mapping, over: &Mapping, prefix: &str, path: &Path) {
        for (key, value) in over {
            let full = format!("{}{}", prefix, key_name(key));
            match (base.get_mut(key), value) {
                (Some(_), Value::Null) => {
                    let from = self.from_note(&full);
                    base.remove(key);
                    self.forget(&full);
                    self.warnings.push(format!(
                        "{}: `{}` is empty, so it removes the value inherited{}",
                        path.display(),
                        full,
                        from
                    ));
                }
                (Some(Value::Mapping(below)), Value::Mapping(above)) => {
                    let mut below = std::mem::take(below);
                    self.merge_map(&mut below, above, &format!("{}.", full), path);
                    base.insert(key.clone(), Value::Mapping(below));
                }
                (below, value) => {
                    if let (Some(Value::Sequence(_)), Value::Sequence(_)) = (&below, value) {
                        self.warnings.push(format!(
                            "{}: `{}` replaces the whole list inherited{}",
                            path.display(),
                            full,
                            self.from_note(&full)
                        ));
                    }
                    base.insert(key.clone(), value.clone());
                    self.forget(&full);
                    self.sources.insert(full, path.to_path_buf());
                }
            }
        }
    }

    fn forget(&mut self, full: &str) {
        let nested = format!("{}.", full);
        self.sources.retain(|k, _| k != full && !k.starts_with(&nested));
    }

    fn from_note(&self, full: &str) -> String {
        match self.source_of(full) {
            Some(p) => format!(" from {}", p.display()),
            None => String::new(),
        }
    }

    /// The file a key's value came from: its own entry, or the nearest
    /// map above it that was set whole.
    pub fn source_of(&self, key_path: &str) -> Option<&Path> {
        let mut path = key_path;
        loop {
            if let Some(source) = self.sources.get(path) {
                return Some(source);
            }
            path = &path[..path.rfind('.')?];
        }
    }

    /// The merged result, ready to be typed.
    pub fn document(&self) -> ParsedDocument {
        ParsedDocument { metadata: Value::Mapping(self.metadata.clone()), body: self.body.clone() }
    }
}

fn key_name(key: &Value) -> String {
    match key {
        Value::String(s) => s.clone(),
        other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(path: &str, text: &str) -> Layer {
        Layer::parse(Path::new(path), text).unwrap()
    }

    #[test]
    fn maps_merge_deeply_scalars_and_lists_replace_and_null_removes() {
        let index = layer(
            "vault/index.md",
            "---\nid: vault\ntitle: Vault\ntype: location\nconnections:\n  north: hall\n  east: yard\nobjects: [chest]\nvars:\n  door: { colour: grey, hinges: 2 }\n---\nA vault.\n",
        );
        let open = layer(
            "vault/open.md",
            "---\ninherits: index\ntitle: Open vault\nconnections:\n  down: stairs\n  east:\nobjects: [chest, lamp]\nvars:\n  door: { hinges: 1 }\n---\n",
        );
        let merged = Merged::of([&index, &open]);
        let m = &merged.metadata;
        assert_eq!(m["title"], Value::from("Open vault"));
        assert!(m.get("inherits").is_none());
        let connections = m["connections"].as_mapping().unwrap();
        assert_eq!(connections.len(), 2);
        assert_eq!(connections["north"], Value::from("hall"));
        assert_eq!(connections["down"], Value::from("stairs"));
        assert_eq!(m["objects"].as_sequence().unwrap().len(), 2);
        assert_eq!(m["vars"]["door"]["colour"], Value::from("grey"));
        assert_eq!(m["vars"]["door"]["hinges"], Value::from(1));

        // Where each part came from.
        assert_eq!(merged.source_of("title"), Some(Path::new("vault/open.md")));
        assert_eq!(merged.source_of("connections.north"), Some(Path::new("vault/index.md")));
        assert_eq!(merged.source_of("connections.down"), Some(Path::new("vault/open.md")));
        assert_eq!(merged.source_of("vars.door.colour"), Some(Path::new("vault/index.md")));
        assert_eq!(merged.source_of("vars.door.hinges"), Some(Path::new("vault/open.md")));

        // No prose in the state, so the index's stays.
        assert_eq!(merged.body, "A vault.");
        assert_eq!(merged.body_source.as_deref(), Some(Path::new("vault/index.md")));

        assert_eq!(merged.warnings.len(), 2, "{:?}", merged.warnings);
        assert!(merged.warnings.iter().any(|w| w.starts_with("vault/open.md: `connections.east` is empty")
            && w.ends_with("from vault/index.md")));
        assert!(merged.warnings.iter().any(|w| w.contains("`objects` replaces the whole list inherited from vault/index.md")));
    }

    #[test]
    fn a_layer_of_only_comments_keeps_the_prose_below() {
        let index = layer("yard.md", "---\ntitle: Yard\n---\nGrass.\n");
        let note = layer("chapters/after.md", "---\n---\n<!-- the court splits -->\n");
        let merged = Merged::of([&index, &note]);
        assert_eq!(merged.body, "Grass.");
        assert!(!note.has_prose());

        let prose = layer("chapters/after.md", "---\n---\n<!-- note -->\nAshes.\n");
        assert_eq!(Merged::of([&index, &prose]).body, "<!-- note -->\nAshes.");
    }

    #[test]
    fn comments_are_stripped_including_an_unclosed_one() {
        assert_eq!(strip_comments("a <!-- x --> b <!-- y"), "a  b ");
        assert_eq!(strip_comments("<!--\nmulti\n-->"), "");
    }

    #[test]
    fn a_null_with_nothing_below_is_kept_and_not_warned() {
        let only = layer("hall.md", "---\ntitle: Hall\nconnections:\n  north:\n---\n");
        let merged = Merged::of([&only]);
        assert!(merged.warnings.is_empty());
        assert_eq!(merged.metadata["connections"]["north"], Value::Null);
    }

    #[test]
    fn frontmatter_that_is_not_a_map_is_refused() {
        assert!(Layer::parse(Path::new("x.md"), "---\n- a\n---\n").is_err());
        assert!(Layer::parse(Path::new("x.md"), "---\n---\n").unwrap().metadata.is_empty());
    }
}
