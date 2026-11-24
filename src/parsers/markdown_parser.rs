use crate::models::character::Character;
use crate::models::world::{Coordinate, Door, DoorSide, GameObject, Location, ObjectAction};
use anyhow::{anyhow, Result};
use serde_yaml;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MarkdownParser;

/// The keys `design/CONTENT_FORMAT.md` defines for a `type`, in its table
/// order: `id`, `title`, `type`, the layer keys `inherits` and `state`
/// (`design/STATES.md`), then the type's own fields, then `vars` and `can`. This is the
/// order a writer adds keys in. Empty for an unknown type.
pub fn known_keys(type_name: &str) -> &'static [&'static str] {
    match type_name {
        "location" => &[
            "id",
            "title",
            "type",
            "inherits",
            "state",
            "visibility",
            "coordinates",
            "connections",
            "characters",
            "objects",
            "vars",
            "can",
        ],
        "character" => &[
            "id",
            "title",
            "type",
            "inherits",
            "state",
            "visibility",
            "character_type",
            "default_location",
            "attributes",
            "dialogue_tree",
            "inventory",
            "relationships",
            "vars",
            "can",
        ],
        "door" => &[
            "id",
            "title",
            "type",
            "inherits",
            "state",
            "visibility",
            "sides",
            "portal",
            "passable",
            "transparent",
            "vars",
            "can",
            "actions",
        ],
        "object" => &[
            "id",
            "title",
            "type",
            "inherits",
            "state",
            "visibility",
            "object_type",
            "category",
            "weight",
            "value",
            "vars",
            "can",
            "actions",
        ],
        _ => &[],
    }
}

/// Warnings for top-level keys that differ only in case from a key the
/// type defines (`Inventory:` for `inventory:`). Keys are case-sensitive,
/// so such a key isn't read as the known one. Other unknown keys aren't
/// warned about, since objects may carry any extra keys (REQ-CF-6).
pub fn key_case_warnings(metadata: &serde_yaml::Value, type_name: &str) -> Vec<String> {
    let known = known_keys(type_name);
    let mut warnings = Vec::new();
    if let Some(map) = metadata.as_mapping() {
        for key in map.keys().filter_map(|k| k.as_str()) {
            if known.contains(&key) {
                continue;
            }
            if let Some(k) = known.iter().find(|k| k.eq_ignore_ascii_case(key)) {
                warnings.push(format!(
                    "key `{}` differs only in case from `{}`; keys are case-sensitive, so it isn't read as `{}`",
                    key, k, k
                ));
            }
        }
    }
    warnings
}

/// A parsed file plus any warnings about it that aren't errors.
#[derive(Debug, Clone)]
pub struct ParseOutcome {
    pub entity: ParsedEntity,
    pub warnings: Vec<String>,
}

/// A markdown file split into its frontmatter and body, before any
/// type-specific interpretation.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub metadata: serde_yaml::Value,
    pub body: String,
}

/// The result of parsing a content file, keyed by its declared `type`.
/// See `design/CONTENT_FORMAT.md` for the field-level spec.
#[derive(Debug, Clone)]
pub enum ParsedEntity {
    Location(Location),
    Character(Character),
    Object(GameObject),
    Door(Door),
}

impl ParsedEntity {
    /// The `type` it was read as.
    pub fn type_name(&self) -> &'static str {
        match self {
            ParsedEntity::Location(_) => "location",
            ParsedEntity::Character(_) => "character",
            ParsedEntity::Object(_) => "object",
            ParsedEntity::Door(_) => "door",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            ParsedEntity::Location(l) => &l.id,
            ParsedEntity::Character(c) => &c.id,
            ParsedEntity::Object(o) => &o.id,
            ParsedEntity::Door(d) => &d.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            ParsedEntity::Location(l) => &l.name,
            ParsedEntity::Character(c) => &c.name,
            ParsedEntity::Object(o) => &o.title,
            ParsedEntity::Door(d) => &d.title,
        }
    }
}

