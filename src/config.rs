//! Configuration management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::Result;
use crate::error::Error;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Workspace directory path
    #[serde(default = "default_workspace")]
    pub workspace: PathBuf,
    
    /// LLM provider to use ("gemini" for API key, "google-cli" for OAuth)
    #[serde(default = "default_provider")]
    pub provider: String,
    
    /// Gemini API key (used when provider is "gemini")
    #[serde(default)]
    pub gemini_api_key: String,
    
    /// Model to use
    #[serde(default = "default_model")]
    pub model: String,
    
    /// Maximum tool iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    
    /// OAuth configuration (used when provider is "google-cli")
    #[serde(default)]
    pub oauth: Option<OAuthConfig>,
    
    /// Telegram configuration
    #[serde(default)]
    pub telegram: TelegramConfig,
}

/// OAuth configuration for manual credential setup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OAuthConfig {
    #[serde(default)]
    pub client_id: String,
    
    #[serde(default)]
    pub client_secret: String,
}

fn default_workspace() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".leo")
        .join("workspace")
}

fn default_model() -> String {
    "gemini-2.0-flash".to_string()
}

fn default_max_iterations() -> usize {
    20
}

fn default_provider() -> String {
    "gemini".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    
    #[serde(default)]
    pub token: String,
    
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace: default_workspace(),
            provider: default_provider(),
            gemini_api_key: String::new(),
            model: default_model(),
            max_iterations: default_max_iterations(),
            oauth: None,
            telegram: TelegramConfig::default(),
        }
    }
}

/// Get the config directory path
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".leo")
}

/// Get the config file path
pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Load configuration from file
pub fn load() -> Result<Config> {
    let path = config_path();
    
    if !path.exists() {
        return Err(Error::Config(format!(
            "Config not found at {:?}. Run 'leo onboard' first.",
            path
        )));
    }
    
    let content = std::fs::read_to_string(&path)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

/// Save configuration to file
pub fn save(config: &Config) -> Result<()> {
    let path = config_path();
    
    // Create parent directory
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Initialize configuration and workspace
pub fn onboard() -> Result<()> {
    let config = Config::default();
    
    // Create workspace directory
    std::fs::create_dir_all(&config.workspace)?;
    
    // Create memory directory
    let memory_dir = config.workspace.join("memory");
    std::fs::create_dir_all(&memory_dir)?;
    
    // Create skills directory
    let skills_dir = config.workspace.join("skills");
    std::fs::create_dir_all(&skills_dir)?;
    
    // Create bootstrap files
    create_bootstrap_files(&config.workspace)?;
    
    // Save config
    save(&config)?;
    
    Ok(())
}

fn create_bootstrap_files(workspace: &PathBuf) -> Result<()> {
    let agents_md = r#"# Agent Instructions

You are a helpful AI assistant. Be concise, accurate, and friendly.

## Guidelines

- Always explain what you're doing before taking actions
- Ask for clarification when the request is ambiguous
- Use tools to help accomplish tasks
- Remember important information in your memory files
"#;

    let memory_md = r#"# Long-term Memory

This file stores important information that should persist across sessions.

## User Information

(Important facts about the user)

## Preferences

(User preferences learned over time)
"#;

    std::fs::write(workspace.join("AGENTS.md"), agents_md)?;
    std::fs::write(workspace.join("memory").join("MEMORY.md"), memory_md)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model, "gemini-2.0-flash");
        assert_eq!(config.max_iterations, 20);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model, config.model);
    }
}
