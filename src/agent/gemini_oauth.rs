//! Gemini OAuth2 client implementation
//!
//! Uses OAuth2 tokens instead of API keys for authentication.
//! Uses Code Assist API (cloudcode-pa.googleapis.com) which is what Gemini CLI uses.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;
use crate::Result;
use crate::error::Error;
use crate::auth::GeminiAuthProvider;
use super::llm::{LlmClient, LlmResponse, Usage};
use super::message::{Message, Role, ToolCallRequest};
use crate::tools::ToolDefinition;

/// Code Assist API endpoint (same as Gemini CLI uses)
const CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";
const CODE_ASSIST_API_VERSION: &str = "v1internal";

/// Gemini API client using OAuth2 authentication via Code Assist API
#[derive(Clone)]
pub struct GeminiOAuthClient {
    auth_provider: GeminiAuthProvider,
    model: String,
    client: Client,
    project_id: Option<String>,
    session_id: String,
}

impl GeminiOAuthClient {
    /// Create a new OAuth-authenticated Gemini client
    ///
    /// Uses credentials extracted from the Gemini CLI.
    pub fn from_cli(model: &str) -> Result<Self> {
        let auth_provider = GeminiAuthProvider::from_cli()?;
        Ok(Self {
            auth_provider,
            model: model.to_string(),
            client: Client::new(),
            project_id: None, // Will be fetched via Code Assist API
            session_id: Uuid::new_v4().to_string(),
        })
    }
    
    /// Create with explicit OAuth credentials
    pub fn new(client_id: String, client_secret: String, model: &str) -> Self {
        Self {
            auth_provider: GeminiAuthProvider::new(client_id, client_secret),
            model: model.to_string(),
            client: Client::new(),
            project_id: None,
            session_id: Uuid::new_v4().to_string(),
        }
    }
    
    /// Build Code Assist API URL for a method
    fn build_code_assist_url(&self, method: &str) -> String {
        format!(
            "{}/{}:{}",
            CODE_ASSIST_ENDPOINT, CODE_ASSIST_API_VERSION, method
        )
    }
    
    /// Get project ID from cache or fetch via Code Assist API (like OpenClaw)
    async fn get_or_fetch_project_id(&self, access_token: &str) -> Result<String> {
        // Try to get from cached project_id
        if let Some(ref pid) = self.project_id {
            return Ok(pid.clone());
        }
        
        // Check environment variable first
        if let Ok(project) = std::env::var("GOOGLE_CLOUD_PROJECT") {
            if !project.is_empty() {
                tracing::debug!("Using GOOGLE_CLOUD_PROJECT: {}", project);
                return Ok(project);
            }
        }
        if let Ok(project) = std::env::var("GOOGLE_CLOUD_PROJECT_ID") {
            if !project.is_empty() {
                tracing::debug!("Using GOOGLE_CLOUD_PROJECT_ID: {}", project);
                return Ok(project);
            }
        }
        
        // Use Code Assist API to load/discover project (same as Gemini CLI and OpenClaw)
        let load_body = json!({
            "metadata": {
                "ideType": "IDE_UNSPECIFIED",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });
        
        let resp = self.client
            .post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist")
            .header("User-Agent", "google-api-nodejs-client/leo")
            .header("X-Goog-Api-Client", "gl-node/leo")
            .bearer_auth(access_token)
            .json(&load_body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            let data: Value = resp.json().await?;
            tracing::debug!("loadCodeAssist response: {:?}", data);
            
            // Check if we have currentTier (means already onboarded)
            if data.get("currentTier").is_some() {
                // Extract project from cloudaicompanionProject
                if let Some(project) = data.get("cloudaicompanionProject") {
                    if let Some(pid) = project.as_str() {
                        tracing::debug!("Using Code Assist project: {}", pid);
                        return Ok(pid.to_string());
                    }
                    if let Some(obj) = project.as_object() {
                        if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                            tracing::debug!("Using Code Assist project: {}", id);
                            return Ok(id.to_string());
                        }
                    }
                }
            }
            
            // Need to onboard - use free tier
            return self.onboard_user(access_token).await;
        }
        
        // If Code Assist API fails, try onboarding anyway
        tracing::warn!("loadCodeAssist failed, attempting onboard");
        self.onboard_user(access_token).await
    }
    
