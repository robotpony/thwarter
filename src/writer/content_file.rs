//! One content file held as text, edited one field at a time (REQ-ENG-7).
//!
//! The file is never re-serialized. An edit finds the key's lines in the
//! frontmatter text and replaces only its value, so field order, comments,
//! spacing, line endings and the body stay as they were. Every edit is
//! then checked by re-parsing: the frontmatter must parse to exactly the
//! old mapping with that one change applied, and the whole file must
//! still parse as the same entity. An edit that fails either check, or
//! that targets a YAML shape this editor doesn't understand, is refused
//! and leaves the file as it was.

use crate::parsers::{known_keys, MarkdownParser, ParsedEntity};
use anyhow::{anyhow, bail, Context, Result};
use serde_yaml::{Mapping, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One change to a content file.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Set a top-level key to a scalar (string, number or bool), adding it
    /// if absent. Refused for `id` and `type`.
    SetScalar { key: String, value: Value },
    /// Set a top-level key to a list of strings (`objects`, `inventory`).
    SetList { key: String, items: Vec<String> },
    /// Set one entry of a top-level map (`connections`, `attributes`).
    SetMapEntry { key: String, subkey: String, value: Value },
    /// Remove one entry of a top-level map. An error if it isn't there.
    RemoveMapEntry { key: String, subkey: String },
    /// Remove a top-level key and its value. Nothing happens if absent.
    /// Refused for `id` and `type`.
    RemoveKey { key: String },
    /// Replace the body (the prose after the frontmatter).
    SetBody(String),
    /// Replace the whole file (full editing, REQ-CON-6). The new text must
    /// parse as an entity with the same `type` and `id`.
    ReplaceAll(String),
}

/// A content file's text, split so the frontmatter can be edited and
/// everything else written back byte for byte.
#[derive(Debug, Clone)]
pub struct ContentFile {
    path: PathBuf,
    /// The opening `---` line, with its line ending.
    opening: String,
    /// Frontmatter lines, each with its line ending.
    lines: Vec<String>,
    /// The closing `---` line, with its line ending if it has one.
    closing: String,
    /// Everything after the closing line, untouched unless `SetBody`.
    body: String,
    eol: &'static str,
    type_name: String,
    id: String,
}

