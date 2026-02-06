//! Filesystem tools - read, write, and list files

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Read file contents
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }
    fn description(&self) -> &str { "Read the contents of a file at the specified path" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'path' parameter".to_string()))?;
        
        std::fs::read_to_string(path)
            .map_err(|e| Error::Tool(format!("Failed to read {}: {}", path, e)))
    }
}

/// Write content to a file
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }
    fn description(&self) -> &str { "Write content to a file at the specified path" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'path' parameter".to_string()))?;
        
        let content = params.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'content' parameter".to_string()))?;
        
        // Create parent directories if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Tool(format!("Failed to create directory: {}", e)))?;
        }
        
        std::fs::write(path, content)
            .map_err(|e| Error::Tool(format!("Failed to write {}: {}", path, e)))?;
        
        Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
    }
}

/// List directory contents
pub struct ListDirTool;

impl ListDirTool {
    fn list_recursive(
        path: &std::path::Path,
        prefix: &str,
        results: &mut Vec<String>,
        current_depth: usize,
        max_depth: usize,
        show_size: bool,
    ) -> std::io::Result<()> {
        if current_depth > max_depth {
            return Ok(());
        }
        
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .collect();
        
        // Sort by name
        entries.sort_by_key(|a| a.file_name());
        
        for entry in entries {
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files
            if name.starts_with('.') {
                continue;
            }
            
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            
            if is_dir {
                results.push(format!("{}{}/", prefix, name));
                if current_depth < max_depth {
                    let new_prefix = format!("{}  ", prefix);
                    Self::list_recursive(&entry.path(), &new_prefix, results, current_depth + 1, max_depth, show_size)?;
                }
            } else {
                if show_size {
                    let size = std::fs::metadata(entry.path())
                        .map(|m| m.len())
                        .unwrap_or(0);
                    let size_str = if size < 1024 {
                        format!("{} B", size)
                    } else if size < 1024 * 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                    };
                    results.push(format!("{}{} ({})", prefix, name, size_str));
                } else {
                    results.push(format!("{}{}", prefix, name));
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str { "list_dir" }
    fn description(&self) -> &str { "List contents of a directory, optionally recursive with file sizes" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "List recursively (default: false)"
                },
                "show_size": {
                    "type": "boolean", 
                    "description": "Show file sizes (default: false)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Max depth for recursive listing (default: 3)"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'path' parameter".to_string()))?;
        
        let recursive = params.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let show_size = params.get("show_size")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let max_depth = params.get("max_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;
        
        let dir_path = std::path::Path::new(path);
        
        if !dir_path.exists() {
            return Err(Error::Tool(format!("Directory not found: {}", path)));
        }
        
        if recursive {
            let mut results = Vec::new();
            Self::list_recursive(dir_path, "", &mut results, 0, max_depth, show_size)
                .map_err(|e| Error::Tool(format!("Failed to list {}: {}", path, e)))?;
            
            if results.is_empty() {
                Ok("Directory is empty.".to_string())
            } else {
                Ok(results.join("\n"))
            }
        } else {
            // Original non-recursive behavior
            let entries: Vec<String> = std::fs::read_dir(path)
                .map_err(|e| Error::Tool(format!("Failed to read directory {}: {}", path, e)))?
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    if is_dir { format!("{}/", name) } else { name }
                })
                .collect();
            
            Ok(entries.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_read_write_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.txt");
        
        // Write
        let write_result = WriteFileTool.execute(json!({
            "path": file_path.to_str().unwrap(),
            "content": "Hello, World!"
        })).await.unwrap();
        assert!(write_result.contains("Successfully wrote"));
        
        // Read
        let read_result = ReadFileTool.execute(json!({
            "path": file_path.to_str().unwrap()
        })).await.unwrap();
        assert_eq!(read_result, "Hello, World!");
    }
    
    #[tokio::test]
    async fn test_list_dir() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "").unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();
        
        let result = ListDirTool.execute(json!({
            "path": tmp.path().to_str().unwrap()
        })).await.unwrap();
        
        assert!(result.contains("a.txt"));
        assert!(result.contains("b.txt"));
        assert!(result.contains("subdir/"));
    }
}