    /// Onboard user to Code Assist (creates a project if needed)
    async fn onboard_user(&self, access_token: &str) -> Result<String> {
        let onboard_body = json!({
            "tierId": "free-tier",
            "metadata": {
                "ideType": "IDE_UNSPECIFIED",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });
        
        let resp = self.client
            .post("https://cloudcode-pa.googleapis.com/v1internal:onboardUser")
            .header("User-Agent", "google-api-nodejs-client/leo")
            .header("X-Goog-Api-Client", "gl-node/leo")
            .bearer_auth(access_token)
            .json(&onboard_body)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(Error::Auth(format!("Failed to onboard: {}", error)));
        }
        
        let data: Value = resp.json().await?;
        tracing::debug!("onboardUser response: {:?}", data);
        
        // Check for long-running operation
        if let Some(name) = data.get("name").and_then(|n| n.as_str()) {
            if !data.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                return self.poll_operation(name, access_token).await;
            }
        }
        
        // Extract project from response
        if let Some(response) = data.get("response") {
            if let Some(project) = response.get("cloudaicompanionProject") {
                if let Some(obj) = project.as_object() {
                    if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                        return Ok(id.to_string());
                    }
                }
            }
        }
        
        Err(Error::Auth(
            "Could not provision project. Set GOOGLE_CLOUD_PROJECT or GOOGLE_CLOUD_PROJECT_ID.".to_string()
        ))
    }
    
    /// Poll a long-running operation until complete
    async fn poll_operation(&self, operation_name: &str, access_token: &str) -> Result<String> {
        for _ in 0..24 {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            let url = format!("https://cloudcode-pa.googleapis.com/v1internal/{}", operation_name);
            let resp = self.client
                .get(&url)
                .bearer_auth(access_token)
                .send()
                .await?;
            
            if !resp.status().is_success() {
                continue;
            }
            
            let data: Value = resp.json().await?;
            
            if data.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                if let Some(response) = data.get("response") {
                    if let Some(project) = response.get("cloudaicompanionProject") {
                        if let Some(obj) = project.as_object() {
                            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                                return Ok(id.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        Err(Error::Auth("Operation polling timeout".to_string()))
    }
    
    fn convert_messages(&self, messages: &[Message]) -> Vec<Value> {
        messages.iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "model",
                    Role::Tool => "function",
                    Role::System => "user", // Should be filtered
                };
                
                if m.role == Role::Tool {
                    json!({
                        "role": "function",
                        "parts": [{
                            "functionResponse": {
                                "name": m.tool_call_id.as_deref().unwrap_or("unknown"),
                                "response": {"result": m.content}
                            }
                        }]
                    })
                } else if let Some(ref tool_calls) = m.tool_calls {
                    let calls: Vec<Value> = tool_calls.iter().map(|tc| {
                        json!({
                            "functionCall": {
                                "name": tc.name,
                                "args": tc.arguments
                            }
                        })
                    }).collect();
                    
                    json!({
                        "role": role,
                        "parts": calls
                    })
                } else {
                    json!({
                        "role": role,
                        "parts": [{"text": m.content}]
                    })
                }
            })
            .collect()
    }
    
    fn get_system_instruction(&self, messages: &[Message]) -> Option<String> {
        messages.iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone())
    }
    
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Option<Value> {
        if tools.is_empty() {
            return None;
        }
        
        let function_declarations: Vec<Value> = tools.iter().map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters
            })
        }).collect();
        
        Some(json!([{
            "functionDeclarations": function_declarations
        }]))
    }
    
    fn parse_response(&self, response: &GeminiResponse) -> Result<LlmResponse> {
        let candidate = response.candidates.first()
            .ok_or_else(|| Error::Llm("No candidates in response".to_string()))?;
        
        let mut content = None;
        let mut tool_calls = Vec::new();
        
        for part in &candidate.content.parts {
            if let Some(ref text) = part.text {
                content = Some(text.clone());
            }
            
            if let Some(ref fc) = part.function_call {
                tool_calls.push(ToolCallRequest {
                    id: format!("tc_{}", tool_calls.len()),
                    name: fc.name.clone(),
                    arguments: fc.args.clone(),
                });
            }
        }
        
        let usage = response.usage_metadata.as_ref()
            .map(|u| Usage {
                prompt_tokens: u.prompt_token_count.unwrap_or(0),
                completion_tokens: u.candidates_token_count.unwrap_or(0),
                total_tokens: u.total_token_count.unwrap_or(0),
            })
            .unwrap_or_default();
        
        Ok(LlmResponse {
            content,
            tool_calls,
            finish_reason: candidate.finish_reason.clone().unwrap_or_else(|| "stop".to_string()),
            usage,
        })
    }
}

