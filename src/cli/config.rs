use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThwartConfig {
    pub world: WorldConfig,
    pub output: OutputConfig,
    pub validation: ValidationConfig,
    pub debug: DebugConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub path: String,
    pub default_player: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: String,
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub strict_mode: bool,
    pub auto_fix: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    pub trace_level: String,
    pub profile_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetadata {
    pub metadata: MetadataSection,
    pub settings: SettingsSection,
    pub characters: CharacterDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSection {
    pub title: String,
    pub author: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsSection {
    pub starting_location: String,
    pub default_map_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDefaults {
    pub default_health: i32,
    pub default_mood: String,
}

impl Default for ThwartConfig {
    fn default() -> Self {
        Self {
            world: WorldConfig {
                path: "authors/world/".to_string(),
                default_player: "player".to_string(),
            },
            output: OutputConfig {
                format: "text".to_string(),
                style: "unicode".to_string(),
            },
            validation: ValidationConfig {
                strict_mode: false,
                auto_fix: false,
            },
            debug: DebugConfig {
                trace_level: "info".to_string(),
                profile_enabled: false,
            },
        }
    }
}

impl Default for WorldMetadata {
    fn default() -> Self {
        Self {
            metadata: MetadataSection {
                title: "My Interactive Fiction".to_string(),
                author: "Author Name".to_string(),
                version: "1.0.0".to_string(),
            },
            settings: SettingsSection {
                starting_location: "home/bedroom".to_string(),
                default_map_style: "ascii".to_string(),
            },
            characters: CharacterDefaults {
                default_health: 100,
                default_mood: "neutral".to_string(),
            },
        }
    }
}

#[derive(Debug)]
pub struct ConfigManager {
    pub global_config: ThwartConfig,
    pub world_config: Option<WorldMetadata>,
    pub config_path: PathBuf,
    pub world_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            global_config: ThwartConfig::default(),
            world_config: None,
            config_path: PathBuf::from("thwart.toml"),
            world_path: PathBuf::from("authors/world"),
        }
    }
    
    pub fn load_config<P: AsRef<Path>>(&mut self, config_path: Option<P>) -> Result<()> {
        if let Some(path) = config_path {
            self.config_path = path.as_ref().to_path_buf();
        }
        
        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path)
                .map_err(|e| anyhow!("Failed to read config file: {}", e))?;
            
            self.global_config = toml::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
        }
        
        // Update world path from config
        self.world_path = PathBuf::from(&self.global_config.world.path);
        
        Ok(())
    }
    
    pub fn load_world_config(&mut self) -> Result<()> {
        let world_config_path = self.world_path.join("world.toml");
        
        if world_config_path.exists() {
            let content = fs::read_to_string(&world_config_path)
                .map_err(|e| anyhow!("Failed to read world config file: {}", e))?;
            
            self.world_config = Some(toml::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse world config file: {}", e))?);
        } else {
            self.world_config = Some(WorldMetadata::default());
        }
        
        Ok(())
    }
    
    pub fn save_config(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self.global_config)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;
        
        fs::write(&self.config_path, content)
            .map_err(|e| anyhow!("Failed to write config file: {}", e))?;
        
        Ok(())
    }
    
    pub fn save_world_config(&self) -> Result<()> {
        if let Some(world_config) = &self.world_config {
            let world_config_path = self.world_path.join("world.toml");
            let content = toml::to_string_pretty(world_config)
                .map_err(|e| anyhow!("Failed to serialize world config: {}", e))?;
            
            // Ensure world directory exists
            if let Some(parent) = world_config_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| anyhow!("Failed to create world directory: {}", e))?;
            }
            
            fs::write(&world_config_path, content)
                .map_err(|e| anyhow!("Failed to write world config file: {}", e))?;
        }
        
        Ok(())
    }
    
    pub fn get_world_path(&self) -> &Path {
        &self.world_path
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}