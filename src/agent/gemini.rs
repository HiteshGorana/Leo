//! Gemini LLM client implementation

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::Result;
use crate::error::Error;
use super::llm::{LlmClient, LlmResponse, Usage};
use super::message::{Message, Role, ToolCallRequest};
use crate::tools::ToolDefinition;
use crate::ui;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Gemini API client
#[derive(Clone)]
pub struct GeminiClient {
    api_key: String,
    model: String,
    client: Client,
}

impl GeminiClient {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: Client::new(),
        }
    }
    
    fn build_url(&self) -> String {
        format!(
            "{}{}:generateContent?key={}",
            GEMINI_API_URL, self.model, self.api_key
        )
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
impl LlmClient for GeminiClient {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        let contents = self.convert_messages(messages);
        let system_instruction = self.get_system_instruction(messages);
        
        let mut request = json!({
            "contents": contents,
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 8192
            }
        });
        
        if let Some(system) = system_instruction {
            request["systemInstruction"] = json!({
                "parts": [{"text": system}]
            });
        }
        
        if let Some(tool_config) = self.convert_tools(tools) {
            request["tools"] = tool_config;
        }
        
        ui::print_api_call(&self.model);
        
        let response = self.client
            .post(&self.build_url())
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(Error::Llm(format!("Gemini API error: {}", error_text)));
        }
        
        let gemini_response: GeminiResponse = response.json().await?;
        self.parse_response(&gemini_response)
    }
    
    fn default_model(&self) -> &str {
        &self.model
    }
}

// Gemini API response types
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