impl ContentFile {
    /// Read and split a file. It must already parse as an entity.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).with_context(|| format!("can't read {}", path.display()))?;
        Self::from_text(path, &text)
    }

    /// Split `text` as if it were the file at `path`.
    pub fn from_text(path: impl AsRef<Path>, text: &str) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let (type_name, id) =
            identity(text, &path).with_context(|| format!("{} doesn't parse, so it can't be edited", path.display()))?;
        Self::split(path, text, type_name, id)
    }

    /// Read a file that is a layer rather than a whole thing (a state, an
    /// overlay, a chapter, `world/index.md`): it has no `id` or `type` of
    /// its own, so edits are checked only for frontmatter that parses.
    pub fn load_layer(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).with_context(|| format!("can't read {}", path.display()))?;
        MarkdownParser::new()
            .split_document(&text)
            .with_context(|| format!("{} doesn't parse, so it can't be edited", path.display()))?;
        Self::split(path.to_path_buf(), &text, String::new(), String::new())
    }

    /// Replace every whole-word `old` with `new` in the value of the
    /// top-level `key` (its lines only, comments included), for
    /// references nested too deep for the other edits (an action's
    /// `becomes:` and `requires:`). The safety net is the result: the
    /// frontmatter must then read back exactly as `expected`, or nothing
    /// changes. A word is bounded by characters an id can't hold.
    pub(crate) fn replace_in_key(&mut self, key: &str, old: &str, new: &str, expected: &Mapping) -> Result<()> {
        let entry = self.find(key)?.ok_or_else(|| anyhow!("there's no `{}` here", key))?;
        let mut draft = self.clone();
        for line in &mut draft.lines[entry.start..entry.end] {
            *line = replace_word(line, old, new);
        }
        if draft.frontmatter_value()? != *expected {
            bail!("rewriting `{}` in `{}` didn't read back as expected, so it was left alone", old, key);
        }
        draft.verify_entity()?;
        *self = draft;
        Ok(())
    }

    /// The same file, to be saved at `path` instead (a rename's move).
    pub(crate) fn at(mut self, path: PathBuf) -> Self {
        self.path = path;
        self
    }

    /// `text` as a layer file at `path` (see `load_layer`).
    pub fn layer_from_text(path: impl AsRef<Path>, text: &str) -> Result<Self> {
        let path = path.as_ref();
        MarkdownParser::new()
            .split_document(text)
            .with_context(|| format!("{} doesn't parse", path.display()))?;
        Self::split(path.to_path_buf(), text, String::new(), String::new())
    }

    /// Whether this file is a whole thing, with its own `id` and `type`.
    pub fn is_thing(&self) -> bool {
        !self.type_name.is_empty()
    }

    /// Give the thing a new `id` (renaming, `WorldWriter::rename_loaded`).
    /// Checked like any edit: the frontmatter must read back with only
    /// `id` changed.
    pub(crate) fn set_id(&mut self, new_id: &str) -> Result<()> {
        let before = self.frontmatter_value()?;
        let mut draft = self.clone();
        draft.set_scalar("id", &Value::String(new_id.to_string()))?;
        let mut expected = before;
        expected.insert(Value::from("id"), Value::String(new_id.to_string()));
        if draft.frontmatter_value()? != expected {
            bail!("the new `id` didn't read back as written");
        }
        draft.id = new_id.to_string();
        draft.verify_entity()?;
        *self = draft;
        Ok(())
    }

    fn split(path: PathBuf, text: &str, type_name: String, id: String) -> Result<Self> {

        let mut parts = text.split_inclusive('\n');
        let opening = parts.next().unwrap_or_default().to_string();
        let eol = if opening.ends_with("\r\n") { "\r\n" } else { "\n" };
        let mut lines = Vec::new();
        let mut closing = None;
        for part in parts.by_ref() {
            if part.trim() == "---" {
                closing = Some(part.to_string());
                break;
            }
            lines.push(part.to_string());
        }
        let closing = closing.ok_or_else(|| anyhow!("frontmatter is never closed"))?;
        let body: String = parts.collect();
        Ok(Self {
            path,
            opening,
            lines,
            closing,
            body,
            eol,
            type_name,
            id,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// The file's full text as it would be written.
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.opening.len() + self.body.len() + 64 * self.lines.len());
        out.push_str(&self.opening);
        for line in &self.lines {
            out.push_str(line);
        }
        out.push_str(&self.closing);
        out.push_str(&self.body);
        out
    }

    /// Apply edits in order. All or nothing: if any is refused, the file
    /// is left as it was before the first.
    pub fn apply_all(&mut self, edits: &[Edit]) -> Result<()> {
        let mut draft = self.clone();
        for edit in edits {
            draft.apply(edit)?;
        }
        *self = draft;
        Ok(())
    }

    /// Apply one edit, checked by re-parsing. Refused edits change nothing.
    pub fn apply(&mut self, edit: &Edit) -> Result<()> {
        let before = self.frontmatter_value()?;
        let mut draft = self.clone();
        match edit {
            Edit::ReplaceAll(text) => {
                let replaced = ContentFile::from_text(&self.path, text)?;
                if replaced.type_name != self.type_name {
                    bail!(
                        "full editing can't change `type` (from `{}` to `{}`)",
                        self.type_name,
                        replaced.type_name
                    );
                }
                if replaced.id != self.id {
                    bail!(
                        "changing `id` (from `{}` to `{}`) isn't supported yet",
                        self.id,
                        replaced.id
                    );
                }
                *self = replaced;
                return Ok(());
            }
            Edit::SetBody(text) => {
                draft.body = draft.render_body(text);
                draft.verify_entity()?;
                let parsed = draft.parsed_body()?;
                if parsed != text.trim() {
                    bail!("the new body didn't read back as written");
                }
                *self = draft;
                return Ok(());
            }
            Edit::SetScalar { key, value } => {
                refuse_identity_key(key)?;
                draft.set_scalar(key, value)?;
            }
            Edit::SetList { key, items } => {
                refuse_identity_key(key)?;
                draft.set_list(key, items)?;
            }
            Edit::SetMapEntry { key, subkey, value } => {
                refuse_identity_key(key)?;
                draft.set_map_entry(key, subkey, value)?;
            }
            Edit::RemoveMapEntry { key, subkey } => {
                refuse_identity_key(key)?;
                draft.remove_map_entry(key, subkey)?;
            }
            Edit::RemoveKey { key } => {
                refuse_identity_key(key)?;
                draft.remove_key(key)?;
            }
        }

        // The safety net: the frontmatter must read back as exactly the
        // old mapping with this one change, and the file must still parse.
        check_reads_back(&before, edit, &draft.frontmatter_value()?)?;
        draft.verify_entity()?;
        *self = draft;
        Ok(())
    }

    /// Write the file in place, atomically, keeping its permissions: the
    /// text goes to a temporary file beside it, which is renamed over it.
    pub fn save(&self) -> Result<()> {
        let perms = fs::metadata(&self.path)
            .with_context(|| format!("can't stat {}", self.path.display()))?
            .permissions();
        let tmp = self.write_temp()?;
        let result = fs::set_permissions(&tmp, perms).and_then(|_| fs::rename(&tmp, &self.path));
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result.with_context(|| format!("can't replace {}", self.path.display()))
    }

    /// Write the file at its path, which must not exist yet. Never
    /// overwrites, even if a file appears there meanwhile: the temporary
    /// file is hard-linked into place, which fails if the name is taken.
    pub fn save_new(&self) -> Result<()> {
        let tmp = self.write_temp()?;
        let result = fs::hard_link(&tmp, &self.path);
        let _ = fs::remove_file(&tmp);
        result.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow!("{} already exists", self.path.display())
            } else {
                anyhow!("can't write {}: {}", self.path.display(), e)
            }
        })
    }

    fn write_temp(&self) -> Result<PathBuf> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| anyhow!("{} has no parent directory", self.path.display()))?;
        let name = self.path.file_name().and_then(|n| n.to_str()).unwrap_or("entity");
        for attempt in 0..100u32 {
            let tmp = dir.join(format!(".{}.thwart-{}-{}.tmp", name, std::process::id(), attempt));
            let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
                Ok(f) => f,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e).with_context(|| format!("can't create a temporary file in {}", dir.display())),
            };
            let written = file.write_all(self.text().as_bytes()).and_then(|_| file.sync_all());
            if let Err(e) = written {
                let _ = fs::remove_file(&tmp);
                return Err(e).with_context(|| format!("can't write {}", tmp.display()));
            }
            return Ok(tmp);
        }
        bail!("can't find a free temporary file name in {}", dir.display())
    }

    // ---- checks ----

    fn frontmatter_value(&self) -> Result<Mapping> {
        let fm: String = self.lines.concat();
        if fm.trim().is_empty() {
            return Ok(Mapping::new());
        }
        match serde_yaml::from_str::<Value>(&fm).context("frontmatter doesn't parse")? {
            Value::Mapping(m) => Ok(m),
            Value::Null => Ok(Mapping::new()),
            _ => bail!("frontmatter isn't a mapping"),
        }
    }

    fn verify_entity(&self) -> Result<()> {
        if !self.is_thing() {
            self.frontmatter_value().context("the edited file wouldn't parse, so the edit was not made")?;
            return Ok(());
        }
        let (type_name, id) =
            identity(&self.text(), &self.path).context("the edited file wouldn't parse, so the edit was not made")?;
        if type_name != self.type_name || id != self.id {
            bail!("the edit would change the entity's `type` or `id`, which isn't supported");
        }
        Ok(())
    }

    /// The body as the parser reads it back, response blocks included.
    fn parsed_body(&self) -> Result<String> {
        Ok(MarkdownParser::new().split_document(&self.text())?.body)
    }

    fn render_body(&self, text: &str) -> String {
        let text = text.trim();
        if text.is_empty() {
            return String::new();
        }
        let normalized: Vec<&str> = text.lines().collect();
        format!("{eol}{}{eol}", normalized.join(self.eol), eol = self.eol)
    }

    // ---- locating keys ----

    fn entries(&self) -> Vec<Entry> {
        let mut entries = Vec::new();
        let mut i = 0;
        while i < self.lines.len() {
            if let Some(key) = top_key(strip_eol(&self.lines[i])) {
                let mut last = i;
                let mut j = i + 1;
                while j < self.lines.len() {
                    let line = strip_eol(&self.lines[j]);
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        j += 1;
                        continue;
                    }
                    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
                        last = j;
                        j += 1;
                        continue;
                    }
                    break;
                }
                entries.push(Entry {
                    key: key.to_string(),
                    start: i,
                    end: last + 1,
                });
                i = last + 1;
            } else {
                i += 1;
            }
        }
        entries
    }

    fn find(&self, key: &str) -> Result<Option<Entry>> {
        let matches: Vec<Entry> = self.entries().into_iter().filter(|e| e.key == key).collect();
        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.into_iter().next()),
            _ => bail!("`{}` appears more than once in the frontmatter", key),
        }
    }

    /// The key line split into `key:` prefix, inline value, and trailing
    /// comment (with its leading whitespace).
    fn key_line(&self, entry: &Entry) -> (String, String, String) {
        let line = strip_eol(&self.lines[entry.start]);
        let colon = entry.key.len();
        let prefix = line[..=colon].to_string();
        let rest = &line[colon + 1..];
        let (value, comment) = split_comment(rest);
        (prefix, value.trim().to_string(), comment.to_string())
    }

    fn shape(&self, entry: &Entry) -> Shape {
        let (_, value, _) = self.key_line(entry);
        let has_lines = entry.end > entry.start + 1;
        if !has_lines {
            if value.is_empty() {
                return Shape::Null;
            }
            if value.starts_with('[') {
                return if value.ends_with(']') {
                    Shape::FlowSeq
                } else {
                    Shape::Other
                };
            }
            if value.starts_with('{') {
                return if value == "{}" {
                    Shape::EmptyFlowMap
                } else {
                    Shape::Other
                };
            }
            if value.starts_with(['|', '>', '&', '*', '!']) {
                return Shape::Other;
            }
            return Shape::Scalar;
        }
        if !value.is_empty() {
            return Shape::Other;
        }
        let value_lines: Vec<&str> = self.lines[entry.start + 1..entry.end]
            .iter()
            .map(|l| strip_eol(l))
            .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
            .collect();
        let first = value_lines[0];
        let indent = first.len() - first.trim_start().len();
        if first.trim_start().starts_with("- ") || first.trim_start() == "-" {
            Shape::BlockSeq { indent }
        } else {
            Shape::BlockMap { indent }
        }
    }

    fn has_inner_comments(&self, entry: &Entry) -> bool {
        self.lines[entry.start + 1..entry.end].iter().any(|l| {
            let t = l.trim();
            t.is_empty() || t.starts_with('#')
        })
    }

    // ---- edits ----

    fn set_scalar(&mut self, key: &str, value: &Value) -> Result<()> {
        let text = emit_scalar(value, false)?;
        match self.find(key)? {
            Some(entry) => match self.shape(&entry) {
                Shape::Scalar | Shape::Null => {
                    let (prefix, _, comment) = self.key_line(&entry);
                    self.lines[entry.start] = format!("{} {}{}{}", prefix, text, comment, self.eol);
                    Ok(())
                }
                _ => bail!(
                    "`{}` isn't a single-line value, so it can't be set as one; use full editing",
                    key
                ),
            },
            None => {
                let line = format!("{}: {}{}", key, text, self.eol);
                self.insert_new_key(key, vec![line]);
                Ok(())
            }
        }
    }

    fn set_list(&mut self, key: &str, items: &[String]) -> Result<()> {
        let rendered: Vec<String> = items
            .iter()
            .map(|i| emit_scalar(&Value::String(i.clone()), true))
            .collect::<Result<_>>()?;
        let flow = format!("[{}]", rendered.join(", "));
        match self.find(key)? {
            Some(entry) => match self.shape(&entry) {
                Shape::FlowSeq | Shape::Null => {
                    let (prefix, _, comment) = self.key_line(&entry);
                    self.lines[entry.start] = format!("{} {}{}{}", prefix, flow, comment, self.eol);
                    Ok(())
                }
                Shape::BlockSeq { indent } => {
                    if self.has_inner_comments(&entry) {
                        bail!(
                            "`{}` has comments or blank lines inside its list; use full editing",
                            key
                        );
                    }
                    if self.lines[entry.start + 1..entry.end]
                        .iter()
                        .any(|l| !strip_eol(l)[indent.min(strip_eol(l).len())..].starts_with('-'))
                    {
                        bail!("`{}` has nested list items; use full editing", key);
                    }
                    let (prefix, _, comment) = self.key_line(&entry);
                    let mut new_lines = Vec::new();
                    if items.is_empty() {
                        new_lines.push(format!("{} []{}{}", prefix, comment, self.eol));
                    } else {
                        new_lines.push(format!("{}{}{}", prefix, comment, self.eol));
                        for item in items {
                            let text = emit_scalar(&Value::String(item.clone()), false)?;
                            new_lines.push(format!("{}- {}{}", " ".repeat(indent), text, self.eol));
                        }
                    }
                    self.lines.splice(entry.start..entry.end, new_lines);
                    Ok(())
                }
                _ => bail!("`{}` isn't a simple list; use full editing", key),
            },
            None => {
                let block_indent = self.entries().iter().find_map(|e| match self.shape(e) {
                    Shape::BlockSeq { indent } => Some(indent),
                    _ => None,
                });
                let new_lines = match block_indent {
                    Some(indent) if !items.is_empty() => {
                        let mut lines = vec![format!("{}:{}", key, self.eol)];
                        for item in items {
                            let text = emit_scalar(&Value::String(item.clone()), false)?;
                            lines.push(format!("{}- {}{}", " ".repeat(indent), text, self.eol));
                        }
                        lines
                    }
                    _ => vec![format!("{}: {}{}", key, flow, self.eol)],
                };
                self.insert_new_key(key, new_lines);
                Ok(())
            }
        }
    }

    fn set_map_entry(&mut self, key: &str, subkey: &str, value: &Value) -> Result<()> {
        let sub = emit_key(subkey)?;
        let text = emit_scalar(value, false)?;
        match self.find(key)? {
            Some(entry) => match self.shape(&entry) {
                Shape::BlockMap { indent } => {
                    match self.find_sub(&entry, indent, subkey)? {
                        Some(idx) => {
                            let line = strip_eol(&self.lines[idx]).to_string();
                            let colon = line.find(':').expect("sub line has a colon");
                            let (_, comment) = split_comment(&line[colon + 1..]);
                            self.lines[idx] = format!("{} {}{}{}", &line[..=colon], text, comment, self.eol);
                        }
                        None => {
                            let line = format!("{}{}: {}{}", " ".repeat(indent), sub, text, self.eol);
                            self.lines.insert(entry.end, line);
                        }
                    }
                    Ok(())
                }
                Shape::Null | Shape::EmptyFlowMap => {
                    let (prefix, _, comment) = self.key_line(&entry);
                    let indent = self.map_indent();
                    self.lines[entry.start] = format!("{}{}{}", prefix, comment, self.eol);
                    self.lines.insert(
                        entry.start + 1,
                        format!("{}{}: {}{}", " ".repeat(indent), sub, text, self.eol),
                    );
                    Ok(())
                }
                _ => bail!("`{}` isn't a simple map; use full editing", key),
            },
            None => {
                let indent = self.map_indent();
                let lines = vec![
                    format!("{}:{}", key, self.eol),
                    format!("{}{}: {}{}", " ".repeat(indent), sub, text, self.eol),
                ];
                self.insert_new_key(key, lines);
                Ok(())
            }
        }
    }

    fn remove_map_entry(&mut self, key: &str, subkey: &str) -> Result<()> {
        let entry = self
            .find(key)?
            .ok_or_else(|| anyhow!("there's no `{}` to remove `{}` from", key, subkey))?;
        let Shape::BlockMap { indent } = self.shape(&entry) else {
            bail!("`{}` has no entry `{}`", key, subkey);
        };
        let idx = self
            .find_sub(&entry, indent, subkey)?
            .ok_or_else(|| anyhow!("`{}` has no entry `{}`", key, subkey))?;
        self.lines.remove(idx);
        let remaining = self.find(key)?.expect("key still present");
        if remaining.end == remaining.start + 1 {
            let (prefix, _, comment) = self.key_line(&remaining);
            self.lines[remaining.start] = format!("{} {{}}{}{}", prefix, comment, self.eol);
        }
        Ok(())
    }

    fn remove_key(&mut self, key: &str) -> Result<()> {
        if let Some(entry) = self.find(key)? {
            self.lines.drain(entry.start..entry.end);
        }
        Ok(())
    }

    /// The line index of `subkey` inside a block map at `indent`. Refused
    /// if that entry spans more than one line.
    fn find_sub(&self, entry: &Entry, indent: usize, subkey: &str) -> Result<Option<usize>> {
        let mut found = None;
        for idx in entry.start + 1..entry.end {
            let line = strip_eol(&self.lines[idx]);
            let line_indent = line.len() - line.trim_start().len();
            if line.trim().is_empty() || line.trim_start().starts_with('#') || line_indent != indent {
                continue;
            }
            let Some(colon) = line.find(':') else { continue };
            if unquote(line[indent..colon].trim()) == subkey {
                found = Some(idx);
                break;
            }
        }
        let Some(idx) = found else { return Ok(None) };
        let next = self.lines.get(idx + 1).map(|l| strip_eol(l));
        let continues = next.map_or(false, |l| {
            idx + 1 < entry.end && !l.trim().is_empty() && (l.len() - l.trim_start().len()) > indent
        });
        let (value, _) = split_comment(&strip_eol(&self.lines[idx])[..]);
        let after_colon = value.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");
        if continues || after_colon.is_empty() || after_colon.starts_with(['[', '{', '|', '>', '&', '*', '!']) {
            bail!("`{}` isn't a single-line entry; use full editing", subkey);
        }
        Ok(Some(idx))
    }

    fn map_indent(&self) -> usize {
        self.entries()
            .iter()
            .find_map(|e| match self.shape(e) {
                Shape::BlockMap { indent } => Some(indent),
                _ => None,
            })
            .unwrap_or(2)
    }

    /// Insert a new key's lines where `design/CONTENT_FORMAT.md` puts it:
    /// after the last present key that comes before it in the type's
    /// table order, or at the end for a key the type doesn't define.
    fn insert_new_key(&mut self, key: &str, new_lines: Vec<String>) {
        let order = known_keys(&self.type_name);
        let rank = |k: &str| order.iter().position(|o| *o == k);
        let entries = self.entries();
        let at = match rank(key) {
            None => entries.last().map_or(self.lines.len(), |e| e.end),
            Some(r) => match entries
                .iter()
                .filter(|e| rank(&e.key).map_or(false, |er| er < r))
                .last()
            {
                Some(prev) => prev.end,
                None => entries.first().map_or(self.lines.len(), |e| e.start),
            },
        };
        self.lines.splice(at..at, new_lines);
    }
}

