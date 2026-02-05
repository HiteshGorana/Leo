//! Edit tool - replace content in files

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Edit file content (replace string)
pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str { "edit_file" }
    fn description(&self) -> &str { "Replace text in a file" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "target": {
                    "type": "string",
                    "description": "Exact text to replace"
                },
                "replacement": {
                    "type": "string",
                    "description": "New text to insert"
                }
            },
            "required": ["path", "target", "replacement"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'path' parameter".to_string()))?;
            
        let target = params.get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'target' parameter".to_string()))?;
            
        let replacement = params.get("replacement")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'replacement' parameter".to_string()))?;
            
        // Read file
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Tool(format!("Failed to read {}: {}", path, e)))?;
            
        // Check if target exists
        if !content.contains(target) {
            return Err(Error::Tool(format!(
                "Target text not found in file. Make sure it matches exactly (including whitespace).\nTarget: '{}'", 
                target
            )));
        }
        
        // Count occurrences
        let count = content.matches(target).count();
        if count > 1 {
            // For safety, might want to error if multiple matches, 
            // but for now let's just replace all and warn in output
            // Or ideally, use line numbers, but simple string replace is generic
        }
        
        // Replace
        let new_content = content.replace(target, replacement);
        
        // Write back
        std::fs::write(path, &new_content)
            .map_err(|e| Error::Tool(format!("Failed to write {}: {}", path, e)))?;
            
        Ok(format!("Successfully replaced {} occurrence(s) in {}", count, path))
    }
}