impl MarkdownParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse any content file and dispatch on its frontmatter `type`
    /// field. This is the entry point `WorldEngine::load_world` uses when
    /// walking a world directory; it does not need to know in advance
    /// whether a given file is a location, character, or object.
    pub fn parse_file<P: AsRef<Path>>(&self, file_path: P) -> Result<ParsedEntity> {
        Ok(self.parse_file_with_warnings(file_path)?.entity)
    }

    /// `parse_file`, plus warnings such as keys that differ only in case.
    pub fn parse_file_with_warnings<P: AsRef<Path>>(&self, file_path: P) -> Result<ParseOutcome> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        self.parse_content_with_warnings(&content, file_path)
    }

    /// Parse a content file's text, dispatching on its frontmatter `type`.
    /// `file_path` is used only in error messages. `parse_file` is this
    /// plus the read; tests and the console can call it on text directly.
    pub fn parse_content<P: AsRef<Path>>(&self, content: &str, file_path: P) -> Result<ParsedEntity> {
        Ok(self.parse_content_with_warnings(content, file_path)?.entity)
    }

    /// `parse_content`, plus warnings such as keys that differ only in case.
    pub fn parse_content_with_warnings<P: AsRef<Path>>(&self, content: &str, file_path: P) -> Result<ParseOutcome> {
        let parsed = self.split_document(content)?;
        self.build_entity(&parsed, file_path)
    }

    /// Type an already split document (one file's, or several layers merged)
    /// as the entity its `type` names. `file_path` is used only in errors.
    pub fn build_entity<P: AsRef<Path>>(&self, parsed: &ParsedDocument, file_path: P) -> Result<ParseOutcome> {
        let entity_type = parsed
            .metadata
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow!(
                    "{:?}: missing required frontmatter field `type` (location, character, or object)",
                    file_path.as_ref()
                )
            })?;

        let entity = match entity_type {
            "location" => ParsedEntity::Location(self.build_location(parsed, &file_path)?),
            "character" => ParsedEntity::Character(self.build_character(parsed, &file_path)?),
            "object" => ParsedEntity::Object(self.build_object(parsed, &file_path)?),
            "door" => ParsedEntity::Door(self.build_door(parsed, &file_path)?),
            other => return Err(anyhow!(
                "{:?}: unknown type `{}` (expected location, character, object or door)",
                file_path.as_ref(),
                other
            )),
        };
        let warnings = key_case_warnings(&parsed.metadata, entity_type);
        Ok(ParseOutcome { entity, warnings })
    }

    pub fn parse_location_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Location> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        let parsed = self.split_document(&content)?;
        self.build_location(&parsed, &file_path)
    }

    pub fn parse_character_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Character> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        let parsed = self.split_document(&content)?;
        self.build_character(&parsed, &file_path)
    }

    pub fn parse_object_file<P: AsRef<Path>>(&self, file_path: P) -> Result<GameObject> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        let parsed = self.split_document(&content)?;
        self.build_object(&parsed, &file_path)
    }

    fn required_str(
        &self,
        metadata: &serde_yaml::Value,
        field: &str,
        path: &Path,
    ) -> Result<String> {
        metadata
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("{:?}: missing required frontmatter field `{}`", path, field))
    }

    fn build_location<P: AsRef<Path>>(
        &self,
        parsed: &ParsedDocument,
        file_path: P,
    ) -> Result<Location> {
        let path = file_path.as_ref();
        let id = self.required_str(&parsed.metadata, "id", path)?;
        let title = self.required_str(&parsed.metadata, "title", path)?;

        let mut location = Location::new(id, title, parsed.body.clone());

        if let Some(coords) = parsed.metadata.get("coordinates").and_then(|v| v.as_sequence()) {
            if coords.len() >= 3 {
                let x = coords[0].as_i64().unwrap_or(0) as i32;
                let y = coords[1].as_i64().unwrap_or(0) as i32;
                let z = coords[2].as_i64().unwrap_or(0) as i32;
                location.coordinates = Some(Coordinate::new(x, y, z));
            }
        }

        if let Some(conn_map) = parsed.metadata.get("connections").and_then(|v| v.as_mapping()) {
            for (direction, target) in conn_map {
                if let (Some(dir), Some(tgt)) = (direction.as_str(), id_value(target)) {
                    location.connections.insert(dir.to_string(), tgt);
                }
            }
        }

        location.characters = string_sequence(&parsed.metadata, "characters");
        location.objects = string_sequence(&parsed.metadata, "objects");

        Ok(location)
    }

    fn build_character<P: AsRef<Path>>(
        &self,
        parsed: &ParsedDocument,
        file_path: P,
    ) -> Result<Character> {
        let path = file_path.as_ref();
        let id = self.required_str(&parsed.metadata, "id", path)?;
        let title = self.required_str(&parsed.metadata, "title", path)?;

        let mut character = Character::new(id, title, parsed.body.clone());

        if let Some(char_type) = parsed.metadata.get("character_type").and_then(|v| v.as_str()) {
            character.character_type = match char_type.to_lowercase().as_str() {
                "player" => crate::models::character::CharacterType::Player,
                "villain" => crate::models::character::CharacterType::Villain,
                "hero" => crate::models::character::CharacterType::Hero,
                _ => crate::models::character::CharacterType::Npc,
            };
        }

        if let Some(location) = parsed.metadata.get("default_location").and_then(id_value) {
            character.default_location = Some(location);
        }

        if let Some(attr_map) = parsed.metadata.get("attributes").and_then(|v| v.as_mapping()) {
            for (key, value) in attr_map {
                if let (Some(k), Some(v)) = (key.as_str(), value.as_i64()) {
                    character.attributes.insert(k.to_string(), v as i32);
                }
            }
        }

        if let Some(dialogue) = parsed.metadata.get("dialogue_tree").and_then(|v| v.as_str()) {
            character.dialogue_tree = Some(dialogue.to_string());
        }

        character.inventory = string_sequence(&parsed.metadata, "inventory");

        if let Some(rel_seq) = parsed.metadata.get("relationships").and_then(|v| v.as_sequence()) {
            // Author-friendly shape: a list of single-key maps, e.g.
            //   relationships:
            //     - king: loyal
            //     - guard-captain: respectful
            for entry in rel_seq {
                if let Some(map) = entry.as_mapping() {
                    for (k, v) in map {
                        if let (Some(k), Some(v)) = (k.as_str(), v.as_str()) {
                            character.relationships.insert(k.to_string(), v.to_string());
                        }
                    }
                }
            }
        }

        Ok(character)
    }

    fn build_door<P: AsRef<Path>>(&self, parsed: &ParsedDocument, file_path: P) -> Result<Door> {
        use serde_yaml::Value;
        let path = file_path.as_ref();
        let id = self.required_str(&parsed.metadata, "id", path)?;
        let title = self.required_str(&parsed.metadata, "title", path)?;
        let flag = |key: &str, default: bool| -> Result<bool> {
            match parsed.metadata.get(key) {
                None | Some(Value::Null) => Ok(default),
                Some(Value::Bool(b)) => Ok(*b),
                Some(Value::String(s)) => match s.to_lowercase().as_str() {
                    "yes" | "true" => Ok(true),
                    "no" | "false" => Ok(false),
                    _ => Err(anyhow!("{:?}: `{}` is yes or no, not `{}`", path, key, s)),
                },
                Some(_) => Err(anyhow!("{:?}: `{}` is yes or no", path, key)),
            }
        };
        let mut sides = Vec::new();
        match parsed.metadata.get("sides") {
            None | Some(Value::Null) => {}
            Some(Value::Mapping(map)) => {
                for (room, side) in map {
                    let room = room.as_str().map(link_target).ok_or_else(|| anyhow!("{:?}: a side is named by its room", path))?;
                    let (direction, to) = match side {
                        Value::String(d) => (d.clone(), None),
                        Value::Mapping(m) => (
                            m.get("direction")
                                .and_then(Value::as_str)
                                .ok_or_else(|| anyhow!("{:?}: side `{}` needs a `direction`", path, room))?
                                .to_string(),
                            m.get("to").and_then(id_value),
                        ),
                        _ => return Err(anyhow!("{:?}: side `{}` is `{{ direction: <way> }}`", path, room)),
                    };
                    sides.push(DoorSide { room, direction, to });
                }
            }
            Some(_) => return Err(anyhow!("{:?}: `sides` is a map of rooms to `{{ direction: <way> }}`", path)),
        }
        let actions = self.actions(parsed, &id, path)?;
        Ok(Door {
            id,
            title,
            description: parsed.body.clone(),
            sides,
            portal: flag("portal", false)?,
            passable: flag("passable", true)?,
            transparent: flag("transparent", false)?,
            actions,
        })
    }

    /// A thing's actions, from frontmatter or a section's block. In
    /// `becomes:`, `this` is the thing itself (`design/SECTIONS.md`).
    fn actions(&self, parsed: &ParsedDocument, id: &str, path: &Path) -> Result<std::collections::HashMap<String, ObjectAction>> {
        let mut actions = std::collections::HashMap::new();
        match parsed.metadata.get("actions") {
            Some(serde_yaml::Value::Mapping(map)) => {
                for (name, value) in map {
                    let name = name.as_str().ok_or_else(|| anyhow!("{:?}: an action's name must be text", path))?;
                    let mut action = parse_action(name, value, path)?;
                    if let Some(to) = action.becomes.remove("this") {
                        action.becomes.insert(id.to_string(), to);
                    }
                    actions.insert(name.to_string(), action);
                }
            }
            Some(serde_yaml::Value::Sequence(_)) => {
                return Err(anyhow!(
                    "{:?}: `actions` is a map of mechanics, like `pull: {{ becomes: {{ this: open }} }}`, and each reply is the prose of the state it reaches (design/SECTIONS.md)",
                    path
                ))
            }
            Some(serde_yaml::Value::Null) | None => {}
            Some(_) => return Err(anyhow!("{:?}: `actions` must be a map of action names", path)),
        }
        Ok(actions)
    }

    fn build_object<P: AsRef<Path>>(
        &self,
        parsed: &ParsedDocument,
        file_path: P,
    ) -> Result<GameObject> {
        let path = file_path.as_ref();
        let id = self.required_str(&parsed.metadata, "id", path)?;
        let title = self.required_str(&parsed.metadata, "title", path)?;
        let object_type = self.required_str(&parsed.metadata, "object_type", path)?;

        let mut object = GameObject::new(id, title, parsed.body.clone(), object_type);

        object.category = parsed
            .metadata
            .get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        object.weight = parsed
            .metadata
            .get("weight")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        object.value = parsed.metadata.get("value").map(value_to_display_string);

        object.actions = self.actions(parsed, &object.id, path)?;

        let known_fields = known_keys("object");
        if let Some(map) = parsed.metadata.as_mapping() {
            for (key, value) in map {
                if let Some(key_str) = key.as_str() {
                    if !known_fields.contains(&key_str) {
                        object.extra.insert(key_str.to_string(), value.clone());
                    }
                }
            }
        }

        Ok(object)
    }

    /// Split a file into YAML frontmatter and markdown body. Frontmatter
    /// is Hugo-standard: a `---`-delimited block at the very top of the
    /// file. Everything after the closing `---` is the body, used as-is
    /// (trimmed) as the entity's description.
    pub fn split_document(&self, content: &str) -> Result<ParsedDocument> {
        let mut lines = content.lines();

        match lines.next() {
            Some(first) if first.trim() == "---" => {}
            _ => {
                return Err(anyhow!(
                    "file must start with `---` frontmatter delimiter on the first line"
                ))
            }
        }

        let mut frontmatter = String::new();
        let mut closed = false;
        let mut body_lines: Vec<&str> = Vec::new();

        for line in lines {
            if !closed {
                if line.trim() == "---" {
                    closed = true;
                    continue;
                }
                frontmatter.push_str(line);
                frontmatter.push('\n');
            } else {
                body_lines.push(line);
            }
        }

        if !closed {
            return Err(anyhow!("frontmatter block is never closed with a second `---`"));
        }

        let metadata: serde_yaml::Value = if frontmatter.trim().is_empty() {
            serde_yaml::Value::Null
        } else {
            serde_yaml::from_str(&frontmatter)
                .map_err(|e| anyhow!("failed to parse YAML frontmatter: {}", e))?
        };

        Ok(ParsedDocument {
            metadata,
            body: body_lines.join("\n").trim().to_string(),
        })
    }
}

