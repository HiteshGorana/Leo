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
    use crate::ui;
    use inquire::{Select, Text, Confirm};

    ui::print_leo_header_with_emotion("Setup Wizard", "Local", ui::LionEmotion::Happy);
    println!("  Welcome! I'll help you get Leo configured in just a few steps.\n");

    let mut config = Config::default();

    // 1. Select Provider
    let providers = vec!["Gemini (API Key - Fast)", "Google CLI (OAuth - No key needed)"];
    let provider_choice = Select::new("Choose your AI provider:", providers).prompt()
        .map_err(|e| Error::Config(format!("Prompt failed: {}", e)))?;

    if provider_choice.contains("API Key") {
        config.provider = "gemini".to_string();
        let key = Text::new("Enter your Gemini API Key:").prompt()
            .map_err(|e| Error::Config(format!("Prompt failed: {}", e)))?;
        config.gemini_api_key = key;
    } else {
        config.provider = "google-cli".to_string();
    }

    // 2. Confirm Workspace
    ui::print_step(&format!("Default workspace is at {:?}", config.workspace));
    let change_path = Confirm::new("Use default workspace path?").with_default(true).prompt()
        .map_err(|e| Error::Config(format!("Prompt failed: {}", e)))?;

    if !change_path {
        let new_path = Text::new("Enter custom workspace path:").prompt()
            .map_err(|e| Error::Config(format!("Prompt failed: {}", e)))?;
        config.workspace = PathBuf::from(new_path);
    }

    // 3. Setup Folders
    ui::print_thinking("Creating directories");
    std::fs::create_dir_all(&config.workspace)?;
    std::fs::create_dir_all(config.workspace.join("memory"))?;
    std::fs::create_dir_all(config.workspace.join("skills"))?;
    
    ui::print_thinking("Bootstrapping AGENTS.md and MEMORY.md");
    create_bootstrap_files(&config.workspace)?;
    
    // 4. Gateway Setup (Optional)
    let gateways = vec!["None (Skip for now)", "Telegram Bot", "WhatsApp (Coming soon)", "Slack (Coming soon)"];
    let gateway_choice = Select::new("Would you like to setup a Gateway (Remote Access)?", gateways).prompt()
        .map_err(|e| Error::Config(format!("Prompt failed: {}", e)))?;

    if gateway_choice.contains("Telegram") {
        setup_telegram_gateway(&mut config)?;
    }

    // 5. Save Config
    ui::print_thinking("Saving configuration");
    save(&config)?;
    
    println!();
    ui::print_success("Setup complete!");
    
    if config.provider == "google-cli" {
        ui::print_step("Preparing Google SDK authentication...");
    } else {
        ui::print_step("You're all set! Run 'leo agent' to start chatting.");
    }
    
    Ok(())
}

/// Helper to setup Telegram gateway interactively
pub fn setup_telegram_gateway(config: &mut Config) -> Result<()> {
    use inquire::Text;
    use crate::ui;
    use colored::Colorize;

    println!();
    ui::print_step("To setup a Telegram bot:");
    println!("    1. Message {} on Telegram", "@BotFather".cyan().bold());
    println!("    2. Send {} and choose a name", "/newbot".cyan());
    println!("    3. Copy the {} provided", "API Token".cyan());
    println!();

    let token = Text::new("Enter your Telegram Bot Token:").prompt()
        .map_err(|e| Error::Config(format!("Prompt failed: {}", e)))?;

    if token.is_empty() {
        return Err(Error::Config("Token cannot be empty".to_string()));
    }

    config.telegram.enabled = true;
    config.telegram.token = token;
    
    let user = whoami::username();
    config.telegram.allow_from = vec![user.clone()];
    
    ui::print_step(&format!("Auto-whitelisted local user: {}", user.cyan()));
    
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

/// Reset Leo by deleting all configuration and data
pub fn reset() -> Result<()> {
    use inquire::Confirm;
    use crate::ui;

    ui::print_warning("CAUTION: This will delete all Leo configuration, memory, and moments.");
    
    let confirmed = Confirm::new("Are you absolutely sure you want to reset Leo?")
        .with_default(false)
        .prompt()
        .map_err(|e| Error::Config(format!("Prompt failed: {}", e)))?;

    if confirmed {
        let dir = config_dir();
        if dir.exists() {
            ui::print_thinking(&format!("Deleting {:?}", dir));
            std::fs::remove_dir_all(dir)?;
            ui::print_success("Leo has been reset.");
        } else {
            ui::print_step("No configuration directory found.");
        }
    } else {
        ui::print_step("Reset cancelled.");
    }

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
