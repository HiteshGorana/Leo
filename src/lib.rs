//! Leo - Ultra-lightweight personal AI assistant
//!
//! This library provides the core functionality for building AI agents
//! with tool execution, memory, and skills support.

pub mod agent;
pub mod memory;
pub mod skills;
pub mod templates;
pub mod tools;
pub mod adapters;
pub mod auth;
pub mod config;
pub mod error;
pub mod ui;

pub use error::{Error, Result};
