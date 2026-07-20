use colored::*;
use serde::Serialize;
use crate::error::Result;

/// Output formatter for CLI results
pub struct OutputFormatter {
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
    Yaml,
}

impl OutputFormatter {
    pub fn new(format_str: &str) -> Result<Self> {
        let format = match format_str.to_lowercase().as_str() {
            "human" | "text" => OutputFormat::Human,
            "json" => OutputFormat::Json,
            "yaml" | "yml" => OutputFormat::Yaml,
            _ => return Err(format!("Unknown output format: {}", format_str).into()),
        };
        
        Ok(Self { format })
    }
    
    pub fn format<T: Serialize>(&self, data: &T) -> Result<String> {
        match self.format {
            OutputFormat::Json => {
                serde_json::to_string_pretty(data)
                    .map_err(|e| format!("JSON serialization failed: {}", e).into())
            }
            OutputFormat::Yaml => {
                // For simplicity, we'll use JSON for now since serde_yaml isn't in dependencies
                serde_json::to_string_pretty(data)
                    .map_err(|e| format!("YAML serialization failed: {}", e).into())
            }
            OutputFormat::Human => {
                // Default to JSON for complex structures
                serde_json::to_string_pretty(data)
                    .map_err(|e| format!("Serialization failed: {}", e).into())
            }
        }
    }
    
    pub fn is_human(&self) -> bool {
        matches!(self.format, OutputFormat::Human)
    }
}

/// Helper functions for colored output
pub fn success(msg: &str) -> String {
    format!("{} {}", "✓".green().bold(), msg.green())
}

pub fn warning(msg: &str) -> String {
    format!("{} {}", "⚠".yellow().bold(), msg.yellow())
}

pub fn error(msg: &str) -> String {
    format!("{} {}", "✗".red().bold(), msg.red())
}

pub fn info(msg: &str) -> String {
    format!("{} {}", "ℹ".blue().bold(), msg)
}

pub fn status_badge(status: &str, is_healthy: bool) -> String {
    let badge = if is_healthy {
        format!(" {} ", status).on_green().black()
    } else {
        format!(" {} ", status).on_red().white()
    };
    badge.to_string()
}