#[derive(Debug, Clone)]
struct Entry {
    key: String,
    /// The key's line.
    start: usize,
    /// One past its last value line (trailing blank and comment lines excluded).
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Null,
    Scalar,
    FlowSeq,
    EmptyFlowMap,
    BlockSeq { indent: usize },
    BlockMap { indent: usize },
    Other,
}

/// A file's `type` and `id`, checked as far as the file alone allows. A
/// file that names a template (`inherits:`) may take required keys such
/// as `object_type` from it, so only its frontmatter, `id` and `type` are
/// checked here; the engine types it with its template and reports any
/// problem. Any other file must parse as a whole entity.
fn identity(text: &str, path: &Path) -> Result<(String, String)> {
    let parser = MarkdownParser::new();
    let doc = parser.split_document(text)?;
    if doc.metadata.get("inherits").is_some() {
        let get = |key: &str| doc.metadata.get(key).and_then(Value::as_str).map(str::to_string);
        let type_name = get("type").ok_or_else(|| anyhow!("missing required frontmatter field `type`"))?;
        if !["location", "character", "object", "door"].contains(&type_name.as_str()) {
            bail!("unknown type `{}` (expected location, character, or object)", type_name);
        }
        let id = get("id").ok_or_else(|| anyhow!("missing required frontmatter field `id`"))?;
        return Ok((type_name, id));
    }
    Ok(entity_key(&parser.parse_content(text, path)?))
}

