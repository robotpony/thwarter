use crate::models::character::Character;
use crate::models::world::{Coordinate, GameObject, Location};
use anyhow::{anyhow, Result};
use serde_yaml;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MarkdownParser;

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
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        let parsed = self.split_frontmatter(&content)?;

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

        match entity_type {
            "location" => Ok(ParsedEntity::Location(self.build_location(&parsed, &file_path)?)),
            "character" => Ok(ParsedEntity::Character(self.build_character(&parsed, &file_path)?)),
            "object" => Ok(ParsedEntity::Object(self.build_object(&parsed, &file_path)?)),
            other => Err(anyhow!(
                "{:?}: unknown type `{}` (expected location, character, or object)",
                file_path.as_ref(),
                other
            )),
        }
    }

    pub fn parse_location_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Location> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        let parsed = self.split_frontmatter(&content)?;
        self.build_location(&parsed, &file_path)
    }

    pub fn parse_character_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Character> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        let parsed = self.split_frontmatter(&content)?;
        self.build_character(&parsed, &file_path)
    }

    pub fn parse_object_file<P: AsRef<Path>>(&self, file_path: P) -> Result<GameObject> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        let parsed = self.split_frontmatter(&content)?;
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
                if let (Some(dir), Some(tgt)) = (direction.as_str(), target.as_str()) {
                    location.connections.insert(dir.to_string(), tgt.to_string());
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

        if let Some(location) = parsed.metadata.get("default_location").and_then(|v| v.as_str()) {
            character.default_location = Some(location.to_string());
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

        if let Some(action_seq) = parsed.metadata.get("actions").and_then(|v| v.as_sequence()) {
            // Author-friendly shape: a list of single-key maps, e.g.
            //   actions:
            //     - wield: "The sword feels like an extension of your arm"
            for entry in action_seq {
                if let Some(map) = entry.as_mapping() {
                    for (k, v) in map {
                        if let (Some(k), Some(v)) = (k.as_str(), v.as_str()) {
                            object.actions.insert(k.to_string(), v.to_string());
                        }
                    }
                }
            }
        }

        let known_fields = [
            "id",
            "title",
            "type",
            "object_type",
            "category",
            "weight",
            "value",
            "actions",
        ];
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
    fn split_frontmatter(&self, content: &str) -> Result<ParsedDocument> {
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

fn string_sequence(metadata: &serde_yaml::Value, field: &str) -> Vec<String> {
    metadata
        .get(field)
        .and_then(|v| v.as_sequence())
        .map(|seq| seq.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
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
