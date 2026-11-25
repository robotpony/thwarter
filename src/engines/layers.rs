//! Layers and merging (`design/STATES.md` › What applies, Inheritance).
//!
//! A thing's version is built from files stacked in order: area overlays,
//! its `index.md` (or its one file), its current state, its own chapter
//! overlays. Each `Layer` is one file split into raw YAML and prose; a
//! `Merged` is the result of stacking them, before it is typed as a
//! location, object or character. Every key remembers the file it came
//! from, for error messages and the console's "(from index)" labels.
//!
//! A file's body may hold keyword sections (`design/SECTIONS.md`):
//! `## When: #locked` or `## Unless: #locked`, each with an optional
//! ` ```thwarter ` block of data first and its own prose. Each section is
//! kept as a layer of its own, out of the description. Sections never
//! create states; they're shown when their tags match the game state.

use crate::parsers::{MarkdownParser, ParsedDocument};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The largest content file the loader reads (REQ-CFG-10): far more than
/// any room or card needs, small enough that a hostile file can't
/// exhaust memory.
pub const MAX_FILE_BYTES: usize = 1024 * 1024;

/// The deepest frontmatter nesting the loader accepts (REQ-CFG-10).
pub const MAX_DEPTH: usize = 32;

fn depth(value: &Value) -> usize {
    match value {
        Value::Mapping(m) => 1 + m.iter().map(|(k, v)| depth(k).max(depth(v))).max().unwrap_or(0),
        Value::Sequence(s) => 1 + s.iter().map(depth).max().unwrap_or(0),
        Value::Tagged(t) => 1 + depth(&t.value),
        _ => 0,
    }
}

/// Tags a section may use that aren't states: replies for engine moments
/// (`design/SECTIONS.md` › Replies), and the outcome names of built-in
/// replies a story may reword (`design/ENGINE_TEXT.md` › Story overrides;
/// `turn::OUTCOMES`). `#locked` and `#closed` are outcomes too, but they
/// mean the door's state of the same name, so they aren't reserved.
pub const RESERVED_TAGS: &[&str] = &[
    "#ambiguous", "#searched", "#not-understood", "#no-noun", "#not-here", "#nowhere", "#no-exit", "#carried", "#person",
    "#scenery", "#immovable", "#nothing", "#found", "#acted", "#refused", "#ambiguous-verb", "#suggest", "#where", "#look-toward", "#not-carried", "#no-person",
];

/// One file's frontmatter and prose, untyped.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layer {
    pub path: PathBuf,
    pub metadata: Mapping,
    /// The prose outside any keyword section.
    pub body: String,
    /// The keyword sections, in file order.
    pub sections: Vec<Section>,
    /// What's odd about the file but still read, such as a `thwarter`
    /// block outside a section.
    pub warnings: Vec<String>,
    /// A ```rhai block in its prose: the thing's script, kept out of the
    /// description (`design/SECTIONS.md` › Scripts).
    pub script: Option<String>,
}

/// A `## When:` or `## Unless:` section.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// `Unless:` rather than `When:`.
    pub unless: bool,
    /// Its tags as written: `#open` for a state or an action, `@stick`
    /// for a thing (`design/SECTIONS.md`).
    pub tags: Vec<String>,
    /// Its block's data and its prose, with the file's path.
    pub layer: Layer,
}

impl Section {
    /// The reserved tag this section answers to (`#ambiguous`), if any.
    pub fn reserved(&self) -> Option<&str> {
        match self.tags.as_slice() {
            [tag] if !self.unless && RESERVED_TAGS.contains(&tag.as_str()) => Some(tag),
            _ => None,
        }
    }

    /// Whether it matches a turn whose tags are `has`: `When:` if every
    /// tag is there, `Unless:` if none is.
    pub fn matches(&self, has: impl Fn(&str) -> bool) -> bool {
        if self.unless {
            !self.tags.iter().any(|t| has(t))
        } else {
            self.tags.iter().all(|t| has(t))
        }
    }
}