/// One action's mechanics: `becomes`, `sets`, `chapter` and `requires: {
/// has: ... }`. Its reply is the prose of the state it reaches
/// (`design/SECTIONS.md` › Replies); an empty value is an action with no
/// mechanics of its own.
fn parse_action(name: &str, value: &serde_yaml::Value, path: &Path) -> Result<ObjectAction> {
    use serde_yaml::Value;
    let bad = |what: &str| anyhow!("{:?}: action `{}`: {}", path, name, what);
    let text_of = |v: &Value| -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Bool(b) => Some(if *b { "yes".into() } else { "no".into() }),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    };
    let map = match value {
        Value::Null => return Ok(ObjectAction::default()),
        Value::Mapping(map) => map,
        Value::String(_) => return Err(bad("its reply is the prose of the state it reaches, in a `## When:` section, not text in frontmatter")),
        _ => return Err(bad("must be a map of its mechanics")),
    };
    let mut action = ObjectAction::default();
    for (key, v) in map {
        match key.as_str() {
            Some("text") => return Err(bad("its reply is the prose of the state it reaches, in a `## When:` section, not in `text:`")),
            Some("chapter") => action.chapter = Some(v.as_str().ok_or_else(|| bad("`chapter` names one chapter"))?.to_string()),
            Some(field @ ("becomes" | "sets")) => {
                let entries = v.as_mapping().ok_or_else(|| bad(&format!("`{}` must be a map", field)))?;
                for (k, val) in entries {
                    let k = k.as_str().ok_or_else(|| bad(&format!("`{}` keys must be text", field)))?;
                    let val = text_of(val).ok_or_else(|| bad(&format!("`{}.{}` must be a single value", field, k)))?;
                    if field == "becomes" {
                        action.becomes.insert(link_target(k), val);
                    } else {
                        action.sets.insert(k.to_string(), val);
                    }
                }
            }
            Some("requires") => {
                let req = v.as_mapping().ok_or_else(|| bad("`requires` must be a map, like `requires: { has: brass-key }`"))?;
                for (k, val) in req {
                    match k.as_str() {
                        Some("has") => match val {
                            Value::String(s) => action.requires_has.push(link_target(s)),
                            Value::Sequence(items) => {
                                for item in items {
                                    action.requires_has.push(id_value(item).ok_or_else(|| bad("`has` names objects"))?);
                                }
                            }
                            _ => return Err(bad("`has` names an object or a list of them")),
                        },
                        other => {
                            return Err(bad(&format!(
                                "`requires` only takes `has` for now, not `{}`",
                                other.unwrap_or("?")
                            )))
                        }
                    }
                }
            }
            other => return Err(bad(&format!("unknown key `{}`; an action takes becomes, sets, chapter and requires", other.unwrap_or("?")))),
        }
    }
    Ok(action)
}

