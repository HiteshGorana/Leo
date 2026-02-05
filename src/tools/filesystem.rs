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

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str { "list_dir" }
    fn description(&self) -> &str { "List contents of a directory" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'path' parameter".to_string()))?;
        
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
