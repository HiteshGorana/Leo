//! Web tools - search and fetch

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::Result;
use crate::error::Error;
use super::Tool;

/// Web search (placeholder - requires API key)
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str { "Search the web for information" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'query' parameter".to_string()))?;
        
        // Placeholder - in production, use a search API (Brave, Google, etc.)
        Ok(format!(
            "Web search is not yet configured.\n\
             Query: {}\n\n\
             To enable web search, configure a search API key in your config.",
            query
        ))
    }
}

/// Fetch web page content
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str { "web_fetch" }
    fn description(&self) -> &str { "Fetch content from a URL" }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                }
            },
            "required": ["url"]
        })
    }
    
    async fn execute(&self, params: Value) -> Result<String> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Tool("Missing 'url' parameter".to_string()))?;
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Tool(format!("Failed to create HTTP client: {}", e)))?;
        
        let response = client.get(url)
            .header("User-Agent", "leo/1.0")
            .send()
            .await
            .map_err(|e| Error::Tool(format!("Failed to fetch {}: {}", url, e)))?;
        
        let status = response.status();
        if !status.is_success() {
            return Err(Error::Tool(format!("HTTP error: {}", status)));
        }
        
        let text = response.text().await
            .map_err(|e| Error::Tool(format!("Failed to read response: {}", e)))?;
        
        // Basic HTML to text conversion (very simplified)
        let clean = html_to_text(&text);
        
        // Truncate if too long
        if clean.len() > 10000 {
            Ok(format!("{}...\n\n[Truncated - {} total chars]", &clean[..10000], clean.len()))
        } else {
            Ok(clean)
        }
    }
}

/// Very basic HTML to text conversion
fn html_to_text(html: &str) -> String {
    // Remove script/style tags and their content
    let mut text = html.to_string();
    
    // Remove scripts
    while let Some(start) = text.find("<script") {
        if let Some(end) = text[start..].find("</script>") {
            text = format!("{}{}", &text[..start], &text[start + end + 9..]);
        } else {
            break;
        }
    }
    
    // Remove styles
    while let Some(start) = text.find("<style") {
        if let Some(end) = text[start..].find("</style>") {
            text = format!("{}{}", &text[..start], &text[start + end + 8..]);
        } else {
            break;
        }
    }
    
    // Remove HTML tags
    let mut result = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    
    // Collapse whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_html_to_text() {
        let html = "<html><head><title>Test</title></head><body><p>Hello World</p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Hello World"));
    }
    
    #[test]
    fn test_html_to_text_removes_scripts() {
        let html = "<body><script>alert('hi');</script><p>Content</p></body>";
        let text = html_to_text(html);
        assert!(text.contains("Content"));
        assert!(!text.contains("alert"));
    }
}