/// `text` with each whole-word `old` replaced: not preceded or followed
/// by a character an id can hold (letters, digits, `-`, `_`, `/`).
fn replace_word(text: &str, old: &str, new: &str) -> String {
    let is_id = |c: char| c.is_alphanumeric() || matches!(c, '-' | '_' | '/');
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(old) {
        let before = rest[..i].chars().next_back();
        let after = rest[i + old.len()..].chars().next();
        out.push_str(&rest[..i]);
        if before.map_or(true, |c| !is_id(c)) && after.map_or(true, |c| !is_id(c)) {
            out.push_str(new);
        } else {
            out.push_str(old);
        }
        rest = &rest[i + old.len()..];
    }
    out.push_str(rest);
    out
}

fn entity_key(entity: &ParsedEntity) -> (String, String) {
    match entity {
        ParsedEntity::Location(l) => ("location".into(), l.id.clone()),
        ParsedEntity::Character(c) => ("character".into(), c.id.clone()),
        ParsedEntity::Object(o) => ("object".into(), o.id.clone()),
        ParsedEntity::Door(d) => ("door".into(), d.id.clone()),
    }
}

fn refuse_identity_key(key: &str) -> Result<()> {
    match key {
        "id" => bail!("changing `id` isn't supported yet: renaming means moving the file and fixing references"),
        "type" => bail!("changing `type` isn't supported"),
        _ => Ok(()),
    }
}

