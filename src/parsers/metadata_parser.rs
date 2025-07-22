use anyhow::{Result, anyhow};
use serde_yaml;
use serde_json;
use toml::{self, Table};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MetadataParser;

#[derive(Debug, Clone)]
pub enum MetadataFormat {
    Yaml,
    Json,
    Toml,
}

impl MetadataParser {
    pub fn new() -> Self {
        Self
    }
    
    pub fn parse_file<P: AsRef<Path>>(&self, file_path: P) -> Result<serde_yaml::Value> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file {:?}: {}", file_path.as_ref(), e))?;
        
        let format = self.detect_format(&file_path)?;
        self.parse_content(&content, format)
    }
    
    pub fn parse_content(&self, content: &str, format: MetadataFormat) -> Result<serde_yaml::Value> {
        match format {
            MetadataFormat::Yaml => {
                serde_yaml::from_str(content)
                    .map_err(|e| anyhow!("Failed to parse YAML: {}", e))
            },
            MetadataFormat::Json => {
                let json_value: serde_json::Value = serde_json::from_str(content)
                    .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;
                
                // Convert JSON to YAML value for consistent handling
                let yaml_str = serde_yaml::to_string(&json_value)
                    .map_err(|e| anyhow!("Failed to convert JSON to YAML: {}", e))?;
                
                serde_yaml::from_str(&yaml_str)
                    .map_err(|e| anyhow!("Failed to parse converted YAML: {}", e))
            },
            MetadataFormat::Toml => {
                let toml_value: toml::Value = toml::from_str(content)
                    .map_err(|e| anyhow!("Failed to parse TOML: {}", e))?;
                
                // Convert TOML to YAML value for consistent handling
                let yaml_str = serde_yaml::to_string(&toml_value)
                    .map_err(|e| anyhow!("Failed to convert TOML to YAML: {}", e))?;
                
                serde_yaml::from_str(&yaml_str)
                    .map_err(|e| anyhow!("Failed to parse converted YAML: {}", e))
            },
        }
    }
    
    fn detect_format<P: AsRef<Path>>(&self, file_path: P) -> Result<MetadataFormat> {
        let path = file_path.as_ref();
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                "yml" | "yaml" => Ok(MetadataFormat::Yaml),
                "json" => Ok(MetadataFormat::Json),
                "toml" => Ok(MetadataFormat::Toml),
                _ => Err(anyhow!("Unsupported metadata format: {}", extension)),
            }
        } else {
            Err(anyhow!("No file extension found for {:?}", path))
        }
    }
    
    pub fn write_file<P: AsRef<Path>>(&self, file_path: P, value: &serde_yaml::Value) -> Result<()> {
        let format = self.detect_format(&file_path)?;
        let content = match format {
            MetadataFormat::Yaml => {
                serde_yaml::to_string(value)
                    .map_err(|e| anyhow!("Failed to serialize to YAML: {}", e))?
            },
            MetadataFormat::Json => {
                serde_json::to_string_pretty(value)
                    .map_err(|e| anyhow!("Failed to serialize to JSON: {}", e))?
            },
            MetadataFormat::Toml => {
                // For TOML, convert through JSON string representation
                let json_str = serde_json::to_string(value)
                    .map_err(|e| anyhow!("Failed to convert to JSON: {}", e))?;
                let json_value: serde_json::Value = serde_json::from_str(&json_str)
                    .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;
                // Create TOML table from JSON
                if let serde_json::Value::Object(map) = json_value {
                    let mut toml_table = Table::new();
                    for (key, val) in map {
                        // Convert basic types only for now
                        match val {
                            serde_json::Value::String(s) => { toml_table.insert(key, toml::Value::String(s)); },
                            serde_json::Value::Number(n) => { 
                                if let Some(i) = n.as_i64() { 
                                    toml_table.insert(key, toml::Value::Integer(i)); 
                                } else if let Some(f) = n.as_f64() { 
                                    toml_table.insert(key, toml::Value::Float(f)); 
                                }
                            },
                            serde_json::Value::Bool(b) => { toml_table.insert(key, toml::Value::Boolean(b)); },
                            _ => {} // Skip complex types for now
                        }
                    }
                    toml::to_string_pretty(&toml_table)
                        .map_err(|e| anyhow!("Failed to serialize to TOML: {}", e))?
                } else {
                    return Err(anyhow!("TOML format requires object structure"));
                }
            },
        };
        
        fs::write(&file_path, content)
            .map_err(|e| anyhow!("Failed to write file {:?}: {}", file_path.as_ref(), e))?;
        
        Ok(())
    }
}

impl Default for MetadataParser {
    fn default() -> Self {
        Self::new()
    }
}