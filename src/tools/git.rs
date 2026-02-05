//! Git tool - manage source control

use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Execute git commands
pub struct GitTool {
    workspace: PathBuf,
}

impl GitTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    async fn run_git(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.workspace)
            .output()
            .await
            .map_err(|e| Error::Tool(format!("Failed to execute git: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            if stdout.trim().is_empty() && !stderr.trim().is_empty() {
                Ok(stderr.to_string())
            } else {
                Ok(stdout.to_string())
            }
        } else {
            Err(Error::Tool(format!(
                "Git command failed: git {}\nError: {}",
                args.join(" "),
                stderr
            )))
        }
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str { "git" }
    fn description(&self) -> &str { "Run git commands (status, diff, commit, log, add)" }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["status", "diff", "commit", "log", "add"],
                    "description": "Git operation to perform"
                },
                "args": {
                    "type": "string",
                    "description": "Arguments for the operation (e.g., file paths, commit message)"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let op = params.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'operation' parameter".to_string()))?;

        let args_str = params.get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match op {
            "status" => self.run_git(&["status"]).await,
            "diff" => self.run_git(&["diff"]).await,
            "log" => self.run_git(&["log", "-n", "10", "--oneline"]).await,
            "add" => {
                let files: Vec<&str> = args_str.split_whitespace().collect();
                if files.is_empty() {
                    return Err(Error::Tool("No files specified for git add".to_string()));
                }
                let mut cmd_args = vec!["add"];
                cmd_args.extend(files);
                self.run_git(&cmd_args).await
            },
            "commit" => {
                if args_str.trim().is_empty() {
                    return Err(Error::Tool("Commit message required".to_string()));
                }
                self.run_git(&["commit", "-m", args_str]).await
            },
            _ => Err(Error::Tool(format!("Unsupported git operation: {}", op)))
        }
    }
}
