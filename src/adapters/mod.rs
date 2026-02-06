//! Adapters module — chat platform integrations.
//!
//! This module provides channel adapters for different chat platforms.
//! Each adapter implements the [`Channel`] trait for uniform handling.
//!
//! # Supported Channels
//!
//! - **CLI** — Interactive command line interface
//! - **Telegram** — Telegram Bot API via teloxide
//!
//! # Adding a New Channel
//!
//! 1. Create a new file (e.g., `slack.rs`)
//! 2. Implement the [`Channel`] trait
//! 3. Add to [`ChannelRegistry`]

pub mod cli;
pub mod telegram;

use crate::config::Config;

/// Channel trait for chat adapters.
///
/// All channel implementations must be [`Send`] + [`Sync`] for async compatibility.
pub trait Channel: Send + Sync {
    /// Channel name (e.g., "telegram", "cli").
    fn name(&self) -> &str;

    /// Start listening for messages.
    fn start(&self) -> impl std::future::Future<Output = crate::Result<()>> + Send;

    /// Stop the channel.
    fn stop(&self) -> impl std::future::Future<Output = crate::Result<()>> + Send;
}

/// Channel registry — metadata about available channels.
///
/// # Example
///
/// ```ignore
/// for name in ChannelRegistry::available() {
///     if ChannelRegistry::is_enabled(name, &config) {
///         println!("{} is enabled", name);
///     }
/// }
/// ```
pub struct ChannelRegistry;

impl ChannelRegistry {
    /// List all available channel names.
    pub fn available() -> &'static [&'static str] {
        &["cli", "telegram"]
    }

    /// Check if a channel is enabled in the config.
    pub fn is_enabled(name: &str, config: &Config) -> bool {
        match name {
            "cli" => true, // CLI is always available
            "telegram" => config.telegram.enabled,
            _ => false,
        }
    }

    /// Get a human-readable description of a channel.
    pub fn description(name: &str) -> &'static str {
        match name {
            "cli" => "Interactive command line interface",
            "telegram" => "Telegram Bot API",
            _ => "Unknown channel",
        }
    }
}