/// The safety net: `after` must be exactly `before` with `edit` applied.
fn check_reads_back(before: &Mapping, edit: &Edit, after: &Mapping) -> Result<()> {
    if *after != expected_after(before, edit)? {
        bail!("the edit didn't read back as intended, so it was not made (this is a writer bug; please report it)");
    }
    Ok(())
}

/// The mapping `before` with `edit` applied, as the re-parse must find it.
fn expected_after(before: &Mapping, edit: &Edit) -> Result<Mapping> {
    let mut m = before.clone();
    match edit {
        Edit::SetScalar { key, value } => {
            m.insert(Value::from(key.as_str()), value.clone());
        }
        Edit::SetList { key, items } => {
            let seq = items.iter().map(|i| Value::String(i.clone())).collect();
            m.insert(Value::from(key.as_str()), Value::Sequence(seq));
        }
        Edit::SetMapEntry { key, subkey, value } => {
            let k = Value::from(key.as_str());
            let mut inner = match m.get(&k) {
                Some(Value::Mapping(inner)) => inner.clone(),
                None | Some(Value::Null) => Mapping::new(),
                Some(_) => bail!("`{}` isn't a map", key),
            };
            inner.insert(Value::from(subkey.as_str()), value.clone());
            m.insert(k, Value::Mapping(inner));
        }
        Edit::RemoveMapEntry { key, subkey } => {
            let k = Value::from(key.as_str());
            if let Some(Value::Mapping(inner)) = m.get_mut(&k) {
                inner.remove(subkey.as_str());
            }
        }
        Edit::RemoveKey { key } => {
            m.remove(key.as_str());
        }
        Edit::SetBody(_) | Edit::ReplaceAll(_) => {}
    }
    Ok(m)
}