/// Split a body into its default prose and its keyword sections. Only
/// `##` headings starting `When:` or `Unless:` open a section, which runs
/// to the next `#` or `##` heading; headings inside fenced code are
/// ignored.
fn split_sections(path: &Path, body: &str) -> Result<(String, Vec<Section>, Vec<String>), String> {
    let mut default: Vec<&str> = Vec::new();
    let mut sections: Vec<(bool, Vec<String>, Vec<&str>, usize)> = Vec::new();
    let mut in_section = false;
    let mut fence: Option<&str> = None;
    for (n, line) in body.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(open) = fence {
            if trimmed.starts_with(open) {
                fence = None;
            }
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(&trimmed[..3]);
        } else if let Some(heading) = line.strip_prefix("## ").or_else(|| line.strip_prefix("# ").map(|_| "")) {
            let heading = heading.trim();
            let keyword = heading.strip_prefix("When:").map(|r| (false, r)).or_else(|| heading.strip_prefix("Unless:").map(|r| (true, r)));
            match keyword {
                Some((unless, rest)) => {
                    let mut tags = Vec::new();
                    for word in rest.split_whitespace() {
                        let name = word.strip_prefix('#').or_else(|| word.strip_prefix('@'));
                        match name.filter(|t| !t.is_empty() && !t.contains(['#', '@'])) {
                            Some(_) => tags.push(word.to_string()),
                            None => return Err(format!("`{}` takes #states and @things, and `{}` is neither", heading, word)),
                        }
                    }
                    if tags.is_empty() {
                        return Err(format!("`{}` names no #state or @thing", heading));
                    }
                    sections.push((unless, tags, Vec::new(), n));
                    in_section = true;
                    continue;
                }
                None => in_section = false,
            }
        }
        match (in_section, sections.last_mut()) {
            (true, Some(section)) => section.2.push(line),
            _ => default.push(line),
        }
    }

    let mut warnings = Vec::new();
    let default = default.join("\n").trim().to_string();
    if default.lines().any(|l| is_block_fence(l)) {
        warnings.push(format!("{}: a ```thwarter block outside a `## When:` section is shown as code; frontmatter holds the default data", path.display()));
    }
    let mut out = Vec::new();
    for (unless, tags, lines, _) in sections {
        let heading = format!("{} {}", if unless { "Unless:" } else { "When:" }, tags.join(" "));
        let text = lines.join("\n");
        let text = text.trim();
        let (metadata, prose) = match text.lines().next() {
            Some(first) if is_block_fence(first) => {
                let rest = &text[first.len()..];
                let end = rest.lines().position(|l| l.trim() == "```").ok_or_else(|| format!("the ```thwarter block in `## {}` is never closed", heading))?;
                let yaml: Vec<&str> = rest.lines().skip(1).take(end - 1).collect();
                let after: Vec<&str> = rest.lines().skip(end + 1).collect();
                let yaml = yaml.join("\n");
                let value: Value = if yaml.trim().is_empty() {
                    Value::Null
                } else {
                    serde_yaml::from_str(&yaml).map_err(|e| format!("the ```thwarter block in `## {}` doesn't parse: {}", heading, e))?
                };
                if depth(&value) > MAX_DEPTH {
                    return Err(format!("the ```thwarter block in `## {}` nests more than {} levels deep", heading, MAX_DEPTH));
                }
                let mut map = match value {
                    Value::Null => Mapping::new(),
                    Value::Mapping(m) => m,
                    _ => return Err(format!("the ```thwarter block in `## {}` must be a map of keys and values", heading)),
                };
                for key in ["id", "type", "state"] {
                    if map.remove(key).is_some() {
                        warnings.push(format!("{}: `## {}` sets `{}`, which a state can't change, so it's ignored", path.display(), heading, key));
                    }
                }
                (map, after.join("\n").trim().to_string())
            }
            _ => (Mapping::new(), text.to_string()),
        };
        if prose.lines().any(|l| is_block_fence(l)) {
            warnings.push(format!("{}: only a ```thwarter block first in `## {}` is read as data; a later one is shown as code", path.display(), heading));
        }
        out.push(Section { unless, tags, layer: Layer { path: path.to_path_buf(), metadata, body: prose, ..Default::default() } });
    }
    Ok((default, out, warnings))
}

