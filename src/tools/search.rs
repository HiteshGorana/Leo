//! Search tool - regex search in files

use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde_json::{json, Value};
use regex::Regex;
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Search for text in files
pub struct SearchTool {
    workspace: PathBuf,
}

impl SearchTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    fn search_recursive(&self, dir: &Path, pattern: &Regex, results: &mut Vec<String>) -> std::io::Result<()> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip hidden directories like .git, .leo
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }
                self.search_recursive(&path, pattern, results)?;
            } else if path.is_file() {
                // Try to read as text
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (i, line) in content.lines().enumerate() {
                        if pattern.is_match(line) {
                            // Format: path:line: content
                            let relative_path = path.strip_prefix(&self.workspace)
                                .unwrap_or(&path)
                                .display();
                            results.push(format!("{}:{}: {}", relative_path, i + 1, line.trim()));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str { "search" }
    fn description(&self) -> &str { "Search for a regex pattern in files within the workspace" }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Sub-directory to search in (optional, defaults to workspace root)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'query' parameter".to_string()))?;

        let sub_path = params.get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let search_path = if let Some(p) = sub_path {
            self.workspace.join(p)
        } else {
            self.workspace.clone()
        };

        if !search_path.exists() {
            return Err(Error::Tool(format!("Path does not exist: {:?}", search_path)));
        }

        let pattern = Regex::new(query)
            .map_err(|e| Error::Tool(format!("Invalid regex: {}", e)))?;

        let mut results = Vec::new();
        // Run blocking search on a separate thread (though for now standard fs is fine for small workspaces)
        // ideally we'd use tokio::task::spawn_blocking
        self.search_recursive(&search_path, &pattern, &mut results)
            .map_err(|e| Error::Tool(format!("Search failed: {}", e)))?;

        if results.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            // Cap results to avoid blowing up context
            if results.len() > 100 {
                let displayed = results.len().min(100);
                Ok(format!(
                    "Found {} matches (showing first {}):\n\n{}", 
                    results.len(), 
                    displayed,
                    results[..displayed].join("\n")
                ))
            } else {
                Ok(results.join("\n"))
            }
        }
    }
}