#[async_trait]
impl LlmClient for GeminiOAuthClient {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        // Get valid access token (may trigger OAuth flow)
        let access_token = self.auth_provider.get_valid_token().await?;
        
        // Get project ID via Code Assist API
        let project_id = self.get_or_fetch_project_id(&access_token).await?;
        
        let contents = self.convert_messages(messages);
        let system_instruction = self.get_system_instruction(messages);
        
        // Build the inner request (Vertex format)
        let mut inner_request = json!({
            "contents": contents,
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 8192
            },
            "session_id": self.session_id
        });
        
        if let Some(system) = system_instruction {
            inner_request["systemInstruction"] = json!({
                "parts": [{"text": system}]
            });
        }
        
        if let Some(tool_config) = self.convert_tools(tools) {
            inner_request["tools"] = tool_config;
        }
        
        // Generate a user prompt ID for tracking
        let user_prompt_id = Uuid::new_v4().to_string();
        
        // Build Code Assist API request (wraps the inner request)
        let code_assist_request = json!({
            "model": self.model,
            "project": project_id,
            "user_prompt_id": user_prompt_id,
            "request": inner_request
        });
        
        // Use Code Assist API endpoint (same as Gemini CLI)
        let url = self.build_code_assist_url("generateContent");
        
        let mut retry_count = 0;
        let max_retries = 5;
        let mut backoff = std::time::Duration::from_secs(1);
        
        loop {
            let response = self.client
                .post(&url)
                .header("User-Agent", "google-api-nodejs-client/leo")
                .header("X-Goog-Api-Client", "gl-node/leo")
                .bearer_auth(&access_token)
                .json(&code_assist_request)
                .send()
                .await?;
            
            if response.status().is_success() {
                // Code Assist API returns { response: { ... standard response ... } }
                let code_assist_response: Value = response.json().await?;
                
                // Extract the nested response
                let inner_response = code_assist_response.get("response")
                    .ok_or_else(|| Error::Llm("Missing 'response' field in Code Assist response".to_string()))?;
                
                // Parse as standard Gemini response
                let gemini_response: GeminiResponse = serde_json::from_value(inner_response.clone())
                    .map_err(|e| Error::Llm(format!("Failed to parse response: {}", e)))?;
                
                return self.parse_response(&gemini_response);
            }
            
            let status = response.status();
            let error_text = response.text().await?;
            
            // Check for rate limit (429) or resource exhausted
            if status.as_u16() == 429 || error_text.contains("RESOURCE_EXHAUSTED") {
                if retry_count < max_retries {
                    retry_count += 1;
                    println!("⚠️ Rate limit exceeded (429). Retrying in {:?} (Ensure quota is sufficient)...", backoff);
                    
                    tokio::time::sleep(backoff).await;
                    
                    // Exponential backoff with jitter
                    let jitter = rand::random::<f64>() * 0.5 + 0.5; // 0.5x to 1.0x
                    let next_backoff = backoff.mul_f64(2.0 * jitter);
                    backoff = next_backoff.min(std::time::Duration::from_secs(60));
                    continue;
                }
            }
            
            println!("❌ Code Assist API error ({}): {}", status, error_text);
            
            // If unauthorized, token might have expired
            if status.as_u16() == 401 {
                return Err(Error::Auth(format!("Authentication failed: {}", error_text)));
            }
            
            return Err(Error::Llm(format!("Code Assist API error ({}): {}", status, error_text)));
        }
    }
    
    fn default_model(&self) -> &str {
        &self.model
    }
}

// Gemini API response types (shared with gemini.rs)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Content,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Part {
    text: Option<String>,
    function_call: Option<FunctionCall>,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    args: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: Option<usize>,
    candidates_token_count: Option<usize>,
    total_token_count: Option<usize>,
}
