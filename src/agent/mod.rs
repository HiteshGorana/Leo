//! Agent module — core agent logic.
//!
//! This module contains:
//! - Message types (InboundMessage, Response)
//! - LLM client trait and implementations
//! - Agent loop for processing messages
//! - Context builder for prompts
//!
//! # Adding a New LLM Provider
//!
//! See [`llm::ProviderRegistry`] for instructions.

mod context;
mod loop_impl;
mod message;
pub mod tokens;

// LLM providers in submodule
pub mod llm;

// Re-exports for convenience
pub use context::Context;
pub use llm::{GeminiClient, GeminiOAuthClient, LlmClient, LlmResponse, ProviderRegistry, Usage};
pub use loop_impl::AgentLoop;
pub use message::{InboundMessage, Message, Response, Role, ToolCall, ToolCallRequest};