fn string_sequence(metadata: &serde_yaml::Value, field: &str) -> Vec<String> {
    metadata
        .get(field)
        .and_then(|v| v.as_sequence())
        .map(|seq| seq.iter().filter_map(id_value).collect())
        .unwrap_or_default()
}

/// What an Obsidian link names (REQ-CF-25): `[[castle/hall]]`,
/// `[[castle/hall|the hall]]`, `[[hall#Exits]]` and `castle/hall.md` all
/// name `castle/hall`. Anything else is returned trimmed, as an id. The
/// engine resolves a name that's a file path rather than an id.
pub fn link_target(text: &str) -> String {
    let mut t = text.trim();
    if let Some(inner) = t.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
        t = inner;
        t = t.split('|').next().unwrap_or(t);
        t = t.split('#').next().unwrap_or(t);
    }
    let t = t.trim();
    t.strip_suffix(".md").unwrap_or(t).trim().to_string()
}

/// An id where frontmatter takes one: a string (a link or an id), or an
/// unquoted `[[castle/hall]]`, which YAML reads as a list in a list.
pub fn id_value(value: &serde_yaml::Value) -> Option<String> {
    use serde_yaml::Value;
    match value {
        Value::String(s) => Some(link_target(s)),
        Value::Sequence(items) if items.len() == 1 => match &items[0] {
            Value::String(s) => Some(link_target(s)),
            Value::Sequence(inner) if inner.len() == 1 => inner[0].as_str().map(link_target),
            _ => None,
        },
        _ => None,
    }
}

