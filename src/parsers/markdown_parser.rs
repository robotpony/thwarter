use crate::models::world::Location;
use crate::models::character::Character;
use anyhow::{Result, anyhow};
use pulldown_cmark::{Parser, Event, Tag};
use serde_yaml;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MarkdownParser;

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub title: String,
    pub content: String,
    pub metadata: serde_yaml::Value,
}

impl MarkdownParser {
    pub fn new() -> Self {
        Self
    }
    
    pub fn parse_location_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Location> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        
        let parsed = self.parse_markdown_with_frontmatter(&content)?;
        
        // Extract location data from metadata
        let location_id = parsed.metadata.get("location_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
            
        let mut location = Location::new(
            location_id,
            parsed.title.clone(),
            parsed.content.clone(),
        );
        
        // Parse coordinates if present
        if let Some(coords) = parsed.metadata.get("coordinates") {
            if let Some(coord_array) = coords.as_sequence() {
                if coord_array.len() >= 3 {
                    let x = coord_array[0].as_i64().unwrap_or(0) as i32;
                    let y = coord_array[1].as_i64().unwrap_or(0) as i32;
                    let z = coord_array[2].as_i64().unwrap_or(0) as i32;
                    location.coordinates = Some((x, y, z));
                }
            }
        }
        
        // Parse connections
        if let Some(connections) = parsed.metadata.get("connections") {
            if let Some(conn_map) = connections.as_mapping() {
                for (direction, target) in conn_map {
                    if let (Some(dir), Some(tgt)) = (direction.as_str(), target.as_str()) {
                        location.connections.insert(dir.to_string(), tgt.to_string());
                    }
                }
            }
        }
        
        // Parse characters
        if let Some(characters) = parsed.metadata.get("characters") {
            if let Some(char_array) = characters.as_sequence() {
                for char_id in char_array {
                    if let Some(id) = char_id.as_str() {
                        location.characters.push(id.to_string());
                    }
                }
            }
        }
        
        // Parse objects
        if let Some(objects) = parsed.metadata.get("objects") {
            if let Some(obj_array) = objects.as_sequence() {
                for obj_id in obj_array {
                    if let Some(id) = obj_id.as_str() {
                        location.objects.push(id.to_string());
                    }
                }
            }
        }
        
        Ok(location)
    }
    
    pub fn parse_character_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Character> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        
        let parsed = self.parse_markdown_with_frontmatter(&content)?;
        
        let character_id = parsed.metadata.get("character_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
            
        let mut character = Character::new(
            character_id,
            parsed.title.clone(),
            parsed.content.clone(),
        );
        
        // Parse character type
        if let Some(char_type) = parsed.metadata.get("type").and_then(|v| v.as_str()) {
            character.character_type = match char_type.to_lowercase().as_str() {
                "player" => crate::models::character::CharacterType::Player,
                "villain" => crate::models::character::CharacterType::Villain,
                "hero" => crate::models::character::CharacterType::Hero,
                _ => crate::models::character::CharacterType::Npc,
            };
        }
        
        // Parse default location
        if let Some(location) = parsed.metadata.get("default_location").and_then(|v| v.as_str()) {
            character.default_location = Some(location.to_string());
        }
        
        // Parse attributes
        if let Some(attributes) = parsed.metadata.get("attributes") {
            if let Some(attr_map) = attributes.as_mapping() {
                for (key, value) in attr_map {
                    if let (Some(k), Some(v)) = (key.as_str(), value.as_i64()) {
                        character.attributes.insert(k.to_string(), v as i32);
                    }
                }
            }
        }
        
        // Parse dialogue tree
        if let Some(dialogue) = parsed.metadata.get("dialogue_tree").and_then(|v| v.as_str()) {
            character.dialogue_tree = Some(dialogue.to_string());
        }
        
        Ok(character)
    }
    
    fn parse_markdown_with_frontmatter(&self, content: &str) -> Result<ParsedDocument> {
        let mut lines = content.lines();
        let mut frontmatter = String::new();
        let mut body = String::new();
        let mut in_frontmatter = false;
        let mut title = String::new();
        
        // Check for frontmatter
        if let Some(first_line) = lines.next() {
            if first_line.trim() == "---" {
                in_frontmatter = true;
            } else {
                body.push_str(first_line);
                body.push('\n');
            }
        }
        
        for line in lines {
            if in_frontmatter {
                if line.trim() == "---" {
                    in_frontmatter = false;
                    continue;
                }
                frontmatter.push_str(line);
                frontmatter.push('\n');
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }
        
        // Parse frontmatter as YAML
        let metadata = if !frontmatter.is_empty() {
            serde_yaml::from_str(&frontmatter)
                .map_err(|e| anyhow!("Failed to parse YAML frontmatter: {}", e))?
        } else {
            serde_yaml::Value::Null
        };
        
        // Extract title from markdown content
        let parser = Parser::new(&body);
        for event in parser {
            match event {
                Event::Start(Tag::Heading(_, _, _)) => {},
                Event::Text(text) => {
                    if title.is_empty() {
                        title = text.to_string();
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if title.is_empty() {
            title = "Untitled".to_string();
        }
        
        Ok(ParsedDocument {
            title,
            content: body.trim().to_string(),
            metadata,
        })
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}