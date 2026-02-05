//! Memory tool - managing long-term memory

use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::{json, Value};
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Manage long-term memory
pub struct MemoryTool {
    memory_path: PathBuf,
}

impl MemoryTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            memory_path: workspace.join("memory").join("MEMORY.md"),
        }
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str { "memory" }
    fn description(&self) -> &str { "Read or Add to long-term memory" }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "add"],
                    "description": "Action to perform"
                },
                "content": {
                    "type": "string",
                    "description": "Content to add (required for 'add' action)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let action = params.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'action' parameter".to_string()))?;

        // Ensure parent dir exists
        if let Some(parent) = self.memory_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        match action {
            "read" => {
                if self.memory_path.exists() {
                    std::fs::read_to_string(&self.memory_path)
                        .map_err(|e| Error::Tool(format!("Failed to read memory: {}", e)))
                } else {
                    Ok("Memory is empty.".to_string())
                }
            },
            "add" => {
                let content = params.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Tool("Missing 'content' parameter for add action".to_string()))?;

                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
                let entry = format!("\n- [{}] {}\n", timestamp, content);

                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.memory_path)
                    .map_err(|e| Error::Tool(format!("Failed to open memory file: {}", e)))?;

                write!(file, "{}", entry)
                    .map_err(|e| Error::Tool(format!("Failed to write to memory: {}", e)))?;

                Ok("Successfully added to long-term memory.".to_string())
            },
            _ => Err(Error::Tool(format!("Unknown action: {}", action)))
        }
    }
}