/// The prose without its ```rhai block, and the block. More than one is
/// a problem, since a thing has one script.
fn take_script(path: &Path, body: &str, warnings: &mut Vec<String>) -> Result<(String, Option<String>), String> {
    let mut prose: Vec<&str> = Vec::new();
    let mut script: Option<Vec<&str>> = None;
    let mut inside = false;
    let mut found = 0;
    for line in body.lines() {
        if inside {
            if line.trim() == "```" {
                inside = false;
            } else if let Some(s) = script.as_mut() {
                s.push(line);
            }
            continue;
        }
        if line.trim().strip_prefix("```").map(str::trim) == Some("rhai") {
            found += 1;
            inside = true;
            script.get_or_insert_with(Vec::new);
            continue;
        }
        prose.push(line);
    }
    if inside {
        return Err("the ```rhai block is never closed".to_string());
    }
    if found > 1 {
        return Err("a thing has one script, and this file has more than one ```rhai block".to_string());
    }
    let _ = (path, warnings);
    Ok((prose.join("\n").trim().to_string(), script.map(|s| s.join("\n"))))
}

fn is_block_fence(line: &str) -> bool {
    line.trim().strip_prefix("```").map(str::trim) == Some("thwarter")
}

impl Layer {
    /// Split `text`, the file at `path`. The error is the reason it can't
    /// be split, without the path.
    pub fn parse(path: &Path, text: &str) -> Result<Layer, String> {
        if text.len() > MAX_FILE_BYTES {
            return Err(format!("is {} KB, over the {} KB a content file may be, so it isn't read", text.len() / 1024, MAX_FILE_BYTES / 1024));
        }
        let doc = MarkdownParser::new().split_document(text).map_err(|e| e.to_string())?;
        if depth(&doc.metadata) > MAX_DEPTH {
            return Err(format!("frontmatter nests more than {} levels deep, so it isn't read", MAX_DEPTH));
        }
        let metadata = match doc.metadata {
            Value::Null => Mapping::new(),
            Value::Mapping(map) => map,
            _ => return Err("frontmatter must be a map of keys and values".to_string()),
        };
        let (body, sections, mut warnings) = split_sections(path, &doc.body)?;
        let (body, script) = take_script(path, &body, &mut warnings)?;
        Ok(Layer { path: path.to_path_buf(), metadata, body, sections, warnings, script })
    }

    /// The value of a top-level key, if it is a string.
    pub fn str_key(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(Value::as_str)
    }

    pub fn has_key(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }

    /// Whether this layer's description replaces the one below it: only
    /// if something is left once author comments are removed, so an
    /// overlay holding only a note never blanks a room.
    pub fn has_prose(&self) -> bool {
        !strip_comments(&self.body).trim().is_empty()
    }

    /// The prose of its section for a reserved tag (`#ambiguous`).
    pub fn reply(&self, tag: &str) -> Option<&str> {
        self.sections.iter().find(|s| s.reserved() == Some(tag)).map(|s| s.layer.body.as_str()).filter(|t| !strip_comments(t).trim().is_empty())
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
    /// The description: the thing's own, then each folder layer's added
    /// after it.
    pub body: String,
    /// The file the thing's own description came from; `None` if no
    /// layer had one.
    pub body_source: Option<PathBuf>,
    /// Each list a layer replaced and each key a `null` removed. These
    /// warn, never fail (Bruce, 2026-11-23).
    pub warnings: Vec<String>,
    own: String,
    added: Vec<String>,
}

impl Merged {
    /// Stack `layers`, first at the bottom, all as the thing's own.
    pub fn of<'a>(layers: impl IntoIterator<Item = &'a Layer>) -> Merged {
        let mut merged = Merged::default();
        for layer in layers {
            merged.apply(layer);
        }
        merged
    }

    /// Put one of the thing's own layers on top. Scalars and lists replace
    /// what's below, maps merge key by key all the way down, and `null`
    /// removes an inherited key. The description replaces the one below
    /// only if the layer has one.
    pub fn apply(&mut self, layer: &Layer) {
        self.merge_frontmatter(layer);
        if layer.has_prose() {
            self.own = layer.body.clone();
            self.body_source = Some(layer.path.clone());
        }
        self.compose();
    }

