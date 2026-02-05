//! Extract OAuth2 credentials from the Google Gemini CLI binary
//!
//! This module searches the system PATH for the `gemini` Node.js binary,
//! locates the bundled oauth2.js file, and extracts the client_id and client_secret.

use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use crate::Result;
use crate::error::Error;

/// OAuth2 client credentials extracted from the CLI
#[derive(Debug, Clone)]
pub struct CliCredentials {
    pub client_id: String,
    pub client_secret: String,
}

/// Extract OAuth2 credentials from the Gemini CLI binary
///
/// This function:
/// 1. Searches $PATH for the `gemini` binary
/// 2. Locates the oauth2.js file within the Node.js package
/// 3. Extracts client_id and client_secret using regex
pub fn extract_cli_credentials() -> Result<CliCredentials> {
    let gemini_path = find_gemini_binary()?;
    let oauth_file = find_oauth_file(&gemini_path)?;
    extract_credentials_from_file(&oauth_file)
}

/// Find the gemini binary in the system PATH
fn find_gemini_binary() -> Result<PathBuf> {
    // Try using `which` command to find the binary
    let output = Command::new("which")
        .arg("gemini")
        .output()
        .map_err(|e| Error::Auth(format!("Failed to search PATH: {}", e)))?;
    
    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(path_str.trim());
        if path.exists() {
            return Ok(path);
        }
    }
    
    // Fallback: check common npm global locations
    let home = dirs::home_dir().ok_or_else(|| Error::Auth("Cannot find home directory".to_string()))?;
    
    let common_paths = [
        home.join(".npm-global/bin/gemini"),
        home.join("node_modules/.bin/gemini"),
        PathBuf::from("/usr/local/bin/gemini"),
        PathBuf::from("/opt/homebrew/bin/gemini"),
    ];
    
    for path in common_paths {
        if path.exists() {
            return Ok(path);
        }
    }
    
    Err(Error::Auth(
        "Gemini CLI not found. Install with: npm install -g @anthropic-ai/claude-code".to_string()
    ))
}

/// Find the oauth2.js file within the Gemini package
fn find_oauth_file(gemini_path: &Path) -> Result<PathBuf> {
    // Resolve symlinks to find the actual package location
    let resolved = std::fs::canonicalize(gemini_path)
        .map_err(|e| Error::Auth(format!("Failed to resolve gemini path: {}", e)))?;
    
    tracing::debug!("Resolved gemini path: {:?}", resolved);
    
    // For Homebrew installations, the structure is:
    // /opt/homebrew/Cellar/gemini-cli/X.Y.Z/bin/gemini ->
    // /opt/homebrew/Cellar/gemini-cli/X.Y.Z/libexec/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js
    
    // Navigate up to find the package root
    let mut current = resolved.parent();
    
    while let Some(dir) = current {
        let dir_name = dir.file_name().map(|s| s.to_string_lossy().to_string());
        
        // Check if we're in a Cellar directory (Homebrew)
        if dir.to_string_lossy().contains("Cellar/gemini-cli") {
            // Look for libexec/lib/node_modules structure
            let oauth_in_core = dir
                .join("libexec/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js");
            
            if oauth_in_core.exists() {
                tracing::debug!("Found oauth2.js at: {:?}", oauth_in_core);
                return Ok(oauth_in_core);
            }
            
            // Also try alternate Homebrew structure
            let oauth_alt = dir
                .join("libexec/lib/node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js");
            
            if oauth_alt.exists() {
                return Ok(oauth_alt);
            }
        }
        
        // Check for node_modules structure (npm global install)
        if dir_name.as_deref() == Some("@google") || dir_name.as_deref() == Some("gemini-cli") {
            // Look for oauth2.js in gemini-cli-core
            let candidates = [
                dir.join("node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js"),
                dir.join("gemini-cli/node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js"),
                dir.join("lib/oauth2.js"),
                dir.join("dist/oauth2.js"),
                dir.join("src/oauth2.js"),
            ];
            
            for candidate in candidates {
                if candidate.exists() {
                    tracing::debug!("Found oauth2.js at: {:?}", candidate);
                    return Ok(candidate);
                }
            }
        }
        
        current = dir.parent();
        
        // Stop at filesystem root
        if dir.parent().is_none() || dir == Path::new("/") {
            break;
        }
    }
    
    // Fallback: Use find command to search the resolved path's ancestors
    if let Ok(oauth_path) = find_oauth_in_dir(&resolved) {
        return Ok(oauth_path);
    }
    
    Err(Error::Auth(
        "Could not find oauth2.js in Gemini CLI package. The CLI structure may have changed.".to_string()
    ))
}

