//! Find files tool - glob-based file finder

use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde_json::{json, Value};
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Find files by name pattern
pub struct FindFilesTool {
    workspace: PathBuf,
}

impl FindFilesTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
    
    fn matches_glob(name: &str, pattern: &str) -> bool {
        // Simple glob matching: * matches any sequence, ? matches single char
        let pattern = pattern.to_lowercase();
        let name = name.to_lowercase();
        
        if pattern == "*" {
            return true;
        }
        
        // Handle *.ext pattern
        if pattern.starts_with("*.") {
            let ext = &pattern[2..];
            return name.ends_with(&format!(".{}", ext));
        }
        
        // Handle prefix* pattern
        if pattern.ends_with('*') && !pattern.contains('?') {
            let prefix = &pattern[..pattern.len()-1];
            return name.starts_with(prefix);
        }
        
        // Handle *suffix pattern  
        if pattern.starts_with('*') && !pattern.contains('?') {
            let suffix = &pattern[1..];
            return name.ends_with(suffix);
        }
        
        // Exact match
        name == pattern
    }
    
    fn find_recursive(
        &self,
        dir: &Path,
        pattern: &str,
        file_type: &str,
        results: &mut Vec<String>,
        current_depth: usize,
        max_depth: usize,
    ) -> std::io::Result<()> {
        if current_depth > max_depth || !dir.exists() || !dir.is_dir() {
            return Ok(());
        }
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            // Skip hidden files/dirs
            if name.starts_with('.') {
                continue;
            }
            
            let is_dir = path.is_dir();
            let is_file = path.is_file();
            
            // Check if matches pattern and type
            let type_matches = match file_type {
                "file" => is_file,
                "dir" => is_dir,
                _ => true, // "all"
            };
            
            if type_matches && Self::matches_glob(name, pattern) {
                let relative = path.strip_prefix(&self.workspace)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                    
                if is_dir {
                    results.push(format!("{}/", relative));
                } else {
                    // Include file size
                    let size = std::fs::metadata(&path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    results.push(format!("{} ({})", relative, format_size(size)));
                }
            }
            
            // Recurse into directories
            if is_dir {
                self.find_recursive(&path, pattern, file_type, results, current_depth + 1, max_depth)?;
            }
        }
        
        Ok(())
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[async_trait]
impl Tool for FindFilesTool {
    fn name(&self) -> &str { "find_files" }
    fn description(&self) -> &str { 
        "Find files by name pattern (glob). Supports *.ext, prefix*, *suffix patterns." 
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g., '*.rs', 'config*', '*test*')"
                },
                "path": {
                    "type": "string",
                    "description": "Subdirectory to search in (optional, defaults to workspace)"
                },
                "type": {
                    "type": "string",
                    "enum": ["file", "dir", "all"],
                    "description": "Type of entries to find (optional, defaults to 'all')"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth to search (optional, defaults to 10)"
                }
            },
            "required": ["pattern"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let pattern = params.get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'pattern' parameter".to_string()))?;
        
        let sub_path = params.get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        
        let file_type = params.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        
        let max_depth = params.get("max_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        
        let search_path = if let Some(p) = sub_path {
            self.workspace.join(p)
        } else {
            self.workspace.clone()
        };
        
        if !search_path.exists() {
            return Err(Error::Tool(format!("Path does not exist: {:?}", search_path)));
        }
        
        let mut results = Vec::new();
        self.find_recursive(&search_path, pattern, file_type, &mut results, 0, max_depth)
            .map_err(|e| Error::Tool(format!("Find failed: {}", e)))?;
        
        if results.is_empty() {
            Ok(format!("No files matching '{}' found.", pattern))
        } else {
            // Sort and limit results
            results.sort();
            let total = results.len();
            if total > 50 {
                Ok(format!(
                    "Found {} files matching '{}':\n\n{}...\n\n(showing first 50)",
                    total, pattern,
                    results[..50].join("\n")
                ))
            } else {
                Ok(format!(
                    "Found {} files matching '{}':\n\n{}",
                    total, pattern,
                    results.join("\n")
                ))
            }
        }
    }
}
