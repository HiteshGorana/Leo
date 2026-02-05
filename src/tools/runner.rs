//! Tool runner - manages and executes tools

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::Result;
use crate::error::Error;
use super::Tool;
use super::filesystem::{ReadFileTool, WriteFileTool, ListDirTool};
use super::edit::EditTool;
use super::search::SearchTool;
use super::git::GitTool;
use super::memory::MemoryTool;
use super::task::TaskTool;
use super::shell::ExecTool;
use super::web::{WebSearchTool, WebFetchTool};

/// Tool definition for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Tool runner manages registered tools and executes them
pub struct ToolRunner {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRunner {
    /// Create an empty tool runner
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    /// Create a tool runner with default tools
    pub fn new_with_defaults(workspace: &PathBuf) -> Self {
        let mut runner = Self::new();
        
        // File tools
        runner.register(ReadFileTool);
        runner.register(WriteFileTool);
        runner.register(ListDirTool);
        runner.register(EditTool);
        runner.register(SearchTool::new(workspace.clone()));
        
        // Shell & Git tools
        runner.register(ExecTool::new(workspace.clone()));
        runner.register(GitTool::new(workspace.clone()));
        
        // Memory & Task tools
        runner.register(MemoryTool::new(workspace.clone()));
        runner.register(TaskTool::new(workspace.clone()));
        
        // Create Browser Bridge (Extension) instance first to share it
        let browser = super::browser_bridge::BrowserBridgeTool::new();

        // Web tools (now with browser support)
        runner.register(WebSearchTool::new(Some(browser.clone())));
        runner.register(WebFetchTool::new(Some(browser.clone())));
        
        // Browser Bridge (registered as its own tool too)
        runner.register(browser);
        
        runner
    }
    
    /// Register a tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }
    
    /// Get tool definitions for LLM
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values()
            .map(|t| t.to_definition())
            .collect()
    }
    
    /// Execute a tool by name
    pub async fn execute(&self, name: &str, params: Value) -> Result<String> {
        let tool = self.tools.get(name)
            .ok_or_else(|| Error::Tool(format!("Unknown tool: {}", name)))?;
        
        tool.execute(params).await
    }
    
    /// Check if a tool exists
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
    
    /// List registered tool names
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ToolRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::DummyTool;
    
    #[tokio::test]
    async fn test_tool_runner_register_and_execute() {
        let mut runner = ToolRunner::new();
        runner.register(DummyTool {
            name: "test_tool".to_string(),
            result: "success".to_string(),
        });
        
        assert!(runner.has("test_tool"));
        
        let result = runner.execute("test_tool", serde_json::json!({})).await.unwrap();
        assert_eq!(result, "success");
    }
    
    #[tokio::test]
    async fn test_tool_runner_unknown_tool() {
        let runner = ToolRunner::new();
        let result = runner.execute("unknown", serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