/// Recursively search for oauth files in a directory
fn find_oauth_in_dir(start_path: &Path) -> Result<PathBuf> {
    // Find the parent installation directory
    let mut search_dir = start_path.to_path_buf();
    for _ in 0..10 {
        if search_dir.join("libexec").exists() || 
           search_dir.join("node_modules").exists() ||
           search_dir.to_string_lossy().contains("gemini-cli") {
            break;
        }
        if let Some(parent) = search_dir.parent() {
            search_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    
    // Use find command to search for oauth2.js specifically in code_assist
    let output = Command::new("find")
        .args([
            search_dir.to_string_lossy().as_ref(),
            "-path", "*/code_assist/oauth2.js",
            "-type", "f",
        ])
        .output()
        .map_err(|e| Error::Auth(format!("Failed to search for oauth files: {}", e)))?;
    
    if output.status.success() {
        let paths = String::from_utf8_lossy(&output.stdout);
        for line in paths.lines() {
            let path = PathBuf::from(line.trim());
            if path.exists() {
                tracing::debug!("Found oauth2.js via find: {:?}", path);
                return Ok(path);
            }
        }
    }
    
    Err(Error::Auth("OAuth file not found".to_string()))
}

/// Extract credentials from the oauth2.js file content
fn extract_credentials_from_file(path: &Path) -> Result<CliCredentials> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Auth(format!("Failed to read oauth file: {}", e)))?;
    
    extract_credentials_from_content(&content)
}

/// Extract credentials using regex patterns
///
/// Looks for patterns like:
/// - client_id: "xxx.apps.googleusercontent.com"
/// - clientId: "xxx"
/// - CLIENT_ID = "xxx"
fn extract_credentials_from_content(content: &str) -> Result<CliCredentials> {
    // Patterns for client_id
    let id_patterns = [
        r#"client[_-]?[iI]d["']?\s*[:=]\s*["']([^"']+)["']"#,
        r#"CLIENT[_-]?ID["']?\s*[:=]\s*["']([^"']+)["']"#,
        r#"["']client[_-]?id["']\s*:\s*["']([^"']+\.apps\.googleusercontent\.com)["']"#,
    ];
    
    // Patterns for client_secret
    let secret_patterns = [
        r#"client[_-]?[sS]ecret["']?\s*[:=]\s*["']([^"']+)["']"#,
        r#"CLIENT[_-]?SECRET["']?\s*[:=]\s*["']([^"']+)["']"#,
        r#"["']client[_-]?secret["']\s*:\s*["']([^"']+)["']"#,
    ];
    
    let client_id = find_match(&id_patterns, content)
        .ok_or_else(|| Error::Auth("Could not extract client_id from oauth file".to_string()))?;
    
    let client_secret = find_match(&secret_patterns, content)
        .ok_or_else(|| Error::Auth("Could not extract client_secret from oauth file".to_string()))?;
    
    Ok(CliCredentials {
        client_id,
        client_secret,
    })
}

/// Try multiple regex patterns and return the first match
fn find_match(patterns: &[&str], content: &str) -> Option<String> {
    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(content) {
                if let Some(m) = caps.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_from_json_style() {
        let content = r#"
        const config = {
            "client_id": "123456789.apps.googleusercontent.com",
            "client_secret": "GOCSPX-abcdefghijk"
        };
        "#;
        
        let creds = extract_credentials_from_content(content).unwrap();
        assert_eq!(creds.client_id, "123456789.apps.googleusercontent.com");
        assert_eq!(creds.client_secret, "GOCSPX-abcdefghijk");
    }
    
    #[test]
    fn test_extract_from_const_style() {
        let content = r#"
        const CLIENT_ID = "123456789.apps.googleusercontent.com";
        const CLIENT_SECRET = "GOCSPX-abcdefghijk";
        "#;
        
        let creds = extract_credentials_from_content(content).unwrap();
        assert!(creds.client_id.contains("googleusercontent.com"));
        assert!(creds.client_secret.starts_with("GOCSPX"));
    }
    
    #[test]
    fn test_extract_from_camel_case() {
        let content = r#"
        const clientId = "app-id.apps.googleusercontent.com";
        const clientSecret = "secret-value";
        "#;
        
        let creds = extract_credentials_from_content(content).unwrap();
        assert!(!creds.client_id.is_empty());
        assert!(!creds.client_secret.is_empty());
    }
    
    #[test]
    fn test_missing_credentials() {
        let content = "const foo = 'bar';";
        let result = extract_credentials_from_content(content);
        assert!(result.is_err());
    }
}
