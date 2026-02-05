//! Authentication module for OAuth2 and credential management
//!
//! This module provides:
//! - PKCE code generation for OAuth2 flows
//! - Credential storage and retrieval
//! - CLI credential extraction from Gemini CLI binary
//! - OAuth2 callback server
//! - GeminiAuthProvider for managing OAuth2 authentication

mod pkce;
mod credentials;
mod cli_extractor;
mod callback_server;
mod provider;

pub use credentials::{Credentials, load_credentials, save_credentials, delete_credentials};
pub use cli_extractor::{extract_cli_credentials, CliCredentials};
pub use provider::GeminiAuthProvider;
