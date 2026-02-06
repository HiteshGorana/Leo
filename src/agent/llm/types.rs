//! Shared types for Gemini API responses.
//!
//! These types are shared between `GeminiClient` (API key auth) and
//! `GeminiOAuthClient` (OAuth auth) to avoid duplication.

use serde::Deserialize;
use serde_json::Value;

/// Top-level Gemini API response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

/// A single response candidate.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<String>,
}

/// Content block containing parts.
#[derive(Debug, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
}

/// A single part of the response (text or function call).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub text: Option<String>,
    pub function_call: Option<FunctionCall>,
}

/// Function call requested by the model.
#[derive(Debug, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: Value,
}

/// Token usage metadata.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: Option<usize>,
    pub candidates_token_count: Option<usize>,
    pub total_token_count: Option<usize>,
}
