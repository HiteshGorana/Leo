//! OAuth2 callback server
//!
//! A temporary local HTTP server that captures the OAuth2 authorization code
//! from the browser redirect.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;
use crate::Result;
use crate::error::Error;

/// Default callback port
pub const CALLBACK_PORT: u16 = 8085;

/// Success HTML page shown after authorization
const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorization Successful</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }
        .container {
            text-align: center;
            background: white;
            padding: 3rem;
            border-radius: 1rem;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
        }
        .checkmark { font-size: 4rem; }
        h1 { color: #333; margin: 1rem 0; }
        p { color: #666; }
    </style>
</head>
<body>
    <div class="container">
        <div class="checkmark">✓</div>
        <h1>Authorization Successful!</h1>
        <p>You can close this window and return to the terminal.</p>
    </div>
</body>
</html>"#;

/// Error HTML page
const ERROR_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorization Failed</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #ff6b6b 0%, #ee5a5a 100%);
        }
        .container {
            text-align: center;
            background: white;
            padding: 3rem;
            border-radius: 1rem;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
        }
        .error { font-size: 4rem; }
        h1 { color: #333; margin: 1rem 0; }
        p { color: #666; }
    </style>
</head>
<body>
    <div class="container">
        <div class="error">✗</div>
        <h1>Authorization Failed</h1>
        <p>Please try again or check the terminal for details.</p>
    </div>
</body>
</html>"#;

/// Authorization code result from the callback
#[derive(Debug, Clone)]
pub struct AuthorizationResult {
    pub code: String,
    #[allow(dead_code)]
    pub state: Option<String>,
}

/// Start a temporary callback server and wait for the authorization code
///
/// Returns the authorization code received from the OAuth2 callback.
pub async fn wait_for_callback(expected_state: Option<&str>) -> Result<AuthorizationResult> {
    let addr = format!("127.0.0.1:{}", CALLBACK_PORT);
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| Error::OAuth(format!("Failed to start callback server on {}: {}", addr, e)))?;
    
    tracing::info!("Callback server listening on http://{}", addr);
    
    let expected_state = expected_state.map(|s| s.to_string());
    
    // Accept one connection
    let (mut socket, _) = listener.accept().await
        .map_err(|e| Error::OAuth(format!("Failed to accept connection: {}", e)))?;
    
    let mut buffer = vec![0u8; 4096];
    let n = socket.read(&mut buffer).await
        .map_err(|e| Error::OAuth(format!("Failed to read request: {}", e)))?;
    
    let request = String::from_utf8_lossy(&buffer[..n]);
    
    // Parse the request to extract the authorization code
    let result = parse_callback_request(&request, expected_state.as_deref());
    
    // Send response
    let (status, body) = match &result {
        Ok(_) => ("200 OK", SUCCESS_HTML),
        Err(_) => ("400 Bad Request", ERROR_HTML),
    };
    
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
    
    result
}

/// Parse the callback request to extract authorization code
fn parse_callback_request(request: &str, expected_state: Option<&str>) -> Result<AuthorizationResult> {
    // Extract the request line
    let first_line = request.lines().next()
        .ok_or_else(|| Error::OAuth("Empty request".to_string()))?;
    
    // Parse: GET /callback?code=xxx&state=yyy HTTP/1.1
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(Error::OAuth("Invalid request format".to_string()));
    }
    
    let path = parts[1];
    
    // Parse URL to extract query parameters
    let full_url = format!("http://localhost{}", path);
    let url = Url::parse(&full_url)
        .map_err(|e| Error::OAuth(format!("Failed to parse callback URL: {}", e)))?;
    
    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;
    
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => error = Some(value.to_string()),
            "error_description" => error_description = Some(value.to_string()),
            _ => {}
        }
    }
    
    // Check for errors
    if let Some(err) = error {
        let description = error_description.unwrap_or_else(|| "Unknown error".to_string());
        return Err(Error::OAuth(format!("Authorization failed: {} - {}", err, description)));
    }
    
    // Validate state if expected
    if let Some(expected) = expected_state {
        match &state {
            Some(s) if s == expected => {}
            Some(s) => return Err(Error::OAuth(format!("State mismatch: expected {}, got {}", expected, s))),
            None => return Err(Error::OAuth("Missing state parameter".to_string())),
        }
    }
    
    // Extract code
    let code = code.ok_or_else(|| Error::OAuth("Missing authorization code".to_string()))?;
    
    Ok(AuthorizationResult { code, state })
}

/// Get the callback redirect URI
pub fn get_redirect_uri() -> String {
    format!("http://127.0.0.1:{}/callback", CALLBACK_PORT)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_callback_success() {
        let request = "GET /callback?code=abc123&state=xyz789 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let result = parse_callback_request(request, Some("xyz789")).unwrap();
        
        assert_eq!(result.code, "abc123");
        assert_eq!(result.state.as_deref(), Some("xyz789"));
    }
    
    #[test]
    fn test_parse_callback_without_state() {
        let request = "GET /callback?code=abc123 HTTP/1.1\r\n\r\n";
        let result = parse_callback_request(request, None).unwrap();
        
        assert_eq!(result.code, "abc123");
        assert!(result.state.is_none());
    }
    
    #[test]
    fn test_parse_callback_error() {
        let request = "GET /callback?error=access_denied&error_description=User+denied HTTP/1.1\r\n\r\n";
        let result = parse_callback_request(request, None);
        
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("access_denied"));
    }
    
    #[test]
    fn test_parse_callback_state_mismatch() {
        let request = "GET /callback?code=abc&state=wrong HTTP/1.1\r\n\r\n";
        let result = parse_callback_request(request, Some("expected"));
        
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("mismatch"));
    }
    
    #[test]
    fn test_redirect_uri() {
        let uri = get_redirect_uri();
        assert_eq!(uri, "http://127.0.0.1:8085/callback");
    }
}
