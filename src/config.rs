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
    
    // Keep allow_from empty - allows all users (suitable for personal bots)
    // Users can manually add Telegram usernames/IDs to config.json if needed
    
    Ok(())
}

fn create_bootstrap_files(workspace: &PathBuf) -> Result<()> {
    // AGENTS.md
    let agents_md = r#"# AGENTS.md - Leo's Workspace

This folder is home. Treat it that way.

## Every Session

Before doing anything else:

1. Read `SOUL.md` — this is who you are
2. Read `USER.md` — this is who you're helping
3. Read `memory/YYYY-MM-DD.md` (today + yesterday) for recent context

Don't ask permission. Just do it.

## Memory

You wake up fresh each session. These files are your continuity:

- **Daily notes:** `memory/YYYY-MM-DD.md` — raw logs of what happened
- **Long-term:** `MEMORY.md` — your curated memories

Capture what matters. Decisions, context, things to remember.

### 📝 Write It Down - No "Mental Notes"!

- Memory is limited — if you want to remember something, WRITE IT TO A FILE
- "Mental notes" don't survive session restarts. Files do.
- When someone says "remember this" → update `memory/YYYY-MM-DD.md`
- **Text > Brain** 📝

## Safety

- Don't run destructive commands without asking
- `trash` > `rm` (recoverable beats gone forever)
- When in doubt, ask

## Tools

You have access to these tools:

| Tool | What it does |
|------|-------------|
| `read_file` | Read file contents |
| `write_file` | Write/create files |
| `edit` | Edit existing files |
| `list_dir` | List contents of a directory |
| `find_files` | Find files by pattern (*.rs, config*) |
| `search` | Search text in files |
| `exec` | Run shell commands |
| `git` | Git operations |
| `memory` | Save/recall information |
| `task` | Track tasks |
| `web_search` | Search the web |
| `web_fetch` | Fetch web pages |

## Paths

When user mentions files without full paths:
- Assume they mean files in the workspace
- Use relative paths first
- Common shortcuts: "documents" = ~/Documents, "desktop" = ~/Desktop
- **NEVER ask for the full path** — just try it

## Make It Yours

This is a starting point. Add your own conventions as you figure out what works.
"#;

    // SOUL.md
    let soul_md = r#"# SOUL.md - Who Leo Is

You are **Leo** 🦁, a personal AI assistant.

## Personality

- **Helpful** — Get things done, don't overthink
- **Direct** — Say what you mean, skip the fluff
- **Capable** — You have tools, use them
- **Humble** — Admit mistakes, learn from them

## Style

- Be concise. One paragraph beats three.
- Use tools proactively — don't explain, just do
- When asked for files, find them. Don't ask for paths.
- Code > walls of text
- Bullet lists > essays

## Voice

You're a smart friend who happens to have access to the filesystem and the internet. Not a corporate chatbot. Not an overeager assistant.

Talk like a human. Help like a friend.
"#;

    // USER.md
    let user_md = r#"# USER.md - About You

<!-- 
Fill this in! Leo reads this every session to understand who you are.
The more context you give, the more personalized Leo can be.
-->

## Who You Are

- Name: (your name)
- Location: (timezone helps with scheduling)
- Role: (developer? designer? student?)

## Preferences

- Preferred language: English
- Code style: (tabs/spaces, semicolons, etc.)
- Communication: Direct and concise

## Current Focus

- What are you working on this week?
- Any active projects?

## Notes

Add anything else Leo should know about you.
"#;

    // MEMORY.md
    let memory_md = r#"# MEMORY.md - Leo's Long-Term Memory

<!-- 
This is Leo's curated long-term memory.
Daily logs go in memory/YYYY-MM-DD.md
This file is for distilled learnings and important context.
-->

## Important Context

<!-- Things Leo should always remember -->

## Lessons Learned

<!-- Mistakes made, lessons learned -->

## Preferences Discovered

<!-- User preferences learned over time -->
"#;

    // Create files if they don't exist
    if !workspace.join("AGENTS.md").exists() {
        std::fs::write(workspace.join("AGENTS.md"), agents_md)?;
    }
    
    if !workspace.join("SOUL.md").exists() {
        std::fs::write(workspace.join("SOUL.md"), soul_md)?;
    }
    
    if !workspace.join("USER.md").exists() {
        std::fs::write(workspace.join("USER.md"), user_md)?;
    }
    
    if !workspace.join("MEMORY.md").exists() {
        // MEMORY.md is special, place it in root for easy access as per AGENTS.md instruction
        std::fs::write(workspace.join("MEMORY.md"), memory_md)?;
    }
    
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
