//! LLM client abstraction layer.
//!
//! This module provides:
//! - [`LlmClient`] trait for swappable LLM providers
//! - [`ProviderRegistry`] for dynamic provider creation
//! - Concrete implementations: Gemini API key, Gemini OAuth
//!
//! # Adding a New Provider
//!
//! 1. Create a new file (e.g., `openai.rs`)
//! 2. Implement `LlmClient` trait
//! 3. Add to `ProviderRegistry::create()`
//! 4. Add config fields in `config.rs`

mod types;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::Error;
use crate::tools::ToolDefinition;
use crate::Result;

pub use types::*;

// Re-export concrete implementations
pub mod gemini;
pub mod gemini_oauth;

pub use gemini::GeminiClient;
pub use gemini_oauth::GeminiOAuthClient;

use super::message::{Message, ToolCallRequest};

/// Response from an LLM provider.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Text content of the response.
    pub content: Option<String>,

    /// Tool calls requested by the LLM.
    pub tool_calls: Vec<ToolCallRequest>,

    /// Reason the response finished.
    pub finish_reason: String,

    /// Token usage statistics.
    pub usage: Usage,
}

impl LlmResponse {
    /// Create a simple text response.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
            usage: Usage::default(),
        }
    }

    /// Check if response has tool calls.
    #[inline]
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Token usage information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// LLM client trait — swappable provider abstraction.
///
/// Implement this trait to add a new LLM provider.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send messages and get response.
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse>;

    /// Get the default model for this provider.
    fn default_model(&self) -> &str;
}

/// Provider registry — creates LLM clients dynamically.
///
/// # Example
///
/// ```ignore
/// let client = ProviderRegistry::create(&config)?;
/// let response = client.chat(&messages, &tools).await?;
/// ```
pub struct ProviderRegistry;

impl ProviderRegistry {
    /// Create an LLM client from configuration.
    ///
    /// Supported providers:
    /// - `"gemini"`: Gemini API with API key authentication
    /// - `"google-cli"`: Gemini with OAuth (uses Gemini CLI credentials)
    pub fn create(config: &Config) -> Result<Box<dyn LlmClient>> {
        match config.provider.as_str() {
            "gemini" => {
                let client = GeminiClient::new(&config.gemini_api_key, &config.model);
                Ok(Box::new(client))
            }
            "google-cli" => {
                let client = GeminiOAuthClient::from_cli(&config.model)?;
                Ok(Box::new(client))
            }
            other => Err(Error::Config(format!("Unknown provider: {other}"))),
        }
    }

    /// List available provider names.
    pub fn available() -> &'static [&'static str] {
        &["gemini", "google-cli"]
    }
}

/// Fake LLM client for testing.
#[cfg(test)]
pub struct FakeLlmClient {
    responses: std::sync::Mutex<std::collections::VecDeque<LlmResponse>>,
}

#[cfg(test)]
impl FakeLlmClient {
    /// Create with predefined text responses.
    pub fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: std::sync::Mutex::new(
                responses.iter().map(|s| LlmResponse::text(*s)).collect(),
            ),
        }
    }

    /// Create with a single tool call followed by a text response.
    pub fn with_tool_call(name: &str, args: serde_json::Value, final_response: &str) -> Self {
        let tool_response = LlmResponse {
            content: None,
            tool_calls: vec![ToolCallRequest {
                id: "tc_1".to_string(),
                name: name.to_string(),
                arguments: args,
            }],
            finish_reason: "tool_calls".to_string(),
            usage: Usage::default(),
        };

        Self {
            responses: std::sync::Mutex::new(
                vec![tool_response, LlmResponse::text(final_response)].into(),
            ),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl LlmClient for FakeLlmClient {
    async fn chat(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        let mut responses = self.responses.lock().unwrap();
        responses
            .pop_front()
            .ok_or_else(|| Error::Llm("No more fake responses".to_string()))
    }

    fn default_model(&self) -> &str {
        "fake-model"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_llm_client() {
        let client = FakeLlmClient::new(vec!["Hello!", "World!"]);

        let resp1 = client.chat(&[], &[]).await.unwrap();
        assert_eq!(resp1.content.as_deref(), Some("Hello!"));

        let resp2 = client.chat(&[], &[]).await.unwrap();
        assert_eq!(resp2.content.as_deref(), Some("World!"));
    }
}
