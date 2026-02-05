//! Task tool - manage task list

use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::{json, Value};
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Manage task list
pub struct TaskTool {
    task_path: PathBuf,
}

impl TaskTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            task_path: workspace.join("task.md"),
        }
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str { "task" }
    fn description(&self) -> &str { "Read or Update the task list" }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "update"],
                    "description": "Action to perform"
                },
                "content": {
                    "type": "string",
                    "description": "New content for the task list (required for 'update')"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let action = params.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'action' parameter".to_string()))?;

        match action {
            "read" => {
                if self.task_path.exists() {
                    std::fs::read_to_string(&self.task_path)
                        .map_err(|e| Error::Tool(format!("Failed to read task list: {}", e)))
                } else {
                    Ok("Task list is empty (no task.md found).".to_string())
                }
            },
            "update" => {
                let content = params.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Tool("Missing 'content' parameter for update action".to_string()))?;

                std::fs::write(&self.task_path, content)
                    .map_err(|e| Error::Tool(format!("Failed to write task list: {}", e)))?;

                Ok("Successfully updated task list.".to_string())
            },
            _ => Err(Error::Tool(format!("Unknown action: {}", action)))
        }
    }
}
