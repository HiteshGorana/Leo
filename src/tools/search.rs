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
    fn description(&self) -> &str { "Search for text in files (supports regex or literal text, case-sensitive or insensitive)" }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Text or regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Sub-directory to search in (optional, defaults to workspace root)"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Ignore case when matching (default: false)"
                },
                "literal": {
                    "type": "boolean",
                    "description": "Treat query as literal text, not regex (default: false)"
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
        
        let case_insensitive = params.get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let literal = params.get("literal")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let search_path = if let Some(p) = sub_path {
            self.workspace.join(p)
        } else {
            self.workspace.clone()
        };

        if !search_path.exists() {
            return Err(Error::Tool(format!("Path does not exist: {:?}", search_path)));
        }

        // Build regex pattern
        let pattern_str = if literal {
            regex::escape(query)
        } else {
            query.to_string()
        };
        
        let pattern_str = if case_insensitive {
            format!("(?i){}", pattern_str)
        } else {
            pattern_str
        };
        
        let pattern = Regex::new(&pattern_str)
            .map_err(|e| Error::Tool(format!("Invalid regex: {}", e)))?;

        let mut results = Vec::new();
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
