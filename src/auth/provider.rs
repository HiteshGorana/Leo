//! GeminiAuthProvider - OAuth2 authentication provider for Gemini API
//!
//! Manages the complete OAuth2 PKCE flow:
//! - Token caching and validation
//! - Browser-based authorization
//! - Token exchange and refresh

use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;
use crate::Result;
use crate::error::Error;
use super::pkce::PkcePair;
use super::credentials::{Credentials, load_credentials, save_credentials};
use super::callback_server::{wait_for_callback, get_redirect_uri};
use super::cli_extractor::{extract_cli_credentials, CliCredentials};

/// Google OAuth2 endpoints
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// OAuth2 scopes required for Gemini API
/// These must match the scopes registered for the Gemini CLI's client ID
const GEMINI_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

/// Google OAuth2 token response
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    token_type: String,
    #[serde(default)]
    scope: Option<String>,
}

/// Token refresh request
#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    refresh_token: &'a str,
    grant_type: &'a str,
}

/// Token exchange request
#[derive(Debug, Serialize)]
struct TokenExchangeRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    code: &'a str,
    code_verifier: &'a str,
    redirect_uri: &'a str,
    grant_type: &'a str,
}

/// Gemini OAuth2 authentication provider
///
/// Handles the complete OAuth2 PKCE flow for authenticating with Google's Gemini API.
#[derive(Clone)]
pub struct GeminiAuthProvider {
    client_credentials: CliCredentials,
    http_client: Client,
}

impl GeminiAuthProvider {
    /// Create a new auth provider using CLI-extracted credentials
    pub fn from_cli() -> Result<Self> {
        let client_credentials = extract_cli_credentials()?;
        Ok(Self {
            client_credentials,
            http_client: Client::new(),
        })
    }
    
    /// Create with explicit credentials
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_credentials: CliCredentials {
                client_id,
                client_secret,
            },
            http_client: Client::new(),
        }
    }
    
    /// Get a valid access token, refreshing or re-authenticating as needed
    pub async fn get_valid_token(&self) -> Result<String> {
        // Try to load existing credentials
        if let Some(creds) = load_credentials()? {
            if !creds.is_expired() {
                tracing::debug!("Using cached access token");
                return Ok(creds.access_token);
            }
            
            // Try to refresh
            if creds.can_refresh() {
                tracing::info!("Access token expired, refreshing...");
                match self.refresh_token(creds.refresh_token.as_ref().unwrap()).await {
                    Ok(new_creds) => {
                        save_credentials(&new_creds)?;
                        return Ok(new_creds.access_token);
                    }
                    Err(e) => {
                        tracing::warn!("Token refresh failed: {}, re-authenticating", e);
                    }
                }
            }
        }
        
        // Need to authenticate
        tracing::info!("No valid token found, starting OAuth2 flow");
        let creds = self.authorize().await?;
        save_credentials(&creds)?;
        Ok(creds.access_token)
    }
    
    /// Start the OAuth2 authorization flow
    pub async fn authorize(&self) -> Result<Credentials> {
        // Generate PKCE pair
        let pkce = PkcePair::new();
        
        // Generate state for CSRF protection
        let state = generate_state();
        
        // Build authorization URL
        let auth_url = self.build_auth_url(&pkce.challenge, &state)?;
        
        println!("\n🔐 Opening browser for Google authentication...\n");
        println!("If the browser doesn't open, visit this URL:\n{}\n", auth_url);
        
        // Open browser
        if let Err(e) = open::that(&auth_url) {
            tracing::warn!("Failed to open browser: {}", e);
        }
        
        // Wait for callback
        println!("⏳ Waiting for authorization...");
        let auth_result = wait_for_callback(Some(&state)).await?;
        
        println!("✓ Authorization received, exchanging token...\n");
        
        // Exchange code for tokens
        self.exchange_code(&auth_result.code, &pkce.verifier).await
    }
    
    /// Build the authorization URL
    fn build_auth_url(&self, code_challenge: &str, state: &str) -> Result<String> {
        let mut url = Url::parse(GOOGLE_AUTH_URL)
            .map_err(|e| Error::OAuth(format!("Invalid auth URL: {}", e)))?;
        
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_credentials.client_id)
            .append_pair("redirect_uri", &get_redirect_uri())
            .append_pair("response_type", "code")
            .append_pair("scope", &GEMINI_SCOPES.join(" "))
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", state)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");
        
        Ok(url.to_string())
    }
    
    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str, code_verifier: &str) -> Result<Credentials> {
        let request = TokenExchangeRequest {
            client_id: &self.client_credentials.client_id,
            client_secret: &self.client_credentials.client_secret,
            code,
            code_verifier,
            redirect_uri: &get_redirect_uri(),
            grant_type: "authorization_code",
        };
        
        let response = self.http_client
            .post(GOOGLE_TOKEN_URL)
            .form(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(Error::OAuth(format!("Token exchange failed: {}", error_text)));
        }
        
        let token_response: TokenResponse = response.json().await?;
        
        Ok(Credentials::new(
            token_response.access_token,
            token_response.refresh_token,
            token_response.expires_in,
        ))
    }
    
    /// Refresh an expired access token
    async fn refresh_token(&self, refresh_token: &str) -> Result<Credentials> {
        let request = RefreshRequest {
            client_id: &self.client_credentials.client_id,
            client_secret: &self.client_credentials.client_secret,
            refresh_token,
            grant_type: "refresh_token",
        };
        
        let response = self.http_client
            .post(GOOGLE_TOKEN_URL)
            .form(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(Error::OAuth(format!("Token refresh failed: {}", error_text)));
        }
        
        let token_response: TokenResponse = response.json().await?;
        
        // Preserve the refresh token if not returned in response
        let refresh = token_response.refresh_token
            .or_else(|| Some(refresh_token.to_string()));
        
        Ok(Credentials::new(
            token_response.access_token,
            refresh,
            token_response.expires_in,
        ))
    }
    
    /// Check if we have valid cached credentials
    pub fn has_valid_credentials() -> Result<bool> {
        match load_credentials()? {
            Some(creds) => Ok(!creds.is_expired() || creds.can_refresh()),
            None => Ok(false),
        }
    }
}

/// Generate a random state string for CSRF protection
fn generate_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_state() {
        let state = generate_state();
        assert_eq!(state.len(), 32);
        
        // Should only contain alphanumeric
        for c in state.chars() {
            assert!(c.is_ascii_alphanumeric());
        }
    }
    
    #[test]
    fn test_states_are_unique() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2);
    }
}
