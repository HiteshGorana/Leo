//! LLM client trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::Result;
use super::message::{Message, ToolCallRequest};
use crate::tools::ToolDefinition;

/// Response from an LLM provider
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Text content of the response
    pub content: Option<String>,
    
    /// Tool calls requested by the LLM
    pub tool_calls: Vec<ToolCallRequest>,
    
    /// Reason the response finished
    pub finish_reason: String,
    
    /// Token usage
    pub usage: Usage,
}

impl LlmResponse {
    /// Create a simple text response
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
            usage: Usage::default(),
        }
    }
    
    /// Check if response has tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// LLM client trait - swappable provider abstraction
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send messages and get response
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse>;
    
    /// Get the default model for this provider
    fn default_model(&self) -> &str;
}

/// Fake LLM client for testing
pub struct FakeLlmClient {
    responses: std::sync::Mutex<std::collections::VecDeque<LlmResponse>>,
}

impl FakeLlmClient {
    /// Create with predefined text responses
    pub fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: std::sync::Mutex::new(
                responses.iter()
                    .map(|s| LlmResponse::text(*s))
                    .collect()
            ),
        }
    }
    
    /// Create with a single tool call then a text response
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
            responses: std::sync::Mutex::new(vec![
                tool_response,
                LlmResponse::text(final_response),
            ].into()),
        }
    }
}

#[async_trait]
impl LlmClient for FakeLlmClient {
    async fn chat(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        let mut responses = self.responses.lock().unwrap();
        responses.pop_front()
            .ok_or_else(|| crate::error::Error::Llm("No more fake responses".to_string()))
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
