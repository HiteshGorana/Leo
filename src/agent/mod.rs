//! Agent module - core agent logic
//!
//! This module contains:
//! - Message types (InboundMessage, Response)
//! - LLM client trait and implementations
//! - Agent loop for processing messages
//! - Context builder for prompts

mod message;
mod llm;
mod context;
mod loop_impl;
pub mod gemini;
pub mod gemini_oauth;

pub use message::{Message, Role, Response, ToolCall, ToolCallRequest, InboundMessage};
pub use llm::{LlmClient, LlmResponse};
pub use context::Context;
pub use loop_impl::AgentLoop;
pub use gemini_oauth::GeminiOAuthClient;