fn strip_eol(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}

/// The key of a top-level `key: value` line, if this is one.
fn top_key(line: &str) -> Option<&str> {
    if line.is_empty() || line.starts_with([' ', '\t', '#', '-', '"', '\'']) {
        return None;
    }
    let colon = line.find(':')?;
    let after = &line[colon + 1..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    Some(&line[..colon])
}

/// Split a value from a trailing `# comment`, honouring quotes. The
/// comment keeps its leading whitespace.
fn split_comment(rest: &str) -> (&str, &str) {
    let bytes = rest.as_bytes();
    let (mut single, mut double) = (false, false);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if double => i += 1,
            b'\'' if !double => single = !single,
            b'"' if !single => double = !double,
            b'#' if !single && !double && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') => {
                let start = rest[..i].trim_end().len();
                return (&rest[..start], &rest[start..]);
            }
            _ => {}
        }
        i += 1;
    }
    (rest.trim_end(), "")
}

fn unquote(s: &str) -> String {
    if s.len() >= 2 && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\''))) {
        if let Ok(Value::String(v)) = serde_yaml::from_str::<Value>(s) {
            return v;
        }
    }
    s.to_string()
}

/// A scalar as YAML text: plain when that reads back as the same value,
/// double-quoted otherwise. `in_flow` is stricter, for items in `[a, b]`.
pub(crate) fn emit_scalar(value: &Value, in_flow: bool) -> Result<String> {
    match value {
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => Ok(if plain_ok(s, in_flow) {
            s.clone()
        } else {
            double_quote(s)
        }),
        Value::Null => Ok("null".into()),
        _ => bail!("only strings, numbers and booleans can be set as single values"),
    }
}

fn emit_key(key: &str) -> Result<String> {
    if key.is_empty() {
        bail!("a map key can't be empty");
    }
    Ok(if plain_ok(key, true) && !key.contains(':') {
        key.to_string()
    } else {
        double_quote(key)
    })
}

