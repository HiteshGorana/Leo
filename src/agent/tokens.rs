//! Token counting and budget management.
//!
//! Provides utilities for estimating token usage and enforcing budgets.
//! Uses a simple heuristic: ~4 characters per token (Gemini/GPT average).

use tracing::debug;

/// Characters per token estimate (conservative for Gemini/GPT models).
const CHARS_PER_TOKEN: usize = 4;

/// Default budget limits (in tokens).
pub mod defaults {
    /// Max tokens for system prompt (identity + bootstrap + memory + skills).
    pub const SYSTEM_PROMPT_BUDGET: usize = 4000;
    /// Max tokens for tool definitions.
    pub const TOOLS_BUDGET: usize = 2000;
    /// Max tokens for conversation history.
    pub const HISTORY_BUDGET: usize = 8000;
    /// Max tokens for a single message.
    pub const MESSAGE_BUDGET: usize = 4000;
    /// Total context budget (model dependent).
    pub const TOTAL_BUDGET: usize = 32000;
}

/// Token budget configuration.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub system_prompt: usize,
    pub tools: usize,
    pub history: usize,
    pub message: usize,
    pub total: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            system_prompt: defaults::SYSTEM_PROMPT_BUDGET,
            tools: defaults::TOOLS_BUDGET,
            history: defaults::HISTORY_BUDGET,
            message: defaults::MESSAGE_BUDGET,
            total: defaults::TOTAL_BUDGET,
        }
    }
}

impl TokenBudget {
    /// Create a budget for smaller models (e.g., 8k context).
    pub fn small() -> Self {
        Self {
            system_prompt: 1500,
            tools: 1000,
            history: 3000,
            message: 1500,
            total: 8000,
        }
    }

    /// Create a budget for larger models (e.g., 128k context).
    pub fn large() -> Self {
        Self {
            system_prompt: 8000,
            tools: 4000,
            history: 32000,
            message: 8000,
            total: 128000,
        }
    }
}

/// Estimate token count for a string.
#[inline]
pub fn estimate_tokens(text: &str) -> usize {
    // Simple heuristic: ~4 chars per token
    // More accurate would use tiktoken, but this is fast and good enough
    (text.len() + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN
}

/// Token usage statistics for a request.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub system_prompt: usize,
    pub tools: usize,
    pub history: usize,
    pub current_message: usize,
    pub total_input: usize,
    pub completion: usize,
}

impl TokenUsage {
    /// Create usage from component parts.
    pub fn new(system_prompt: usize, tools: usize, history: usize, current_message: usize) -> Self {
        let total_input = system_prompt + tools + history + current_message;
        Self {
            system_prompt,
            tools,
            history,
            current_message,
            total_input,
            completion: 0,
        }
    }

    /// Add completion tokens.
    pub fn with_completion(mut self, completion: usize) -> Self {
        self.completion = completion;
        self
    }

    /// Total tokens (input + completion).
    pub fn total(&self) -> usize {
        self.total_input + self.completion
    }

    /// Log usage at debug level.
    pub fn log(&self) {
        debug!(
            "Token usage: system={}, tools={}, history={}, msg={}, total_in={}, completion={}",
            self.system_prompt,
            self.tools,
            self.history,
            self.current_message,
            self.total_input,
            self.completion
        );
    }

    /// Format as a compact summary string.
    pub fn summary(&self) -> String {
        format!(
            "{}↓ {}↑ (sys:{} tools:{} hist:{} msg:{})",
            self.total_input,
            self.completion,
            self.system_prompt,
            self.tools,
            self.history,
            self.current_message
        )
    }
}

/// Truncate text to fit within a token budget.
pub fn truncate_to_budget(text: &str, max_tokens: usize) -> &str {
    let max_chars = max_tokens * CHARS_PER_TOKEN;
    if text.len() <= max_chars {
        text
    } else {
        // Find a safe UTF-8 boundary
        let mut end = max_chars.min(text.len());
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    }
}

/// Truncate history messages to fit within a token budget.
/// Returns a slice of the most recent messages that fit.
pub fn truncate_history<'a, T: AsRef<str>>(
    messages: &'a [T],
    max_tokens: usize,
) -> &'a [T] {
    let mut total = 0;
    let mut start_idx = messages.len();

    // Work backwards from most recent
    for (i, msg) in messages.iter().enumerate().rev() {
        let msg_tokens = estimate_tokens(msg.as_ref());
        if total + msg_tokens > max_tokens {
            break;
        }
        total += msg_tokens;
        start_idx = i;
    }

    &messages[start_idx..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("Hi"), 1); // 2 chars → 1 token
        assert_eq!(estimate_tokens("Hello"), 2); // 5 chars → 2 tokens
        assert_eq!(estimate_tokens("Hello, world!"), 4); // 13 chars → 4 tokens
    }

    #[test]
    fn test_token_usage_summary() {
        let usage = TokenUsage::new(1000, 500, 2000, 100).with_completion(500);
        let summary = usage.summary();
        assert!(summary.contains("3600↓"));
        assert!(summary.contains("500↑"));
    }

    #[test]
    fn test_truncate_to_budget() {
        let text = "Hello, world! This is a test.";
        
        // Plenty of budget
        assert_eq!(truncate_to_budget(text, 100), text);
        
        // Limited budget (2 tokens = 8 chars)
        let truncated = truncate_to_budget(text, 2);
        assert!(truncated.len() <= 8);
    }

    #[test]
    fn test_truncate_history() {
        let messages = vec![
            "First message",
            "Second message",
            "Third message",
            "Fourth message",
        ];
        
        // Plenty of budget
        let result = truncate_history(&messages, 1000);
        assert_eq!(result.len(), 4);
        
        // Limited budget - should keep most recent
        let result = truncate_history(&messages, 10);
        assert!(result.len() < 4);
        assert_eq!(result.last(), Some(&"Fourth message"));
    }

    #[test]
    fn test_default_budget() {
        let budget = TokenBudget::default();
        assert_eq!(budget.total, defaults::TOTAL_BUDGET);
    }
}
