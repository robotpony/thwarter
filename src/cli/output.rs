use colored::*;
use serde_json;
use serde_yaml;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
}

impl From<&str> for OutputFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "yaml" | "yml" => OutputFormat::Yaml,
            _ => OutputFormat::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputFormatter {
    pub format: OutputFormat,
    pub colored_output: bool,
    pub verbose: bool,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            colored_output: true,
            verbose: false,
        }
    }
    
    pub fn with_color(mut self, colored: bool) -> Self {
        self.colored_output = colored;
        self
    }
    
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
    
    pub fn print_success(&self, message: &str) {
        match self.format {
            OutputFormat::Text => {
                if self.colored_output {
                    println!("{}", message.green());
                } else {
                    println!("{}", message);
                }
            },
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "success",
                    "message": message
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            },
            OutputFormat::Yaml => {
                let output = serde_yaml::to_string(&serde_json::json!({
                    "status": "success",
                    "message": message
                })).unwrap();
                println!("{}", output);
            },
        }
    }
    
    pub fn print_error(&self, message: &str) {
        match self.format {
            OutputFormat::Text => {
                if self.colored_output {
                    eprintln!("{} {}", "ERROR:".red().bold(), message);
                } else {
                    eprintln!("ERROR: {}", message);
                }
            },
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "error",
                    "message": message
                });
                eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
            },
            OutputFormat::Yaml => {
                let output = serde_yaml::to_string(&serde_json::json!({
                    "status": "error",
                    "message": message
                })).unwrap();
                eprintln!("{}", output);
            },
        }
    }
    
    pub fn print_warning(&self, message: &str) {
        match self.format {
            OutputFormat::Text => {
                if self.colored_output {
                    println!("{} {}", "WARNING:".yellow().bold(), message);
                } else {
                    println!("WARNING: {}", message);
                }
            },
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "warning",
                    "message": message
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            },
            OutputFormat::Yaml => {
                let output = serde_yaml::to_string(&serde_json::json!({
                    "status": "warning",
                    "message": message
                })).unwrap();
                println!("{}", output);
            },
        }
    }
    
    pub fn print_info(&self, message: &str) {
        if self.verbose || matches!(self.format, OutputFormat::Json | OutputFormat::Yaml) {
            match self.format {
                OutputFormat::Text => {
                    if self.colored_output {
                        println!("{} {}", "INFO:".blue().bold(), message);
                    } else {
                        println!("INFO: {}", message);
                    }
                },
                OutputFormat::Json => {
                    let output = serde_json::json!({
                        "status": "info",
                        "message": message
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                },
                OutputFormat::Yaml => {
                    let output = serde_yaml::to_string(&serde_json::json!({
                        "status": "info",
                        "message": message
                    })).unwrap();
                    println!("{}", output);
                },
            }
        }
    }
    
    pub fn print_data<T: serde::Serialize + std::fmt::Debug>(&self, data: &T) -> Result<()> {
        match self.format {
            OutputFormat::Text => {
                // For text format, we'll need custom implementations per data type
                println!("{:#?}", data);
            },
            OutputFormat::Json => {
                let output = serde_json::to_string_pretty(data)?;
                println!("{}", output);
            },
            OutputFormat::Yaml => {
                let output = serde_yaml::to_string(data)?;
                println!("{}", output);
            },
        }
        Ok(())
    }
}

impl Default for OutputFormatter {
    fn default() -> Self {
        Self::new(OutputFormat::Text)
    }
}