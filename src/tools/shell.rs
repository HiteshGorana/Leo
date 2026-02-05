//! Shell tool - execute commands

use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Execute shell commands
pub struct ExecTool {
    workspace: PathBuf,
}

impl ExecTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for ExecTool {
    fn name(&self) -> &str { "exec" }
    fn description(&self) -> &str { "Execute a shell command in the workspace" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory (optional, defaults to workspace)"
                }
            },
            "required": ["command"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let command = params.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'command' parameter".to_string()))?;
        
        let working_dir = params.get("working_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.workspace.clone());
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&working_dir)
            .output()
            .await
            .map_err(|e| Error::Tool(format!("Failed to execute command: {}", e)))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if output.status.success() {
            if stderr.is_empty() {
                Ok(stdout.to_string())
            } else {
                Ok(format!("{}\n\n[stderr]\n{}", stdout, stderr))
            }
        } else {
            Err(Error::Tool(format!(
                "Command failed with exit code {}\nstdout: {}\nstderr: {}",
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_exec_echo() {
        let tmp = TempDir::new().unwrap();
        let exec = ExecTool::new(tmp.path().to_path_buf());
        
        let result = exec.execute(json!({
            "command": "echo 'Hello, World!'"
        })).await.unwrap();
        
        assert!(result.contains("Hello, World!"));
    }
    
    #[tokio::test]
    async fn test_exec_failed_command() {
        let tmp = TempDir::new().unwrap();
        let exec = ExecTool::new(tmp.path().to_path_buf());
        
        let result = exec.execute(json!({
            "command": "exit 1"
        })).await;
        
        assert!(result.is_err());
    }
}