    /// Put a layer from a folder above the thing on top: frontmatter
    /// merges as for `apply`, but its description is added after the
    /// thing's rather than replacing it (Bruce, 2026-11-23).
    pub fn apply_area(&mut self, layer: &Layer) {
        self.merge_frontmatter(layer);
        if layer.has_prose() {
            self.added.push(layer.body.clone());
        }
        self.compose();
    }

    fn merge_frontmatter(&mut self, layer: &Layer) {
        let mut over = layer.metadata.clone();
        for key in DIRECTIVES {
            over.remove(*key);
        }
        let mut metadata = std::mem::take(&mut self.metadata);
        self.merge_map(&mut metadata, &over, "", &layer.path);
        self.metadata = metadata;
    }

    fn compose(&mut self) {
        let parts: Vec<&str> = std::iter::once(self.own.as_str())
            .chain(self.added.iter().map(String::as_str))
            .filter(|p| !p.trim().is_empty())
            .collect();
        self.body = parts.join("\n\n");
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

    #[test]
    fn sections_split_out_with_their_blocks_and_never_join_the_description() {
        let text = "---\ntitle: Door\n---\nA heavy door.\n\n## Notes\nPlain heading.\n\n## When: #locked\n```thwarter\npassable: no\nid: nope\n```\nA padlock hangs here.\n\n### Detail\nStill locked prose.\n\n## Unless: #open @lamp\nDark.\n\n```\n## When: #fake\n```\n";
        let l = layer("door.md", text);
        assert_eq!(l.body, "A heavy door.\n\n## Notes\nPlain heading.");
        assert_eq!(l.sections.len(), 2);
        let locked = &l.sections[0];
        assert!(!locked.unless);
        assert_eq!(locked.tags, ["#locked"]);
        assert_eq!(locked.layer.metadata["passable"], Value::from("no")); // YAML 1.2
        assert!(locked.layer.metadata.get("id").is_none());
        assert_eq!(locked.layer.body, "A padlock hangs here.\n\n### Detail\nStill locked prose.");
        let dark = &l.sections[1];
        // A heading inside fenced code doesn't open a section.
        assert!(dark.unless && dark.tags == ["#open", "@lamp"] && dark.layer.body == "Dark.\n\n```\n## When: #fake\n```");
        assert!(l.warnings.iter().any(|w| w.contains("sets `id`")), "{:?}", l.warnings);

        // All of `When:`, none of `Unless:`.
        let turn = |tags: &'static [&'static str]| move |t: &str| tags.contains(&t);
        assert!(locked.matches(turn(&["#locked", "@door"])));
        assert!(!locked.matches(turn(&["#open"])));
        assert!(dark.matches(turn(&["#closed"])));
        assert!(!dark.matches(turn(&["@lamp"])));

        let reply = layer("t.md", "---\n---\n## When: #ambiguous\nWhich one?\n");
        assert_eq!(reply.reply("#ambiguous"), Some("Which one?"));
        assert_eq!(reply.sections[0].reserved(), Some("#ambiguous"));
    }

    #[test]
    fn bad_sections_are_problems_or_warnings() {
        for (body, why) in [
            ("## When: locked\n", "is neither"),
            ("## When:\n", "names no #state"),
            ("## When: #a\n```thwarter\nx: [\n```\n", "doesn't parse"),
            ("## When: #a\n```thwarter\nx: 1\n", "never closed"),
            ("## When: #a\n```thwarter\n- 1\n```\n", "must be a map"),
        ] {
            let err = Layer::parse(Path::new("x.md"), &format!("---\n---\n{}", body)).unwrap_err();
            assert!(err.contains(why), "{}: {}", body, err);
        }
        let l = layer("x.md", "---\n---\n```thwarter\na: 1\n```\n\n## When: #a\nText.\n\n```thwarter\nb: 2\n```\n");
        assert!(l.warnings.iter().any(|w| w.contains("outside a `## When:` section")), "{:?}", l.warnings);
        assert!(l.warnings.iter().any(|w| w.contains("a later one is shown as code")), "{:?}", l.warnings);
    }
}
