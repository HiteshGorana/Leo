//! Adapters module - chat platform integrations
//!
//! Currently supports:
//! - Telegram (via teloxide)
//! - CLI (direct interaction)

// Telegram adapter
pub mod telegram;

/// Channel trait for chat adapters
pub trait Channel: Send + Sync {
    /// Channel name (e.g., "telegram", "cli")
    fn name(&self) -> &str;
    
    /// Start listening for messages
    fn start(&self) -> impl std::future::Future<Output = crate::Result<()>> + Send;
    
    /// Stop the channel
    fn stop(&self) -> impl std::future::Future<Output = crate::Result<()>> + Send;
}