fn value_to_display_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::character::CharacterType;

    fn parse(content: &str) -> Result<ParsedEntity> {
        MarkdownParser::new().parse_content(content, "test.md")
    }

    fn err(content: &str) -> String {
        parse(content).unwrap_err().to_string()
    }

    const LOCATION: &str = "\
---
id: castle/courtyard
title: Castle Courtyard
type: location
coordinates: [0, 2, -1]
connections:
  north: castle/great-hall
  down: castle/cellar
characters: [guard-captain]
objects: [fountain, stone-bench]
---

You stand in the courtyard.

A fountain sits in the centre.
";

    #[test]
    fn location_fields_land() {
        let ParsedEntity::Location(loc) = parse(LOCATION).unwrap() else { panic!("not a location") };
        assert_eq!(loc.id, "castle/courtyard");
        assert_eq!(loc.name, "Castle Courtyard");
        assert_eq!(loc.coordinates, Some(Coordinate::new(0, 2, -1)));
        assert_eq!(loc.connections.len(), 2);
        assert_eq!(loc.connections["north"], "castle/great-hall");
        assert_eq!(loc.connections["down"], "castle/cellar");
        assert_eq!(loc.characters, vec!["guard-captain"]);
        assert_eq!(loc.objects, vec!["fountain", "stone-bench"]);
        // The body is trimmed, with inner blank lines kept.
        assert_eq!(loc.description, "You stand in the courtyard.\n\nA fountain sits in the centre.");
    }

    #[test]
    fn location_optional_fields_default_empty() {
        let ParsedEntity::Location(loc) = parse("---\nid: a/b\ntitle: B\ntype: location\n---\n").unwrap() else {
            panic!("not a location")
        };
        assert_eq!(loc.coordinates, None);
        assert!(loc.connections.is_empty() && loc.characters.is_empty() && loc.objects.is_empty());
        assert_eq!(loc.description, "");
    }

    #[test]
    fn short_coordinates_are_ignored() {
        let ParsedEntity::Location(loc) =
            parse("---\nid: a/b\ntitle: B\ntype: location\ncoordinates: [1, 2]\n---\n").unwrap()
        else {
            panic!("not a location")
        };
        assert_eq!(loc.coordinates, None);
    }

    #[test]
    fn character_fields_land() {
        let content = "\
---
id: king
title: King Aldric
type: character
character_type: Hero
default_location: castle/throne-room
attributes:
  health: 80
  wisdom: 95
dialogue_tree: king-dialogue.yml
inventory: [royal-scepter, crown-of-kings]
relationships:
  - guard-captain: trusted
  - old-servant: fond
---
An elderly monarch.
";
        let ParsedEntity::Character(c) = parse(content).unwrap() else { panic!("not a character") };
        assert_eq!(c.id, "king");
        assert_eq!(c.name, "King Aldric");
        assert!(matches!(c.character_type, CharacterType::Hero));
        assert_eq!(c.default_location.as_deref(), Some("castle/throne-room"));
        assert_eq!(c.attributes["health"], 80);
        assert_eq!(c.attributes["wisdom"], 95);
        assert_eq!(c.dialogue_tree.as_deref(), Some("king-dialogue.yml"));
        assert_eq!(c.inventory, vec!["royal-scepter", "crown-of-kings"]);
        assert_eq!(c.relationships["guard-captain"], "trusted");
        assert_eq!(c.relationships["old-servant"], "fond");
        assert_eq!(c.description, "An elderly monarch.");
    }

    #[test]
    fn unknown_character_type_is_npc() {
        let ParsedEntity::Character(c) =
            parse("---\nid: x\ntitle: X\ntype: character\ncharacter_type: wizard\n---\n").unwrap()
        else {
            panic!("not a character")
        };
        assert!(matches!(c.character_type, CharacterType::Npc));
    }

    #[test]
    fn object_fields_land_and_extras_are_kept() {
        let content = "\
---
id: rusty-sword
title: Rusty Sword
type: object
object_type: weapon
category: blade
weight: 3 kg
value: 15
damage: 4
tags: [old, iron]
---
A sword past its best.
";
        let ParsedEntity::Object(o) = parse(content).unwrap() else { panic!("not an object") };
        assert_eq!(o.id, "rusty-sword");
        assert_eq!(o.title, "Rusty Sword");
        assert_eq!(o.object_type, "weapon");
        assert_eq!(o.category.as_deref(), Some("blade"));
        assert_eq!(o.weight.as_deref(), Some("3 kg"));
        // A numeric value is kept as its display string.
        assert_eq!(o.value.as_deref(), Some("15"));
        // Fields the engine doesn't model are preserved (REQ-CF-6).
        assert_eq!(o.extra.len(), 2);
        assert_eq!(o.extra["damage"], serde_yaml::Value::from(4));
        assert!(o.extra["tags"].is_sequence());
        assert_eq!(o.description, "A sword past its best.");
    }

    #[test]
    fn missing_type_is_an_error() {
        let e = err("---\nid: a/b\ntitle: B\n---\nBody.\n");
        assert!(e.contains("missing required frontmatter field `type`"), "{}", e);
    }

    #[test]
    fn unknown_type_is_an_error() {
        let e = err("---\nid: a/b\ntitle: B\ntype: room\n---\n");
        assert!(e.contains("unknown type `room`"), "{}", e);
    }

    #[test]
    fn missing_required_fields_are_errors() {
        for (content, field) in [
            ("---\ntitle: B\ntype: location\n---\n", "id"),
            ("---\nid: a/b\ntype: location\n---\n", "title"),
            ("---\ntitle: K\ntype: character\n---\n", "id"),
            ("---\nid: k\ntype: character\n---\n", "title"),
            ("---\ntitle: S\ntype: object\nobject_type: weapon\n---\n", "id"),
            ("---\nid: s\ntype: object\nobject_type: weapon\n---\n", "title"),
            ("---\nid: s\ntitle: S\ntype: object\n---\n", "object_type"),
        ] {
            let e = err(content);
            assert!(e.contains(&format!("missing required frontmatter field `{}`", field)), "{}: {}", field, e);
        }
    }

    #[test]
    fn unclosed_frontmatter_is_an_error() {
        let e = err("---\nid: a/b\ntitle: B\ntype: location\nBody with no closing delimiter.\n");
        assert!(e.contains("never closed"), "{}", e);
    }

    #[test]
    fn no_frontmatter_is_an_error() {
        let e = err("# Courtyard\n\nJust prose.\n");
        assert!(e.contains("must start with `---`"), "{}", e);
        // A blank first line counts as no frontmatter, too.
        assert!(err("\n---\nid: a/b\ntitle: B\ntype: location\n---\n").contains("must start with `---`"));
    }

    #[test]
    fn empty_frontmatter_reports_missing_type() {
        assert!(err("---\n---\nBody.\n").contains("missing required frontmatter field `type`"));
    }

    #[test]
    fn invalid_yaml_is_an_error() {
        let e = err("---\nid: [unclosed\ntype: location\n---\n");
        assert!(e.contains("failed to parse YAML frontmatter"), "{}", e);
    }

    #[test]
    fn a_second_delimiter_in_the_body_stays_in_the_body() {
        let ParsedEntity::Location(loc) =
            parse("---\nid: a/b\ntitle: B\ntype: location\n---\nAbove.\n---\nBelow.\n").unwrap()
        else {
            panic!("not a location")
        };
        assert_eq!(loc.description, "Above.\n---\nBelow.");
    }

    /// Keys are case-sensitive: `Inventory:` is not `inventory:`, so it is
    /// not read (a real sample file once had this). This pins today's
    /// behaviour, which drops the key silently.
    #[test]
    fn capitalized_inventory_key_is_not_read() {
        let ParsedEntity::Character(c) =
            parse("---\nid: k\ntitle: K\ntype: character\nInventory: [sword]\n---\n").unwrap()
        else {
            panic!("not a character")
        };
        assert!(c.inventory.is_empty());
    }

    #[test]
    fn case_only_key_differences_are_warned_about() {
        let parser = MarkdownParser::new();
        let outcome = parser
            .parse_content_with_warnings(
                "---\nid: k\ntitle: K\ntype: character\nInventory: [sword]\nMood: grim\n---\n",
                "k.md",
            )
            .unwrap();
        assert_eq!(outcome.warnings.len(), 1, "{:?}", outcome.warnings);
        assert!(outcome.warnings[0].starts_with("key `Inventory` differs only in case from `inventory`"));

        // Objects: extra keys are fine, but a case-only difference still warns.
        let outcome = parser
            .parse_content_with_warnings(
                "---\nid: s\ntitle: S\ntype: object\nobject_type: x\nCategory: y\ndamage: 4\n---\n",
                "s.md",
            )
            .unwrap();
        assert_eq!(outcome.warnings.len(), 1, "{:?}", outcome.warnings);
        assert!(outcome.warnings[0].contains("`Category` differs only in case from `category`"));

        // A clean file has none.
        assert!(parser.parse_content_with_warnings(LOCATION, "c.md").unwrap().warnings.is_empty());
    }

    #[test]
    fn parse_file_reads_from_disk_and_reports_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("courtyard.md");
        std::fs::write(&path, LOCATION).unwrap();
        assert!(matches!(MarkdownParser::new().parse_file(&path).unwrap(), ParsedEntity::Location(_)));

        let missing = dir.path().join("missing.md");
        let e = MarkdownParser::new().parse_file(&missing).unwrap_err().to_string();
        assert!(e.contains("Failed to read file"), "{}", e);
    }

    #[test]
    fn actions_are_mechanics_and_a_marker_is_only_a_comment() {
        let text = "---\nid: lever\ntitle: Lever\ntype: object\nobject_type: x\nactions:\n  pull:\n    becomes: { castle/vault: open, this: pulled }\n    sets: { story.vault: open, this.pulled: yes }\n    chapter: after-the-king\n    requires: { has: [lamp, key] }\n  wiggle:\n---\nAn iron lever.\n\n<!-- thwarter:action kick -->\nOw.\n";
        let ParsedEntity::Object(o) = MarkdownParser::new().parse_content(text, "lever.md").unwrap() else { panic!() };
        assert_eq!(o.description, "An iron lever.\n\n<!-- thwarter:action kick -->\nOw.");
        let pull = &o.actions["pull"];
        assert_eq!(pull.becomes["castle/vault"], "open");
        // `this` is the thing itself.
        assert_eq!(pull.becomes["lever"], "pulled");
        assert!(!pull.becomes.contains_key("this"));
        assert_eq!(pull.sets["this.pulled"], "yes");
        assert_eq!(pull.chapter.as_deref(), Some("after-the-king"));
        assert_eq!(pull.requires_has, vec!["lamp", "key"]);
        assert_eq!(o.actions["wiggle"], ObjectAction::default());
        assert!(!o.actions.contains_key("kick"));

        for (bad, why) in [
            ("actions:\n  pull: Ow.", "the prose of the state it reaches"),
            ("actions:\n  pull: { text: Ow. }", "not in `text:`"),
            ("actions:\n- kick: Ow.", "is a map of mechanics"),
            ("actions:\n  pull: { teleport: yes }", "unknown key `teleport`"),
            ("actions:\n  pull: { requires: { knows: secret } }", "only takes `has`"),
            ("actions:\n  pull: { becomes: open }", "`becomes` must be a map"),
        ] {
            let text = format!("---\nid: x\ntitle: X\ntype: object\nobject_type: x\n{}\n---\n", bad);
            let err = MarkdownParser::new().parse_content(&text, "x.md").unwrap_err().to_string();
            assert!(err.contains(why), "{}: {}", bad, err);
        }
    }


    #[test]
    fn links_name_what_they_point_at() {
        for (text, name) in [
            ("[[castle/hall]]", "castle/hall"),
            ("[[castle/hall|the hall]]", "castle/hall"),
            ("[[hall#Exits]]", "hall"),
            ("[[objects/lever.md]]", "objects/lever"),
            ("  castle/hall  ", "castle/hall"),
            ("objects/lever.md", "objects/lever"),
        ] {
            assert_eq!(link_target(text), name, "{}", text);
        }
        let unquoted: serde_yaml::Value = serde_yaml::from_str("[[castle/hall]]").unwrap();
        assert_eq!(id_value(&unquoted).as_deref(), Some("castle/hall"));
        let pair: serde_yaml::Value = serde_yaml::from_str("[a, b]").unwrap();
        assert_eq!(id_value(&pair), None);
    }
}
