//! Credential storage and management
//!
//! Handles saving and loading OAuth2 tokens from ~/.leo/credentials.json

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::Result;

/// OAuth2 credentials with access and refresh tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    /// The access token for API requests
    pub access_token: String,
    
    /// The refresh token for obtaining new access tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    
    /// Token type (usually "Bearer")
    #[serde(default = "default_token_type")]
    pub token_type: String,
    
    /// When the access token expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    
    /// Scopes granted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl Credentials {
    /// Create new credentials from token response
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_in_secs: Option<i64>,
    ) -> Self {
        let expires_at = expires_in_secs.map(|secs| {
            Utc::now() + chrono::Duration::seconds(secs)
        });
        
        Self {
            access_token,
            refresh_token,
            token_type: default_token_type(),
            expires_at,
            scope: None,
        }
    }
    
    /// Check if the access token is expired or about to expire
    ///
    /// Returns true if the token expires within the next 5 minutes
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => {
                let buffer = chrono::Duration::minutes(5);
                Utc::now() + buffer >= expires
            }
            None => false, // No expiry means token doesn't expire
        }
    }
    
    /// Check if we have a refresh token
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }
}

/// Get the credentials file path
pub fn credentials_path() -> PathBuf {
    crate::config::config_dir().join("credentials.json")
}

/// Load credentials from file
pub fn load_credentials() -> Result<Option<Credentials>> {
    let path = credentials_path();
    
    if !path.exists() {
        return Ok(None);
    }
    
    let content = std::fs::read_to_string(&path)?;
    let creds: Credentials = serde_json::from_str(&content)?;
    Ok(Some(creds))
}

/// Save credentials to file
pub fn save_credentials(credentials: &Credentials) -> Result<()> {
    let path = credentials_path();
    
    // Create parent directory
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let content = serde_json::to_string_pretty(credentials)?;
    std::fs::write(&path, content)?;
    
    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    
    Ok(())
}

/// Delete stored credentials
pub fn delete_credentials() -> Result<()> {
    let path = credentials_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_credentials_not_expired() {
        let creds = Credentials::new(
            "test_token".to_string(),
            Some("refresh".to_string()),
            Some(3600), // 1 hour
        );
        assert!(!creds.is_expired());
    }
    
    #[test]
    fn test_credentials_expired() {
        let mut creds = Credentials::new(
            "test_token".to_string(),
            Some("refresh".to_string()),
            Some(0), // Already expired
        );
        // Force expiry in the past
        creds.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(creds.is_expired());
    }
    
    #[test]
    fn test_credentials_expiring_soon() {
        let creds = Credentials::new(
            "test_token".to_string(),
            Some("refresh".to_string()),
            Some(120), // 2 minutes (within 5 minute buffer)
        );
        assert!(creds.is_expired()); // Should be considered expired due to buffer
    }
    
    #[test]
    fn test_credentials_no_expiry() {
        let creds = Credentials {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_at: None,
            scope: None,
        };
        assert!(!creds.is_expired());
    }
    
    #[test]
    fn test_credentials_serialization() {
        let creds = Credentials::new(
            "access".to_string(),
            Some("refresh".to_string()),
            Some(3600),
        );
        
        let json = serde_json::to_string(&creds).unwrap();
        let parsed: Credentials = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.access_token, creds.access_token);
        assert_eq!(parsed.refresh_token, creds.refresh_token);
    }
}
