//! Tools module - agent capabilities
//!
//! Tools are external actions the agent can take, such as reading files,
//! executing commands, and searching the web.

mod runner;
mod filesystem;
mod shell;
mod web;
mod search;
mod edit;
mod git;
mod memory;
mod task;

pub use runner::{ToolRunner, ToolDefinition};

use async_trait::async_trait;
use serde_json::Value;
use crate::Result;

/// Tool trait - interface for all agent tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name used in function calls
    fn name(&self) -> &str;
    
    /// Description of what the tool does
    fn description(&self) -> &str;
    
    /// JSON Schema for parameters
    fn parameters(&self) -> Value;
    
    /// Execute the tool with given parameters
    async fn execute(&self, params: Value) -> Result<String>;
    
    /// Convert to tool definition for LLM
    fn to_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }
}

/// Dummy tool for testing
pub struct DummyTool {
    pub name: String,
    pub result: String,
}

#[async_trait]
impl Tool for DummyTool {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { "Dummy tool for testing" }
    fn parameters(&self) -> Value { serde_json::json!({"type": "object"}) }
    
    async fn execute(&self, _params: Value) -> Result<String> {
        Ok(self.result.clone())
    }
}