fn plain_ok(s: &str, in_flow: bool) -> bool {
    if s.is_empty()
        || s != s.trim()
        || s.contains(['\n', '\r', '\t'])
        || s.contains(" #")
        || s.contains(": ")
        || s.ends_with(':')
    {
        return false;
    }
    if s.starts_with([
        '[', ']', '{', '}', ',', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`', '#', '?', '-', ':',
    ]) {
        // A leading `-` is fine for e.g. `-3` only if it reads back as a
        // string, which it doesn't; quote all of these.
        return false;
    }
    if in_flow && s.contains([',', '[', ']', '{', '}']) {
        return false;
    }
    matches!(serde_yaml::from_str::<Value>(s), Ok(Value::String(ref v)) if v == s)
}

fn double_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(text: &str) -> ContentFile {
        ContentFile::from_text("test.md", text).unwrap()
    }

    fn s(v: &str) -> Value {
        Value::String(v.into())
    }

    const ROOM: &str = "\
---
id: castle/yard   # the main yard
title: Yard
type: location
# exits below
connections:
  north: castle/hall  # big doors
  east: castle/garden
objects:
- well
- bench
extra_note: keep me
---

Grass and stone.
";

    #[test]
    fn every_content_file_round_trips_byte_for_byte() {
        let mut checked = 0;
        for root in ["authors", "tests/worlds", "tests/transcripts/worlds"] {
            for entry in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let text = fs::read_to_string(path).unwrap();
                let Ok(f) = ContentFile::from_text(path, &text) else {
                    continue;
                };
                assert_eq!(f.text(), text, "{} didn't round-trip", path.display());
                checked += 1;
            }
        }
        assert!(checked >= 20, "only {} files checked", checked);
    }

    #[test]
    fn set_scalar_replaces_only_the_value_and_keeps_its_comment() {
        let mut f = file(ROOM);
        f.apply(&Edit::SetScalar {
            key: "title".into(),
            value: s("Old Yard"),
        })
        .unwrap();
        assert_eq!(f.text(), ROOM.replace("title: Yard\n", "title: Old Yard\n"));
    }

    #[test]
    fn added_keys_follow_table_order_and_extras_go_last() {
        let mut f = file(ROOM);
        f.apply(&Edit::SetList {
            key: "characters".into(),
            items: vec!["king".into()],
        })
        .unwrap();
        f.apply(&Edit::SetScalar {
            key: "mood".into(),
            value: s("calm"),
        })
        .unwrap();
        let text = f.text();
        // characters comes before objects in the location table; the file
        // already uses block lists, so the new list is a block list too.
        assert!(
            text.contains("  east: castle/garden\ncharacters:\n- king\nobjects:\n"),
            "{}",
            text
        );
        assert!(text.contains("extra_note: keep me\nmood: calm\n---\n"), "{}", text);
    }

    #[test]
    fn character_key_added_between_known_neighbours() {
        let mut f = file("---\nid: k\ntitle: K\ntype: character\ncharacter_type: npc\ninventory: [a]\n---\nBody\n");
        f.apply(&Edit::SetScalar {
            key: "default_location".into(),
            value: s("castle/yard"),
        })
        .unwrap();
        assert!(f
            .text()
            .contains("character_type: npc\ndefault_location: castle/yard\ninventory: [a]\n"));
    }

    #[test]
    fn set_list_keeps_block_or_flow_style() {
        let mut f = file(ROOM);
        f.apply(&Edit::SetList {
            key: "objects".into(),
            items: vec!["well".into(), "rope, old".into()],
        })
        .unwrap();
        assert!(
            f.text().contains("objects:\n- well\n- rope, old\nextra_note"),
            "{}",
            f.text()
        );
        f.apply(&Edit::SetList {
            key: "objects".into(),
            items: vec![],
        })
        .unwrap();
        assert!(f.text().contains("objects: []\nextra_note"), "{}", f.text());

        let mut g = file("---\nid: a/b\ntitle: B\ntype: location\nobjects: [x, y]\n---\n");
        g.apply(&Edit::SetList {
            key: "objects".into(),
            items: vec!["x".into(), "rope, old".into()],
        })
        .unwrap();
        assert!(g.text().contains("objects: [x, \"rope, old\"]\n"), "{}", g.text());
    }

    #[test]
    fn map_entries_change_add_and_remove() {
        let mut f = file(ROOM);
        f.apply(&Edit::SetMapEntry {
            key: "connections".into(),
            subkey: "north".into(),
            value: s("castle/keep"),
        })
        .unwrap();
        assert!(f.text().contains("  north: castle/keep  # big doors\n"), "{}", f.text());
        f.apply(&Edit::SetMapEntry {
            key: "connections".into(),
            subkey: "through the hedge".into(),
            value: s("forest/path"),
        })
        .unwrap();
        assert!(
            f.text()
                .contains("  east: castle/garden\n  through the hedge: forest/path\nobjects:"),
            "{}",
            f.text()
        );
        f.apply(&Edit::RemoveMapEntry {
            key: "connections".into(),
            subkey: "east".into(),
        })
        .unwrap();
        assert!(!f.text().contains("east:"));
        assert!(f
            .apply(&Edit::RemoveMapEntry {
                key: "connections".into(),
                subkey: "west".into()
            })
            .is_err());
    }

    #[test]
    fn removing_the_last_map_entry_leaves_an_empty_map() {
        let mut f = file("---\nid: a/b\ntitle: B\ntype: location\nconnections:\n  up: a/c\n---\n");
        f.apply(&Edit::RemoveMapEntry {
            key: "connections".into(),
            subkey: "up".into(),
        })
        .unwrap();
        assert_eq!(
            f.text(),
            "---\nid: a/b\ntitle: B\ntype: location\nconnections: {}\n---\n"
        );
        // And adding to an empty map makes it a block map again.
        f.apply(&Edit::SetMapEntry {
            key: "connections".into(),
            subkey: "down".into(),
            value: s("a/d"),
        })
        .unwrap();
        assert_eq!(
            f.text(),
            "---\nid: a/b\ntitle: B\ntype: location\nconnections:\n  down: a/d\n---\n"
        );
    }

    #[test]
    fn numeric_map_values_stay_numbers() {
        let mut f = file("---\nid: k\ntitle: K\ntype: character\nattributes:\n  health: 80\n---\n");
        f.apply(&Edit::SetMapEntry {
            key: "attributes".into(),
            subkey: "health".into(),
            value: Value::from(75),
        })
        .unwrap();
        f.apply(&Edit::SetMapEntry {
            key: "attributes".into(),
            subkey: "wisdom".into(),
            value: Value::from(9),
        })
        .unwrap();
        assert!(
            f.text().contains("attributes:\n  health: 75\n  wisdom: 9\n"),
            "{}",
            f.text()
        );
    }

    #[test]
    fn remove_key_takes_its_value_lines_and_nothing_else() {
        let mut f = file(ROOM);
        f.apply(&Edit::RemoveKey {
            key: "connections".into(),
        })
        .unwrap();
        assert!(
            f.text().contains("type: location\n# exits below\nobjects:\n"),
            "{}",
            f.text()
        );
        f.apply(&Edit::RemoveKey {
            key: "not-there".into(),
        })
        .unwrap();
    }

    #[test]
    fn strings_that_need_quotes_get_them() {
        let mut f = file(ROOM);
        for title in [
            "Yard: north side",
            "Yard #2",
            "yes",
            "42",
            "- dash",
            "'quoted'",
            "tab\there",
            "",
        ] {
            f.apply(&Edit::SetScalar {
                key: "title".into(),
                value: s(title),
            })
            .unwrap_or_else(|e| {
                // An empty title still parses (it's a string), so only report real failures.
                panic!("{:?}: {}", title, e)
            });
        }
    }

    #[test]
    fn set_body_replaces_only_the_body() {
        let mut f = file(ROOM);
        f.apply(&Edit::SetBody("New prose.\n\nSecond paragraph.".into()))
            .unwrap();
        let (head, _) = ROOM.split_at(ROOM.rfind("---\n").unwrap() + 4);
        assert_eq!(f.text(), format!("{}\nNew prose.\n\nSecond paragraph.\n", head));
    }

    #[test]
    fn crlf_files_stay_crlf() {
        let text = ROOM.replace('\n', "\r\n");
        let mut f = file(&text);
        assert_eq!(f.text(), text);
        f.apply(&Edit::SetScalar {
            key: "mood".into(),
            value: s("calm"),
        })
        .unwrap();
        f.apply(&Edit::SetBody("A\nB".into())).unwrap();
        assert!(
            !f.text().replace("\r\n", "").contains('\n'),
            "a bare LF slipped in: {:?}",
            f.text()
        );
    }

    #[test]
    fn id_and_type_changes_are_refused() {
        let mut f = file(ROOM);
        for edit in [
            Edit::SetScalar {
                key: "id".into(),
                value: s("castle/other"),
            },
            Edit::RemoveKey { key: "id".into() },
            Edit::SetScalar {
                key: "type".into(),
                value: s("object"),
            },
            Edit::ReplaceAll(ROOM.replace("id: castle/yard", "id: castle/other")),
            Edit::ReplaceAll("---\nid: castle/yard\ntitle: Y\ntype: object\nobject_type: x\n---\n".into()),
        ] {
            assert!(f.apply(&edit).is_err(), "{:?} wasn't refused", edit);
        }
        assert_eq!(f.text(), ROOM);
    }

    #[test]
    fn edits_that_break_the_entity_are_refused() {
        let mut f = file(ROOM);
        let e = f.apply(&Edit::RemoveKey { key: "title".into() }).unwrap_err();
        assert!(format!("{:#}", e).contains("wouldn't parse"), "{:#}", e);
        assert!(f.apply(&Edit::ReplaceAll("no frontmatter".into())).is_err());
        assert_eq!(f.text(), ROOM);
    }

    #[test]
    fn shapes_it_does_not_understand_are_refused() {
        let text = "---\nid: s\ntitle: S\ntype: object\nobject_type: x\nnotes: |\n  line one\n  line two\ntags: {a: 1}\nlist:\n  - a\n  - nested:\n      deep: 1\n---\n";
        let mut f = file(text);
        assert!(f
            .apply(&Edit::SetScalar {
                key: "notes".into(),
                value: s("x")
            })
            .is_err());
        assert!(f
            .apply(&Edit::SetMapEntry {
                key: "tags".into(),
                subkey: "b".into(),
                value: Value::from(2)
            })
            .is_err());
        assert!(f
            .apply(&Edit::SetList {
                key: "list".into(),
                items: vec!["a".into()]
            })
            .is_err());
        assert!(f
            .apply(&Edit::SetMapEntry {
                key: "title".into(),
                subkey: "b".into(),
                value: Value::from(2)
            })
            .is_err());
        // Keys around them can still be edited, and they stay as they were.
        f.apply(&Edit::SetScalar {
            key: "title".into(),
            value: s("Scroll"),
        })
        .unwrap();
        assert_eq!(f.text(), text.replace("title: S\n", "title: Scroll\n"));
    }

    #[test]
    fn apply_all_is_all_or_nothing() {
        let mut f = file(ROOM);
        let result = f.apply_all(&[
            Edit::SetScalar {
                key: "title".into(),
                value: s("Changed"),
            },
            Edit::SetScalar {
                key: "id".into(),
                value: s("nope"),
            },
        ]);
        assert!(result.is_err());
        assert_eq!(f.text(), ROOM);
    }

    #[test]
    fn replace_all_takes_new_text_that_parses() {
        let mut f = file(ROOM);
        let new = "---\nid: castle/yard\ntitle: Yard\ntype: location\n---\nRewritten.\n";
        f.apply(&Edit::ReplaceAll(new.into())).unwrap();
        assert_eq!(f.text(), new);
    }

    #[test]
    fn the_safety_net_refuses_any_other_change() {
        let before: Mapping = serde_yaml::from_str("id: a\ntitle: A\nweight: light\n").unwrap();
        let edit = Edit::SetScalar {
            key: "title".into(),
            value: s("B"),
        };
        let right: Mapping = serde_yaml::from_str("id: a\ntitle: B\nweight: light\n").unwrap();
        assert!(check_reads_back(&before, &edit, &right).is_ok());
        // The intended change, plus a stray one elsewhere.
        let stray: Mapping = serde_yaml::from_str("id: a\ntitle: B\nweight: heavy\n").unwrap();
        assert!(check_reads_back(&before, &edit, &stray).is_err());
        // The intended change written with the wrong type.
        let wrong: Mapping = serde_yaml::from_str("id: a\ntitle: 7\nweight: light\n").unwrap();
        let edit7 = Edit::SetScalar {
            key: "title".into(),
            value: s("7"),
        };
        assert!(check_reads_back(&before, &edit7, &wrong).is_err());
        // The change not made at all.
        assert!(check_reads_back(&before, &edit, &before).is_err());
    }

    #[test]
    fn mapping_comparison_ignores_order() {
        // The safety net compares mappings; make sure a reordered but equal
        // mapping counts as equal, so the check depends on content only.
        let a: Value = serde_yaml::from_str("a: 1\nb: 2\n").unwrap();
        let b: Value = serde_yaml::from_str("b: 2\na: 1\n").unwrap();
        assert_eq!(a, b);
    }
}
